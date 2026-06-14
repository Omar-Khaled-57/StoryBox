use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose::STANDARD};
/// Story generation, management, and automation.
///
/// Handles:
/// - Creating stories from analyzed images (theme-based grouping)
/// - Caption generation via AI (OpenRouter/Ollama)
/// - CRUD operations for stories (list, delete, pin, favorite)
/// - Automation: periodic story generation and cleanup of old unpinned stories
/// - AI settings read/write (provider config, proxy URL, intervals)
use crate::ai::Storyteller;
use crate::processor::ProcessingState;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageRecord {
    pub id: String,
    pub path: String,
    pub date_taken: Option<String>,
    pub ai_analyzed: bool,
    pub tags: Option<Vec<String>>,
    pub vibe: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub theme_type: String,
    pub caption: String,
    pub created_at: String,
    pub images: Vec<ImageRecord>,
    pub is_favorite: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiStatus {
    pub total_images: i64,
    pub analyzed_images: i64,
    pub pending_images: i64,
    pub is_mock: bool,
    pub engine_name: String,
    pub is_indexing_paused: bool,
    pub is_analysis_paused: bool,
    pub total_found: usize,
    pub indexed_count: usize,
}

fn get_color_vibe(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return "memorable".to_string(); }
    
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);

    // Simplistic mapping
    if r > 200 && g > 150 && b < 100 { "Golden Hour".into() }
    else if r > 200 && g < 100 && b < 100 { "Crimson Tones".into() }
    else if r < 100 && g > 150 && b < 100 { "Lush Greenery".into() }
    else if r < 100 && g < 150 && b > 200 { "Deep Ocean".into() }
    else if r > 180 && g > 180 && b > 180 { "Bright & Airy".into() }
    else if r < 50 && g < 50 && b < 50 { "Moody Noir".into() }
    else if (r as i16 - g as i16).abs() < 20 && (g as i16 - b as i16).abs() < 20 { "Monochrome".into() }
    else { "Vibrant".into() }
}

/// Pick a smart caption based on the tags collected from story images
fn build_caption(all_tags: &[String], vibe: Option<&str>, color_vibe: Option<&str>) -> String {
    let vibe_word = vibe.unwrap_or("memorable");
    let color_word = color_vibe.unwrap_or("timeless");

    if all_tags.is_empty() {
        return format!("A {} chapter in {}", vibe_word, color_word);
    }

    // Find top 2 most common tags
    let mut counts = std::collections::HashMap::new();
    for tag in all_tags {
        *counts.entry(tag.as_str()).or_insert(0u32) += 1;
    }
    
    let mut sorted_tags: Vec<(&&str, &u32)> = counts.iter().collect();
    sorted_tags.sort_by(|a, b| b.1.cmp(a.1));

    let t1 = sorted_tags.get(0).map(|(t, _)| **t).unwrap_or("visual moments");
    let t2 = sorted_tags.get(1).map(|(t, _)| **t);

    let templates = match (t1, t2) {
        (a, Some(b)) => vec![
            format!("{} & {}: A {} narrative", a, b, color_word),
            format!("The {} side of {} and {}", vibe_word, a, b),
            format!("{} and {}: Captured in a {} light", a, b, vibe_word),
            format!("Where {} meets {}: A {} composition", a, b, color_word),
            format!("Exploring {} through a {} lens", a, color_word),
            format!("Atmospheric fragments of {} & {}", a, b),
        ],
        (a, None) => vec![
            format!("The essence of {}: {}", a, vibe_word),
            format!("{} in {}, captured", a, color_word),
            format!("A {} perspective on {}", vibe_word, a),
            format!("Just {}. {}", a, color_word),
            format!("Finding the beauty in {}", a),
            format!("A quiet observation of {}", a),
        ]
    };

    use rand::Rng;
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..templates.len());
    templates[idx].clone()
}

