# StoryBox3 deployment

## Build commands

From the `scripts` block of [`../package.json`](../package.json):

| Script | Command | Output |
| --- | --- | --- |
| `build` | `tsc && vite build` | Type-checked frontend bundle in `dist/` |
| `preview` | `vite preview` | Locally served production frontend |
| `tauri` | `tauri` | Tauri CLI entry point used by the README's build workflows |

The README documents `npm run tauri dev` for development and
`npm run tauri build` for desktop release builds, plus
`npx tauri android init`, `npm run tauri android dev`, and
`npm run tauri android build` for Android. Those are Tauri CLI invocations on
top of the `tauri` script, not separate manifest scripts. Tauri's
`beforeBuildCommand` in [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
is `npm run build`, and `frontendDist` is `../dist`.

## Artifacts and targets

| Artifact | Source / config |
| --- | --- |
| Frontend bundle `dist/` | Vite build; ignored by [`../.gitignore`](../.gitignore) |
| Rust/native binaries in `src-tauri/target/` | Tauri/Cargo build; ignored |
| Desktop and mobile bundles | `bundle.targets: "all"` in [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) |
| Bundle icons | `icons/icon.ico`, `icon.icns`, and PNG sizes 32/64/128/128@2x in `tauri.conf.json` |
| macOS minimum version | `10.15` in `tauri.conf.json` |
| Web/PWA assets | [`../public/manifest.json`](../public/manifest.json) with standalone display and `#0a0a1a` theme; favicons and logos in `public/` |
| Windows GNU linker stack | 16 MB via [`../src-tauri/.cargo/config.toml`](../src-tauri/.cargo/config.toml) |

The README's platform table claims Windows, macOS, Linux, and Android support
with iOS in beta, and notes that Android builds need Android Studio, SDK, and
NDK. The code supports those paths through desktop scanning plus the mobile
dialog/fs flow, but no store packages are tracked.

## Deployment configuration present in the repo

- Tauri app config, capabilities, bundle icons, and PWA manifest are tracked.
- [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts)
  is the only server-side component; deploying it requires setting
  `OPENROUTER_API_KEY` in the Supabase function environment. No Supabase
  project config or deployment manifest is tracked under `supabase/`.
- [`../dev/release/`](../dev/release) and other `dev/` artifacts are local
  build outputs excluded by [`../.gitignore`](../.gitignore). Signing material
  (`dev/key`, `dev/my-release-key.jks`) is also ignored and must not be
  committed or copied into docs.
- [`../CHANGELOG.md`](../CHANGELOG.md) records a past Android init step; no
  generated Android project is tracked (`src-tauri/gen/` is ignored).

## Known gaps

- No CI/CD pipeline, Dockerfile, hosting config, or automated release workflow.
- No code-signing configuration tracked; no store submission metadata.
- No environment-specific build flags beyond `TAURI_DEV_HOST` for the dev
  server.
- No documented rollback strategy or release channel.
- Deployment to the Supabase proxy is assumed by the default AI mode but not
  described by a tracked config file.

## Sources

- [`../package.json`](../package.json)
- [`../README.md`](../README.md)
- [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
- [`../src-tauri/.cargo/config.toml`](../src-tauri/.cargo/config.toml)
- [`../public/manifest.json`](../public/manifest.json)
- [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts)
- [`../.gitignore`](../.gitignore)
