<div align="center">

# 👻 PhantomEar (Now Fomy.io)

### **Always Listening. Never Seen.**

*The privacy-first AI meeting assistant that lives entirely on your machine.*

<br/>

**✨ Visit our new home: [fomy.io](https://fomy.io) ✨**

<br/>

[![Website](https://img.shields.io/badge/Website-fomy.io-6c5ce7?style=for-the-badge&logo=vercel)](https://fomy.io)
[![Release](https://img.shields.io/github/v/release/Phantom-Ear/phantom-ear?style=for-the-badge&logo=github&color=00b894)](https://github.com/Phantom-Ear/phantom-ear/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Phantom-Ear/phantom-ear/total?style=for-the-badge&logo=download&color=0984e3)](https://github.com/Phantom-Ear/phantom-ear/releases)
[![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/Phantom-Ear/phantom-ear/releases)

<br/>

[![Build](https://img.shields.io/github/actions/workflow/status/Phantom-Ear/phantom-ear/release.yml?style=flat-square&logo=github-actions&logoColor=white&label=Build)](https://github.com/Phantom-Ear/phantom-ear/actions)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white)](https://svelte.dev/)

<br/>

[**Download**](#-download) • [**Features**](#-features) • [**Phomy AI**](#-meet-phomy) • [**How It Works**](#-how-it-works) • [**Tech Stack**](#-architecture)

<br/>

<img width="1117" height="970" alt="PhantomEar Dashboard" src="https://github.com/user-attachments/assets/18563c7c-07dc-47ea-ad28-5ab9c25af249" />
<img width="1117" height="970" alt="Phomy Chat Interface" src="https://github.com/user-attachments/assets/a3dc61ce-b0ef-4020-8727-c5b0fa9b655f" />

</div>

---

<br/>

## 🎯 The Problem with Modern Meeting AI

Traditional meeting assistants **expose your presence**:
- 🤖 **Bots join the call:** Everyone knows you're recording.
- ☁️ **Cloud uploads:** Your private conversations leave your secure environment.
- 🔒 **Admin privileges:** IT blocks the installation of invasive tools.
- 💰 **Expensive subscriptions:** Monthly costs rack up for simple transcription features.

<br/>

## ✨ The PhantomEar Solution

PhantomEar (fomy) is fundamentally different. It is an ambient intelligence tool built for privacy purists.

<table>
<tr>
<td width="50%">

### 🔒 **100% Local Processing**
Audio never leaves your machine. Transcription runs locally using optimized Whisper models. Your meetings, your data, your control.

</td>
<td width="50%">

### 👻 **Zero Meeting Footprint**
No jarring bots invading your Zoom calls. No "Recording in progress" banners. No participant prompts. Completely invisible.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 **Intelligent Recall**
Ask questions about any meeting. Get instant algorithmic summaries. Search across all your conversations effortlessly with bleeding-edge semantic search.

</td>
<td width="50%">

### 🏢 **Enterprise Ready out of the Box**
User-level installation. Runs smoothly without admin rights or IT tickets. Functions perfectly on restricted corporate laptops.

</td>
</tr>
</table>

<br/>

---

## 📥 Download

<div align="center">

Ready to take back your meeting privacy?

### 🚀 **[Download Latest Release](https://github.com/Phantom-Ear/phantom-ear/releases/latest)**

*(Or visit [fomy.io](https://fomy.io) for more information)*

</div>

<br/>

| Platform | Architecture | Binary |
|----------|-------------|----------|
| **macOS** | Apple Silicon (M1/M2/M3) | [`.dmg`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) |
| **Windows** | x64 | [`.exe`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) / [`.msi`](https://github.com/Phantom-Ear/phantom-ear/releases/latest) |

<details>
<summary><b>🛠 Troubleshooting Installation</b></summary>

### macOS
Since PhantomEar is open-source and not yet notarized with Apple, you may see an "App is damaged" warning. Fix it instantly with:
```bash
xattr -cr /Applications/PhantomEar.app
```

### Windows
- If `.msi` is blocked by strict corporate policy, use the portable `.exe` installer.
- If you see an `MSVCP140.dll` error on a fresh Windows system, install the [Visual C++ Runtime](https://aka.ms/vs/17/release/vc_redist.x64.exe).

</details>

<br/>

---

## 🎬 Core Features

<br/>

<table>
<tr>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/microphone.png" width="48"/>
<br/><b>Real-Time Transcription</b>
<br/><sub>Live, hyper-accurate speech-to-text powered by local Whisper.</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/chat.png" width="48"/>
<br/><b>AI Q&A</b>
<br/><sub>Interact naturally and ask hyper-specific questions about any past meeting.</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/document.png" width="48"/>
<br/><b>Auto Summaries</b>
<br/><sub>Distill hour-long meetings into key points, decisions & action items.</sub>
</td>
</tr>
<tr>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/search.png" width="48"/>
<br/><b>Semantic Search</b>
<br/><sub>Find abstract concepts or exact phrases across all your stored meetings.</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/database.png" width="48"/>
<br/><b>Persistent Memory</b>
<br/><sub>Robust SQLite backed with full-text fuzzy search.</sub>
</td>
<td align="center" width="33%">
<img src="https://img.icons8.com/fluency/96/privacy.png" width="48"/>
<br/><b>Air-Gapped Privacy</b>
<br/><sub>Everything stays on your device. Period.</sub>
</td>
</tr>
</table>

### 👻 Meet Phomy: Your Intelligent Meeting Co-Pilot

Phomy is your dedicated AI meeting assistant. It uses advanced RAG (Retrieval-Augmented Generation) to understand your intent:
- **Time Windows:** *"What did we discuss in the last 15 minutes?"*
- **Action Items:** *"List all the tasks assigned to me during this sync."*
- **Global Search:** *"When did we last talk about the Q3 marketing budget across all meetings?"*
- **Web Fallback:** If you ask a question outside the scope of your transcripts (e.g. *"Who won the world cup in the year we discussed?"*), Phomy seamlessly falls back to a live, configurable Web Search that you can cancel at any time.

### Feature Highlights

- 🎙️ **Multi-Model ASR:** Choose between varying Whisper variants (*tiny → large*) or Parakeet CTC models based on your hardware specs.
- 🤖 **LLM Flexibility:** Plug in an OpenAI API key or use **Ollama** (for a fully air-gapped, 100% local operation).
- ⏸️ **Pause/Resume:** Pause transcription on-the-fly without breaking the recording session.
- 📝 **Transcript Editing:** Modify generated segments or fix rare transcription errors.
- 🗣️ **Speaker Labels:** Manually tag speakers to dialogue segments for better context tracking.
- 📊 **Visual Timeline:** Navigate long recordings effortlessly with an interactive timeline.
- 🔍 **Quick Global Search:** Hit `Cmd+K` from anywhere to search across every recorded meeting.
- 🖥️ **System Tray Integration:** Run headlessly in the background. Start/stop directly from the menu bar.
- 📋 **Seamless Export:** One-click copy interactions as Markdown or plain text to share.

<br/>

---

## 🔬 How It Works Architecture

```text
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
                          │         OPTIONAL: LLM Context             │
                          │    (Only curated text, never raw audio)   │
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

### Data Privacy Contract

| Data Type | Location | Sent to Cloud? |
|-----------|----------|----------------|
| 🎤 Raw Audio | Ephemeral RAM only (never stored) | ❌ **Never** |
| 📝 Transcripts | Encrypted Local SQLite | ❌ **Never** |
| 🧠 Embeddings | Encrypted Local SQLite | ❌ **Never** |
| 💬 LLM Context | Snippet text | ⚠️ **Optional*** |

*\*Only if explicitly using the OpenAI API integration. Switch to Ollama for a 100% local, air-gapped data loop.*

<br/>

---

## 🏗️ Development & Contribution

### Tech Stack Breakdown

<table>
<tr>
<td align="center"><img src="https://www.rust-lang.org/logos/rust-logo-512x512.png" width="40"/><br/><b>Rust</b></td>
<td align="center"><img src="https://tauri.app/img/index/header_light.svg" width="40"/><br/><b>Tauri 2.0</b></td>
<td align="center"><img src="https://svelte.dev/favicon.png" width="40"/><br/><b>Svelte 5</b></td>
<td align="center"><img src="https://tailwindcss.com/favicons/favicon-32x32.png" width="40"/><br/><b>TailwindCSS</b></td>
</tr>
</table>

### Getting Started Locally

```bash
# 1. Clone the project
git clone https://github.com/Phantom-Ear/phantom-ear.git
cd phantom-ear

# 2. Install Node dependencies
npm install

# 3. Spin up the Tauri Dev Server
npm run tauri dev

# 4. Build for Production Release
npm run tauri build
```

*Note: You must have Node v18+, Rust 1.75+, and CMake (for whisper.cpp) installed.*

<br/>

## 🤝 Contributing

PhantomEar is built by the community. Check out our ongoing Roadmap to see what we're working on next!
1. Fork the repo.
2. Create your feature branch (`git checkout -b feature/amazing-feature`).
3. Commit your changes (`git commit -m "Add amazing feature"`).
4. Push to the branch (`git push origin feature/amazing-feature`).
5. Open a Pull Request.

<br/>

---

## 📄 License & Legal

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

<br/>

<div align="center">

### Built with 🖤 for privacy advocates at [fomy.io](https://fomy.io)

**[⬆ Back to Top](#-phantomear-now-fomyio)**

<br/>

<sub>PhantomEar is not affiliated with any meeting platform (Zoom, Teams, Google Meet, etc). Please use responsibly and ensure you comply with all local and applicable laws regarding audio recording.</sub>

</div>