/// Generate a random story from unfiltered images in the DB.
/// Generates a template-based story using the most recent analyzed images, excluding
/// images from disabled folders or overridden child folders.
pub async fn generate_random_story(pool: &Pool<Sqlite>) -> Result<Option<Story>> {
    // Pick up to 8 random images, excluding any from disabled folders or overridden child folders
    let images: Vec<(String, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, path, date_taken, ai_analyzed FROM images
         WHERE NOT EXISTS (
             SELECT 1 FROM scanned_folders sf
             WHERE sf.is_enabled = 0
             AND (images.path = sf.path OR images.path LIKE sf.path || '/' || '%' OR images.path LIKE sf.path || '\\' || '%')
         )
         AND NOT EXISTS (
             SELECT 1 FROM folder_child_overrides fco
             WHERE fco.is_disabled = 1
             AND (images.path = fco.parent_path || '/' || fco.child_name
                  OR images.path LIKE fco.parent_path || '/' || fco.child_name || '/' || '%'
                  OR images.path = fco.parent_path || '\\' || fco.child_name
                  OR images.path LIKE fco.parent_path || '\\' || fco.child_name || '\\' || '%')
         )
         ORDER BY RANDOM() LIMIT 8",
    )
    .fetch_all(pool)
    .await?;

    if images.is_empty() {
        return Ok(None);
    }

    // Pull AI features for all images in one batch query
    let ids: Vec<String> = images.iter().map(|(id, _, _, _)| format!("'{}'", id)).collect();
    let features_rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        &format!("SELECT image_id, tags, vibe FROM image_features WHERE image_id IN ({})", ids.join(","))
    )
    .fetch_all(pool)
    .await?;

    let mut features_map: std::collections::HashMap<String, (Option<String>, Option<String>)> = std::collections::HashMap::new();
    for (img_id, tags_json, vibe) in features_rows {
        features_map.insert(img_id, (tags_json, vibe));
    }

    let mut story_images: Vec<ImageRecord> = Vec::new();
    let mut all_tags: Vec<String> = Vec::new();
    let mut last_vibe: Option<String> = None;

    for (id, path, date_taken, ai_analyzed) in images {
        let (tags_json, vibe) = features_map.remove(&id).unwrap_or((None, None));
        let parsed_tags: Vec<String> = tags_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();
        all_tags.extend(parsed_tags.clone());
        if vibe.is_some() { last_vibe = vibe.clone(); }
        story_images.push(ImageRecord { id, path, date_taken, ai_analyzed, tags: Some(parsed_tags), vibe });
    }

    let caption = build_caption(&all_tags, last_vibe.as_deref(), None);
    let story_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Persist story
    sqlx::query(
        "INSERT INTO stories (id, theme_type, caption, created_at) VALUES (?, 'random', ?, ?)",
    )
    .bind(&story_id)
    .bind(&caption)
    .bind(&now)
    .execute(pool)
    .await?;

    // Persist story_images mapping
    for (i, img) in story_images.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO story_images (story_id, image_id, sequence_order) VALUES (?, ?, ?)"
        )
        .bind(&story_id)
        .bind(&img.id)
        .bind(i as i32)
        .execute(pool)
        .await?;
    }

    Ok(Some(Story {
        id: story_id,
        theme_type: "random".to_string(),
        caption,
        created_at: now,
        images: story_images,
        is_favorite: false,
        is_pinned: false,
    }))
}

