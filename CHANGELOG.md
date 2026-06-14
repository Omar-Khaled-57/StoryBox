# 📋 StoryBox3 — Changelog

A running log of every change, fix, and decision during development. Newest entries are at the top.

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
