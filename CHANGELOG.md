# 📋 StoryBox3 — Changelog

A running log of every change, fix, and decision during development. Newest entries are at the top.

---

## v3.1.0 — Pipeline, Intelligence & Production Polish · 2025-06-15

**Focus**: Production-grade two-stage processing pipeline, architectural modularization, AI intelligence upgrades, and comprehensive UI/UX refinement.

### 🚀 Architecture & Modularization

- **Two-stage processing pipeline** (`processor.rs`) — image indexing and AI analysis now run as independent semaphore-throttled workers connected by bounded channels. Indexing uses `num_cpus - 1` permits; analysis uses `max(2, num_cpus / 4)`.
- **Dynamic resource shifting** — when indexing reaches 90% completion, capacity is transferred from indexing to analysis via `compare_exchange` semantics, maximizing throughput without starving either stage.
- **`ImageCache`** (`cache.rs`) — LRU in-memory cache (10,000 entries desktop / 2,500 mobile) populated from DB at startup. Eliminates repetitive DB reads during story generation. Atomic counters for total/analyzed counts.
- **`FolderWatcher`** (`watcher.rs`) — `notify`-based recursive watcher for all enabled scanned folders. Automatically queues newly created or modified image files into the indexing pipeline. Desktop-only via `#[cfg]`.
- **`exif.rs`** — dedicated EXIF extraction module for `DateTimeOriginal`, GPS latitude/longitude, with graceful fallback across timestamp formats.
- **Custom React hooks** — logic extracted from `App.tsx` into 5 focused hooks: `useStories` (CRUD + pin/favorite), `useScan` (scan lifecycle), `useScannedFolders` (folder + child management), `useAiHealth` (30s polling), `useActivities` (activity tracking with auto-incrementing IDs).
- **Centralized `Icons.tsx`** — 16 named SVG icon components replacing inline markup, with consistent viewBox and stroke styling.
- **Benchmark harness** (`bin/bench.rs`) — standalone binary for synthetic performance testing of thumbnail generation, pixel sampling, EXIF extraction, resize filters, and DB throughput.

### 🤖 AI Intelligence

- **Structured vision prompt** — replaced ad-hoc one-liner with a multi-rule prompt enforcing 5 objective tags, a single-word vibe, and explicit prohibitions on full sentences, abstractions, repeated tags, and brand names (both OpenRouter and Ollama variants).
- **Enhanced caption prompt** — "cinematic photo story title" prompt requiring 3–5 natural words, forbidding clichés, hashtags, and punctuation overload.
- **Background caption enrichment** — after `generate_ai_story` persists a template caption, a background async task queries the AI provider for a polished caption, updates the DB, and emits `story-updated` to the frontend.
- **`Storyteller` struct** — encapsulates provider selection, URL construction, and API key/proxy routing for caption generation.
- **Unified `check_ai_availability`** — pings OpenRouter (proxy or direct), Ollama (`/api/tags`), or always-available Mock, returning structured availability with model-loaded flags.
- **`test_ai_generation`** — end-to-end diagnostic: picks a real image from DB, runs vision analysis, generates caption, returns full result for debugging.

### 🔧 Indexing & Analysis

- **Pause / Resume** — independent `stop_indexing`/`resume_indexing` and `stop_analysis`/`resume_analysis` commands. Workers use `tokio::sync::Notify` for zero-CPU waiting while paused.
- **Skip-detection** — on re-scan, compares `file_size + file_modified` to skip unchanged files. Partial re-indexing when metadata changes.
- **30-second per-image timeout** — slow or corrupted files emit `indexing-status` with `"skipped"` and continue without blocking the pipeline.
- **Junk reanalysis** — `trigger_junk_reanalysis` re-queues images with garbled/missing AI tags in batches of 100, regenerates thumbnails and re-runs analysis.
- **Priority-aware UI status** — `get_ai_status` surfaces `is_indexing_paused`, `is_analysis_paused`, `total_found`, `indexed_count` for real-time control panel feedback.

### 📚 Stories Engine

