/// Background processing pipeline: indexing, thumbnail generation, and AI analysis.
///
/// Architecture:
/// - `start_processing_worker` — main event loop consuming index/analysis jobs from mpsc channels
/// - `index_image` — inserts new images into DB, creates thumbnails, computes dominant color
/// - `run_ai_analysis` — sends images to AI engine for tag/vibe extraction
///
/// Uses a two-stage semaphore (indexing:analysis) that dynamically shifts capacity
/// from 8:2 to 2:8 as indexing nears completion.
use crate::ai::{AIEngine, AISettings};
use crate::exif;
use tauri::Manager;
use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::mpsc;
use uuid::Uuid;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct IndexingJob {
    pub path: PathBuf,
}

pub struct AnalysisJob {
    pub id: String,
    pub path: PathBuf,
    pub file_name: String,
}

pub struct ProcessingState {
    pub stop_indexing: AtomicBool,
    pub stop_analysis: AtomicBool,
    pub total_found: AtomicUsize,
    pub indexed_count: AtomicUsize,
    pub shifted_to_analysis: AtomicBool,
    pub notify_indexing: tokio::sync::Notify,
    pub notify_analysis: tokio::sync::Notify,
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self {
            stop_indexing: AtomicBool::new(false),
            stop_analysis: AtomicBool::new(false),
            total_found: AtomicUsize::new(0),
            indexed_count: AtomicUsize::new(0),
            shifted_to_analysis: AtomicBool::new(false),
            notify_indexing: tokio::sync::Notify::new(),
            notify_analysis: tokio::sync::Notify::new(),
        }
    }
}