/// Helper to parse tags from JSON
fn parse_tags(tags_json: Option<String>) -> Vec<String> {
    tags_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// Generate a story based on AI analysis or date clusters
/// Generates an AI-powered story by analyzing image features, asking the AI for a
/// caption, and assembling a story from the best images. Excludes images from disabled folders.
pub async fn generate_ai_story(pool: &Pool<Sqlite>, app: &tauri::AppHandle) -> Result<Option<Story>> {
    // 1. Fetch analyzed images, excluding any from disabled folders or overridden child folders
    let features: Vec<(String, String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT f.image_id, i.path, i.date_taken, f.tags, f.dominant_color FROM image_features f
         JOIN images i ON f.image_id = i.id
         WHERE NOT EXISTS (
             SELECT 1 FROM scanned_folders sf
             WHERE sf.is_enabled = 0
             AND (i.path = sf.path OR i.path LIKE sf.path || '/' || '%' OR i.path LIKE sf.path || '\\' || '%')
         )
         AND NOT EXISTS (
             SELECT 1 FROM folder_child_overrides fco
             WHERE fco.is_disabled = 1
             AND (i.path = fco.parent_path || '/' || fco.child_name
                  OR i.path LIKE fco.parent_path || '/' || fco.child_name || '/' || '%'
                  OR i.path = fco.parent_path || '\\' || fco.child_name
                  OR i.path LIKE fco.parent_path || '\\' || fco.child_name || '\\' || '%')
         )"
    )
    .fetch_all(pool)
    .await?;

    if features.is_empty() {
        return Ok(None);
    }

    // 2. Clustering Strategy (Date vs Tag)
    let mut date_clusters: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut tag_counts = std::collections::HashMap::new();

    for (id, _, date_taken, tags_json, _) in &features {
        // Date clustering (YYYY-MM-DD or YYYY-MM)
        if let Some(dt) = date_taken {
            let day = &dt[..10.min(dt.len())]; // Daily cluster
            date_clusters.entry(day.to_string()).or_default().push(id.clone());
        }

        // Tag counts
        let tags = parse_tags(tags_json.clone());
        for tag in tags {
            *tag_counts.entry(tag).or_insert(0u32) += 1;
        }
    }

    // Filter clusters with 3+ images
    let mut viable_dates: Vec<(String, usize)> = date_clusters.iter()
        .filter(|(_, v)| v.len() >= 3)
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();
    
    // Sort by size or recency
    viable_dates.sort_by(|a, b| b.0.cmp(&a.0)); 

    let (theme, selected_ids) = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        if !viable_dates.is_empty() && rng.gen_bool(0.6) {
            // Option A: Date-based cluster (Likely an event)
            let idx = rng.gen_range(0..viable_dates.len().min(3));
            let date_key = &viable_dates[idx].0;
            let ids = date_clusters.get(date_key).unwrap().clone();
            let theme_templates = vec![
                format!("A day to remember: {}", date_key),
                format!("Moments from {}", date_key),
                format!("Captured on {}", date_key),
                format!("Recalling {}", date_key),
            ];
            let theme = theme_templates[rng.gen_range(0..theme_templates.len())].clone();
            (theme, ids)
        } else {
            // Option B: Tag-based cluster
            let mut sorted_tags: Vec<(String, u32)> = tag_counts.into_iter().collect();
            sorted_tags.sort_by(|a, b| b.1.cmp(&a.1));
            
            let pool_size = sorted_tags.len().min(5);
            if pool_size == 0 { return Ok(None); }
            let theme_tag = sorted_tags[rng.gen_range(0..pool_size)].0.clone();
            
            let ids = features.iter()
                .filter(|(_, _, _, tj, _)| parse_tags(tj.clone()).contains(&theme_tag))
                .map(|(id, _, _, _, _)| id.clone())
                .collect();
            (theme_tag, ids)
        }
    };

    // 3. Shuffle and pick up to 8
    use rand::seq::SliceRandom;
    let mut final_ids = selected_ids;
    {
        let mut rng = rand::thread_rng();
        final_ids.shuffle(&mut rng);
    }
    let selected_ids = &final_ids[..final_ids.len().min(8)];

    // 4. Build story images and collect context for AI — batched queries
    let ids_str: Vec<String> = selected_ids.iter().map(|id| format!("'{}'", id)).collect();
    let id_list = ids_str.join(",");

    let img_rows: Vec<(String, String, Option<String>, bool)> = sqlx::query_as(
        &format!("SELECT id, path, date_taken, ai_analyzed FROM images WHERE id IN ({})", id_list)
    )
    .fetch_all(pool)
    .await?;

    let feat_rows: Vec<(String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        &format!("SELECT image_id, tags, dominant_color, vibe FROM image_features WHERE image_id IN ({})", id_list)
    )
    .fetch_all(pool)
    .await?;

    let feat_map: std::collections::HashMap<String, (Option<String>, Option<String>, Option<String>)> =
        feat_rows.into_iter().map(|(id, t, c, v)| (id, (t, c, v))).collect();

    let mut story_images = Vec::new();
    let mut all_tags = Vec::new();
    let mut last_vibe = None;
    let mut local_color_counts = std::collections::HashMap::new();

    for (img_id, path, date_taken, ai_analyzed) in img_rows {
        let (tags_json, color_hex, vibe) = feat_map.get(&img_id).cloned().unwrap_or((None, None, None));
        let tags = parse_tags(tags_json);
        all_tags.extend(tags.clone());
        if vibe.is_some() { last_vibe = vibe.clone(); }
        if let Some(hex) = &color_hex {
            *local_color_counts.entry(hex.clone()).or_insert(0u32) += 1;
        }

        story_images.push(ImageRecord {
            id: img_id,
            path,
            date_taken,
            ai_analyzed,
            tags: Some(tags),
            vibe,
        });
    }

    let mut story_colors: Vec<(&String, &u32)> = local_color_counts.iter().collect();
    story_colors.sort_by(|a, b| b.1.cmp(a.1));
    let color_vibe = story_colors.get(0).map(|(c, _)| get_color_vibe(c));

    // 5. Immediate Return: Template Caption
    let template_caption = build_caption(&all_tags, last_vibe.as_deref(), color_vibe.as_deref());
    let story_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO stories (id, theme_type, caption, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&story_id)
    .bind(&theme)
    .bind(&template_caption)
    .bind(&now)
    .execute(pool)
    .await?;

    for (i, img) in story_images.iter().enumerate() {
        sqlx::query("INSERT OR IGNORE INTO story_images (story_id, image_id, sequence_order) VALUES (?, ?, ?)")
            .bind(&story_id)
            .bind(&img.id)
            .bind(i as i32)
            .execute(pool)
            .await?;
    }

    // 6. Background Enrichment: AI Caption
    let pool_bg = pool.clone();
    let app_bg = app.clone();
    let story_id_bg = story_id.clone();
    let all_tags_bg = all_tags.clone();
    let last_vibe_bg = last_vibe.clone();
    let color_vibe_bg = color_vibe.clone();

    tokio::spawn(async move {
        let settings: Result<(String, String, String, String, String), _> = sqlx::query_as("SELECT provider, base_url, model_name, api_key, proxy_url FROM ai_settings WHERE id = 1")
            .fetch_one(&pool_bg)
            .await;
        
        if let Ok(s) = settings {
            let teller = Storyteller::new(s.0, s.1, s.2, s.3, s.4);
            if let Some(ai_caption) = teller.generate_caption(&all_tags_bg, last_vibe_bg.as_deref(), color_vibe_bg.as_deref()).await {
                let _ = sqlx::query("UPDATE stories SET caption = ? WHERE id = ?")
                    .bind(&ai_caption)
                    .bind(&story_id_bg)
                    .execute(&pool_bg)
                    .await;
                
                use tauri::Emitter;
                let _ = app_bg.emit("story-updated", serde_json::json!({
                    "id": story_id_bg,
                    "caption": ai_caption
                }));
            }
        }
    });

    Ok(Some(Story {
        id: story_id,
        theme_type: theme,
        caption: template_caption,
        created_at: now,
        images: story_images,
        is_favorite: false,
        is_pinned: false,
    }))
}

