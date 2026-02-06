# Phantom Ear

A privacy-first desktop meeting assistant that provides real-time transcription, contextual Q&A, and automated summaries — without joining meetings or leaking raw data.

## Why Phantom Ear?

- **Invisible by default** — No bot participants, no recording banners. Captures system/mic audio silently.
- **Local-first** — Audio, transcripts, and embeddings stay on your device. Nothing leaves your machine unless you choose an external LLM.
- **Selective cloud intelligence** — Only curated context is sent to LLM APIs (OpenAI/Ollama). Raw audio never leaves the device.
- **No admin required** — Portable, user-level app. No IT approval needed.

## Features

- **Real-time transcription** — Live speech-to-text powered by Whisper (via whisper.cpp) with optional Parakeet ONNX backend
- **AI Q&A** — Ask questions about the current or any past meeting using RAG-style context
- **Meeting summaries** — One-click structured summaries with key points and action items
- **Meeting persistence** — SQLite database with full-text search across all past meetings
- **Pause/Resume** — Pause transcription mid-meeting without ending the session
- **Export** — Copy transcripts as Markdown to clipboard
- **Multi-model support** — Choose between Whisper models (tiny → large) or Parakeet CTC models
- **LLM flexibility** — Switch between Ollama (local) and OpenAI from the top bar

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | Tauri 2.0 + Rust |
| Frontend | Svelte 5 + TailwindCSS v4 |
| ASR | Whisper (whisper-rs) / Parakeet (ONNX Runtime) |
| Database | SQLite (rusqlite) with WAL mode + FTS5 |
| LLM | OpenAI API / Ollama (local) |
| Audio | cpal (cross-platform capture) |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) (latest stable)
- cmake: `brew install cmake` (macOS) or `sudo apt install cmake` (Linux)

### Install & Run

```bash
# Clone the repo
git clone https://github.com/Phantom-Ear/phantom-ear.git
cd phantom-ear

# Install frontend dependencies
npm install

# Run in development mode (starts Vite + Tauri)
npm run tauri dev
```

On first launch, you'll be prompted to download a speech recognition model (~150MB for the recommended `small` model). After that, click the record button and start talking.

### Build for Production

```bash
npm run tauri build
```

Produces platform-specific installers in `src-tauri/target/release/bundle/`.

### Optional: Parakeet Backend

For faster English-only transcription using ONNX Runtime:

```bash
cd src-tauri && cargo build --features parakeet
```

## Architecture

```
phantom-ear/
├── src/                        # Svelte 5 frontend
│   ├── app.css                 # TailwindCSS theme (dark/light)
│   ├── routes/+page.svelte     # Main app page
│   └── lib/
│       ├── components/         # UI components (TopBar, Sidebar, Settings, etc.)
│       ├── stores/             # Svelte 5 rune-based stores
│       └── types.ts            # TypeScript interfaces
├── src-tauri/                  # Rust backend
│   └── src/
│       ├── lib.rs              # Tauri app setup & state initialization
│       ├── commands.rs         # IPC command handlers
│       ├── audio/              # Audio capture (cpal)
│       ├── asr/                # ASR engines (Whisper + Parakeet)
│       ├── transcription/      # Real-time chunked transcription pipeline
│       ├── llm/                # OpenAI & Ollama clients
│       ├── storage/            # SQLite persistence + FTS5 search
│       ├── models/             # Model download & management
│       └── specs/              # Device capability detection
```

## How It Works

1. **Audio Capture** — cpal captures audio from the default input device at native sample rate, resampled to 16kHz
2. **Chunked Transcription** — Audio is processed in 5-second chunks with silence detection (RMS threshold)
3. **ASR Inference** — Each chunk is transcribed by Whisper or Parakeet, emitting results as Tauri events
4. **Persistence** — Segments are stored in SQLite in real-time; full-text search via FTS5
5. **LLM Integration** — Transcript context is sent to Ollama or OpenAI for Q&A and summaries

## Development Commands

```bash
npm run tauri dev       # Dev mode with hot reload
npm run check           # Svelte type checking
cd src-tauri && cargo check   # Rust type checking
```

## License

MIT