pub async fn start_processing_worker(
    mut rx: mpsc::Receiver<IndexingJob>,
    pool: Pool<Sqlite>,
    app_data_dir: PathBuf,
    ai_engine: Arc<AIEngine>,
    app_handle: tauri::AppHandle,
    state: Arc<ProcessingState>,
    ai_settings: Arc<RwLock<AISettings>>,
) {
    let thumbs_dir = app_data_dir.join("thumbnails");
    let display_dir = app_data_dir.join("display");

    // Ensure cache dirs exist
    std::fs::create_dir_all(&thumbs_dir).ok();
    std::fs::create_dir_all(&display_dir).ok();

    let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Reserve at least 1 CPU for UI/OS responsiveness. On single-core, no reservation.
    let reserved = if num_cpus > 1 { 1 } else { 0 };
    let effective_cpus = num_cpus - reserved;

    // Stage 1: Indexing Parallelism — use all but reserved CPUs for indexing
    let indexing_parallelism = effective_cpus;
    
    // Stage 2: AI Analysis Parallelism — conservative start
    let analysis_parallelism = std::cmp::max(2, effective_cpus / 4);

    println!("Scaling pipeline: Indexing ({}) | AI Analysis ({}) on {} CPUs ({} reserved for UI)", indexing_parallelism, analysis_parallelism, num_cpus, reserved);
    
    let (analysis_tx, mut analysis_rx) = mpsc::channel::<AnalysisJob>(200);
    let indexing_semaphore = Arc::new(tokio::sync::Semaphore::new(indexing_parallelism));
    let analysis_semaphore = Arc::new(tokio::sync::Semaphore::new(analysis_parallelism));
    let analysis_settings = ai_settings.clone();

    // Spawn Analysis Worker Loop
    let analysis_pool = pool.clone();
    let analysis_ai = ai_engine.clone();
    let analysis_handle = app_handle.clone();
    let analysis_sem = analysis_semaphore.clone();
    let analysis_state = state.clone();
    
    tokio::spawn(async move {
        // Gate: Wait until 70% of indexing is complete before starting analysis.
        // Uses Notify instead of busy-polling — the indexing worker notifies
        // when it crosses thresholds so we wake up immediately.
        loop {
            if analysis_state.stop_analysis.load(Ordering::Relaxed) {
                analysis_state.notify_analysis.notified().await;
                continue;
            }

            let total = analysis_state.total_found.load(Ordering::Relaxed);
            let indexed = analysis_state.indexed_count.load(Ordering::Relaxed);

            if total > 5 && (indexed as f32 / total as f32) < 0.7 {
                // Wait for notification from indexing worker instead of polling
                analysis_state.notify_analysis.notified().await;
                continue;
            }
            break;
        }

        println!("[Analysis] Indexing past 70%, starting analysis worker");

        let mut analysis_count: u64 = 0;
        while let Some(job) = analysis_rx.recv().await {
            let pool = analysis_pool.clone();
            let ai = analysis_ai.clone();
            let handle = analysis_handle.clone();
            let permit = analysis_sem.clone().acquire_owned().await.unwrap();
            let settings = analysis_settings.clone();

            analysis_count += 1;
            // Rate-limited analysis progress — every 100 images
            if analysis_count % 100 == 0 {
                println!("[Analysis] Analyzed {} images", analysis_count);
            }

            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = run_ai_analysis(job, &pool, &ai, &handle, &settings).await {
                    eprintln!("AI Analysis failed: {}", e);
                }
            });
        }
    });

    // Main Indexing Worker Loop
    while let Some(job) = rx.recv().await {
        if state.stop_indexing.load(Ordering::Relaxed) {
            // Block via Notify instead of busy-polling — zero CPU while paused
            while state.stop_indexing.load(Ordering::Relaxed) {
                state.notify_indexing.notified().await;
            }
        }

        let pool = pool.clone();
        let thumbs = thumbs_dir.clone();
        let display = display_dir.clone();
        let handle = app_handle.clone();
        let a_tx = analysis_tx.clone();
        let permit = indexing_semaphore.clone().acquire_owned().await.unwrap();
        let index_state = state.clone();

        // --- DYNAMIC SCALING SHIFT ---
        // If >90% complete, shift from index-heavy (8:2) to analysis-heavy (2:8)
        let total = state.total_found.load(Ordering::Relaxed);
        let indexed = state.indexed_count.load(Ordering::Relaxed);
        if total > 10 && (indexed as f32 / total as f32) >= 0.9 && !state.shifted_to_analysis.load(Ordering::Relaxed) {
            if let Ok(false) = state.shifted_to_analysis.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
                let shift = std::cmp::max(2, num_cpus * 3 / 4);
                println!("[Processor] 90% Threshold: Shifting resources Index({num_cpus}->{}) | Analysis({}->{num_cpus})", num_cpus - shift, num_cpus / 4);
                
                // 1. Drain indexing permits to reduce to min(2, num_cpus/4)
                let idx_sem = indexing_semaphore.clone();
                tokio::spawn(async move {
                    if let Ok(permits) = idx_sem.acquire_many_owned(shift as u32).await {
                        std::mem::forget(permits);
                    }
                });

                // 2. Boost analysis by the same amount
                analysis_semaphore.add_permits(shift);
            }
        }
        // -----------------------------

        tokio::spawn(async move {
            let _permit = permit;
            let timeout_duration = std::time::Duration::from_secs(30);
            
            let result = tokio::time::timeout(timeout_duration, index_image(job.path.clone(), &pool, &thumbs, &display, &handle)).await;
            
            // Increment counter for all outcomes and get the new count for notification decisions
            let counted = index_state.indexed_count.fetch_add(1, Ordering::Relaxed) + 1;
            
            match result {
                Ok(Ok(Some(analysis_job))) => {
                    // Backpressure: await send so analysis jobs are never silently dropped
                    if a_tx.send(analysis_job).await.is_err() {
                        eprintln!("[Processor] Analysis channel closed");
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => {
                    eprintln!("Failed to index image: {}", e);
                }
                Err(_) => {
                    eprintln!("[Processor] Indexing timed out (30s) for {:?}. Skipping.", job.path);
                    // Emit a specific event for the UI
                    use tauri::Emitter;
                    let _ = handle.emit("indexing-status", serde_json::json!({
                        "status": "skipped",
                        "message": "Timed out (30s)",
                        "path": job.path.to_string_lossy()
                    }));
                }
            }
            
            // Wake analysis gate periodically (first 5 + every 100 images) to re-check 70% threshold.
            // Without this, analysis would never start for large libraries since the gate only
            // gets woken during the first few images.
            if counted <= 5 || counted % 100 == 0 {
                index_state.notify_analysis.notify_waiters();
            }
            
            // Rate-limited progress log — every 100 images
            let cur = counted;
            let tot = index_state.total_found.load(Ordering::Relaxed);
            if cur % 100 == 0 {
                let pct = if tot > 0 { (cur as f32 / tot as f32 * 100.0) as i32 } else { 0 };
                println!("[Processor] Progress: {}/{} ({}%)", cur, tot, pct);
            }
        });
    }
}

