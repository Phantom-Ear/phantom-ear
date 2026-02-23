// Whisper ASR Backend
// Implementation of AsrBackend for OpenAI Whisper via whisper-rs

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::backend::{AsrBackend, TranscriptionResult, TranscriptionSegment};

pub struct WhisperBackend {
    context: Option<Arc<Mutex<WhisperContext>>>,
    model_path: Option<PathBuf>,
    language: String,
}

impl WhisperBackend {
    pub fn new() -> Self {
        Self {
            context: None,
            model_path: None,
            language: "en".to_string(),
        }
    }

    pub fn model_path(&self) -> Option<&PathBuf> {
        self.model_path.as_ref()
    }
}

impl Default for WhisperBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsrBackend for WhisperBackend {
    fn name(&self) -> &str {
        "Whisper"
    }

    fn is_loaded(&self) -> bool {
        self.context.is_some()
    }

    fn load_model(&mut self, path: &Path) -> Result<()> {
        log::info!("Loading Whisper model from: {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Model file not found: {:?}", path));
        }

        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
            params,
        )
        .map_err(|e| anyhow!("Failed to load Whisper model: {:?}", e))?;

        self.context = Some(Arc::new(Mutex::new(ctx)));
        self.model_path = Some(path.to_path_buf());

        log::info!("Whisper model loaded successfully");
        Ok(())
    }

    fn set_language(&mut self, lang: &str) {
        self.language = lang.to_string();
    }

    fn language(&self) -> &str {
        &self.language
    }

    async fn transcribe(&self, audio_samples: &[f32]) -> Result<TranscriptionResult> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| anyhow!("No model loaded"))?;

        if audio_samples.is_empty() {
            return Ok(TranscriptionResult {
                segments: vec![],
                full_text: String::new(),
            });
        }

        let ctx = context.lock().await;

        // Create a new state for this transcription
        let mut state = ctx
            .create_state()
            .map_err(|e| anyhow!("Failed to create state: {:?}", e))?;

        // Configure transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set language
        if self.language == "auto" {
            params.set_language(None); // Auto-detect
        } else {
            params.set_language(Some(&self.language));
        }

        // Optimize for real-time transcription
        params.set_translate(false);
        params.set_no_context(true);
        params.set_single_segment(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_non_speech_tokens(true);

        // Run transcription
        state
            .full(params, audio_samples)
            .map_err(|e| anyhow!("Transcription failed: {:?}", e))?;

        // Extract segments
        let num_segments = state
            .full_n_segments()
            .map_err(|e| anyhow!("Failed to get segments: {:?}", e))?;

        let mut segments = Vec::new();
        let mut full_text = String::new();

        for i in 0..num_segments {
            let text = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow!("Failed to get segment text: {:?}", e))?;

            let start = state
                .full_get_segment_t0(i)
                .map_err(|e| anyhow!("Failed to get segment start: {:?}", e))?;

            let end = state
                .full_get_segment_t1(i)
                .map_err(|e| anyhow!("Failed to get segment end: {:?}", e))?;

            // Convert from centiseconds to milliseconds
            let start_ms = start * 10;
            let end_ms = end * 10;

            let trimmed_text = text.trim().to_string();
            if !trimmed_text.is_empty() {
                if !full_text.is_empty() {
                    full_text.push(' ');
                }
                full_text.push_str(&trimmed_text);

                segments.push(TranscriptionSegment {
                    text: trimmed_text,
                    start_ms,
                    end_ms,
                });
            }
        }

        Ok(TranscriptionResult {
            segments,
            full_text,
        })
    }
}
