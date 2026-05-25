pub mod ai;
pub mod db;
pub mod processor;
pub mod scanner;
pub mod stories;

/// Tauri application entry point and command registration.
///
/// This module wires together the Rust backend:
/// - Initializes the SQLite database and AI engine
/// - Registers all Tauri IPC commands (AI health, scanning, stories, settings)
/// - Starts background workers (processing pipeline, automation loop)
/// - Handles app lifecycle (setup, reset)
use std::sync::Arc;
use tauri::{Manager, Emitter};
use tokio::sync::mpsc;
use sqlx::Pool;
use sqlx::sqlite::Sqlite;
use std::sync::atomic::Ordering;

#[tauri::command]
async fn check_ai_availability(
    pool: tauri::State<'_, Pool<Sqlite>>,
    ai_engine: tauri::State<'_, Arc<ai::AIEngine>>,
) -> Result<serde_json::Value, String> {
    let settings: (String, String, String, String, String, String) = sqlx::query_as(
        "SELECT provider, base_url, model_name, vision_model_name, api_key, proxy_url FROM ai_settings WHERE id = 1"
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or_else(|_| (
        "openrouter".to_string(),
        "https://openrouter.ai/api/v1".to_string(),
        "meta-llama/llama-3.1-8b-instruct".to_string(),
        "qwen/qwen-vl-plus".to_string(),
        String::new(),
        ai::DEFAULT_PROXY_URL.to_string(),
    ));

    ai_engine.check_availability(&settings.0, &settings.1, &settings.3, &settings.2, &settings.4, &settings.5)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_ai_generation(
    pool: tauri::State<'_, Pool<Sqlite>>,
    ai_engine: tauri::State<'_, Arc<ai::AIEngine>>,
) -> Result<serde_json::Value, String> {
    // 1. Pick an image that actually exists on disk
    let images: Vec<(String, String)> = sqlx::query_as("SELECT id, path FROM images ORDER BY ai_analyzed DESC LIMIT 50")
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut valid_image = None;
    for (id, path) in images {
        if std::path::Path::new(&path).exists() {
            valid_image = Some((id, path));
            break;
        }
    }

    let Some((id, path)) = valid_image else {
        return Ok(serde_json::json!({ 
            "success": false, 
            "message": "No valid image files found on disk. Please add a folder and wait for indexing." 
        }));
    };

    // 2. Run analysis
    let settings: (String, String, String, String, String, String) = sqlx::query_as(
        "SELECT provider, base_url, model_name, vision_model_name, api_key, proxy_url FROM ai_settings WHERE id = 1"
    )
    .fetch_one(&*pool)
    .await
    .unwrap_or_else(|_| (
        "openrouter".to_string(),
        "https://openrouter.ai/api/v1".to_string(),
        "meta-llama/llama-3.1-8b-instruct".to_string(),
        "qwen/qwen-vl-plus".to_string(),
        String::new(),
        ai::DEFAULT_PROXY_URL.to_string(),
    ));

    let analysis = ai_engine.analyze_image(std::path::Path::new(&path), &settings.0, &settings.1, &settings.3, &settings.4, &settings.5)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Generate a caption
    let teller = ai::Storyteller::new(settings.0, settings.1, settings.2, settings.4, settings.5);
    let caption = teller.generate_caption(&analysis.tags, Some(&analysis.vibe), None).await;

    Ok(serde_json::json!({
        "success": true,
        "image_id": id,
        "image_path": path,
        "tags": analysis.tags,
        "vibe": analysis.vibe,
        "generated_caption": caption,
        "message": "AI generation test complete."
    }))
}

/// Accepts base64-encoded image data (from iOS Photos plugin), writes to a
/// temp file, and queues it for indexing. Works cross-platform but is mainly
/// used on iOS where the medialibrary plugin is unavailable.
#[tauri::command]
async fn index_ios_image_data(
    data: String,
    file_name: String,
    indexing_tx: tauri::State<'_, mpsc::Sender<processor::IndexingJob>>,
    state: tauri::State<'_, Arc<processor::ProcessingState>>,
) -> Result<(), String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let bytes = STANDARD.decode(&data).map_err(|e| format!("Base64 decode failed: {}", e))?;
    let temp_dir = std::env::temp_dir().join("storybox-ios");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let file_path = temp_dir.join(&file_name);
    std::fs::write(&file_path, &bytes).map_err(|e| format!("Failed to write temp file: {}", e))?;
    state.total_found.fetch_add(1, Ordering::Relaxed);
    indexing_tx
        .send(processor::IndexingJob { path: file_path })
        .await
        .map_err(|e| format!("Failed to queue iOS image: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn reset_app(app_handle: tauri::AppHandle) -> Result<(), String> {
    // 1. Close the database pool to release the file lock
    if let Some(pool) = app_handle.try_state::<Pool<Sqlite>>() {
        pool.close().await;
    }

    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
        
    let db_path = app_dir.join("storybox.db");
    
    // 2. Attempt to delete the database file
    if db_path.exists() {
        // Add a tiny delay to ensure file handles are truly released by the OS
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::fs::remove_file(db_path).map_err(|e| {
            format!("Failed to delete database: {}. Ensure no other processes are accessing 'storybox.db'.", e)
        })?;
    }
    
    // 3. Restart the application
    app_handle.restart();
    #[allow(unreachable_code)]
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_medialibrary::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(target_os = "ios")]
    let builder = builder.plugin(tauri_plugin_ios_photos::init());

    builder
        .setup(|app| {
            let handle = app.handle().clone();

            // Set up our job queue
            let (tx, rx) = mpsc::channel::<processor::IndexingJob>(100);
            app.manage(tx.clone());

            let app_data_dir = handle.path().app_local_data_dir().expect("requires app local data dir");

            tauri::async_runtime::block_on(async move {
                let pool = db::init_db(&handle).await.expect("DB init");
                handle.manage(pool.clone());
                
                // Processing State
                let proc_state = Arc::new(processor::ProcessingState::default());
                handle.manage(proc_state.clone());

                // AI engine
                let ai_engine = Arc::new(ai::AIEngine::new(&app_data_dir).unwrap_or_default());
                handle.manage(ai_engine.clone());

                // Worker
                tokio::spawn(processor::start_processing_worker(
                    rx, 
                    pool.clone(), 
                    app_data_dir, 
                    ai_engine.clone(), 
                    handle.clone(), 
                    proc_state.clone()
                ));

                // Startup scan
                let tx_scan = tx.clone();
                let handle_scan = handle.clone();
                let proc_state_scan = proc_state.clone();
                tokio::spawn(async move {
                    let _ = scanner::internal_scan_device(&tx_scan, proc_state_scan, &handle_scan).await;
                });

                // Onboarding stories
                let onboarding_pool = pool.clone();
                let onboarding_handle = handle.clone();
                tokio::spawn(async move {
                    // Wait for some images to be indexed before generating stories
                    for _ in 0..10 {
                        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM images")
                            .fetch_one(&onboarding_pool)
                            .await
                            .unwrap_or((0,));
                        if count.0 >= 5 { break; }
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }

                    let _ = onboarding_handle.emit("refresh-stories", ());
                });

                // Periodic automation: story generation + cleanup every 30 minutes
                let auto_pool = pool.clone();
                let auto_handle = handle.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
                        let _ = stories::run_automation_tasks(&auto_pool, &auto_handle).await;
                    }
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            index_ios_image_data,
            scanner::start_scan,
            scanner::start_scan_device,
            stories::generate_story,
            stories::get_stories,
            stories::get_cached_image_base64,
            stories::get_ai_status,
            stories::delete_story,
            stories::delete_all_stories,
            stories::toggle_story_pin,
            stories::toggle_story_favorite,
            stories::get_ai_settings,
            stories::update_ai_settings,
            processor::index_mobile_image,
            processor::trigger_junk_reanalysis,
            processor::stop_indexing,
            processor::stop_analysis,
            processor::resume_indexing,
            processor::resume_analysis,
            processor::resume_processing,
            check_ai_availability,
            test_ai_generation,
            reset_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

