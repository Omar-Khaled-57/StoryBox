/// File system scanner: discovers image files from user directories or selected folders.
///
/// Platforms:
/// - Windows: scans OneDrive/Pictures, user-selected folders
/// - macOS: scans Photos library originals, ~/Pictures
/// - Linux: scans ~/Pictures
/// - iOS/Android: emits events for the frontend to push photo data
use crate::processor::{IndexingJob, ProcessingState};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
use walkdir::WalkDir;
use directories::UserDirs;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScannedFolder {
    pub id: String,
    pub path: String,
    pub is_enabled: bool,
    pub added_at: String,
}

pub async fn get_enabled_folders(pool: &Pool<Sqlite>) -> Vec<ScannedFolder> {
    sqlx::query_as::<_, (String, String, bool, String)>(
        "SELECT id, path, is_enabled, COALESCE(added_at, '') FROM scanned_folders WHERE is_enabled = 1 ORDER BY added_at DESC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, path, is_enabled, added_at)| ScannedFolder { id, path, is_enabled, added_at })
    .collect()
}

pub async fn get_all_folders(pool: &Pool<Sqlite>) -> Vec<ScannedFolder> {
    sqlx::query_as::<_, (String, String, bool, String)>(
        "SELECT id, path, is_enabled, COALESCE(added_at, '') FROM scanned_folders ORDER BY added_at DESC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, path, is_enabled, added_at)| ScannedFolder { id, path, is_enabled, added_at })
    .collect()
}

pub async fn scan_persisted_folders(
    pool: &Pool<Sqlite>,
    tx: &Sender<IndexingJob>,
    proc_state: Arc<ProcessingState>,
) -> Result<usize, String> {
    let folders = get_enabled_folders(pool).await;
    let mut total_count = 0;
    for folder in &folders {
        let path = PathBuf::from(&folder.path);
        if path.exists() {
            match scan_directory(&path, tx, proc_state.clone()).await {
                Ok(c) => total_count += c,
                Err(e) => eprintln!("Error scanning {:?}: {}", path, e),
            }
        }
    }
    Ok(total_count)
}

#[tauri::command]
pub async fn get_scanned_folders(pool: tauri::State<'_, Pool<Sqlite>>) -> Result<Vec<ScannedFolder>, String> {
    Ok(get_all_folders(&pool).await)
}

#[tauri::command]
pub async fn add_scanned_folder(
    path: String,
    pool: tauri::State<'_, Pool<Sqlite>>,
) -> Result<ScannedFolder, String> {
    use uuid::Uuid;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO scanned_folders (id, path, is_enabled) VALUES (?, ?, 1)")
        .bind(&id)
        .bind(&path)
        .execute(&*pool)
        .await
        .map_err(|e| format!("Failed to add folder: {}. Possibly a duplicate.", e))?;
    let row = sqlx::query_as::<_, (String, String, bool, String)>(
        "SELECT id, path, is_enabled, COALESCE(added_at, '') FROM scanned_folders WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(ScannedFolder { id: row.0, path: row.1, is_enabled: row.2, added_at: row.3 })
}

#[tauri::command]
pub async fn remove_scanned_folder(id: String, pool: tauri::State<'_, Pool<Sqlite>>) -> Result<(), String> {
    sqlx::query("DELETE FROM scanned_folders WHERE id = ?")
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_scanned_folder(id: String, pool: tauri::State<'_, Pool<Sqlite>>) -> Result<ScannedFolder, String> {
    sqlx::query("UPDATE scanned_folders SET is_enabled = CASE WHEN is_enabled = 1 THEN 0 ELSE 1 END WHERE id = ?")
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    let row = sqlx::query_as::<_, (String, String, bool, String)>(
        "SELECT id, path, is_enabled, COALESCE(added_at, '') FROM scanned_folders WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&*pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(ScannedFolder { id: row.0, path: row.1, is_enabled: row.2, added_at: row.3 })
}

#[tauri::command]
pub async fn check_folders_accessibility(pool: tauri::State<'_, Pool<Sqlite>>) -> Result<Vec<serde_json::Value>, String> {
    let folders = get_all_folders(&pool).await;
    let mut results = Vec::new();
    for folder in &folders {
        let accessible = Path::new(&folder.path).exists();
        results.push(serde_json::json!({
            "id": folder.id,
            "path": folder.path,
            "accessible": accessible,
            "is_enabled": folder.is_enabled
        }));
    }
    Ok(results)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FolderChild {
    pub name: String,
    pub path: String,
    pub disabled: bool,
}

#[tauri::command]
pub async fn get_folder_children(
    path: String,
    pool: tauri::State<'_, Pool<Sqlite>>,
) -> Result<Vec<FolderChild>, String> {
    // Load disabled overrides for this parent
    let disabled_children: Vec<String> = sqlx::query_scalar(
        "SELECT child_name FROM folder_child_overrides WHERE parent_path = ? AND is_disabled = 1"
    )
    .bind(&path)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    let dir = PathBuf::from(&path);
    if !dir.exists() || !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut children = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                let child_path = entry.path().to_string_lossy().to_string();
                let disabled = disabled_children.contains(&name.to_string());
                children.push(FolderChild { name: name.to_string(), path: child_path, disabled });
            }
        }
    }
    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(children)
}

#[tauri::command]
pub async fn toggle_child_override(
    parent_path: String,
    child_name: String,
    pool: tauri::State<'_, Pool<Sqlite>>,
) -> Result<bool, String> {
    // Check if an override exists
    let existing: Option<(bool,)> = sqlx::query_as(
        "SELECT is_disabled FROM folder_child_overrides WHERE parent_path = ? AND child_name = ?"
    )
    .bind(&parent_path)
    .bind(&child_name)
    .fetch_optional(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    match existing {
        Some((disabled,)) => {
            // Toggle: if currently disabled, remove override (re-enable); if not disabled, insert (shouldn't happen)
            if disabled {
                sqlx::query("DELETE FROM folder_child_overrides WHERE parent_path = ? AND child_name = ?")
                    .bind(&parent_path)
                    .bind(&child_name)
                    .execute(&*pool)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(false) // no longer disabled
            } else {
                Ok(false)
            }
        }
        None => {
            // Insert new override (disabled)
            sqlx::query("INSERT INTO folder_child_overrides (parent_path, child_name, is_disabled) VALUES (?, ?, 1)")
                .bind(&parent_path)
                .bind(&child_name)
                .execute(&*pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(true) // now disabled
        }
    }
}

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
    handle: tauri::AppHandle,
    pool: tauri::State<'_, Pool<Sqlite>>,
) -> Result<usize, String> {
    internal_scan_device(&*state, proc_state.inner().clone(), &handle, Some(&pool)).await
}

pub async fn internal_scan_device(
    tx: &Sender<IndexingJob>, 
    proc_state: Arc<ProcessingState>,
    handle: &tauri::AppHandle,
    pool: Option<&Pool<Sqlite>>,
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

    // Persist discovered paths to the scanned_folders table so they appear in the UI
    if let Some(pool) = pool {
        use uuid::Uuid;
        for p in &search_paths {
            let p_str = p.to_string_lossy().to_string();
            let id = Uuid::new_v4().to_string();
            let _ = sqlx::query("INSERT OR IGNORE INTO scanned_folders (id, path, is_enabled) VALUES (?, ?, 1)")
                .bind(&id)
                .bind(&p_str)
                .execute(pool)
                .await;
        }
        // Remove any paths the user has explicitly disabled from the scan list
        let disabled: Vec<String> = sqlx::query_as::<_, (String,)>(
            "SELECT path FROM scanned_folders WHERE is_enabled = 0"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(p,)| p)
        .collect();
        search_paths.retain(|p| !disabled.contains(&p.to_string_lossy().to_string()));
    }

    for p in search_paths {
        if p.exists() {
            println!("[Scanner] Auto-scanning: {:?}", p);
            match scan_directory(&p, tx, proc_state.clone()).await {
                Ok(c) => total_count += c,
                Err(e) => eprintln!("Error scanning {:?}: {}", p, e),
            }
        }
    }

    // Also scan persisted enabled folders from the database
    if let Some(pool) = pool {
        match scan_persisted_folders(pool, tx, proc_state.clone()).await {
            Ok(c) => total_count += c,
            Err(e) => eprintln!("Error scanning persisted folders: {}", e),
        }
    }

    let _ = handle.emit("scan-complete", total_count);
    Ok(total_count)
}