- **Two-path generation** — `generate_story` tries AI story first (45s timeout), falls back to template-generated random story automatically.
- **Date & tag clustering** — AI story builder groups images by date cluster (3+ same-day) or top tag frequency, picks up to 8 images with shuffle randomization.
- **Pin to Top** — `toggle_story_pin` flips pinned state; story feed sorts pinned stories first, then by `created_at` descending.
- **Favorites** — `toggle_story_favorite` with dedicated "Favorites" view in the sidebar. Heart icon badge on favorited cards.
- **Auto-generation** — configurable interval (default 12h) via `auto_gen_interval_hours`. Background loop checks every 30 minutes.
- **Auto-cleanup** — configurable expiration (default 24h) via `cleanup_interval_hours`. Batch-deletes unpinned stories older than the threshold.
- **Paginated feed** — `get_stories` supports `LIMIT`/`OFFSET` parameters (default 200/0).
- **`build_caption` template engine** — 12 caption templates keyed to tag frequency and color-vibe mapping (8 named vibes from hex colors).

### 🗄️ Database

- **Complete schema** — 7 tables: `images`, `image_features`, `stories`, `story_images`, `ai_settings`, `scanned_folders`, `folder_child_overrides` with foreign keys and composite primary keys.
- **WAL mode + 64 MB cache** — `PRAGMA journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000` for reliable concurrent access.
- **7 indexes** — on `ai_analyzed`, `file_size+file_modified`, `date_taken`, `story_id`, `image_id`, `is_pinned+created_at`, `path`, `is_enabled`.
- **Graceful migrations** — `ALTER TABLE ADD COLUMN` with error suppression for incremental schema evolution (`embedding`, `is_favorite`, `is_pinned`, `vision_model_name`, proxy/interval columns).
- **`folder_child_overrides`** — composite primary key `(parent_path, child_name)` for per-subfolder disable state.

### 🎨 UI / UX

- **ActivityIndicator with pause states** — button shows spinning progress, amber indexing-paused badge, green analysis-paused badge, or combined blue pause icon. Popup displays per-item status messages with pause overlays on progress bars.
- **Activity status panel** — real-time progress bars, pause/resume toggles, "Flash Indexing" vs "AI Queue" labels, and engine info in Settings.
- **Toast notifications** — fixed top-center toast with success/error/info variants, 4-second auto-dismiss, scale-in animation and backdrop blur.
- **Folder accessibility detection** — `check_folders_accessibility` verifies folder existence on disk; UI shows amber warning banner counting inaccessible folders and red "Missing" badges.
- **Safe area / notch support** — CSS `env(safe-area-inset-*)` applied to nav and content containers via `safe-area-nav` and `safe-area-content` classes.
- **PWA manifest** — `manifest.json` with standalone display, 3 icon sizes (192, 512, 484), theme color `#0a0a1a`, categories `photography`/`entertainment`/`lifestyle`.
- **Apple touch / mobile web** — `apple-mobile-web-app-capable`, `viewport-fit=cover`, `apple-touch-icon`, safe-area CSS variables.
- **Lazy image loading** — `IntersectionObserver` with 200px root margin defers thumbnail loading until cards enter the viewport.
- **Drag-to-scroll** — horizontal story cards container with `cursor-grab`/`cursor-grabbing` states.
- **Full-screen story viewer** — auto-advancing slideshow (5s per image), animated progress bars, tap zones for prev/next (40%/60%), press-and-hold to hide UI, keyboard arrows + Escape, full-view image mode.
- **Subfolder management UI** — expandable folder rows with individual child toggles and optimistic UI updates.
- **Story card animations** — animated gradient border (`gradientBorder` 4s infinite), hover lift, image scale, glow intensification. Cyber background with animated grid, 12 floating particles, and CRT scanline overlay.

### ⚡ Performance

- **LRU image cache** — 10,000 entries desktop / 2,500 mobile, avoids repeated DB reads and thumbnail regeneration.
- **Pixel sampling for dominant color** — every-5th-pixel sampling (claimed 96% faster than full scan).
- **Two-stage thumbnail generation** — 1080px display image first, then 500px thumbnail from display (avoids double decode). JPEG quality 85.
- **Bounded channels** — indexing channel capacity 256, analysis channel capacity 200.
- **Atomic counters** — `total_found`, `indexed_count`, `stop_indexing`, `stop_analysis` use relaxed `AtomicUsize`/`AtomicBool` — no mutex contention.
- **Early memory release** — explicit `drop(img)` after thumbnail generation frees decoded image buffers.
- **`spawn_blocking` for I/O** — all blocking operations (decode, EXIF, file writes) offloaded to Tokio's blocking thread pool.
- **`yield_now()` during directory scan** — prevents async runtime starvation on large folders (every 1000 entries).

### 📱 Mobile

