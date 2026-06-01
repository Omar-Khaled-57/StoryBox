/// Database initialization, schema creation, and migrations.
///
/// Manages the local SQLite database with:
/// - `images` / `image_features` tables for photo metadata and AI results
/// - `stories` / `story_images` tables for generated stories
/// - `ai_settings` table with provider config, proxy URL, and automation intervals
/// - `locations` table for watched folders
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub async fn init_db(app_handle: &AppHandle) -> Result<Pool<Sqlite>, sqlx::Error> {
    // Get the application local data directory
    let app_dir: PathBuf = app_handle
        .path()
        .app_local_data_dir()
        .expect("Failed to get app local data dir");

    // Ensure the directory exists
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create app local data dir");
    }

    let db_path = app_dir.join("storybox.db");

    // Set up connection options
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .connect_with(options)
        .await?;

    // Apply the schema
    create_schema(&pool).await?;

    Ok(pool)
}

async fn create_schema(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS images (
            id TEXT PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            date_taken DATETIME,
            lat REAL,
            lon REAL,
            ai_analyzed BOOLEAN DEFAULT 0
        );",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS image_features (
            image_id TEXT PRIMARY KEY,
            tags TEXT,
            dominant_color TEXT,
            vibe TEXT,
            embedding BLOB,
            FOREIGN KEY(image_id) REFERENCES images(id)
        );",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS stories (
            id TEXT PRIMARY KEY,
            theme_type TEXT,
            caption TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            is_favorite BOOLEAN DEFAULT 0,
            is_pinned BOOLEAN DEFAULT 0
        );",
    )
    .execute(pool)
    .await?;

    // Migration: ensure is_pinned and is_favorite exist if the table was already created
    let _ = sqlx::query("ALTER TABLE stories ADD COLUMN is_favorite BOOLEAN DEFAULT 0").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE stories ADD COLUMN is_pinned BOOLEAN DEFAULT 0").execute(pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS story_images (
            story_id TEXT,
            image_id TEXT,
            sequence_order INTEGER,
            PRIMARY KEY (story_id, image_id),
            FOREIGN KEY(story_id) REFERENCES stories(id),
            FOREIGN KEY(image_id) REFERENCES images(id)
        );",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ai_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            provider TEXT DEFAULT 'openrouter',
            base_url TEXT DEFAULT 'https://openrouter.ai/api/v1',
            model_name TEXT DEFAULT 'meta-llama/llama-3.1-8b-instruct',
            vision_model_name TEXT DEFAULT 'qwen/qwen-vl-plus',
            api_key TEXT DEFAULT '',
            proxy_url TEXT DEFAULT '',
            auto_gen_interval_hours INTEGER DEFAULT 12,
            cleanup_interval_hours INTEGER DEFAULT 24,
            last_auto_gen_at DATETIME,
            last_cleanup_at DATETIME
        );",
    )
    .execute(pool)
    .await?;

    // Migrations for older schemas
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN vision_model_name TEXT DEFAULT 'qwen/qwen-vl-plus'").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN auto_gen_interval_hours INTEGER DEFAULT 12").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN cleanup_interval_hours INTEGER DEFAULT 24").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN last_auto_gen_at DATETIME").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN last_cleanup_at DATETIME").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN api_key TEXT DEFAULT ''").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE ai_settings ADD COLUMN proxy_url TEXT DEFAULT ''").execute(pool).await;

    // Default settings entry — proxy is pre-configured so the app works out of the box
    let _ = sqlx::query("INSERT OR IGNORE INTO ai_settings (id, provider, proxy_url) VALUES (1, 'openrouter', 'https://pawylsallgbfnzzmakac.supabase.co/functions/v1/openrouter-proxy')")
        .execute(pool)
        .await?;

    // Fix existing rows that were created before proxy_url default was added
    let _ = sqlx::query("UPDATE ai_settings SET proxy_url = 'https://pawylsallgbfnzzmakac.supabase.co/functions/v1/openrouter-proxy' WHERE id = 1 AND (proxy_url IS NULL OR proxy_url = '')")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scanned_folders (
            id TEXT PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            is_enabled BOOLEAN DEFAULT 1,
            added_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS folder_child_overrides (
            parent_path TEXT NOT NULL,
            child_name TEXT NOT NULL,
            is_disabled BOOLEAN DEFAULT 1,
            PRIMARY KEY (parent_path, child_name)
        );",
    )
    .execute(pool)
    .await?;

    Ok(())
}