/// Fetch all stories with their images — uses a single JOIN query (O(1) DB round-trips).
pub async fn get_all_stories(pool: &Pool<Sqlite>) -> Result<Vec<Story>> {
    // Single query: stories → story_images → images → image_features in one JOIN
    let rows: Vec<(String, String, String, String, bool, bool, String, String, Option<String>, bool, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT s.id, s.theme_type, COALESCE(s.caption, ''), s.created_at,
                s.is_favorite, s.is_pinned,
                i.id, i.path, i.date_taken, i.ai_analyzed,
                f.tags, f.vibe
         FROM stories s
         JOIN story_images si ON si.story_id = s.id
         JOIN images i ON i.id = si.image_id
         LEFT JOIN image_features f ON f.image_id = i.id
         ORDER BY s.is_pinned DESC, s.created_at DESC, si.sequence_order
         LIMIT 200"
    )
    .fetch_all(pool)
    .await?;

    let mut stories: Vec<Story> = Vec::new();

    for (s_id, theme_type, caption, created_at, is_favorite, is_pinned,
         i_id, i_path, i_date_taken, i_ai_analyzed, tags_json, vibe) in rows {

        let story = stories.last_mut().and_then(|s| if s.id == s_id { Some(s) } else { None });

        let tags: Vec<String> = tags_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        if let Some(story) = story {
            story.images.push(ImageRecord {
                id: i_id,
                path: i_path,
                date_taken: i_date_taken,
                ai_analyzed: i_ai_analyzed,
                tags: Some(tags),
                vibe,
            });
        } else {
            stories.push(Story {
                id: s_id,
                theme_type,
                caption,
                created_at,
                images: vec![ImageRecord {
                    id: i_id,
                    path: i_path,
                    date_taken: i_date_taken,
                    ai_analyzed: i_ai_analyzed,
                    tags: Some(tags),
                    vibe,
                }],
                is_favorite,
                is_pinned,
            });
        }
    }

    Ok(stories)
}

