# StoryBox3 architecture

## Technology stack

| Layer | Choice | Evidence |
| --- | --- | --- |
| Desktop/mobile shell | Tauri 2, identifier `com.ok.storybox3`, version `3.1.0` | [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) |
| Frontend | React `^19.1.0`, React DOM, TypeScript `~5.8.3` | [`../package.json`](../package.json) |
| Frontend build | Vite `^7.0.4`, `@vitejs/plugin-react`, Tailwind CSS `^3.4.19`, PostCSS, autoprefixer | [`../package.json`](../package.json), [`../vite.config.ts`](../vite.config.ts), [`../tailwind.config.js`](../tailwind.config.js) |
| Rust backend | Rust edition 2021, Tokio 1 (`full`), `tauri-plugin-fs/dialog/opener` | [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml), [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) |
| Database | SQLite via `sqlx` 0.7 (`sqlite`, `runtime-tokio-rustls`) | [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) |
| HTTP/AI | `reqwest` 0.12 with `rustls-tls`; OpenRouter, Ollama, Mock providers | [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs) |
| Indexing support | `walkdir`, `notify`, `kamadak-exif`, `image`, `lru`, `base64` | [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) |
| Proxy | Supabase Edge Function (Deno), `OPENROUTER_API_KEY` from server env | [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts) |

## Entry points

- Web frontend: [`../index.html`](../index.html) mounts `#root` and loads
  `/src/main.tsx`; [`../src/main.tsx`](../src/main.tsx) renders
  [`../src/App.tsx`](../src/App.tsx) inside `React.StrictMode`.
- Native backend: `src-tauri/src/main.rs` (the `[[bin]]` target) calls into
  `src-tauri/src/lib.rs`, whose `run()` builds the Tauri app, registers plugins,
  sets up state, starts workers, and registers IPC commands.

## Directory map

```
StoryBox3/
├── src/
│   ├── components/   ActivityIndicator, HomeFeed, Icons, Logo, SettingsPanel, StoryViewer
│   ├── hooks/        useActivities, useAiHealth, useScan, useScannedFolders, useStories
│   ├── assets/
│   ├── App.tsx       layout, IPC listeners, mobile scan handling
│   ├── index.css     cyber theme tokens and utilities
│   └── main.tsx      React root
├── src-tauri/
│   ├── src/          ai, cache, db, exif, lib, main, processor, scanner, stories, watcher
│   ├── capabilities/ default.json — core/opener/dialog/fs permissions
│   ├── icons/, Info.plist, build.rs, Cargo.toml, tauri.conf.json
│   └── .cargo/config.toml — Windows GNU stack setting
├── supabase/functions/openrouter-proxy/  Deno proxy for OpenRouter
├── public/           favicons, logos, manifest.json, tauri.svg
├── docs/             this documentation set + visual-identity.md
├── dev/              ignored local assets, releases, signing material
├── CHANGELOG.md
└── package.json / pnpm-lock.yaml / vite.config.ts / tsconfig*.json
```

## Module responsibilities

| File | Responsibility |
| --- | --- |
| [`../src/App.tsx`](../src/App.tsx) | Sidebar/feed layout, story state via hooks, Tauri event listeners (`refresh-stories`, `no-photos-found`, `trigger-mobile-scan`) |
| [`../src/hooks/`](../src/hooks) | Frontend state for stories, scanning, folders, AI health (30 s polling), and activities |
| [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx) | Scanned-folder management, AI provider settings, health check UI |
| [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) | App setup, shared state, worker startup, automation timer, `generate_handler!` command list |
| [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs) | Default-path discovery, folder CRUD/overrides, scan commands |
| [`../src-tauri/src/processor.rs`](../src-tauri/src/processor.rs) | Two-stage indexing/analysis pipeline, pause/resume, change detection |
| [`../src-tauri/src/watcher.rs`](../src-tauri/src/watcher.rs) | `notify`-based folder watcher (desktop only) |
| [`../src-tauri/src/cache.rs`](../src-tauri/src/cache.rs) | In-memory LRU image cache loaded from SQLite at startup |
| [`../src-tauri/src/exif.rs`](../src-tauri/src/exif.rs) | EXIF date/GPS extraction |
| [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs) | `AIEngine` (vision), `Storyteller` (captions), provider routing, proxy fallback, Rust tests |
| [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs) | Story generation, pin/favorite/delete, settings read/write, automation and cleanup |
| [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) | SQLite schema, migrations, pragmas, seeded `ai_settings` row |

## Data and process flow

1. **Startup.** `lib.rs` registers `fs`, `opener`, and `dialog` plugins, then
   creates an MPSC channel with capacity 256. It initialises SQLite at
   `app_local_data_dir()/storybox.db`, loads the image cache, creates the AI
   engine and cached `AISettings`, and starts the processing worker.
2. **Discovery.** A background task calls `internal_scan_device`, emitting
   `scan-complete` and `refresh-stories` to the frontend. On desktop only, a
   `FolderWatcher` watches persisted folders for new/modified files. The
   automation loop sleeps 1800 seconds between `run_automation_tasks` calls.
3. **Indexing and analysis.** `processor.rs` takes queued jobs, extracts EXIF
   data, generates thumbnails, and calls `AIEngine::analyze_image`. OpenRouter
   and Ollama paths are selected by the configured provider; Mock returns
   deterministic sample data.
4. **AI calls.** `ai.rs` downscales images to at most 1024 px, encodes JPEG at
   quality 85, and sends a structured vision prompt. When OpenRouter is used
   with a proxy, requests go to `DEFAULT_PROXY_URL` (or the configured proxy)
   without an `Authorization` header; direct mode sends `Bearer <api_key>` to
   `https://openrouter.ai/api/v1`. The Deno proxy reads the key from its own
   environment and forwards to OpenRouter.
5. **Stories.** `stories.rs` builds stories from AI output or template/random
   fallbacks, stores them in SQLite, emits `story-updated`, and runs scheduled
   generation/cleanup.
6. **Frontend.** Components call Tauri `invoke` commands (for example
   `get_stories`, `update_ai_settings`, `check_ai_availability`) and subscribe
   to emitted events. Hook state drives the feed, settings, activity HUD, and
   full-screen viewer.

## Storage and integration boundaries

| Boundary | Detail |
| --- | --- |
| Local database | `storybox.db` in `app_local_data_dir()`; tables `images`, `image_features`, `stories`, `story_images`, `ai_settings`, `scanned_folders`, `folder_child_overrides`; WAL mode, 20 connections, 64 MB cache — [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) |
| Filesystem access | `fs:default` scoped to `$APPDATA/**`, `$RESOURCE/**`, `$HOME/**`, `$PICTURE/**`, `$DOWNLOAD/**`, `$DOCUMENT/**`, `$TEMP/**` — [`../src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json) |
| Frontend storage | No `localStorage`/`sessionStorage` usage was found; settings are persisted in SQLite |
| Network | HTTPS to the configured proxy or OpenRouter base URL; Ollama uses the configured local URL; no analytics endpoints found |
| Secret boundary | The OpenRouter key is either entered in Settings (stored in `ai_settings.api_key`) or held by the Supabase function in `OPENROUTER_API_KEY` — never in source |

## Sources

- [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)
- [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs)
- [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs)
- [`../src-tauri/src/processor.rs`](../src-tauri/src/processor.rs)
- [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs)
- [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs)
- [`../src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json)
- [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts)
- [`../vite.config.ts`](../vite.config.ts)
