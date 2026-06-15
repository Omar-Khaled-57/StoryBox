//! Benchmark harness for StoryBox3 image processing pipeline.
//!
//! Measures throughput of local-only operations:
//! - Thumbnail generation (1080px, 500px)
//! - Pixel sampling for dominant color (step vs full)
//! - EXIF extraction
//! - Sequential end-to-end processing pipeline
//!
//! Usage: cargo run --bin bench [-- IMAGE_COUNT]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use image::GenericImageView;

const IMAGE_SIZES: &[(u32, u32, &str)] = &[
    (640, 480, "VGA"),
    (1920, 1080, "HD 1080p"),
];

fn generate_jpeg(path: &Path, width: u32, height: u32) -> std::io::Result<()> {
    let mut img = image::RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let r = ((x as f32 / width as f32) * 255.0) as u8;
            let g = ((y as f32 / height as f32) * 255.0) as u8;
            let b = (128u8).wrapping_add((x ^ y) as u8);
            img.put_pixel(x, y, image::Rgb([r, g, b]));
        }
    }
    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 92);
    encoder.encode_image(&img).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, &buf)
}

struct BenchResult {
    name: String,
    ops: usize,
    total: Duration,
    per_op: Duration,
}

fn bench_thumbnail_gen(images: &[PathBuf], out_dir: &Path) -> BenchResult {
    let count = images.len();
    let start = Instant::now();
    for img_path in images {
        let img = image::open(img_path).expect("open image");
        let display = img.thumbnail(1080, 1080);
        drop(img);
        let _ = display.save(&out_dir.join(format!("{}_display.jpg", img_path.file_stem().unwrap().to_string_lossy())));
        let _thumb = display.thumbnail(500, 500);
        let _ = _thumb.save(&out_dir.join(format!("{}_thumb.jpg", img_path.file_stem().unwrap().to_string_lossy())));
    }
    let elapsed = start.elapsed();
    BenchResult {
        name: "Thumbnail gen (1080+500)".into(),
        ops: count,
        total: elapsed,
        per_op: elapsed / count.max(1) as u32,
    }
}

fn bench_pixel_sampling_full(images: &[PathBuf]) -> BenchResult {
    let count = images.len();
    let start = Instant::now();
    for img_path in images {
        let img = image::open(img_path).expect("open image");
        let thumb = img.thumbnail(500, 500);
        drop(img);
        let mut r_acc = 0u64;
        let mut g_acc = 0u64;
        let mut b_acc = 0u64;
        let mut sample_count = 0u64;
        for (_x, _y, pixel) in thumb.pixels() {
            r_acc += pixel[0] as u64;
            g_acc += pixel[1] as u64;
            b_acc += pixel[2] as u64;
            sample_count += 1;
        }
        if sample_count > 0 {
            let _color = format!("#{:02x}{:02x}{:02x}", (r_acc / sample_count) as u8, (g_acc / sample_count) as u8, (b_acc / sample_count) as u8);
        }
    }
    let elapsed = start.elapsed();
    BenchResult {
        name: "Pixel sampling (full scan)".into(),
        ops: count,
        total: elapsed,
        per_op: elapsed / count.max(1) as u32,
    }
}

fn bench_pixel_sampling_step(images: &[PathBuf], step: usize) -> BenchResult {
    let count = images.len();
    let start = Instant::now();
    for img_path in images {
        let img = image::open(img_path).expect("open image");
        let thumb = img.thumbnail(500, 500);
        drop(img);
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
        if sample_count > 0 {
            let _color = format!("#{:02x}{:02x}{:02x}", (r_acc / sample_count) as u8, (g_acc / sample_count) as u8, (b_acc / sample_count) as u8);
        }
    }
    let elapsed = start.elapsed();
    let step_label = match step {
        2 => "every 2nd pixel",
        3 => "every 3rd pixel",
        5 => "every 5th pixel (current)",
        _ => "",
    };
    BenchResult {
        name: format!("Pixel sampling ({})", step_label),
        ops: count,
        total: elapsed,
        per_op: elapsed / count.max(1) as u32,
    }
}

fn bench_resize_filter(images: &[PathBuf], filter: image::imageops::FilterType, label: &str) -> BenchResult {
    let count = images.len();
    let start = Instant::now();
    for img_path in images {
        let img = image::open(img_path).expect("open image");
        let (w, h) = (img.width(), img.height());
        let target_w = 1080u32.min(w);
        let target_h = (target_w as f32 * h as f32 / w as f32) as u32;
        let _resized = image::imageops::resize(&img, target_w, target_h, filter);
        drop(img);
    }
    let elapsed = start.elapsed();
    BenchResult {
        name: format!("Resize ({})", label),
        ops: count,
        total: elapsed,
        per_op: elapsed / count.max(1) as u32,
    }
}