// --- Tauri Commands ---

/// Attempts AI story generation; falls back to template-based if AI is mock or fails.
pub async fn internal_generate_story(
    pool: &Pool<Sqlite>,
    app: &tauri::AppHandle
) -> Result<Option<Story>, String> {
    // 1. Check if genuine AI is configured (not mock)
    let provider: Result<(String,), _> = sqlx::query_as("SELECT provider FROM ai_settings WHERE id = 1")
        .fetch_one(pool)
        .await;

    let ai_configured = matches!(&provider, Ok((p,)) if p != "mock");

    if !ai_configured {
        println!("[Stories] AI provider is mock or unconfigured. Falling back to template-based random story.");
        return generate_random_story(pool)
            .await
            .map_err(|e| e.to_string());
    }

    // 2. Try AI generation with a 45-second global timeout (increased for reliability)
    println!("[Stories] AI configured, starting AI story generation...");
    let ai_future = generate_ai_story(pool, app);
    match tokio::time::timeout(std::time::Duration::from_secs(45), ai_future).await {
        Ok(Ok(Some(story))) => {
            println!("[Stories] Success: AI story '{}' generated.", story.caption);
            Ok(Some(story))
        },
        Ok(Ok(None)) | Ok(Err(_)) => {
            println!("[Stories] AI generation yielded no results or failed. Falling back to template-based random story.");
            generate_random_story(pool).await.map_err(|e| e.to_string())
        }
        Err(_) => {
            println!("[Stories] AI generation timed out (45s). Falling back to template-based random story.");
            generate_random_story(pool).await.map_err(|e| e.to_string())
        }
    }
}

/// Entry point for story generation: routes to AI or template depending on config.
#[tauri::command]
pub async fn generate_story(
    state: tauri::State<'_, Pool<Sqlite>>,
    app: tauri::AppHandle,
) -> Result<Option<Story>, String> {
    internal_generate_story(&*state, &app).await
}

/// Returns all stories with their images for the frontend feed.
#[tauri::command]
pub async fn get_stories(state: tauri::State<'_, Pool<Sqlite>>) -> Result<Vec<Story>, String> {
    get_all_stories(&*state).await.map_err(|e| e.to_string())
}

/// Returns a base64 data URI for an image (thumbnail or display) by id.
/// Falls back to reading the original file if the cache is missing.
#[tauri::command]
pub async fn get_cached_image_base64(
    state: tauri::State<'_, Pool<Sqlite>>,
    app: tauri::AppHandle, 
    id: String, 
    image_type: String
) -> Result<String, String> {
    use tauri::Manager;
    let pool = &*state;
    
    // 1. Try to find the file in the cache directory
    let app_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let sub_dir = match image_type.as_str() {
        "thumb" => "thumbnails",
        "display" => "display",
        _ => return Err("Invalid image_type (thumb/display)".to_string()),
    };
    
    let file_path = app_dir.join(sub_dir).join(format!("{}.jpg", id));
    
    if file_path.exists() {
        let bytes = tokio::fs::read(&file_path).await.map_err(|e| e.to_string())?;
        return Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)));
    }

    // Fallback: This image is indexed but its thumbnail isn't safe yet.
    // Instead of an SVG placeholder, let's try to get the original path and read it directly.
    // This is slower but avoids placeholders.
    let original_path: String = sqlx::query_scalar("SELECT path FROM images WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    if std::path::Path::new(&original_path).exists() {
        let bytes = tokio::fs::read(&original_path).await.map_err(|e| e.to_string())?;
        // We still return it as a data URI. 
        // Note: For very large images, this might be slow/heavy, but it satisfies "no placeholders".
        let mime = if original_path.to_lowercase().ends_with(".png") { "image/png" } 
                   else if original_path.to_lowercase().ends_with(".webp") { "image/webp" }
                   else { "image/jpeg" };
        return Ok(format!("data:{};base64,{}", mime, STANDARD.encode(bytes)));
    }

    Err("Image file not found on disk and no thumbnail available.".to_string())
}