/// Re-queues images with garbled or missing AI tags for re-analysis.
#[tauri::command]
pub async fn trigger_junk_reanalysis(
    pool: tauri::State<'_, Pool<Sqlite>>,
    indexing_tx: tauri::State<'_, mpsc::Sender<IndexingJob>>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<ProcessingState>>,
    ai_engine: tauri::State<'_, Arc<AIEngine>>,
    ai_settings: tauri::State<'_, Arc<RwLock<AISettings>>>,
) -> Result<usize, String> {
    internal_trigger_junk_reanalysis(&*pool, &*indexing_tx, &app_handle, state.inner().clone(), &*ai_engine, &*ai_settings).await
}

pub async fn internal_trigger_junk_reanalysis(
    pool: &Pool<Sqlite>,
    _indexing_tx: &mpsc::Sender<IndexingJob>,
    app_handle: &tauri::AppHandle,
    _state: Arc<ProcessingState>,
    ai_engine: &AIEngine,
    ai_settings: &Arc<RwLock<AISettings>>,
) -> Result<usize, String> {
    let app_dir = app_handle.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let thumbs_dir = app_dir.join("thumbnails");
    let display_dir = app_dir.join("display");

    let mut repaired = 0;
    let batch_size = 100;

    loop {
        // Process in batches to keep memory bounded
        let items: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT i.id, i.path, f.tags 
             FROM images i 
             LEFT JOIN image_features f ON i.id = f.image_id
             LIMIT ? OFFSET ?"
        )
        .bind(batch_size)
        .bind(repaired as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

        if items.is_empty() {
            break;
        }

        for (id, path, tags) in items {
        let thumb_path = thumbs_dir.join(format!("{}.jpg", id));
        let display_path = display_dir.join(format!("{}.jpg", id));
        
        let is_junk = match tags {
            Some(t) => t.is_empty() || t == "[]" || t.contains("(arabic)"),
            None => true,
        };

        if !thumb_path.exists() || is_junk {
            // Regenerate thumbnails if missing
            if !thumb_path.exists() || !display_path.exists() {
                let path_buf = PathBuf::from(&path);
                if path_buf.exists() {
                    let _ = tokio::task::spawn_blocking({
                        let path_buf = path_buf.clone();
                        let thumbs_dir = thumbs_dir.clone();
                        let display_dir = display_dir.clone();
                        let id = id.clone();
                        move || {
                            let img = image::open(&path_buf).ok()?;
                            let display = img.thumbnail(1080, 1080);
                            drop(img);
                            let _ = display.save(&display_dir.join(format!("{}.jpg", id)));
                            let thumb = display.thumbnail(500, 500);
                            let _ = thumb.save(&thumbs_dir.join(format!("{}.jpg", id)));
                            Some(())
                        }
                    }).await;
                } else {
                    continue;
                }
            }

            // Re-run AI analysis for junk tags
            if is_junk {
                let settings = ai_settings.read().expect("lock poisoned").clone();
                match ai_engine.analyze_image(
                    std::path::Path::new(&path),
                    &settings.provider, &settings.base_url, &settings.vision_model_name, &settings.api_key, &settings.proxy_url
                ).await {
                    Ok(analysis) => {
                        let tags_json = serde_json::to_string(&analysis.tags).unwrap_or_default();
                        let embedding_bytes: Vec<u8> = analysis
                            .embedding
                            .iter()
                            .flat_map(|&f| f.to_ne_bytes())
                            .collect();

            if let Some(ref color) = analysis.dominant_color {
                            let _ = sqlx::query(
                                "UPDATE image_features SET tags=?, dominant_color=?, vibe=?, embedding=? WHERE image_id=?"
                            )
                            .bind(&tags_json)
                            .bind(color)
                            .bind(&analysis.vibe)
                            .bind(embedding_bytes)
                            .bind(&id)
                            .execute(pool).await;
                        } else {
                            let _ = sqlx::query(
                                "UPDATE image_features SET tags=?, vibe=?, embedding=? WHERE image_id=?"
                            )
                            .bind(&tags_json)
                            .bind(&analysis.vibe)
                            .bind(embedding_bytes)
                            .bind(&id)
                            .execute(pool).await;
                        }

                        let _ = sqlx::query("UPDATE images SET ai_analyzed = 1 WHERE id = ?")
                            .bind(&id)
                            .execute(pool).await;

                        repaired += 1;
                    }
                    Err(e) => {
                        eprintln!("[Repair] AI analysis failed for {}: {}", path, e);
                    }
                }
            } else {
                repaired += 1;
            }
        }
    } // end for
    } // end loop

    Ok(repaired)
}