- **SAF-based scanning** — replaces unreliable `@universalappfactory/tauri-plugin-medialibrary` and `@gbyte/tauri-plugin-ios-photos` with `tauri-plugin-dialog` (folder picker) + `tauri-plugin-fs` (`readDir`/`readFile`).
- **Base64 image ingestion** — `index_ios_image_data` accepts base64-encoded image data from mobile JS, writes to temporary directory, queues for indexing.
- **`#[cfg]` platform gating** — `FolderWatcher` desktop-only; mobile `internal_scan_device` emits `trigger-mobile-scan` event; cache capacity reduced on mobile.
- **iOS capability scoping** — `ios-photos:default` removed; `fs:default` with `$APPDATA`, `$HOME`, `$PICTURE`, `$DOWNLOAD`, `$DOCUMENT`, `$TEMP` paths in `capabilities/default.json`.

### 🧪 Reliability & Error Handling

- **Graceful degradation** — story generation falls back AI → template → random. Image analysis errors logged but never crash the worker. AI availability check never panics.
- **Lock poisoning recovery** — `Mutex` on cache uses `.unwrap_or_else(|e| e.into_inner())`. `RwLock` on settings propagates with `.map_err`.
- **Configurable timeouts** — vision API 60s (OpenRouter) / 90s (Ollama), caption 30s, per-image indexing 30s, story generation 45s.
- **`INSERT OR IGNORE`** — deduplication for image features, story images, and scanned folders.

### 🧹 Changes

| File | What changed |
|------|-------------|
| `src-tauri/src/cache.rs` | **New** — `ImageCache` with `LRU`, atomic counters, DB load |
| `src-tauri/src/watcher.rs` | **New** — `FolderWatcher` via `notify`, desktop-only |
| `src-tauri/src/exif.rs` | **New** — EXIF date + GPS extraction |
| `src-tauri/src/bin/bench.rs` | **New** — performance benchmark harness |
| `src/components/Icons.tsx` | **New** — 16 named SVG icons |
| `src/hooks/useStories.ts` | **New** — story CRUD + pin/favorite |
| `src/hooks/useScan.ts` | **New** — scan lifecycle state |
| `src/hooks/useScannedFolders.ts` | **New** — folder + child management |
| `src/hooks/useAiHealth.ts` | **New** — 30s AI health polling |
| `src/hooks/useActivities.ts` | **New** — activity tracking |
| `src/index.css` | **New** — design system + cyber effects |
| `public/manifest.json` | **New** — PWA manifest with icons |
| `src-tauri/src/processor.rs` | Rewritten two-stage pipeline, pause/resume, semaphore scaling |
| `src-tauri/src/stories.rs` | AI + template generation, pin, favorite, enrichment, automation |
| `src-tauri/src/ai.rs` | Structured vision/caption prompts, Storyteller, availability check |
| `src-tauri/src/db.rs` | Full schema, WAL, 7 indexes, migrations, child overrides table |
| `src-tauri/src/scanner.rs` | Folder CRUD, accessibility check, mobile path gating |
| `src-tauri/src/lib.rs` | Cache/watcher/worker integration, automation loop, 35 commands |
| `src/App.tsx` | Hook-based refactor, pause state polling, mobile SAF scanning |
| `src/components/ActivityIndicator.tsx` | Pause badges, status messages, portal popup |
| `src/components/HomeFeed.tsx` | Memoized cards, lazy loading, drag-scroll, context menu |
| `src/components/StoryViewer.tsx` | Auto-advance, progress bars, tap/keys, full-view mode |
| `src/components/SettingsPanel.tsx` | Sections, toasts, subfolder UI, pause controls |
| `src-tauri/tauri.conf.json` | Schema v2, removed invalid `icon`, version 3.1.0 |
| `src-tauri/Cargo.toml` | Added `notify`, `base64`, `lru`, `thiserror`, `kamadak-exif`, `tracing` |
| `src-tauri/capabilities/default.json` | Scoped `fs:default` with allow paths |
| `package.json` | Version 3.1.0 |

---

## v3.0.2 — Architecture & Mobile Reliability · 2025-06-14

**Focus**: Production‑ready architecture — in‑memory image cache, filesystem watcher, startup optimization, and reliable mobile file access via SAF + `tauri-plugin-fs`.

### 🚀 New

