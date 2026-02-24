<div align="center">

# 👻 PhantomEar

### **Always Listening. Never Seen.**

*The privacy-first AI meeting assistant that lives on your machine*

<br/>

[![Release](https://img.shields.io/github/v/release/Phantom-Ear/phantom-ear?style=for-the-badge&logo=github&color=6c5ce7)](https://github.com/Phantom-Ear/phantom-ear/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Phantom-Ear/phantom-ear/total?style=for-the-badge&logo=download&color=00b894)](https://github.com/Phantom-Ear/phantom-ear/releases)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/Phantom-Ear/phantom-ear/releases)

<br/>

[![Build](https://img.shields.io/github/actions/workflow/status/Phantom-Ear/phantom-ear/release.yml?style=flat-square&logo=github-actions&logoColor=white&label=Build)](https://github.com/Phantom-Ear/phantom-ear/actions)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev/)

<br/>

[**Download**](#-download) • [**Features**](#-features) • [**How It Works**](#-how-it-works) • [**Tech Stack**](#-architecture) • [**Development**](#-development)

<br/>
<img width="1117" height="970" alt="image" src="https://github.com/user-attachments/assets/18563c7c-07dc-47ea-ad28-5ab9c25af249" />
<img width="1117" height="970" alt="image" src="https://github.com/user-attachments/assets/a3dc61ce-b0ef-4020-8727-c5b0fa9b655f" />

</div>

---

<br/>

## 🎯 The Problem

Traditional meeting assistants **expose your presence**:
- 🤖 Bot joins the call → Everyone knows you're recording
- ☁️ Audio uploaded to cloud → Your data leaves your control
- 🔒 Admin install required → IT blocks the tool
- 💰 Monthly subscriptions → Costs add up

<br/>

## ✨ The PhantomEar Difference

<table>
<tr>
<td width="50%">

### 🔒 **100% Local Processing**
Audio never leaves your machine. Transcription runs locally using Whisper. Your meetings, your data, your control.

</td>
<td width="50%">

### 👻 **Zero Meeting Footprint**
No bots joining calls. No recording banners. No participant notifications. Completely invisible.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 **AI-Powered Intelligence**
Ask questions about any meeting. Get instant summaries. Search across all your conversations with semantic search.

</td>
<td width="50%">

### 🏢 **Enterprise Ready**
User-level installation. No admin rights. No IT tickets. Works on restricted corporate machines.

</td>
</tr>
</table>

<br/>

---

## 📥 Download

<div align="center">

### 🚀 **[Download Latest Release](https://github.com/Phantom-Ear/phantom-ear/releases/latest)**

</div>

<br/>

| Platform | Architecture | Download |
|----------|-------------|----------|
| **macOS** | Apple Silicon (M1/M2/M3) | [`.dmg`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) |
| **Windows** | x64 | [`.exe`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) / [`.msi`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) |

<details>
<summary><b>📋 Installation Notes</b></summary>

### macOS
Since PhantomEar is not yet notarized with Apple, you may see a "damaged" warning. Fix it with:
```bash
xattr -cr /Applications/PhantomEar.app
```

### Windows
- If `.msi` is blocked by corporate policy, use the `.exe` installer
- If you see `MSVCP140.dll` error, install [Visual C++ Runtime](https://aka.ms/vs/17/release/vc_redist.x64.exe)

</details>

<br/>

---

## 🎬 Features

<br/>

<table>
<tr>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/microphone.png" width="48"/>
<br/><b>Real-Time Transcription</b>
<br/><sub>Live speech-to-text powered by Whisper</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/chat.png" width="48"/>
<br/><b>AI Q&A</b>
<br/><sub>Ask questions about any meeting</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/document.png" width="48"/>
<br/><b>Auto Summaries</b>
<br/><sub>Key points, decisions & action items</sub>
</td>
</tr>
<tr>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/search.png" width="48"/>
<br/><b>Semantic Search</b>
<br/><sub>Find anything across all meetings</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/database.png" width="48"/>
<br/><b>Persistent Memory</b>
<br/><sub>SQLite with full-text search</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/privacy.png" width="48"/>
<br/><b>100% Private</b>
<br/><sub>Everything stays on your device</sub>
</td>
</tr>
</table>

<br/>

### Feature Highlights

| Feature | Description |
|---------|-------------|
| 🎙️ **Multi-Model ASR** | Whisper (tiny → large) or Parakeet CTC models |
| 🤖 **LLM Flexibility** | OpenAI API or Ollama (fully local) |
| ⏸️ **Pause/Resume** | Pause transcription without stopping the session |
| 📝 **Transcript Editing** | Edit segments, fix transcription errors |
| 🗣️ **Speaker Labels** | Manually assign speakers to segments |
| 📊 **Timeline View** | Visual timeline with quick navigation |
| 🔍 **Quick Search** | `Cmd+K` to search across all meetings |
| 🖥️ **System Tray** | Minimize to tray, start/stop from menu |
| 📋 **Export** | Copy as Markdown or plain text |

<br/>

---

## 🔬 How It Works

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           YOUR MACHINE                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────────┐ │
│  │  Microphone │───▶│   Whisper   │───▶│   SQLite    │───▶│    UI    │ │
│  │   (cpal)    │    │  (local)    │    │  + FTS5     │    │ (Svelte) │ │
│  └─────────────┘    └─────────────┘    └─────────────┘    └──────────┘ │
│                            │                  │                         │
│                            ▼                  ▼                         │
│                     ┌─────────────┐    ┌─────────────┐                  │
│                     │ Embeddings  │    │  Semantic   │                  │
│                     │ (BGE-small) │───▶│   Search    │                  │
│                     └─────────────┘    └─────────────┘                  │
│                                               │                         │
└───────────────────────────────────────────────│─────────────────────────┘
                                                │
                          ┌─────────────────────┴─────────────────────┐
                          │         OPTIONAL: LLM Context              │
                          │    (Only curated text, never raw audio)    │
                          └─────────────────────┬─────────────────────┘
                                                │
                                    ┌───────────┴───────────┐
                                    ▼                       ▼
                             ┌──────────┐            ┌──────────┐
                             │  Ollama  │            │  OpenAI  │
                             │ (local)  │            │  (API)   │
                             └──────────┘            └──────────┘
```

<br/>

### Privacy by Design

| Data Type | Location | Sent to Cloud? |
|-----------|----------|----------------|
| 🎤 Raw Audio | RAM only (never saved) | ❌ Never |
| 📝 Transcripts | Local SQLite | ❌ Never |
| 🧠 Embeddings | Local SQLite | ❌ Never |
| 💬 LLM Context | Selected text only | ⚠️ Optional* |

*Only if using OpenAI. Use Ollama for 100% local operation.

<br/>

---

## 🏗️ Architecture

<br/>

```
phantom-ear/
├── src/                          # Svelte 5 Frontend
│   ├── routes/+page.svelte       # Main application
│   ├── lib/
│   │   ├── components/           # UI components
│   │   ├── stores/               # State management
│   │   └── utils/                # Utilities
│   └── app.css                   # TailwindCSS v4
│
├── src-tauri/                    # Rust Backend
│   └── src/
│       ├── lib.rs                # Tauri entry point
│       ├── commands.rs           # IPC handlers
│       ├── audio/                # Audio capture (cpal)
│       ├── asr/                  # Whisper integration
│       ├── transcription/        # Real-time pipeline
│       ├── embeddings/           # BGE-small vectors
│       ├── storage/              # SQLite + FTS5
│       └── llm/                  # OpenAI/Ollama clients
```

<br/>

### Tech Stack

<table>
<tr>
<td align="center"><img src="https://www.rust-lang.org/logos/rust-logo-512x512.png" width="40"/><br/><b>Rust</b></td>
<td align="center"><img src="https://tauri.app/img/index/header_light.svg" width="40"/><br/><b>Tauri 2.0</b></td>
<td align="center"><img src="https://svelte.dev/favicon.png" width="40"/><br/><b>Svelte 5</b></td>
<td align="center"><img src="https://tailwindcss.com/favicons/favicon-32x32.png" width="40"/><br/><b>TailwindCSS</b></td>
</tr>
</table>

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Runtime** | Tauri 2.0 | Lightweight native wrapper |
| **Backend** | Rust | Performance-critical processing |
| **Frontend** | Svelte 5 | Reactive UI with runes |
| **Styling** | TailwindCSS v4 | Dark theme, glassmorphism |
| **ASR** | whisper-rs | Local speech recognition |
| **Embeddings** | ONNX Runtime | BGE-small vectors |
| **Database** | SQLite + FTS5 | Full-text search |
| **Audio** | cpal | Cross-platform capture |

<br/>

---

## 🛠️ Development

### Prerequisites

```bash
# Node.js 18+
node --version  # v18.0.0+

# Rust (latest stable)
rustc --version  # 1.75.0+

# CMake (for whisper.cpp)
cmake --version  # 3.20+
```

<details>
<summary><b>Install CMake</b></summary>

```bash
# macOS
brew install cmake

# Windows
choco install cmake

# Linux
sudo apt install cmake
```

</details>

### Quick Start

```bash
# Clone
git clone https://github.com/Phantom-Ear/phantom-ear.git
cd phantom-ear

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

<br/>

---

## 🗺️ Roadmap

<br/>

| Status | Feature | Target |
|--------|---------|--------|
| ✅ | Real-time transcription | v0.1.0 |
| ✅ | Meeting persistence | v0.1.0 |
| ✅ | AI Q&A (RAG) | v0.1.0 |
| ✅ | Semantic search | v0.1.0 |
| ✅ | System tray | v0.2.0 |
| ✅ | Transcript editing | v0.2.0 |
| ✅ | Speaker labels | v0.2.0 |
| 🔄 | Audio device selection | v0.3.0 |
| 📋 | SRT subtitle export | v0.3.0 |
| 📋 | Auto-meeting detection | v0.4.0 |
| 📋 | Light theme | v0.4.0 |
| 📋 | Auto-titling with AI | v0.5.0 |

<br/>

---

## 🤝 Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a PR.

```bash
# Fork the repo
# Create your feature branch
git checkout -b feature/amazing-feature

# Commit your changes
git commit -m "Add amazing feature"

# Push to the branch
git push origin feature/amazing-feature

# Open a Pull Request
```

<br/>

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

<br/>

---

<div align="center">

### Built with 🖤 for privacy advocates

<br/>

**[⬆ Back to Top](#-phantomear)**

<br/>

<sub>PhantomEar is not affiliated with any meeting platform. Use responsibly and in compliance with applicable laws.</sub>

</div>
