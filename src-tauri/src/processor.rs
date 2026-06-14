/// Background processing pipeline: indexing, thumbnail generation, and AI analysis.
///
/// Architecture:
/// - `start_processing_worker` — main event loop consuming index/analysis jobs from mpsc channels
/// - `index_image` — inserts new images into DB, creates thumbnails, computes dominant color
/// - `run_ai_analysis` — sends images to AI engine for tag/vibe extraction
///
/// Uses a two-stage semaphore (indexing:analysis) that dynamically shifts capacity
/// from 8:2 to 2:8 as indexing nears completion.
use crate::ai;
use crate::ai::AIEngine;
use tauri::Manager;
use anyhow::Result;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
}

impl Default for ProcessingState {
    fn default() -> Self {
        Self {
            stop_indexing: AtomicBool::new(false),
            stop_analysis: AtomicBool::new(false),
            total_found: AtomicUsize::new(0),
            indexed_count: AtomicUsize::new(0),
            shifted_to_analysis: AtomicBool::new(false),
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
) {
    let thumbs_dir = app_data_dir.join("thumbnails");
    let display_dir = app_data_dir.join("display");

    // Ensure cache dirs exist
    std::fs::create_dir_all(&thumbs_dir).ok();
    std::fs::create_dir_all(&display_dir).ok();

    // Stage 1: Indexing Parallelism (Fast, CPU-bound)
    let indexing_parallelism = 8;
    
    // Stage 2: AI Analysis Parallelism (Slow, GPU/Memory-bound)
    // We start low (2) and scale up later
    let analysis_parallelism = 2;

    println!("Scaling pipeline: Indexing ({}) | AI Analysis ({})", indexing_parallelism, analysis_parallelism);
    
    let (analysis_tx, mut analysis_rx) = mpsc::channel::<AnalysisJob>(200);
    let indexing_semaphore = Arc::new(tokio::sync::Semaphore::new(indexing_parallelism));
    let analysis_semaphore = Arc::new(tokio::sync::Semaphore::new(analysis_parallelism));

    // Spawn Analysis Worker Loop
    let analysis_pool = pool.clone();
    let analysis_ai = ai_engine.clone();
    let analysis_handle = app_handle.clone();
    let analysis_sem = analysis_semaphore.clone();
    let analysis_state = state.clone();
    
    tokio::spawn(async move {
        // Gate: Wait until 70% of indexing is complete before starting analysis
        loop {
            if analysis_state.stop_analysis.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }

            let total = analysis_state.total_found.load(Ordering::Relaxed);
            let indexed = analysis_state.indexed_count.load(Ordering::Relaxed);

            if total > 5 && (indexed as f32 / total as f32) < 0.7 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
            break;
        }

        println!("[Analysis] Indexing past 70%, starting analysis worker");

        while let Some(job) = analysis_rx.recv().await {
            let pool = analysis_pool.clone();
            let ai = analysis_ai.clone();
            let handle = analysis_handle.clone();
            let permit = analysis_sem.clone().acquire_owned().await.unwrap();

            tokio::spawn(async move {
                let _permit = permit;
                if let Err(e) = run_ai_analysis(job, &pool, &ai, &handle).await {
                    eprintln!("AI Analysis failed: {}", e);
                }
            });
        }
    });

    // Main Indexing Worker Loop
    while let Some(job) = rx.recv().await {
        if state.stop_indexing.load(Ordering::Relaxed) {
            // If stopped, we just drop the jobs or skip them
            // For now, let's just wait so we don't drain the channel if the user might resume
            while state.stop_indexing.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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
                println!("[Processor] 90% Threshold Reached: Shifting resources Index(8->2) | Analysis(2->8)");
                
                // 1. Drain Indexing: Hold 6 permits forever to effectively reduce capacity from 8 to 2
                let idx_sem = indexing_semaphore.clone();
                tokio::spawn(async move {
                    if let Ok(permits) = idx_sem.acquire_many_owned(6).await {
                        // We "forget" or just hold these permits in this long-running task to reduce capacity
                        std::mem::forget(permits); 
                    }
                });

                // 2. Boost Analysis: Add 6 permits to increase capacity from 2 to 8
                analysis_semaphore.add_permits(6);
            }
        }
        // -----------------------------

        // Stage 1.5: Timeout and concurrency handling
        tokio::spawn(async move {
            let _permit = permit;
            let timeout_duration = std::time::Duration::from_secs(30);
            
            match tokio::time::timeout(timeout_duration, index_image(job.path.clone(), &pool, &thumbs, &display, &handle)).await {
                Ok(Ok(Some(analysis_job))) => {
                    index_state.indexed_count.fetch_add(1, Ordering::Relaxed);
                    // Non-blocking send — analysis channel full is not a reason to deadlock indexing
                    match a_tx.try_send(analysis_job) {
                        Err(tokio::sync::mpsc::error::TrySendError::Full(job)) => {
                            eprintln!("[Processor] Analysis queue full, skipping later re-analysis for {}", job.file_name);
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            eprintln!("[Processor] Analysis channel closed");
                        }
                        Ok(_) => {}
                    }
                }
                Ok(Ok(None)) => {
                    index_state.indexed_count.fetch_add(1, Ordering::Relaxed);
                }
                Ok(Err(e)) => {
                    index_state.indexed_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Failed to index image: {}", e);
                }
                Err(_) => {
                    index_state.indexed_count.fetch_add(1, Ordering::Relaxed);
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
            
            let cur = index_state.indexed_count.load(Ordering::Relaxed);
            let tot = index_state.total_found.load(Ordering::Relaxed);
            println!("[Processor] Progress: {}/{} ({}%)", cur, tot, if tot > 0 { (cur as f32 / tot as f32 * 100.0) as i32 } else { 0 });
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
) -> Result<usize, String> {
    internal_trigger_junk_reanalysis(&*pool, &*indexing_tx, &app_handle, state.inner().clone(), &*ai_engine).await
}

pub async fn internal_trigger_junk_reanalysis(
    pool: &Pool<Sqlite>,
    _indexing_tx: &mpsc::Sender<IndexingJob>,
    app_handle: &tauri::AppHandle,
    _state: Arc<ProcessingState>,
    ai_engine: &AIEngine,
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
                let settings: (String, String, String, String, String, String) = sqlx::query_as(
                    "SELECT provider, base_url, model_name, vision_model_name, api_key, proxy_url FROM ai_settings WHERE id = 1"
                )
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| (
                    "openrouter".to_string(),
                    "https://openrouter.ai/api/v1".to_string(),
                    "meta-llama/llama-3.1-8b-instruct".to_string(),
                    "qwen/qwen-vl-plus".to_string(),
                    String::new(),
                    ai::DEFAULT_PROXY_URL.to_string(),
                ));

                match ai_engine.analyze_image(
                    std::path::Path::new(&path),
                    &settings.0, &settings.1, &settings.3, &settings.4, &settings.5
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
    println!("[Processor] Indexing paused.");
    Ok(())
}

/// Pauses the AI analysis pipeline.
#[tauri::command]
pub async fn stop_analysis(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_analysis.store(true, Ordering::Relaxed);
    println!("[Processor] AI Analysis paused.");
    Ok(())
}

/// Resumes the image indexing pipeline.
#[tauri::command]
pub async fn resume_indexing(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_indexing.store(false, Ordering::Relaxed);
    println!("[Processor] Indexing resumed.");
    Ok(())
}

/// Resumes the AI analysis pipeline.
#[tauri::command]
pub async fn resume_analysis(state: tauri::State<'_ , Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_analysis.store(false, Ordering::Relaxed);
    println!("[Processor] AI Analysis resumed.");
    Ok(())
}

/// Resumes both indexing and analysis pipelines simultaneously.
#[tauri::command]
pub async fn resume_processing(state: tauri::State<'_, Arc<ProcessingState>>) -> Result<(), String> {
    state.stop_indexing.store(false, Ordering::Relaxed);
    state.stop_analysis.store(false, Ordering::Relaxed);
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
    use tauri::Emitter;
    use image::GenericImageView;
    let path_str = path.to_string_lossy().to_string();
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("image").to_string();

    // Check if already in DB
    let existing: Option<(String, bool)> = sqlx::query_as("SELECT id, ai_analyzed FROM images WHERE path = ?")
        .bind(&path_str)
        .fetch_optional(pool)
        .await?;

    let (id, already_analyzed) = match existing {
        Some((id, analyzed)) => {
            // Even if it exists in DB, ensure thumbnails are on disk
            let thumb_path = thumbs_dir.join(format!("{}.jpg", id));
            let display_path = display_dir.join(format!("{}.jpg", id));
            
            if thumb_path.exists() && display_path.exists() {
                (id, analyzed)
            } else {
                // Files missing! Regenerate them
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
                
                (id, analyzed)
            }
        },
        None => {
            let id = Uuid::new_v4().to_string();
            let id_clone = id.clone();
            let path_clone = path.clone();
            let thumbs_dir_clone = thumbs_dir.to_path_buf();
            let display_dir_clone = display_dir.to_path_buf();

            let res_opt =
                tokio::task::spawn_blocking(move || -> Result<Option<(String, Option<f64>, Option<f64>, String)>> {
                    let img = match image::open(&path_clone) {
                        Ok(i) => i,
                        Err(_e) => {
                            return Ok(None);
                        }
                    };

                    // Create display (1080px) first, then drop full img to free ~72MB per worker
                    let display = img.thumbnail(1080, 1080);
                    drop(img);

                    // Create thumb from the display version — negligible quality difference vs original
                    let thumb = display.thumbnail(500, 500);

                    // Dominant color from thumb pixels
                    let mut r_acc = 0u64;
                    let mut g_acc = 0u64;
                    let mut b_acc = 0u64;
                    let mut pix_count = 0u64;
                    for (_x, _y, pixel) in thumb.pixels() {
                        r_acc += pixel[0] as u64;
                        g_acc += pixel[1] as u64;
                        b_acc += pixel[2] as u64;
                        pix_count += 1;
                    }

                    if pix_count == 0 {
                        return Ok(None);
                    }

                    let dom_color = format!("#{:02x}{:02x}{:02x}", (r_acc/pix_count) as u8, (g_acc/pix_count) as u8, (b_acc/pix_count) as u8);

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

                    let now = Utc::now().to_rfc3339();
                    Ok(Some((now, None, None, dom_color)))
                })
                .await??;

            let Some((creation_date, lat, lon, dominant_color)) = res_opt else {
                return Ok(None);
            };

            sqlx::query(
                "INSERT INTO images (id, path, date_taken, lat, lon, ai_analyzed) VALUES (?, ?, ?, ?, ?, 0)"
            )
            .bind(&id)
            .bind(&path_str)
            .bind(&creation_date)
            .bind(lat)
            .bind(lon)
            .execute(pool)
            .await?;

            sqlx::query("INSERT OR IGNORE INTO image_features (image_id, dominant_color) VALUES (?, ?)")
                .bind(&id)
                .bind(&dominant_color)
                .execute(pool)
                .await?;

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
) -> Result<()> {
    use tauri::Emitter;

    // Use the cached 1080px display image if available instead of decoding the original
    let app_dir = app_handle.path().app_local_data_dir().ok();
    let display_path = app_dir.map(|d| d.join("display").join(format!("{}.jpg", job.id)));
    let analysis_path = display_path.as_ref()
        .filter(|p| p.exists())
        .unwrap_or(&job.path);

    // Fetch AI Settings for analysis
    let settings: (String, String, String, String, String, String) = sqlx::query_as(
        "SELECT provider, base_url, model_name, vision_model_name, api_key, proxy_url FROM ai_settings WHERE id = 1"
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| (
        "openrouter".to_string(),
        "https://openrouter.ai/api/v1".to_string(),
        "meta-llama/llama-3.1-8b-instruct".to_string(),
        "qwen/qwen-vl-plus".to_string(),
        String::new(),
        ai::DEFAULT_PROXY_URL.to_string(),
    ));

    // Run AI Analysis with the smaller display image (or original as fallback)
    match ai_engine.analyze_image(analysis_path, &settings.0, &settings.1, &settings.3, &settings.4, &settings.5).await {
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