/// Returns the current AI/intelligence processing status for the UI.
#[tauri::command]
pub async fn get_ai_status(
    state: tauri::State<'_, Pool<Sqlite>>,
    proc_state: tauri::State<'_, Arc<ProcessingState>>
) -> Result<AiStatus, String> {
    let counts: (i64, i64) = sqlx::query_as("SELECT COUNT(*) as total, COALESCE(SUM(ai_analyzed), 0) as analyzed FROM images")
        .fetch_one(&*state)
        .await
        .map_err(|e| e.to_string())?;

    let total = counts.0;
    let analyzed = counts.1;

    let settings: (String, String, String) = sqlx::query_as("SELECT provider, model_name, proxy_url FROM ai_settings WHERE id = 1")
        .fetch_one(&*state)
        .await
        .map_err(|e| e.to_string())?;

    let is_mock = settings.0 == "mock";
    let engine_name = match settings.0.as_str() {
        "openrouter" => {
            if !settings.2.is_empty() {
                format!("OpenRouter via Proxy")
            } else {
                format!("OpenRouter ({})", settings.1)
            }
        }
        "ollama" => format!("Ollama ({})", settings.1),
        _ => "Mock Simulation (Rust)".to_string(),
    };

    Ok(AiStatus {
        total_images: total,
        analyzed_images: analyzed,
        pending_images: total - analyzed,
        is_mock,
        engine_name,
        is_indexing_paused: proc_state.stop_indexing.load(Ordering::Relaxed),
        is_analysis_paused: proc_state.stop_analysis.load(Ordering::Relaxed),
        total_found: proc_state.total_found.load(Ordering::Relaxed),
        indexed_count: proc_state.indexed_count.load(Ordering::Relaxed),
    })
}