- **`cache.rs`** — in‑memory `ImageCache` (`RwLock<HashMap>`) loaded from DB at startup for instant image lookups during story generation; count emitted immediately as `scan‑complete` so the UI never blocks.
- **`watcher.rs`** — `FolderWatcher` using `notify` crate (inotify/FSEvent/ReadDirectoryChanges); monitors all enabled `scanned_folders` recursively and auto‑queues new image files into the indexing pipeline.
- **Mobile scanning via SAF** — replaces broken `@universalappfactory/tauri-plugin-medialibrary` and `@gbyte/tauri-plugin-ios-photos` with `tauri‑plugin‑dialog` (folder picker) + `tauri‑plugin‑fs` (`readDir`/`readFile`); works on Android 10+ scoped storage and iOS sandbox.

### 🔧 Backend

- **Startup no longer rescans** — loads image count from DB, emits `scan‑complete` immediately; background discovery still runs for new/default paths.
- **Removed unused plugins** — `tauri‑plugin‑medialibrary` and `tauri‑plugin‑ios‑photos` deleted from `Cargo.toml` and `lib.rs` (both unreliable on Tauri v2).
- **`ImageCache` managed as Tauri state** — refreshable via `load_from_db()` after background scans complete.
- **File scope via capabilities** — dropped invalid `plugins.fs.scope` from `tauri.conf.json`; properly scoped `fs:default` with `$APPDATA`, `$HOME`, `$PICTURE`, `$DOWNLOAD`, `$DOCUMENT`, `$TEMP` paths in `src‑tauri/capabilities/default.json`.

### 🎨 Frontend

- **`walkDir()` recursive walker** — uses `readDir`/`readFile` from `@tauri-apps/plugin-fs` to enumerate image files from SAF URIs; sends each via base64 to `index_ios_image_data`.
- **Mobile `handleScanDevice`** — opens SAF folder picker instead of auto‑discovering default paths; `handleReScanFolders` works identically.
- **Desktop path unchanged** — existing `walkdir`‑based Rust scanning preserved for desktop.

### 🧹 Changes

| File | What changed |
|------|-------------|
| `src-tauri/src/cache.rs` | **New** — `ImageCache` with `load_from_db`, `get_random_analyzed`, `insert`, `remove` |
| `src-tauri/src/watcher.rs` | **New** — `FolderWatcher::start()` using `notify` crate, desktop‑only (`#[cfg]`) |
| `src-tauri/Cargo.toml` | Added `notify = "7"`, removed `tauri-plugin-medialibrary`, removed `tauri-plugin-ios-photos` |
| `src-tauri/src/lib.rs` | Integrated cache + watcher in setup; removed plugin registrations; startup emits DB count |
| `src-tauri/tauri.conf.json` | Removed invalid `plugins.fs.scope` |
| `src-tauri/capabilities/default.json` | Scoped `fs:default` with allow paths |
| `src/App.tsx` | Rewrote `trigger‑mobile‑scan` handler to use `tauri‑plugin‑dialog` + `tauri‑plugin‑fs`; added `walkDir`; mobile‑aware `handleScanDevice`/`handleReScanFolders` |

---

## v3.0.1 — Scanned Folders Management · 2025-06-01

**Focus**: Add persistent Scanned Folder management with subfolder toggle hierarchy.

### 🚀 New

- **Scanned Folders in Settings** — add/remove folders, toggle enable/disable per folder, see subfolder counts with expand/collapse.
- **Subfolder toggle overrides** — individual subfolders can be disabled while the parent stays enabled; state stored via `folder_child_overrides` table.
- **Auto-persist default paths** — discovered Pictures/OneDrive paths are saved to `scanned_folders` on startup; no need to re-add them manually.
- **Folder accessibility check** — missing folders show a red "Missing" badge and an amber warning banner at the top of the list.

### 🔧 Backend

- **`folder_child_overrides` table** (SQLite) — `(parent_path, child_name, is_disabled)` with composite primary key.
- **New IPC commands** — `toggle_child_override`, `get_folder_children` now returns `disabled: bool` per child.
- **Image filtering** — both `generate_random_story` and `generate_ai_story` exclude images from disabled folders (parent & subfolder overrides).
- **`internal_scan_device`** — skips disabled folders during auto-scan; persists discovered default paths.

### 🎨 Frontend

- **`ScannedFoldersSection`** — scrollable list in Settings with Add Folder / Re-scan buttons.
- **`FolderRow`** — expandable rows with toggle switch (`w-10 h-6`, symmetrical 4px gaps) and nested child toggle list.
- **Optimistic child toggle** — subfolder state flips instantly in UI then syncs to backend.

### 🧹 Changes