/// Pauses the image indexing pipeline.
#[tauri::command]
pub async fn stop_indexing(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_indexing.store(true, Ordering::Relaxed);
    // Wake the worker so it enters the paused state immediately instead of processing one more job
    state.notify_indexing.notify_waiters();
    println!("[Processor] Indexing paused.");
    Ok(())
}

/// Pauses the AI analysis pipeline.
#[tauri::command]
pub async fn stop_analysis(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_analysis.store(true, Ordering::Relaxed);
    state.notify_analysis.notify_waiters();
    println!("[Processor] AI Analysis paused.");
    Ok(())
}

/// Resumes the image indexing pipeline.
#[tauri::command]
pub async fn resume_indexing(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_indexing.store(false, Ordering::Relaxed);
    state.notify_indexing.notify_waiters();
    println!("[Processor] Indexing resumed.");
    Ok(())
}

/// Resumes the AI analysis pipeline.
#[tauri::command]
pub async fn resume_analysis(state: tauri::State<'_ , Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_analysis.store(false, Ordering::Relaxed);
    state.notify_analysis.notify_waiters();
    println!("[Processor] AI Analysis resumed.");
    Ok(())
}

/// Resumes both indexing and analysis pipelines simultaneously.
#[tauri::command]
pub async fn resume_processing(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_indexing.store(false, Ordering::Relaxed);
    state.stop_analysis.store(false, Ordering::Relaxed);
    state.notify_indexing.notify_waiters();
    state.notify_analysis.notify_waiters();
    println!("[Processor] Processing resumed.");
    Ok(())
}

#[tauri::command]
pub async fn index_mobile_image(
    path: String,
    _pool: tauri::State<'_, Pool<Sqlite>>,
    _app_handle: tauri::AppHandle,
    _ai_engine: tauri::State<'_, Arc<AIEngine>>,
    indexing_tx: tauri::State<'_, mpsc::Sender<IndexingJob>>,
    state: tauri::State<'_, Arc<ProcessingState>>,
) -> Result<(), String> {
    let path_buf = PathBuf::from(path);
    indexing_tx.send(IndexingJob { path: path_buf }).await
        .map(|_| state.total_found.fetch_add(1, Ordering::Relaxed))
        .map_err(|e| format!("Failed to queue mobile image: {}", e))?;
    Ok(())
}