/// Deletes a single story and its image associations by id.
#[tauri::command]
pub async fn delete_story(
    state: tauri::State<'_, Pool<Sqlite>>,
    id: String,
) -> Result<(), String> {
    let pool = &*state;
    
    // Delete associations first
    sqlx::query("DELETE FROM story_images WHERE story_id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete the story
    sqlx::query("DELETE FROM stories WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Deletes all non-favorited stories and their image associations.
#[tauri::command]
pub async fn delete_all_stories(
    state: tauri::State<'_, Pool<Sqlite>>,
) -> Result<(), String> {
    let pool = &*state;
    
    // Delete associations for non-favorite stories first
    sqlx::query("DELETE FROM story_images WHERE story_id IN (SELECT id FROM stories WHERE is_favorite = 0)")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Delete non-favorite stories
    sqlx::query("DELETE FROM stories WHERE is_favorite = 0")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Periodic automation: generates stories on a schedule and cleans up old unpinned stories.
pub async fn run_automation_tasks(pool: &Pool<Sqlite>, app: &tauri::AppHandle) -> Result<()> {
    // 1. Fetch current automation timers and intervals
    let row: Option<(Option<String>, Option<String>, i32, i32)> = sqlx::query_as(
        "SELECT last_auto_gen_at, last_cleanup_at, auto_gen_interval_hours, cleanup_interval_hours FROM ai_settings WHERE id = 1"
    )
    .fetch_optional(pool)
    .await?;

    let (last_gen_str, last_cleanup_str, gen_interval, cleanup_interval) = 
        row.unwrap_or((None, None, 12, 24));
    
    let now = Utc::now();
    
    // 2. Dynamic Generation logic
    let should_gen = match last_gen_str {
        Some(s) => {
            let last_gen = chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)).ok();
            match last_gen {
                Some(lg) => (now - lg).num_hours() >= gen_interval as i64,
                None => true,
            }
        },
        None => true,
    };

    if should_gen {
        println!("[Automation] Generating periodic AI story...");
        // Re-use logic for generating an AI story
        if let Ok(Some(_)) = generate_ai_story(pool, app).await {
            sqlx::query("UPDATE ai_settings SET last_auto_gen_at = ? WHERE id = 1")
                .bind(now.to_rfc3339())
                .execute(pool)
                .await?;
        }
    }

    // 3. Dynamic Cleanup logic
    let should_cleanup = match last_cleanup_str {
        Some(s) => {
            let last_cl = chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)).ok();
            match last_cl {
                Some(lc) => (now - lc).num_hours() >= cleanup_interval as i64,
                None => true,
            }
        },
        None => true,
    };

    if should_cleanup {
        println!("[Automation] Performing 24h cleanup of unpinned stories...");
        // Define "expiry" as older than 24 hours
        let expiry = now - chrono::Duration::hours(24);
        let expiry_str = expiry.to_rfc3339();

        // Batch delete expired stories
        sqlx::query("DELETE FROM story_images WHERE story_id IN (SELECT id FROM stories WHERE is_pinned = 0 AND created_at < ?)")
            .bind(&expiry_str)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM stories WHERE is_pinned = 0 AND created_at < ?")
            .bind(&expiry_str)
            .execute(pool)
            .await?;

        sqlx::query("UPDATE ai_settings SET last_cleanup_at = ? WHERE id = 1")
            .bind(now.to_rfc3339())
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Toggles the pinned state of a story and returns the new state.
#[tauri::command]
pub async fn toggle_story_pin(
    state: tauri::State<'_, Pool<Sqlite>>,
    id: String,
) -> Result<bool, String> {
    let pool = &*state;
    
    // Get current state
    let (current,): (bool,) = sqlx::query_as("SELECT is_pinned FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let next = !current;

    sqlx::query("UPDATE stories SET is_pinned = ? WHERE id = ?")
        .bind(next)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(next)
}

/// Toggles the favorite state of a story and returns the new state.
#[tauri::command]
pub async fn toggle_story_favorite(
    state: tauri::State<'_, Pool<Sqlite>>,
    id: String,
) -> Result<bool, String> {
    let pool = &*state;
    
    // Get current state
    let (current,): (bool,) = sqlx::query_as("SELECT is_favorite FROM stories WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let next = !current;

    sqlx::query("UPDATE stories SET is_favorite = ? WHERE id = ?")
        .bind(next)
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(next)
}

/// Returns the current AI provider configuration for the frontend settings form.
#[tauri::command]
pub async fn get_ai_settings(state: tauri::State<'_, Pool<Sqlite>>) -> Result<(String, String, String, String, String, String, i32, i32), String> {
    let row: (String, String, String, String, String, String, i32, i32) = sqlx::query_as(
        "SELECT provider, base_url, model_name, vision_model_name, api_key, proxy_url, auto_gen_interval_hours, cleanup_interval_hours FROM ai_settings WHERE id = 1"
    )
    .fetch_one(&*state)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row)
}

/// Persists the AI provider configuration from the frontend settings form.
#[tauri::command]
pub async fn update_ai_settings(
    state: tauri::State<'_, Pool<Sqlite>>,
    provider: String,
    base_url: String,
    model_name: String,
    vision_model_name: String,
    api_key: String,
    proxy_url: String,
    auto_gen_interval_hours: i32,
    cleanup_interval_hours: i32,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE ai_settings SET provider = ?, base_url = ?, model_name = ?, vision_model_name = ?, api_key = ?, proxy_url = ?, auto_gen_interval_hours = ?, cleanup_interval_hours = ? WHERE id = 1"
    )
    .bind(provider)
    .bind(base_url)
    .bind(model_name)
    .bind(vision_model_name)
    .bind(api_key)
    .bind(proxy_url)
    .bind(auto_gen_interval_hours)
    .bind(cleanup_interval_hours)
    .execute(&*state)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