| File | What changed |
|------|-------------|
| `src-tauri/src/db.rs` | Added `folder_child_overrides` table |
| `src-tauri/src/scanner.rs` | Added `toggle_child_override`, updated `get_folder_children` with `disabled` field |
| `src-tauri/src/stories.rs` | Both generators exclude overridden child folder images |
| `src-tauri/src/lib.rs` | Registered `toggle_child_override` command |
| `src/App.tsx` | `handleToggleChildFolder` uses override pattern |
| `src/components/SettingsPanel.tsx` | Rewrote `FolderRow` with local optimistic state; removed `allFolders` prop |

---

## v3.0.2 — Polish, Prompts & Build Fix · 2025-05-25

**Focus**: Improve AI prompt quality, fix build errors, and polish the UI.

### 🤖 AI Improvements

- **Upgraded vision prompt** — replaced one-liner with a structured multi-rule prompt that enforces 5 objective descriptive tags, a single-word vibe, and explicitly forbids full sentences, abstract interpretations, repeated tags, and brand names.
- **Upgraded caption prompt** — replaced generic title prompt with a "cinematic photo story title" prompt that requires 3–5 words, natural phrasing, and explicitly rules out clichés, hashtags, and punctuation overload.
- Both prompts updated for **both OpenRouter and Ollama** providers.

### 🎨 UI / UX

- **Logo glow without flicker** — removed `animate-pulse` from the logo image; the neon glow is now static and smooth.
- **Stories scroll horizontally** — the stories container now scrolls on X within a transparent container; the rest of the page no longer scrolls sideways.
- **Logo removed from feed header** — the logo now only lives in the sidebar. The feed header just shows the title.
- **Sidebar landscape padding** — added `sm:py-12` to the sidebar nav and `mt-8` to the logo wrapper for better vertical spacing on desktop.

### 🐛 Bug Fixes

- **`Logo` unused import** — removed stale `import Logo from "./Logo"` from `HomeFeed.tsx` that caused a TypeScript compile error (`TS6133`) and blocked `npm run tauri build`.
- **Invalid `icon` in `tauri.conf.json`** — removed the `"icon"` property from the `app > windows` array; it is not part of the Tauri v2 schema and broke `tauri android init`.
- **Bundle icon order** — reordered the `bundle.icon` array to list `icon.ico` and high-resolution PNGs first so Windows picks the correct icon for the taskbar.

### 🧹 Changes

| File | What changed |
|------|-------------|
| `src-tauri/src/ai.rs` | Upgraded vision prompt (×2: OpenRouter + Ollama) and caption prompt |
| `src/components/HomeFeed.tsx` | Removed Logo from header; added `overflow-x-hidden` to root div; removed unused Logo import |
| `src/components/Logo.tsx` | Removed `animate-pulse` from logo image className |
| `src/App.tsx` | Sidebar `sm:py-12`, logo wrapper `mt-8`, removed inline style override |
| `src-tauri/tauri.conf.json` | Removed invalid `icon` from windows config; reordered bundle icons |
| `dev/bg.html` | **New** — standalone cyber-bg poster page with StoryBox3 title |
| `README.md` | Full rewrite with emojis, tables, links, architecture diagram |
| `CHANGELOG.md` | Updated with this entry |

---

## v3.0.1 — Android Init & Docs · 2025-05-25

**Focus**: Initialize Android build target and document the setup.

### 🚀 New

- **Android project initialized** — ran `npx tauri android init` to generate the Android Studio project under `src-tauri/gen/android/storybox3`.
- **Android build target confirmed working** with NDK `29.0.13846066` and Android Studio's bundled JBR.

### 📝 Docs

- README updated with Android build instructions and the correct Tauri CLI command (`npx tauri android init`).

---

## v3.0.0 — StoryBox3: Cloud AI & New Identity · 2025-05-24

**Focus**: Full rewrite — new name, cloud AI via OpenRouter, secure proxy, and futuristic UI.

### 🚀 New Features

- **OpenRouter Cloud AI** — no local GPU required. Uses free models: Qwen VL Plus (vision), Llama 3.1 8B (text).
- **Supabase Edge Function proxy** — the OpenRouter API key lives server-side. The app binary contains zero credentials.
- **Futuristic cyber UI** — animated grid background, floating neon particles, scanline overlay, animated gradient story card borders, glassmorphism panels.
- **Activity Indicator** — floating HUD pill in the sidebar shows live indexing and AI processing progress.
- **Story card context menu** — three-dot menu per card: Pin to Top, Save to Favorites, Delete.
- **AI Health banner** — red warning banner in the feed links directly to the AI settings section when the AI provider is unreachable.
- **Auto-retry AI health check** — the app polls every 15 seconds when the AI is disconnected and updates the banner automatically.

