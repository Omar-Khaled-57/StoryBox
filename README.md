# 📦 StoryBox3 — AI Photo Storyteller

> **Your memories, augmented by AI.**
> StoryBox3 is a cross-platform desktop & mobile app that indexes your photos and automatically crafts cinematic stories, poetic titles, and smart tags — powered by free AI models through [OpenRouter](https://openrouter.ai).

---

## ✨ Features

| Feature | Description |
|---|---|
| 🤖 **Cloud AI Analysis** | Vision AI extracts tags & vibe from every photo |
| 📖 **Auto Story Generation** | Stories auto-generate on a configurable schedule |
| 🎨 **Cinematic Titles** | AI writes short-film-style titles from your photos |
| 🔒 **Secure Proxy** | API keys never leave the server — no key in the binary |
| 📌 **Pin & Favorite** | Keep the stories you love forever |
| 🗑️ **Smart Cleanup** | Unpinned stories auto-clean after 24 hours |
| 🌐 **Cross-Platform** | Windows, macOS, Linux, Android, iOS |
| 🏠 **Fully Local DB** | All your data stays on your device (SQLite) |

<p align="center">
  <img src="dev/evice-mockup.png" alt="StoryBox3 desktop and mobile story library preview" width="100%">
</p>

## Visual identity

StoryBox3 uses a near-black blue canvas, electric blue actions, and violet/cyan
gradients to make photo stories feel cinematic and AI-assisted.

| Role | Color |
|---|---|
| Background | `#05080c` |
| Primary action | `#0080ff` |
| Secondary glow | `#8c1aff` |
| Story gradient | `#0080ff` → `#8c1aff` → `#00c8ff` → `#ff00cc` |

See the [visual identity guide](docs/visual-identity.md) and canonical theme in
[`src/index.css`](src/index.css).

## Documentation

The full documentation set is indexed in [`docs/README.md`](docs/README.md):
product, architecture, development, configuration, testing, deployment, and
changelog. Release notes remain in [`CHANGELOG.md`](CHANGELOG.md).

---

## 🚀 Quick Start

### 1. Prerequisites

| Tool | Required For | Link |
|------|-------------|------|
| **Node.js** v18+ | Frontend build | [nodejs.org](https://nodejs.org) |
| **Rust & Cargo** | Tauri backend | [rustup.rs](https://rustup.rs) |
| **OpenRouter API Key** | AI features | [openrouter.ai/keys](https://openrouter.ai/keys) |
| **Android Studio** *(optional)* | Android build | [developer.android.com/studio](https://developer.android.com/studio) |

> 💡 You do **not** need Ollama installed. Cloud AI is routed through the built-in Supabase proxy by default, so no local key is needed — but that proxy must be reachable. See [configuration](docs/configuration.md#ai-provider-settings).

---

### 2. Install & Run

```bash
# Clone the repo
git clone https://github.com/Omar-Khaled-57/StoryBox.git
cd StoryBox3

# Install JS dependencies
npm install

# Run in development mode (desktop)
npm run tauri dev
```

---

### 3. Configure AI (Optional — for your own key)

By default, StoryBox3 routes AI calls through a **Supabase Edge Function proxy** — no setup needed.

To use your own [OpenRouter](https://openrouter.ai/keys) API key:

1. Launch the app
2. Click the ⚙️ **Settings** icon in the sidebar
3. Scroll to the **AI Storyteller** section
4. Select **OpenRouter** as the provider
5. Paste your key (starts with `sk-or-v1-`) into the **API Key** field
6. Click **Save Config**
7. Go to **AI Health Check** and hit **Refresh** to verify the connection

#### 🆓 Default Free Models

| Purpose | Model | Slug |
|---------|-------|------|
| 🖼️ Vision / Image Analysis | Qwen VL Plus | `qwen/qwen-vl-plus` |
| ✍️ Caption / Title Generation | Llama 3.1 8B Instruct | `meta-llama/llama-3.1-8b-instruct` |

> You can swap these in Settings at any time. Check [openrouter.ai/models](https://openrouter.ai/models?q=free) for other free options.

---

### 4. Add Your Photos

1. Click the **➕ Add** button at the bottom of the sidebar
2. Select a folder containing your photos
3. StoryBox3 will scan and index everything automatically
4. Hit **New Story** to generate your first story

---

## 🏗️ Building for Production

```bash
# Desktop (Windows / macOS / Linux)
npm run tauri build

# Android — initialize project first (one-time)
npx tauri android init
npm run tauri android dev        # development
npm run tauri android build      # release APK
```

> ⚠️ Android builds require **Android Studio**, the **Android SDK**, and a configured **NDK**. See [Tauri Android setup docs](https://v2.tauri.app/start/prerequisites/#android) for the full guide.

---

## 🗂️ Project Structure

```
StoryBox3/
├── src/                    # React + TypeScript frontend
│   ├── components/         # UI components
│   ├── assets/             # Icons and images
│   └── index.css           # Global styles (cyber theme)
├── src-tauri/              # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── ai.rs           # AI engine (OpenRouter / Ollama / Mock)
│   │   ├── stories.rs      # Story generation & automation
│   │   ├── scanner.rs      # Photo indexing
│   │   └── lib.rs          # Tauri commands & app setup
│   ├── icons/              # App icons (all platforms)
│   └── tauri.conf.json     # App configuration
├── supabase/               # Supabase Edge Function (OpenRouter proxy)
├── dev/                    # Dev assets & experiments
└── public/                 # Static assets
```

---

## 🧱 Architecture

```
┌─────────────────────┐     ┌──────────────────────────┐
│   React Frontend    │◄───►│   Rust Backend (Tauri 2) │
│  (Vite + Tailwind)  │ IPC │                          │
└─────────────────────┘     │  ┌────────────────────┐  │
                            │  │  SQLite (sqlx)     │  │
                            │  │  stories / images  │  │
                            │  │  ai_settings       │  │
                            │  └─────────┬──────────┘  │
                            │            ▼             │
                            │  ┌────────────────────┐  │
                            │  │  AI Engine         │  │
                            │  │  ├─ OpenRouter ────┤  │
                            │  │  ├─ Ollama ────────┤  │
                            │  │  └─ Mock ──────────┘  │
                            └──────────┬───────────────┘
                                       │ HTTPS
                                       ▼
                        ┌──────────────────────────┐
                        │  Supabase Edge Function  │
                        │  (openrouter-proxy)      │
                        └──────────┬───────────────┘
                                   │ HTTPS
                                   ▼
                        ┌──────────────────────────┐
                        │  OpenRouter API          │
                        │  (free cloud AI models)  │
                        └──────────────────────────┘
```

---

## 🛠️ Tech Stack

| Layer | Technology |
|-------|-----------|
| **Frontend** | React 19, TypeScript, Tailwind CSS v3, Vite |
| **Backend** | Rust, Tauri 2, SQLite via `sqlx` |
| **AI** | OpenRouter (proxy or direct), Ollama, Mock |
| **Mobile** | Tauri dialog + fs folder picker (SAF) with base64 image ingestion |
| **Proxy** | Supabase Edge Function (Deno) |

---

## 🔐 Security

- **🔑 No hardcoded keys** — Zero API keys are embedded in the source code or binary
- **🛡️ Proxy mode** — The default Supabase proxy holds the OpenRouter key server-side; your device never sees it
- **📡 HTTPS only** — Provider calls use HTTPS URLs and `reqwest`'s `rustls-tls` feature; no explicit TLS version is pinned in code
- **🖼️ Downscaled images** — Photos are resized to 1024px before AI analysis
- **🗄️ Local-first** — All user data lives in `storybox.db` under Tauri's app-local data directory (e.g. `%APPDATA%\com.ok.storybox3\` on Windows)
- **🚫 No telemetry** — No analytics, tracking, or logging of user data

---

## 🌍 Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| 🪟 Windows | ✅ Full | Pictures, OneDrive, custom folders |
| 🍎 macOS | ✅ Full | Pictures, Apple Photos library |
| 🐧 Linux | ✅ Full | Pictures directory |
| 🤖 Android | ✅ Supported | Folder picker + base64 ingestion (SAF) |
| 📱 iOS | ⚠️ Beta | Folder picker + base64 ingestion (untested) |

---

## 🧰 Commands

Scripts defined in [`package.json`](package.json):

| Script | Command | Purpose |
| --- | --- | --- |
| `dev` | `vite` | Start the Vite dev server (fixed port 1420) |
| `build` | `tsc && vite build` | Type-check and build the frontend into `dist/` |
| `preview` | `vite preview` | Serve the built frontend locally |
| `tauri` | `tauri` | Tauri CLI entry point used by the workflows above |

The manifest declares pnpm 11.2.2 and tracks `pnpm-lock.yaml`, while the setup
steps and `tauri.conf.json` use npm. Pick one package manager consistently.

## ⚠️ Known gaps

- No `test` script or JavaScript test suite; Rust tests in `src-tauri/src/ai.rs`
  are not wrapped by a manifest command and several need live proxy access.
- The "Expire After" setting changes the cleanup schedule, but the deletion
  cutoff is a fixed 24 hours in `stories.rs`.
- `@gbyte/tauri-plugin-ios-photos` and
  `@universalappfactory/tauri-plugin-medialibrary` remain in `package.json`
  even though the Rust plugins were removed; mobile uses dialog + fs instead.
- No CI/CD or tracked store-signing configuration. `dev/` is gitignored and
  contains local release artifacts and signing material.
- See [testing](docs/testing.md) and [configuration](docs/configuration.md)
  for details.

## 📄 License

MIT — see [LICENSE](./LICENSE) for details.