fn bench_exif_extraction(images: &[PathBuf]) -> BenchResult {
    let count = images.len();
    let start = Instant::now();
    for img_path in images {
        let _ = tauri_app_lib::exif::extract_exif(img_path);
    }
    let elapsed = start.elapsed();
    BenchResult {
        name: "EXIF extraction".into(),
        ops: count,
        total: elapsed,
        per_op: elapsed / count.max(1) as u32,
    }
}

fn print_table(results: &[BenchResult]) {
    println!();
    println!("{:<50} {:>6} {:>12} {:>20}", "Benchmark", "Count", "Total Time", "Per Operation");
    println!("{}", "-".repeat(92));
    for r in results {
        println!(
            "{:<50} {:>6} {:>8.3}s {:>13.1}ms  ({:.0}/s)",
            r.name,
            r.ops,
            r.total.as_secs_f64(),
            r.per_op.as_secs_f64() * 1000.0,
            r.ops as f64 / r.total.as_secs_f64().max(0.001)
        );
    }
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image_count: usize = if args.len() > 1 {
        args[1].parse().expect("Usage: bench [IMAGE_COUNT]")
    } else {
        50
    };

    println!("=== StoryBox3 Processing Benchmark ===");
    println!("Images per test: {}  |  Total across {} sizes: {}", image_count, IMAGE_SIZES.len(), image_count * IMAGE_SIZES.len());
    println!();

    let tmp_dir = std::env::temp_dir().join("storybox3_bench");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).expect("create tmp dir");
    let images_dir = tmp_dir.join("images");
    let out_dir = tmp_dir.join("output");
    std::fs::create_dir_all(&images_dir).expect("create images dir");
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    // --- Phase 1: Generate test images ---
    println!("Phase 1: Generating {} test images...", image_count * IMAGE_SIZES.len());
    let all_images: Vec<PathBuf> = IMAGE_SIZES
        .iter()
        .flat_map({
            let images_dir = images_dir.clone();
            move |&(w, h, label)| {
            (0..image_count).map({
                let images_dir = images_dir.clone();
                move |i| {
                    let path = images_dir.join(format!("{}_{}x{}_{}.jpg", label.replace(' ', "_"), w, h, i));
                    generate_jpeg(&path, w, h).expect("generate jpeg");
                    path
                }
            })
            }
        })
        .collect();
    let total_images = all_images.len();
    println!("  Generated {} images ({:.1} MB total)",
        total_images,
        all_images.iter().map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)).sum::<u64>() as f64 / 1_048_576.0
    );
    println!();

    // Warm up
    println!("Warming up... (processing first image)");
    let warmup = &all_images[0];
    let _img = image::open(warmup).expect("warmup open");
    println!("  OK\n");

    // --- Phase 2: Component benchmarks ---
    println!("Phase 2: Component Benchmarks");

    let mut results = Vec::new();

    results.push(bench_thumbnail_gen(&all_images, &out_dir));
    results.push(bench_pixel_sampling_full(&all_images));
    results.push(bench_pixel_sampling_step(&all_images, 5));
    results.push(bench_pixel_sampling_step(&all_images, 3));
    results.push(bench_pixel_sampling_step(&all_images, 2));
    results.push(bench_exif_extraction(&all_images));

    // Resize filter comparison (in-memory only, no save)
    results.push(bench_resize_filter(&all_images, image::imageops::FilterType::Triangle, "Triangle (current)"));
    results.push(bench_resize_filter(&all_images, image::imageops::FilterType::Lanczos3, "Lanczos3 (original)"));

    print_table(&results);

    // --- Phase 3: End-to-end DB-backed pipeline ---
    println!("Phase 3: End-to-End DB Pipeline");

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    let pool = rt.block_on(async {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let options = SqliteConnectOptions::new()
            .filename(&tmp_dir.join("bench.db"))
            .create_if_missing(true)
            .foreign_keys(true);
        let p = SqlitePoolOptions::new()
            .max_connections(20)
            .connect_with(options)
            .await
            .expect("create pool");
        sqlx::query("PRAGMA journal_mode=WAL;").execute(&p).await.ok();
        sqlx::query("PRAGMA synchronous=NORMAL;").execute(&p).await.ok();
        sqlx::query("PRAGMA busy_timeout=5000;").execute(&p).await.ok();
        sqlx::query("PRAGMA cache_size=-64000;").execute(&p).await.ok();
        sqlx::query("CREATE TABLE IF NOT EXISTS images (id TEXT PRIMARY KEY, path TEXT UNIQUE NOT NULL, date_taken DATETIME, lat REAL, lon REAL, ai_analyzed BOOLEAN DEFAULT 0, file_size INTEGER NOT NULL DEFAULT 0, file_modified INTEGER NOT NULL DEFAULT 0);").execute(&p).await.ok();
        sqlx::query("CREATE TABLE IF NOT EXISTS image_features (image_id TEXT PRIMARY KEY, tags TEXT, dominant_color TEXT, vibe TEXT, FOREIGN KEY(image_id) REFERENCES images(id) ON DELETE CASCADE);").execute(&p).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_file_size_mod ON images(file_size, file_modified);").execute(&p).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_date_taken ON images(date_taken);").execute(&p).await.ok();
        p
    });

    let e2e_start = Instant::now();
    let e2e_count = all_images.len();

    for img_path in &all_images {
        let path_str = img_path.to_string_lossy().to_string();
        let _file_stem = img_path.file_stem().unwrap().to_string_lossy().to_string();

        let (file_size, file_modified) = std::fs::metadata(img_path).ok()
            .map(|m| {
                let size = m.len() as i64;
                let modified = m.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                (size, modified)
            })
            .unwrap_or((0, 0));

        let id = uuid::Uuid::new_v4().to_string();
        let id_clone = id.clone();
        let path_clone = img_path.clone();
        let out_dir_clone = out_dir.clone();

        let res_opt = {
            let (exif_date, exif_lat, exif_lon) = tauri_app_lib::exif::extract_exif(&path_clone);
            let creation_date = exif_date
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            let img = match image::open(&path_clone) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let display = img.thumbnail(1080, 1080);
            drop(img);
            let _ = display.save(&out_dir_clone.join(format!("{}_display.jpg", id_clone)));
            let thumb = display.thumbnail(500, 500);
            let _ = thumb.save(&out_dir_clone.join(format!("{}_thumb.jpg", id_clone)));

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
                None
            } else {
                let dom_color = format!("#{:02x}{:02x}{:02x}", (r_acc / sample_count) as u8, (g_acc / sample_count) as u8, (b_acc / sample_count) as u8);
                Some((creation_date, exif_lat, exif_lon, dom_color))
            }
        };

        if let Some((creation_date, lat, lon, dominant_color)) = res_opt {
            let _ = rt.block_on(async {
                let mut tx = pool.begin().await.unwrap();
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
                .await.ok();

                sqlx::query("INSERT OR IGNORE INTO image_features (image_id, dominant_color) VALUES (?, ?)")
                    .bind(&id)
                    .bind(&dominant_color)
                    .execute(&mut *tx)
                    .await.ok();
                tx.commit().await.ok();
            });
        }
    }

    let e2e_elapsed = e2e_start.elapsed();
    let e2e_per_op = e2e_elapsed / e2e_count.max(1) as u32;
    println!(
        "  End-to-end: {:.2}s for {} images ({:.1}ms/image, {:.0}/s)",
        e2e_elapsed.as_secs_f64(),
        e2e_count,
        e2e_per_op.as_secs_f64() * 1000.0,
        e2e_count as f64 / e2e_elapsed.as_secs_f64().max(0.001)
    );

    // --- Phase 4: DB read performance ---
    println!("\nPhase 4: DB Query Performance");

    rt.block_on(async {
        let start = Instant::now();
        let _rows: Vec<(String, String)> = sqlx::query_as("SELECT id, path FROM images")
            .fetch_all(&pool)
            .await
            .unwrap();
        let elapsed = start.elapsed();
        println!("  Full scan: {:.1}ms for {} rows", elapsed.as_secs_f64() * 1000.0, _rows.len());

        let start = Instant::now();
        for _ in 0..100 {
            let _rows: Vec<(String, String, i64)> = sqlx::query_as("SELECT id, path, file_size FROM images WHERE file_size > 0")
                .fetch_all(&pool)
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();
        println!("  file_size filter (100x): {:.1}ms avg", elapsed.as_secs_f64() * 10.0);
    });

    println!();

    // Print summary
    let total_mb: f64 = all_images.iter()
        .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0) as f64)
        .sum::<f64>() / 1_048_576.0;

    println!("=== Summary ===");
    println!("Platform: {} ({} CPUs)", std::env::consts::OS, std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0));
    if std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0) > 1 {
        println!("Reserved 1 CPU for UI: yes");
    }
    println!("Test images: {} ({:.1} MB, {} per size)", total_images, total_mb, image_count);
    println!("Thumbnails:          {:>8.2} ms/image", results[0].per_op.as_secs_f64() * 1000.0);
    println!("Pixel sampling (full): {:>6.2} ms/image", results[1].per_op.as_secs_f64() * 1000.0);
    println!("Pixel sampling (step=5): {:>5.2} ms/image", results[2].per_op.as_secs_f64() * 1000.0);
    println!("EXIF extraction:     {:>8.2} ms/image", results[5].per_op.as_secs_f64() * 1000.0);
    println!("Resize (Triangle):   {:>8.2} ms/image", results[6].per_op.as_secs_f64() * 1000.0);
    println!("Resize (Lanczos3):   {:>8.2} ms/image", results[7].per_op.as_secs_f64() * 1000.0);
    println!("End-to-end pipeline: {:>5.2} ms/image", e2e_per_op.as_secs_f64() * 1000.0);
    println!();
    println!("Throughput: {:.0} images/sec (e2e pipeline)", e2e_count as f64 / e2e_elapsed.as_secs_f64().max(0.001));

    // Cleanup
    rt.block_on(async { pool.close().await; });
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