### 🎨 Design

- Color palette: deep navy (`#05080c`) background, `#0080ff` neon blue accent, `#8c1aff` cyber purple.
- Google Font: Inter (800 weight for headings, 400 for body).
- Story cards: animated gradient border, drag-to-scroll with inertia, hover lift effect.
- Sidebar: compact 80px-wide icon rail with logo, nav buttons, activity indicator, and Add button.

### 🔐 Security

- Zero hardcoded API keys in source or binary.
- All external calls over HTTPS / TLS 1.3.
- Images downscaled to 1024px before transmission.
- No analytics or telemetry.

### 🧹 Changes from StoryBox2

| Area | Change |
|------|--------|
| Identity | Renamed to `StoryBox3`, identifier `com.storybox3.app`, version `3.0.0` |
| AI Provider | Added OpenRouter + Supabase proxy as primary; Ollama demoted to fallback |
| Frontend | Rewrote all components with cyber design system |
| `ai.rs` | Added `analyze_image_openrouter` and `Storyteller` for text generation |
| `tauri.conf.json` | Updated identifier, version, and bundle icon list |
| `index.css` | Full design system: cyber-bg, particles, glow effects, animated borders |

---

<details>
<summary><strong>📦 v2.1.1 — iOS, Docs & Housekeeping</strong></summary>

**Focus**: Wire iOS Photos framework support, rewrite documentation, add this changelog.

### 🚀 New

- **iOS Photos framework** — `tauri-plugin-ios-photos` (v0.3): `requestPhotosAuth()` → `requestAlbums()` → `requestAlbumMedias()` → `index_ios_image_data`.
- **`Info.plist`** — `NSPhotoLibraryUsageDescription` and `NSPhotoLibraryAddUsageDescription`.
- **iOS capability** — `ios-photos:default` in `capabilities/default.json`.

### 📝 Docs

- README rewritten with Prerequisites table, Ollama setup, Android/iOS build steps.

### 🧹 Changes

| File | What changed |
|------|-------------|
| `CHANGELOG.md` | **New** |
| `README.md` | Full rewrite |
| `src-tauri/Info.plist` | **New** |
| `src-tauri/Cargo.toml` | Added `tauri-plugin-ios-photos` |
| `src-tauri/src/lib.rs` | Added `index_ios_image_data`, iOS plugin registration |
| `src/App.tsx` | iOS photo scan branch in `trigger-mobile-scan` |
| `package.json` | Added `@gbyte/tauri-plugin-ios-photos` |

</details>

---

<details>
<summary><strong>📦 v2.1.0 — Architecture, Automation & Cleanup</strong></summary>

**Focus**: Bug fixes, scheduled automation, and dead code removal.

### 🚀 New

- **Automated story generation** — `run_automation_tasks()` called every 30 min via background `tokio::spawn`. Auto-generates stories and cleans unpinned stories older than 24 hours.

### 🐛 Bug Fixes

- **Vibe parsing truncation** (`ai.rs`) — multi-word vibes like "very peaceful" were truncated to "very". Fixed by splitting only on sentence-ending punctuation.
- **Missing CSS utilities** — added keyframes and classes for `scrollbar-hide`, `animate-fade-in`, `animate-zoom-in`, `animate-slide-up`, `animate-in`, `zoom-in-95`, `animate-scale-in`.
- **Unlisten cleanup** — replaced sequential `.then(f => f())` with `Promise.all(...).then(fns => fns.forEach(f => f()))`.

### 🧹 Cleanup

- Removed dead `greet` Tauri command.
- Removed unused `moondream` npm package.
- Removed unused `notify` and `dotenvy` Rust crates.
- Updated all version references to `2.1.0`.

| File | What changed |
|------|-------------|
| `src-tauri/Cargo.toml` | Removed `notify`/`dotenvy` |
| `src-tauri/src/lib.rs` | Removed `greet`, added automation timer |
| `src-tauri/src/ai.rs` | Fixed vibe parsing |
| `src-tauri/src/stories.rs` | `run_automation_tasks` now actually scheduled |
| `src/App.tsx` | Fixed unlisten cleanup pattern |
| `src/index.css` | Added missing animation/scrollbar utilities |

</details>