async fn index_image(
    path: PathBuf,
    pool: &Pool<Sqlite>,
    thumbs_dir: &Path,
    display_dir: &Path,
    app_handle: &tauri::AppHandle,
) -> Result<Option<AnalysisJob>> {
    use std::time::SystemTime;
    use tauri::Emitter;
    use image::GenericImageView;
    let path_str = path.to_string_lossy().to_string();
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("image").to_string();

    // Get current file metadata for skip-detection
    let (file_size, file_modified) = std::fs::metadata(&path).ok()
        .map(|m| {
            let size = m.len() as i64;
            let modified = m.modified().ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            (size, modified)
        })
        .unwrap_or((0, 0));

    // Check if already in DB — also fetch file metadata for change detection
    let existing: Option<(String, bool, i64, i64)> = sqlx::query_as(
        "SELECT id, ai_analyzed, file_size, file_modified FROM images WHERE path = ?"
    )
        .bind(&path_str)
        .fetch_optional(pool)
        .await?;

    let (id, already_analyzed) = match existing {
        Some((id, analyzed, db_size, db_modified)) => {
            // If file metadata matches AND thumbnails exist, skip entirely
            if db_size == file_size && db_modified == file_modified {
                let thumb_path = thumbs_dir.join(format!("{}.jpg", id));
                let display_path = display_dir.join(format!("{}.jpg", id));
                if thumb_path.exists() && display_path.exists() {
                    return if analyzed { Ok(None) } else { Ok(Some(AnalysisJob { id, path, file_name })) };
                }
            }
            // Metadata changed or thumbnails missing — regenerate
            let id_clone = id.clone();
            let path_clone = path.clone();
            let thumbs_dir_clone = thumbs_dir.to_path_buf();
            let display_dir_clone = display_dir.to_path_buf();

            let _ = tokio::task::spawn_blocking(move || {
                let img = image::open(&path_clone).ok()?;
                let display = img.thumbnail(1080, 1080);
                drop(img);
                let _ = display.save(&display_dir_clone.join(format!("{}.jpg", id_clone)));
                let thumb = display.thumbnail(500, 500);
                let _ = thumb.save(&thumbs_dir_clone.join(format!("{}.jpg", id_clone)));
                Some(())
            }).await;
            
            // Update file metadata in DB
            sqlx::query("UPDATE images SET file_size = ?, file_modified = ? WHERE id = ?")
                .bind(file_size)
                .bind(file_modified)
                .bind(&id)
                .execute(pool)
                .await?;
            
            (id, analyzed)
        },
        None => {
            let id = Uuid::new_v4().to_string();
            let id_clone = id.clone();
            let path_clone = path.clone();
            let thumbs_dir_clone = thumbs_dir.to_path_buf();
            let display_dir_clone = display_dir.to_path_buf();

            let res_opt =
                tokio::task::spawn_blocking(move || -> Result<Option<(String, Option<f64>, Option<f64>, String)>> {
                    let (exif_date, exif_lat, exif_lon) = exif::extract_exif(&path_clone);
                    let creation_date = exif_date
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| Utc::now().to_rfc3339());

                    let img = match image::open(&path_clone) {
                        Ok(i) => i,
                        Err(_e) => return Ok(None),
                    };

                    let display = img.thumbnail(1080, 1080);
                    drop(img);

                    let thumb = display.thumbnail(500, 500);

                    // Dominant color via pixel sampling — every 5th pixel (1/25th of total)
                    // This is 96% faster than full scan with negligible accuracy loss
                    let step = 5usize;
                    let mut r_acc = 0u64;
                    let mut g_acc = 0u64;
                    let mut b_acc = 0u64;
                    let mut sample_count = 0u64;
                    for (i, (_x, _y, pixel)) in thumb.pixels().enumerate() {
                        if i % step == 0 {
                            r_acc += pixel[0] as u64;
                            g_acc += pixel[1] as u64;
                            b_acc += pixel[2] as u64;
                            sample_count += 1;
                        }
                    }

                    if sample_count == 0 {
                        return Ok(None);
                    }

                    let dom_color = format!("#{:02x}{:02x}{:02x}", (r_acc / sample_count) as u8, (g_acc / sample_count) as u8, (b_acc / sample_count) as u8);

                    let thumb_path = thumbs_dir_clone.join(format!("{}.jpg", id_clone));
                    if let Err(e) = thumb.save(&thumb_path) {
                        eprintln!("[Processor] ERROR: Failed to save thumbnail for {}: {}", path_clone.display(), e);
                        return Ok(None);
                    }

                    let display_path = display_dir_clone.join(format!("{}.jpg", id_clone));
                    if let Err(e) = display.save(&display_path) {
                        eprintln!("[Processor] ERROR: Failed to save display image for {}: {}", path_clone.display(), e);
                        return Ok(None);
                    }

                    Ok(Some((creation_date, exif_lat, exif_lon, dom_color)))
                })
                .await??;

            let Some((creation_date, lat, lon, dominant_color)) = res_opt else {
                return Ok(None);
            };

            // Wrap both inserts in a single transaction for atomicity and fewer WAL commits
            let mut tx = pool.begin().await?;
            sqlx::query(
                "INSERT INTO images (id, path, date_taken, lat, lon, ai_analyzed, file_size, file_modified) VALUES (?, ?, ?, ?, ?, 0, ?, ?)"
            )
            .bind(&id)
            .bind(&path_str)
            .bind(&creation_date)
            .bind(lat)
            .bind(lon)
            .bind(file_size)
            .bind(file_modified)
            .execute(&mut *tx)
            .await?;

            sqlx::query("INSERT OR IGNORE INTO image_features (image_id, dominant_color) VALUES (?, ?)")
                .bind(&id)
                .bind(&dominant_color)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;

            let _ = app_handle.emit("indexing-progress", serde_json::json!({
                "message": format!("Indexed: {}", file_name),
                "path": path_str
            }));

            (id, false)
        }
    };

    if already_analyzed {
        return Ok(None);
    }

    Ok(Some(AnalysisJob { id, path, file_name }))
}

