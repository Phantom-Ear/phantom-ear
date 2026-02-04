# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- **ASR**: Parakeet (with Whisper fallback)
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

# Format check
npm run check:watch
```

## Architecture

```
sidecar/
├── src/                    # Svelte frontend
│   ├── app.css             # TailwindCSS with custom theme
│   ├── routes/             # SvelteKit pages
│   └── lib/                # Components, stores, utilities
├── src-tauri/              # Rust backend
│   └── src/
│       ├── lib.rs          # Tauri app entry point
│       ├── commands.rs     # IPC command handlers
│       ├── audio/          # System audio capture (macOS/Windows)
│       ├── asr/            # Speech recognition (Parakeet/Whisper)
│       ├── embeddings/     # Text embedding generation
│       ├── storage/        # SQLite + vector search
│       ├── llm/            # OpenAI/Ollama clients
│       └── detection/      # Meeting app detection
```

## Key Design Decisions

### Audio Capture
- macOS: ScreenCaptureKit (requires user permission, no admin)
- Windows: WASAPI loopback

### Transcription Pipeline
1. Audio captured in 15-second sliding windows
2. Each chunk transcribed locally via Parakeet
3. Embeddings generated and stored in sqlite-vec
4. Enables real-time RAG for Q&A

### LLM Integration
- OpenAI-compatible interface for both providers
- Context window managed by selecting relevant chunks via vector search
- Raw audio/full transcripts never leave device

## Frontend Theme

Custom Sidecar colors defined in `src/app.css`:
- `bg-sidecar-bg` - Main background (#0a0a0b)
- `bg-sidecar-surface` - Card/panel background (#141416)
- `text-sidecar-text` - Primary text (#fafafa)
- `text-sidecar-text-muted` - Secondary text (#71717a)
- `bg-sidecar-accent` - Primary accent blue (#3b82f6)

## Platform Targets

Primary: macOS and Windows (corporate deployments)
- Windows is priority for corporate expansion
- macOS uses user-level permissions only
