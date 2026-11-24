# StoryBox3 product

StoryBox3 turns a local photo library into short, cinematic stories. It is a
Tauri 2 application: a React frontend talks to a Rust backend through Tauri
IPC, the backend keeps metadata in a local SQLite database, and AI calls go to
OpenRouter either directly or through a Supabase Edge Function proxy. An
Ollama provider and a Mock provider are also implemented.

## Who it is for

- People who want automatic, styled stories from their own photos without
  uploading the library to a photo service.
- Users who prefer a local database and on-device indexing, with AI used only
  for tags, vibes, and short captions.
- Desktop users first (Windows, macOS, Linux), with Android/iOS paths handled
  through a folder picker rather than the removed native photo plugins.

## What it does today

| Capability | Evidence |
| --- | --- |
| Scan local folders, persist enabled folders, skip unchanged files | [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs), [`../src-tauri/src/processor.rs`](../src-tauri/src/processor.rs) |
| Watch scanned folders for new files on desktop | `FolderWatcher` in [`../src-tauri/src/watcher.rs`](../src-tauri/src/watcher.rs), gated by `#[cfg]` in `lib.rs` |
| Extract EXIF date and GPS metadata | [`../src-tauri/src/exif.rs`](../src-tauri/src/exif.rs) |
| Analyze images into 5 tags, a one-word vibe, dominant color, and embedding | `AIEngine` in [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs) |
| Generate stories with AI first and template/random fallbacks | [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs) |
| Pin, favorite, paginate, and delete stories | Commands registered in [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs), UI in [`../src/components/HomeFeed.tsx`](../src/components/HomeFeed.tsx) |
| Auto-generate stories on a configurable schedule | Automation loop in `lib.rs` (every 1800 s) and `run_automation_tasks` in `stories.rs` |
| Clean up old unpinned stories | `run_automation_tasks` in [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs) |
| Full-screen story viewer with auto-advance | [`../src/components/StoryViewer.tsx`](../src/components/StoryViewer.tsx) |
| Configure provider, models, API key, proxy, and intervals | [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx) → `update_ai_settings` |
| AI health check with 30-second polling | [`../src/hooks/useAiHealth.ts`](../src/hooks/useAiHealth.ts), `check_ai_availability` in `ai.rs` |
| Mobile folder picking via Tauri dialog/fs, base64 image ingestion | [`../src/App.tsx`](../src/App.tsx), `index_ios_image_data` in `lib.rs` |
| PWA-style web metadata alongside the native shell | [`../public/manifest.json`](../public/manifest.json), [`../index.html`](../index.html) |

## Non-goals and known gaps

- **The default AI path depends on a hosted proxy.** `DEFAULT_PROXY_URL` in
  `ai.rs` and the seeded row in `db.rs` point at a Supabase Edge Function. The
  README's "no API key required for basic use" relies on that proxy being
  reachable; if it is down or unconfigured, OpenRouter mode is unavailable
  unless the user supplies a key or another proxy.
- **Cleanup cutoff is hard-coded.** The Settings panel exposes "Expire After
  (hours)", which controls how often cleanup runs, but `stories.rs` deletes
  unpinned stories older than a fixed `chrono::Duration::hours(24)`. See
  [configuration](./configuration.md#automation-settings).
- **Stale native photo plugin dependencies.** `Cargo.toml` and `lib.rs` no
  longer register the iOS Photos / medialibrary plugins, but
  `@gbyte/tauri-plugin-ios-photos` and
  `@universalappfactory/tauri-plugin-medialibrary` remain in
  `package.json`. Mobile scanning is implemented through dialog + fs instead.
- **TLS 1.3 is not pinned in code.** `Cargo.toml` enables `rustls-tls` for
  `reqwest`; no explicit protocol version is configured. Requests use HTTPS
  URLs, but the README's "TLS 1.3" wording is stronger than the code evidence.
- **Package manager mismatch.** `package.json` declares
  `pnpm@11.2.2` and the lockfile is `pnpm-lock.yaml`, while the README and
  `tauri.conf.json` invoke npm scripts.
- **No JavaScript/TypeScript test suite and no CI.** Rust unit tests exist but
  several require live network/proxy access. See [testing](./testing.md).
- **No tracked deployment configuration** beyond Tauri/PWA config; local
  release artifacts and signing material live in ignored `dev/`.
- **No `.env.example`** is present, even though `.gitignore` explicitly allows
  one. The only server-side secret is documented in
  [configuration](./configuration.md#secrets).

## Sources

- [`../README.md`](../README.md)
- [`../package.json`](../package.json)
- [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs)
- [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs)
- [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)
- [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs)
- [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs)
- [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx)
- [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts)
