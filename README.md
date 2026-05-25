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

---

## 🚀 Quick Start

### 1. Prerequisites

| Tool | Required For | Link |
|------|-------------|------|
| **Node.js** v18+ | Frontend build | [nodejs.org](https://nodejs.org) |
| **Rust & Cargo** | Tauri backend | [rustup.rs](https://rustup.rs) |
| **OpenRouter API Key** | AI features | [openrouter.ai/keys](https://openrouter.ai/keys) |
| **Android Studio** *(optional)* | Android build | [developer.android.com/studio](https://developer.android.com/studio) |

> 💡 You do **not** need Ollama installed. Cloud AI works out of the box via the built-in proxy — no API key required for basic use.

---

### 2. Install & Run

```bash
# Clone the repo
git clone https://github.com/Omar-Khaled-57/StoryBox3.git
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
                            │            ▼              │
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
| **Mobile** | `tauri-plugin-medialibrary` (Android), `tauri-plugin-ios-photos` (iOS) |
| **Proxy** | Supabase Edge Function (Deno) |

---

## 🔐 Security

- **🔑 No hardcoded keys** — Zero API keys are embedded in the source code or binary
- **🛡️ Proxy mode** — The default Supabase proxy holds the OpenRouter key server-side; your device never sees it
- **📡 HTTPS only** — All external calls use TLS 1.3
- **🖼️ Downscaled images** — Photos are resized to 1024px before AI analysis
- **🗄️ Local-first** — All user data lives in SQLite on your device (`%APPDATA%\com.storybox3.app\` on Windows)
- **🚫 No telemetry** — No analytics, tracking, or logging of user data

---

## 🌍 Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| 🪟 Windows | ✅ Full | Pictures, OneDrive, custom folders |
| 🍎 macOS | ✅ Full | Pictures, Apple Photos library |
| 🐧 Linux | ✅ Full | Pictures directory |
| 🤖 Android | ✅ Supported | `tauri-plugin-medialibrary` |
| 📱 iOS | ⚠️ Beta | `tauri-plugin-ios-photos` (untested) |

---

## 📄 License

MIT — see [LICENSE](./LICENSE) for details.
