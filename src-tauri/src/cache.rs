use lru::LruCache;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedImage {
    pub id: String,
    pub path: PathBuf,
    pub date_taken: Option<String>,
    pub ai_analyzed: bool,
    pub tags: Vec<String>,
    pub vibe: Option<String>,
    pub dominant_color: Option<String>,
}

/// Bounded in-memory image cache with LRU eviction.
/// Default capacity: 5,000 entries (~200-300 MB on desktop, ~50 MB on mobile).
/// Thread-safe via Mutex<LruCache>.
pub struct ImageCache {
    cache: Mutex<LruCache<String, CachedImage>>,
    pub total_count: AtomicUsize,
    pub analyzed_count: AtomicUsize,
}

impl ImageCache {
    /// Desktop: 10,000 entries. Mobile: 2,500 entries.
    fn capacity() -> NonZeroUsize {
        if cfg!(any(target_os = "android", target_os = "ios")) {
            NonZeroUsize::new(2500).unwrap()
        } else {
            NonZeroUsize::new(10000).unwrap()
        }
    }

    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(Self::capacity())),
            total_count: AtomicUsize::new(0),
            analyzed_count: AtomicUsize::new(0),
        }
    }

    /// Reloads all entries from the database into the cache.
    /// Evicts the oldest entries if the cache exceeds capacity.
    pub async fn load_from_db(&self, pool: &Pool<Sqlite>) {
        let rows: Vec<(String, String, Option<String>, bool, Option<String>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT i.id, i.path, i.date_taken, i.ai_analyzed,
                        f.tags, f.vibe, f.dominant_color
                 FROM images i
                 LEFT JOIN image_features f ON f.image_id = i.id"
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        let mut cache = self.cache.lock().unwrap();
        cache.clear();

        let mut total = 0usize;
        let mut analyzed = 0usize;

        for (id, path, date_taken, ai_analyzed, tags_json, vibe, dominant_color) in rows {
            let tags: Vec<String> = tags_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default();

            if ai_analyzed {
                analyzed += 1;
            }
            total += 1;

            cache.put(id.clone(), CachedImage {
                id,
                path: PathBuf::from(path),
                date_taken,
                ai_analyzed,
                tags,
                vibe,
                dominant_color,
            });
        }

        self.total_count.store(total, Ordering::Relaxed);
        self.analyzed_count.store(analyzed, Ordering::Relaxed);
    }

    pub fn get(&self, id: &str) -> Option<CachedImage> {
        self.cache.lock().unwrap().get(id).cloned()
    }

    /// Returns `count` random analyzed images. Avoids cloning the entire cache.
    pub fn get_random_analyzed(&self, count: usize) -> Vec<CachedImage> {
        use rand::seq::SliceRandom;
        let cache = self.cache.lock().unwrap();
        // Collect analyzed entries only
        let analyzed: Vec<&CachedImage> = cache.iter().filter(|(_, img)| img.ai_analyzed).map(|(_, v)| v).collect();
        let mut rng = rand::thread_rng();
        let chosen: Vec<CachedImage> = analyzed.choose_multiple(&mut rng, count).map(|&img| img.clone()).collect();
        chosen
    }

    pub fn insert(&self, image: CachedImage) {
        let mut cache = self.cache.lock().unwrap();
        let added = cache.put(image.id.clone(), image);
        // If the entry already existed, don't double-count
        if added.is_none() {
            self.total_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Update the metadata for an existing entry (e.g. after AI analysis completes).
    pub fn update_analysis(&self, id: &str, tags: Vec<String>, vibe: Option<String>, dominant_color: Option<String>) {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(id) {
            entry.ai_analyzed = true;
            entry.tags = tags;
            entry.vibe = vibe;
            entry.dominant_color = dominant_color;
        }
        // If not in cache (evicted), no need to insert — next load_from_db will pick it up
    }

    pub fn remove(&self, id: &str) {
        let mut cache = self.cache.lock().unwrap();
        if cache.pop(id).is_some() {
            let old_total = self.total_count.fetch_sub(1, Ordering::Relaxed);
            let _ = old_total; // allow underflow only if bug
        }
    }

    pub fn all_paths(&self) -> Vec<PathBuf> {
        self.cache
            .lock()
            .unwrap()
            .iter()
            .map(|(_, img)| img.path.clone())
            .collect()
    }
}
