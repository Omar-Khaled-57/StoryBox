use crate::processor::IndexingJob;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub struct FolderWatcher {
    _watcher: notify::RecommendedWatcher,
}

impl FolderWatcher {
    /// Starts watching all enabled scanned folders for new image files.
    pub async fn start(pool: Pool<Sqlite>, tx: Sender<IndexingJob>, state: Arc<crate::processor::ProcessingState>) -> Result<Self> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(256);

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = event_tx.blocking_send(res);
        })?;

        // Query enabled folders from DB
        let folders: Vec<String> = sqlx::query_scalar(
            "SELECT path FROM scanned_folders WHERE is_enabled = 1"
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        for folder in &folders {
            let p = std::path::PathBuf::from(folder);
            if p.exists() {
                if let Err(e) = watcher.watch(&p, RecursiveMode::Recursive) {
                    eprintln!("[Watcher] Failed to watch {}: {}", folder, e);
                }
            }
        }

        // Process events in background
        let tx_clone = tx.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            while let Some(Ok(event)) = event_rx.recv().await {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in &event.paths {
                            if path.is_file() {
                                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                                    let ext_lower = ext.to_lowercase();
                                    if matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "webp") {
                                        if let Err(e) = tx_clone.send(IndexingJob { path: path.to_path_buf() }).await {
                                            eprintln!("[Watcher] Failed to send job: {}", e);
                                        } else {
                                            state_clone.total_found.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
