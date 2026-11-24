# StoryBox3 documentation

Start here. StoryBox3 is a Tauri 2 desktop/mobile app that indexes local
photos and generates AI-assisted stories through OpenRouter. The
[product page](./product.md) describes what the app does, the
[architecture page](./architecture.md) explains the React ↔ Rust ↔ SQLite ↔
proxy flow, and the [configuration page](./configuration.md) lists every
setting, environment variable, and secret boundary.

## Documentation index

| Page | What it answers |
| --- | --- |
| [Product](./product.md) | Problem, audience, feature inventory, non-goals and known gaps. |
| [Architecture](./architecture.md) | Stack, entry points, directory map, IPC/data flow, storage and process model. |
| [Development](./development.md) | Prerequisites, install, commands, conventions, troubleshooting. |
| [Configuration](./configuration.md) | AI settings, env vars, config files, capabilities, secrets. |
| [Testing](./testing.md) | Manifest commands, the Rust test module, what is covered, and gaps. |
| [Deployment](./deployment.md) | Build commands, artifacts, targets, and unconfigured deployment. |
| [Changelog](./changelog.md) | Pointer to the existing release log and its history. |
| [Visual identity](./visual-identity.md) | Pre-existing brand colors and visual language. |
| [Project changelog](../CHANGELOG.md) | The author's running development log (outside `docs/`). |

## Canonical source-of-truth files

| Concern | File |
| --- | --- |
| Commands, dependencies, declared package manager | [`../package.json`](../package.json) |
| Tauri app config, identifier, bundle targets | [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json) |
| Rust dependencies and features | [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml) |
| Filesystem permissions | [`../src-tauri/capabilities/default.json`](../src-tauri/capabilities/default.json) |
| Vite dev server and `TAURI_DEV_HOST` | [`../vite.config.ts`](../vite.config.ts) |
| AI providers, defaults, proxy, Rust tests | [`../src-tauri/src/ai.rs`](../src-tauri/src/ai.rs) |
| SQLite schema and default settings row | [`../src-tauri/src/db.rs`](../src-tauri/src/db.rs) |
| Tauri command registration and startup workers | [`../src-tauri/src/lib.rs`](../src-tauri/src/lib.rs) |
| Story automation and cleanup | [`../src-tauri/src/stories.rs`](../src-tauri/src/stories.rs) |
| Settings UI fields | [`../src/components/SettingsPanel.tsx`](../src/components/SettingsPanel.tsx) |
| Supabase proxy and its server-side secret | [`../supabase/functions/openrouter-proxy/index.ts`](../supabase/functions/openrouter-proxy/index.ts) |
| Product overview and release log | [`../README.md`](../README.md), [`../CHANGELOG.md`](../CHANGELOG.md) |

## Local-only material

`../dev/` is excluded by [`../.gitignore`](../.gitignore) and holds local
assets, build output, a complexity analysis, and signing material (including
`dev/key` and `dev/my-release-key.jks`). None of those files should be copied
into documentation or committed. The preview image used in the README and
visual identity guide is also in that ignored folder.

These pages were written against the hub's
[documentation standard](../../docs/DOC-STANDARD.md).
