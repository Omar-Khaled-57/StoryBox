# StoryBox3 configuration

## Environment variables

Only three environment variables are read in tracked code:

| Variable | Read by | Purpose |
| --- | --- | --- |
| `TAURI_DEV_HOST` | [`../vite.config.ts`](../vite.config.ts) via `process.env` | Optional dev host; when set, Vite binds to it and enables HMR over `ws` on port 1421 |
| `USERPROFILE` | [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs), Windows `#[cfg]` | Finds `OneDrive/Pictures` and `One Drive/Pictures` during desktop discovery |
| `HOME` | [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs), Linux `#[cfg]` | Finds `$HOME/Pictures` during desktop discovery |
| `OPENROUTER_API_KEY` | [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts) via `Deno.env.get` | **Server-side secret** used by the Supabase Edge Function to call OpenRouter |

macOS discovery uses `directories::UserDirs` rather than an environment
variable. No `VITE_*` variables are used, and no `.env` file is tracked.

## Configuration files

| File | What it controls |
| --- | --- |
| [`../package.json`](../package.json) | Scripts, dependencies, declared `pnpm@11.2.2` package manager |
| [`../pnpm-workspace.yaml`](../pnpm-workspace.yaml) | `allowBuilds` / `onlyBuiltDependencies` for `esbuild` |
| [`../vite.config.ts`](../vite.config.ts) | React plugin, fixed port 1420, `strictPort`, HMR host, ignored `src-tauri` watch path |
| [`../tsconfig.json`](../tsconfig.json), [`../tsconfig.node.json`](../tsconfig.node.json) | Frontend TS strictness; separate config for `vite.config.ts` |
| [`../tailwind.config.js`](../tailwind.config.js), [`../postcss.config.js`](../postcss.config.js) | Tailwind theme/utilities and PostCSS pipeline |
| [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) | Identifier `com.ok.storybox3`, version `3.1.0`, dev/build commands, window size, bundle targets |
| [`../src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json) | `core:default`, `opener:default`, `dialog:default`, scoped `fs:default` |
| [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) | Rust crates and features (SQLite, rustls-tls, Tokio, image, EXIF, notify, LRU) |
| [`../src-tauri/.cargo/config.toml`](../src-tauri/.cargo/config.toml) | 16 MB stack link flag for `x86_64-pc-windows-gnu` |
| [`../public/manifest.json`](../public/manifest.json) | Web/PWA name, standalone display, theme/background `#0a0a1a`, icons |
| [`../index.html`](../index.html) | Favicons, Apple mobile web metadata, manifest link, `#root` mount |

## AI provider settings

Settings are persisted in the single `ai_settings` row (`id = 1`) defined in
[`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) and cached in memory by
`AISettings::load` in [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs).
Defaults below combine the SQL schema and the Rust fallback; note that the SQL
column default for `proxy_url` is an empty string while the seeded row and Rust
fallback use the hosted proxy URL.

| Setting | Default | Where the user changes it |
| --- | --- | --- |
| `provider` | `openrouter` | Provider buttons in `AiSettingsSection` |
| `base_url` | `https://openrouter.ai/api/v1` | API Base URL / Ollama URL field |
| `model_name` (text) | `meta-llama/llama-3.1-8b-instruct` | Text Model field |
| `vision_model_name` | `qwen/qwen-vl-plus` | Vision Model field |
| `api_key` | `''` | API Key field (password input) |
| `proxy_url` | Hosted proxy URL; SQL default `''` | Proxy URL field (optional) |
| `auto_gen_interval_hours` | `12` | Generate Every (hours) |
| `cleanup_interval_hours` | `24` | Expire After (hours) |

Hosted proxy URL (from `DEFAULT_PROXY_URL` in `ai.rs`, the seeded row in
`db.rs`, and the Rust test constant):
`https://pawylsallgbfnzzmakac.supabase.co/functions/v1/openrouter-proxy`.

Provider behavior in [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs):

- `openrouter`: with a proxy URL, calls the proxy without an `Authorization`
  header; otherwise calls `${base_url}/chat/completions` with
  `Bearer ${api_key}`. If `proxy_url` is empty, the hardcoded default proxy is
  used. Health check uses a 5-second timeout.
- `ollama`: calls `${base_url}/api/tags` (2-second timeout) and
  `${base_url}/api/generate`; model names are matched by substring.
- `mock`: deterministic local responses; caption generation deliberately
  returns `None`.

## Automation settings

`run_automation_tasks` in [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs)
reads `auto_gen_interval_hours` and `cleanup_interval_hours` to decide when to
run. However, cleanup deletes unpinned stories older than a hard-coded
`now - chrono::Duration::hours(24)`; changing "Expire After" changes the
schedule, not that cutoff. The background loop in
[`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) wakes every 1800 seconds
(30 minutes).

## Local storage and filesystem settings

- No `localStorage` or `sessionStorage` keys were found in the frontend.
- [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) writes `storybox.db` into
  Tauri's `app_local_data_dir()`. The README's older
  `%APPDATA%\com.storybox3.app\` path is stale; the configured identifier is
  `com.ok.storybox3`.
- [`../src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json)
  allows `fs:default` access under `$APPDATA/**`, `$RESOURCE/**`, `$HOME/**`,
  `$PICTURE/**`, `$DOWNLOAD/**`, `$DOCUMENT/**`, and `$TEMP/**`.
- Scanned folders and per-subfolder disable overrides are persisted in the
  `scanned_folders` and `folder_child_overrides` tables.

## Secrets

- **`OPENROUTER_API_KEY` belongs on the Supabase function**, not in the app
  binary or repository. The proxy returns a 500 error when it is missing.
- A user-entered key is stored in `ai_settings.api_key` on that user's device;
  it is never required in source or CI.
- [`../.gitignore`](../.gitignore) excludes `.env`, `.env.*`, `*.jks`,
  `*.keystore`, and `dev/`. Local signing material (`dev/key`,
  `dev/my-release-key.jks`) sits in that ignored folder. No `.env.example` is
  present even though the ignore file allows one.

## Known gaps

- No TLS version is pinned in code; `reqwest` uses the `rustls-tls` feature and
  HTTPS URLs. The README's "TLS 1.3" claim has been softened accordingly.
- The SQL default for `proxy_url` (`''`) differs from the seeded/runtime
  default; both are documented above.
- `TAURI_DEV_HOST` is only consumed by the Vite dev server, not by the packaged
  app.

## Sources

- [`../vite.config.ts`](../vite.config.ts)
- [`../src-tauri/src/scanner.rs`](../src-tauri/src/scanner.rs)
- [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs)
- [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs)
- [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs)
- [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)
- [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx)
- [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts)
- [`../.gitignore`](../.gitignore)
