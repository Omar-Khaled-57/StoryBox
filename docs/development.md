# StoryBox3 development

## Prerequisites

The README lists Node.js v18+, Rust & Cargo, an optional OpenRouter API key,
and Android Studio for Android builds. `package.json` has no `engines` field,
so the Node requirement is documentation rather than an enforced constraint.

## Install and run

The README documents `npm install` followed by the Tauri development workflow.
Note the mismatch: [`../package.json`](../package.json) declares
`packageManager: pnpm@11.2.2` and the tracked lockfile is `pnpm-lock.yaml`,
while both the README and [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
invoke npm scripts (`beforeDevCommand: npm run dev`,
`beforeBuildCommand: npm run build`). Choose one package manager consistently
to avoid a second lockfile.

## Command reference

These commands are verbatim entries in the `scripts` block of
[`../package.json`](../package.json).

| Script | Command | Purpose |
| --- | --- | --- |
| `dev` | `vite` | Starts the Vite dev server on the fixed Tauri port (1420) |
| `build` | `tsc && vite build` | Type-checks `src/` and builds the frontend into `dist/` |
| `preview` | `vite preview` | Serves the built frontend locally |
| `tauri` | `tauri` | Exposes the Tauri CLI; the README documents `dev`, `build`, and Android subcommands on top of it |

The README's `npm run tauri dev`, `npm run tauri build`,
`npx tauri android init`, `npm run tauri android dev`, and
`npm run tauri android build` are Tauri CLI invocations documented in the
existing README; they are not separate manifest scripts.

## Repository structure

```
src/                 React app, hooks, components, global CSS
src-tauri/src/       Rust modules (ai, cache, db, exif, lib, processor, scanner, stories, watcher)
src-tauri/capabilities/  Tauri permission scopes
src-tauri/.cargo/    Windows GNU linker stack setting
supabase/functions/  Deno OpenRouter proxy
public/              Web/PWA static assets
docs/                This standard set plus visual-identity.md
dev/                 Ignored local assets, releases, signing material, analysis notes
```

## Conventions

- Frontend state lives in five hooks under `src/hooks`; components consume
  them rather than owning all state in `App.tsx`.
- Backend calls go through Tauri `invoke(<command>, args)` and events through
  `listen(...)`; every exposed command must be listed in `generate_handler!`
  in [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs).
- Styling is Tailwind utility classes plus custom tokens/utilities in
  [`../src/index.css`](../src/index.css) and
  [`../tailwind.config.js`](../tailwind.config.js).
- Rust modules are plain files under `src-tauri/src/`; shared runtime state
  (DB pool, channels, cache, AI engine/settings) is registered with
  `app.manage(...)` during setup.
- SQLite schema changes are additive `CREATE TABLE IF NOT EXISTS` plus
  best-effort `ALTER TABLE ... ADD COLUMN` migrations in
  [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs).

## How to add a feature

- **New Tauri command:** implement it in the relevant `src-tauri/src` module
  with `#[tauri::command]`, then add it to `generate_handler!` in `lib.rs`.
- **New AI provider:** extend the provider matches in `AIEngine`,
  `check_availability`, and `Storyteller` in
  [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs), add a button in
  `AiSettingsSection`, and persist it through `update_ai_settings`.
- **New AI setting:** add the column/migration to `ai_settings` in `db.rs`,
  include it in `get_ai_settings`/`update_ai_settings` in `stories.rs`, extend
  `AISettings` in `ai.rs`, and add an input in `SettingsPanel.tsx`.
- **New scanned-folder behavior:** modify `scanner.rs` and the corresponding
  hook in `src/hooks/useScannedFolders.ts`.

## Troubleshooting grounded in the code

- **AI unavailable on first run.** OpenRouter defaults to the hosted proxy;
  check the AI Health Check panel. A direct key is only used when no proxy is
  configured. See [configuration](./configuration.md#ai-provider-settings).
- **Cleanup interval seems ignored.** The schedule is configurable, but the
  deletion cutoff is a fixed 24 hours in `stories.rs`.
- **Mobile photos not found automatically.** Mobile uses a folder picker and
  base64 ingestion through `trigger-mobile-scan`; the iOS Photos/medialibrary
  plugins are no longer registered in Rust.
- **Windows GNU link errors.** [`../src-tauri/.cargo/config.toml`](../src-tauri/.cargo/config.toml)
  sets a 16 MB stack for `x86_64-pc-windows-gnu`.
- **Missing secrets or signing material.** `.env`, `.env.*`, `*.jks`,
  `*.keystore`, and `dev/` are ignored by [`../.gitignore`](../.gitignore).
  No `.env.example` is tracked. The proxy expects `OPENROUTER_API_KEY` on the
  server, not in the repo.
- **Local performance notes.** `dev/complexity-report.md` is an ignored local
  analysis of indexing and cache complexity; it is not a test result.

## Sources

- [`../README.md`](../README.md)
- [`../package.json`](../package.json)
- [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
- [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml)
- [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs)
- [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs)
- [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx)
- [`../.gitignore`](../.gitignore)
