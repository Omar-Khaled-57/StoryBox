/// File system scanner: discovers image files from user directories or selected folders.
///
/// Platforms:
/// - Windows: scans OneDrive/Pictures, user-selected folders
/// - macOS: scans Photos library originals, ~/Pictures
/// - Linux: scans ~/Pictures
/// - iOS/Android: emits events for the frontend to push photo data
use crate::processor::{IndexingJob, ProcessingState};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
use walkdir::WalkDir;
use directories::UserDirs;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub async fn scan_directory(dir: &Path, tx: &Sender<IndexingJob>, state: Arc<ProcessingState>) -> Result<usize> {
    // We do a simple blocking walkdir inside a spawned blocking thread.
    let dir = dir.to_path_buf();
    let tx = tx.clone();
    let scan_state = state.clone();

    let added_count = tokio::task::spawn_blocking(move || {
        let mut local_count = 0;
        // Follow links is VITAL on Windows 11 where Pictures is often a junction to OneDrive
        for entry in WalkDir::new(dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                        // Increment total BEFORE blocking send so the UI "Found" count goes up immediately
                        scan_state.total_found.fetch_add(1, Ordering::Relaxed);
                        local_count += 1;

                        if let Err(e) = tx.blocking_send(IndexingJob {
                            path: path.to_path_buf(),
                        }) {
                            eprintln!("Failed to send indexing job: {}", e);
                        }
                    }
                }
            }
        }
        local_count
    })
    .await?;

    Ok(added_count)
}

#[tauri::command]
pub async fn start_scan(
    dir: String,
    state: tauri::State<'_, Sender<IndexingJob>>,
    proc_state: tauri::State<'_, Arc<ProcessingState>>,
    handle: tauri::AppHandle,
) -> Result<usize, String> {
    let path = PathBuf::from(dir);
    let count = scan_directory(&path, &*state, proc_state.inner().clone())
        .await
        .map_err(|e| e.to_string())?;
    
    use tauri::Emitter;
    let _ = handle.emit("scan-complete", count);
    Ok(count)
}

#[tauri::command]
pub async fn start_scan_device(
    state: tauri::State<'_, Sender<IndexingJob>>, 
    proc_state: tauri::State<'_, Arc<ProcessingState>>,
    handle: tauri::AppHandle
) -> Result<usize, String> {
    internal_scan_device(&*state, proc_state.inner().clone(), &handle).await
}

pub async fn internal_scan_device(
    tx: &Sender<IndexingJob>, 
    proc_state: Arc<ProcessingState>,
    handle: &tauri::AppHandle
) -> Result<usize, String> {
    use tauri::Emitter;
    let mut total_count = 0;
    
    let mut search_paths = Vec::new();

    #[cfg(any(target_os = "android", target_os = "ios"))]
    println!("[Scanner] Platform detected: Mobile");
    #[cfg(target_os = "windows")]
    println!("[Scanner] Platform detected: Windows");
    #[cfg(target_os = "macos")]
    println!("[Scanner] Platform detected: macOS");
    #[cfg(target_os = "linux")]
    println!("[Scanner] Platform detected: Linux");

    // On mobile (Android / iOS) we delegate scanning to the frontend, which
    // uses the platform-specific photo library plugin to fetch images and
    // sends them back to the backend for indexing.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        if cfg!(target_os = "ios") {
            println!("[Scanner] iOS detected — using iOS Photos plugin for library access.");
        } else {
            println!("[Scanner] Android detected — using medialibrary plugin for content access.");
        }
        let _ = handle.emit("trigger-mobile-scan", ());
        return Ok(0);
    }

    if let Some(user_dirs) = UserDirs::new() {
        if let Some(p) = user_dirs.picture_dir() { search_paths.push(p.to_path_buf()); }
    }

    // Windows-specific: Add OneDrive Pictures if it exists
    #[cfg(target_os = "windows")]
    {
        if let Ok(home) = std::env::var("USERPROFILE") {
            let home_path = PathBuf::from(home);
            let onedrive_variants = vec!["OneDrive", "One Drive"];
            for var in onedrive_variants {
                let od_pics = home_path.join(var).join("Pictures");
                if od_pics.exists() { 
                    search_paths.push(od_pics); 
                }
            }
        }
    }

    // macOS-specific: Add Apple Photos library discovery
    #[cfg(target_os = "macos")]
    {
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(home) = user_dirs.home_dir() {
                let photos_lib = home.join("Pictures").join("Photos Library.photoslibrary").join("originals");
                if photos_lib.exists() {
                    search_paths.push(photos_lib);
                }
            }
        }
    }

    // Linux-specific: Ensure standard Pictures folder is searched even if UserDirs fails
    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join("Pictures");
            if p.exists() { search_paths.push(p); }
        }
    }

    // De-duplicate paths
    search_paths.sort();
    search_paths.dedup();

    for p in search_paths {
        if p.exists() {
            println!("[Scanner] Auto-scanning: {:?}", p);
            match scan_directory(&p, tx, proc_state.clone()).await {
                Ok(c) => total_count += c,
                Err(e) => eprintln!("Error scanning {:?}: {}", p, e),
            }
        }
    }

    let _ = handle.emit("scan-complete", total_count);
    Ok(total_count)
}
