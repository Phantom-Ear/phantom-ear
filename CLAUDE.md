# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Phases

### Phase 1: Project Setup ✅
- Initialize Tauri 2.0 + Svelte 5 project
- Configure TailwindCSS v4 with custom dark theme
- Set up project structure and CLAUDE.md
- Git repository initialization

### Phase 2: Core Infrastructure ✅
- Audio capture module using cpal (cross-platform)
- Whisper ASR integration via whisper-rs
- Model download system with progress events
- Basic UI with Setup flow for first-run model download

### Phase 3: Real-time Transcription ✅
- Transcription pipeline (5-second audio chunks)
- Silence detection to skip quiet audio
- Real-time event emission to frontend
- Live transcript display with auto-scroll
- Model loading on app restart fix

### Phase 4: Settings & Language Support ✅
- Settings UI panel (language, model selection, LLM config)
- Apply language setting to transcription engine
- LLM provider configuration (Ollama/OpenAI)

### Phase 5: LLM Integration ✅
- Ollama client for local LLM inference
- OpenAI API client
- RAG-based Q&A using transcript context
- Meeting summary generation with structured output

### Phase 6: Meeting Persistence (Upcoming)
- Save transcripts to SQLite database
- Meeting history browser
- Export to text/markdown/PDF
- Search across past meetings

### Phase 7: Embeddings & Semantic Search (Upcoming)
- BGE-small embedding generation
- sqlite-vec for vector storage
- Semantic search across transcripts
- Context retrieval for Q&A

### Phase 8: Polish & Distribution (Upcoming)
- Meeting app auto-detection
- System tray integration
- Auto-update mechanism
- macOS/Windows installers

---

## Project Overview

**Sidecar** - A privacy-first desktop meeting assistant that provides real-time transcription, contextual Q&A, and automated summaries without joining meetings or leaking raw data.

Core principles:
- Invisible by default (no bot participants, no recording banners)
- Local-first (audio, transcripts, embeddings stay on device)
- Selective cloud intelligence (only curated context sent to LLM APIs)
- No-admin install (portable, user-level app)

## Tech Stack

- **Shell**: Tauri 2.0 + Rust
- **Frontend**: Svelte 5 + TailwindCSS v4 (via Vite plugin)
- **ASR**: Whisper (via whisper-rs/whisper.cpp) - models downloaded on first run
- **Embeddings**: BGE-small (default), upgradeable to Nomic
- **Vector DB**: SQLite + sqlite-vec
- **LLM**: OpenAI / Ollama (user-configurable)

## Development Commands

```bash
# Install dependencies
npm install

# Run development (starts both Vite and Tauri)
npm run tauri dev

# Build for production
npm run tauri build

# Type check Svelte
npm run check

# Rust check only
cd src-tauri && cargo check
```

## Prerequisites

- Node.js 18+
- Rust (latest stable via rustup)
- cmake (for whisper.cpp compilation): `brew install cmake`

## Architecture

```
sidecar/
├── src/                      # Svelte frontend
│   ├── app.css               # TailwindCSS with custom theme
│   ├── routes/+page.svelte   # Main recording UI
│   └── lib/components/       # Reusable components
│       └── Setup.svelte      # First-run model download UI
├── src-tauri/                # Rust backend
│   └── src/
│       ├── lib.rs            # Tauri app entry, state management
│       ├── commands.rs       # IPC command handlers + AppState
│       ├── audio/            # Audio capture via cpal
│       ├── asr/              # Whisper transcription engine
│       ├── models/           # Model download with progress events
│       ├── embeddings/       # Text embedding generation
│       ├── storage/          # SQLite + vector search
│       ├── llm/              # OpenAI/Ollama clients
│       └── detection/        # Meeting app detection
```

## Key Modules

### Audio Capture (`src-tauri/src/audio/`)
- Uses `cpal` crate for cross-platform audio input
- Captures from default microphone (system audio requires additional setup)
- Stores samples in thread-safe buffer

### ASR (`src-tauri/src/asr/`)
- Whisper models: tiny (75MB), base (142MB, recommended), small, medium, large
- Models downloaded from HuggingFace on first run
- Resampling to 16kHz via `rubato`

### Model Download (`src-tauri/src/models/`)
- Downloads models to `~/Library/Application Support/com.sidecar.Sidecar/models/`
- Emits `model-download-progress` events to frontend

### Transcription (`src-tauri/src/transcription/`)
- Processes audio in 5-second chunks
- Silence threshold detection (RMS < 0.01)
- Emits `transcription` events to frontend in real-time

### Commands (`src-tauri/src/commands.rs`)
- `start_recording` / `stop_recording` - Audio capture + transcription control
- `download_model` - Download and load Whisper model
- `load_model` - Load already-downloaded model into memory
- `check_model_status` - Check if model file exists
- `get_models_info` - List all available models with download status
- `ask_question` / `generate_summary` - LLM features (requires config)
- `get_settings` / `save_settings` - User preferences

## Frontend

### Custom Theme (`src/app.css`)
Colors defined in `@theme`:
- `--color-sidecar-bg`: #0a0a0b (main background)
- `--color-sidecar-surface`: #141416 (cards/panels)
- `--color-sidecar-accent`: #3b82f6 (primary blue)
- `--color-sidecar-danger`: #ef4444 (recording indicator)

### State Management
Uses Svelte 5 runes (`$state`, `$props`) for reactive state.

## Tauri Events

Listen for events from Rust:
```typescript
import { listen } from "@tauri-apps/api/event";

// Model download progress
await listen<DownloadProgress>("model-download-progress", (event) => {
  console.log(event.payload.percentage);
});

// Real-time transcription
await listen<TranscriptionEvent>("transcription", (event) => {
  const { id, text, start_ms, end_ms, is_partial } = event.payload;
});
```

## Platform Notes

- **macOS**: Microphone access requires user permission (granted at runtime)
- **Windows**: WASAPI for audio, same permission model
- Models stored in platform-specific app data directory
