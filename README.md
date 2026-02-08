# 👻 PhantomEar

**Always Listening. Never Seen.**

PhantomEar is a **privacy-first desktop meeting assistant** that runs silently on your machine, providing real-time transcription, contextual Q&A, and automatic summaries — **without joining meetings, showing bots, or leaking your data**.

<p align="center">
  <a href="https://github.com/Phantom-Ear/phantom-ear/releases/latest">
    <img src="https://img.shields.io/github/v/release/Phantom-Ear/phantom-ear?style=for-the-badge" />
  </a>
  <a href="https://phantomear.com">
    <img src="https://img.shields.io/badge/Website-phantomear.com-6c5ce7?style=for-the-badge" />
  </a>
</p>

---

## 🚀 Download

**Prebuilt installers (macOS & Windows):**

👉 **Download PhantomEar v0.1.1**  
https://github.com/Phantom-Ear/phantom-ear/releases/latest

- macOS (Apple Silicon)
- Windows (x64)
- No admin permissions required
- Runs on restricted corporate machines

---

## Why PhantomEar?

- **Invisible by default**  
  No bot participants. No recording banners. No visible presence in meetings.

- **Local-first by design**  
  Audio, transcripts, and embeddings stay on your device. Always.

- **Selective cloud intelligence**  
  Only curated context is sent to LLM APIs (OpenAI or Ollama).  
  **Raw audio never leaves your machine.**

- **Enterprise-friendly**  
  User-level installation. No admin rights. No IT tickets.

---

## ✨ Features

- **Real-time transcription**  
  Live speech-to-text powered by Whisper (`whisper.cpp`) with optional Parakeet (ONNX) backend.

- **AI Q&A (RAG)**  
  Ask questions about the current meeting or any past meeting using local semantic search.

- **Automatic meeting summaries**  
  Structured summaries with key points, decisions, and action items generated at meeting end.

- **Persistent memory**  
  SQLite database with full-text search across all meetings.

- **Pause / Resume**  
  Pause transcription mid-meeting without stopping the session.

- **Export**  
  Copy transcripts and summaries as Markdown.

- **Multi-model ASR**  
  Whisper (tiny → large) or Parakeet CTC models.

- **LLM flexibility**  
  Switch between Ollama (local) and OpenAI directly from the UI.

---

## 🧱 Tech Stack

| Layer | Technology |
|------|-----------|
| Shell | Tauri 2.0 + Rust |
| Frontend | Svelte 5 + TailwindCSS v4 |
| ASR | Whisper (`whisper-rs`) / Parakeet (ONNX Runtime) |
| Database | SQLite (WAL mode) + FTS5 |
| LLM | OpenAI API / Ollama (local) |
| Audio | `cpal` (cross-platform capture) |

---

## 🛠 Getting Started (Development)

### Prerequisites

- Node.js 18+ — https://nodejs.org/
- Rust (latest stable) — https://rustup.rs/
- CMake  
  - macOS: `brew install cmake`  
  - Linux: `sudo apt install cmake`

### Install & Run

```bash
git clone https://github.com/Phantom-Ear/phantom-ear.git
cd phantom-ear
npm install
npm run tauri dev