async fn run_ai_analysis(
    job: AnalysisJob,
    pool: &Pool<Sqlite>,
    ai_engine: &AIEngine,
    app_handle: &tauri::AppHandle,
    ai_settings: &Arc<RwLock<AISettings>>,
) -> Result<()> {
    use tauri::Emitter;

    // Use the cached 1080px display image if available instead of decoding the original
    let app_dir = app_handle.path().app_local_data_dir().ok();
    let display_path = app_dir.map(|d| d.join("display").join(format!("{}.jpg", job.id)));
    let analysis_path = display_path.as_ref()
        .filter(|p| p.exists())
        .unwrap_or(&job.path);

    // Use cached AI settings instead of querying the DB for every image
    let settings = ai_settings.read().expect("lock poisoned").clone();

    // Run AI Analysis with the smaller display image (or original as fallback)
    match ai_engine.analyze_image(analysis_path, &settings.provider, &settings.base_url, &settings.vision_model_name, &settings.api_key, &settings.proxy_url).await {
        Ok(analysis) => {
            let tags_json = serde_json::to_string(&analysis.tags)?;
            let embedding_bytes: Vec<u8> = analysis
                .embedding
                .iter()
                .flat_map(|&f| f.to_ne_bytes())
                .collect();
            let dominant_color = analysis.dominant_color.clone();

            if let Some(ref color) = analysis.dominant_color {
                sqlx::query(
                    "UPDATE image_features SET tags=?, dominant_color=?, vibe=?, embedding=? WHERE image_id=?"
                )
                .bind(&tags_json)
                .bind(color)
                .bind(&analysis.vibe)
                .bind(embedding_bytes)
                .bind(&job.id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE image_features SET tags=?, vibe=?, embedding=? WHERE image_id=?"
                )
                .bind(&tags_json)
                .bind(&analysis.vibe)
                .bind(embedding_bytes)
                .bind(&job.id)
                .execute(pool)
                .await?;
            }

            sqlx::query("UPDATE images SET ai_analyzed = 1 WHERE id = ?")
                .bind(&job.id)
                .execute(pool)
                .await?;

            // Update in-memory cache so UI sees fresh data immediately
            if let Some(cache) = app_handle.try_state::<Arc<crate::cache::ImageCache>>() {
                cache.update_analysis(&job.id, analysis.tags, Some(analysis.vibe), dominant_color);
            }

            let _ = app_handle.emit("analysis-progress", serde_json::json!({
                "message": format!("AI Analysis complete for: {}", job.file_name),
                "id": job.id
            }));
        }
        Err(e) => {
            eprintln!("AI Analysis failed for {}: {}", job.path.display(), e);
        }
    }

    Ok(())
}
