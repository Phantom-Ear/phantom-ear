// Automatic Speech Recognition module
// Handles transcription using Whisper models via whisper-rs

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Available Whisper model sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhisperModel {
    Tiny,    // ~75MB, fastest, lowest quality
    Base,    // ~142MB, good balance for real-time
    Small,   // ~466MB, better quality
    Medium,  // ~1.5GB, high quality
    Large,   // ~2.9GB, best quality (slow)
}

impl WhisperModel {
    pub fn filename(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "ggml-tiny.bin",
            WhisperModel::Base => "ggml-base.bin",
            WhisperModel::Small => "ggml-small.bin",
            WhisperModel::Medium => "ggml-medium.bin",
            WhisperModel::Large => "ggml-large-v3.bin",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            WhisperModel::Base => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            WhisperModel::Small => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            WhisperModel::Medium => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            WhisperModel::Large => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        }
    }

    pub fn size_mb(&self) -> u64 {
        match self {
            WhisperModel::Tiny => 75,
            WhisperModel::Base => 142,
            WhisperModel::Small => 466,
            WhisperModel::Medium => 1500,
            WhisperModel::Large => 2900,
        }
    }
}

impl std::str::FromStr for WhisperModel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tiny" => Ok(WhisperModel::Tiny),
            "base" => Ok(WhisperModel::Base),
            "small" => Ok(WhisperModel::Small),
            "medium" => Ok(WhisperModel::Medium),
            "large" => Ok(WhisperModel::Large),
            _ => Err(anyhow!("Unknown model: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptionSegment>,
    pub full_text: String,
}

pub struct TranscriptionEngine {
    context: Option<Arc<Mutex<WhisperContext>>>,
    model_path: Option<PathBuf>,
    language: String,
}

impl TranscriptionEngine {
    pub fn new() -> Self {
        Self {
            context: None,
            model_path: None,
            language: "en".to_string(),
        }
    }

    /// Set the language for transcription (e.g., "en", "es", "fr", "auto")
    pub fn set_language(&mut self, lang: &str) {
        self.language = lang.to_string();
    }

    /// Load the Whisper model from a file path
    pub fn load_model(&mut self, model_path: &PathBuf) -> Result<()> {
        log::info!("Loading Whisper model from: {:?}", model_path);

        if !model_path.exists() {
            return Err(anyhow!("Model file not found: {:?}", model_path));
        }

        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| anyhow!("Invalid path"))?,
            params,
        ).map_err(|e| anyhow!("Failed to load Whisper model: {:?}", e))?;

        self.context = Some(Arc::new(Mutex::new(ctx)));
        self.model_path = Some(model_path.clone());

        log::info!("Whisper model loaded successfully");
        Ok(())
    }

    /// Check if a model is loaded
    pub fn is_loaded(&self) -> bool {
        self.context.is_some()
    }

    /// Get the loaded model path
    pub fn model_path(&self) -> Option<&PathBuf> {
        self.model_path.as_ref()
    }

    /// Transcribe audio samples (must be 16kHz mono f32)
    pub async fn transcribe(&self, audio_samples: &[f32]) -> Result<TranscriptionResult> {
        let context = self.context.as_ref()
            .ok_or_else(|| anyhow!("No model loaded"))?;

        if audio_samples.is_empty() {
            return Ok(TranscriptionResult {
                segments: vec![],
                full_text: String::new(),
            });
        }

        let ctx = context.lock().await;

        // Create a new state for this transcription
        let mut state = ctx.create_state()
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
        state.full(params, audio_samples)
            .map_err(|e| anyhow!("Transcription failed: {:?}", e))?;

        // Extract segments
        let num_segments = state.full_n_segments()
            .map_err(|e| anyhow!("Failed to get segments: {:?}", e))?;

        let mut segments = Vec::new();
        let mut full_text = String::new();

        for i in 0..num_segments {
            let text = state.full_get_segment_text(i)
                .map_err(|e| anyhow!("Failed to get segment text: {:?}", e))?;

            let start = state.full_get_segment_t0(i)
                .map_err(|e| anyhow!("Failed to get segment start: {:?}", e))?;

            let end = state.full_get_segment_t1(i)
                .map_err(|e| anyhow!("Failed to get segment end: {:?}", e))?;

            // Convert from centiseconds to milliseconds
            let start_ms = (start as i64) * 10;
            let end_ms = (end as i64) * 10;

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

    /// Transcribe with a time offset (for streaming transcription)
    pub async fn transcribe_with_offset(
        &self,
        audio_samples: &[f32],
        offset_ms: i64,
    ) -> Result<TranscriptionResult> {
        let mut result = self.transcribe(audio_samples).await?;

        // Adjust timestamps with offset
        for segment in &mut result.segments {
            segment.start_ms += offset_ms;
            segment.end_ms += offset_ms;
        }

        Ok(result)
    }
}

/// Resample audio from source sample rate to 16kHz (required by Whisper)
pub fn resample_to_16khz(samples: &[f32], source_rate: u32) -> Result<Vec<f32>> {
    if source_rate == 16000 {
        return Ok(samples.to_vec());
    }

    use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        16000.0 / source_rate as f64,
        2.0,
        params,
        samples.len(),
        1, // mono
    ).map_err(|e| anyhow!("Failed to create resampler: {:?}", e))?;

    let waves_in = vec![samples.to_vec()];
    let waves_out = resampler.process(&waves_in, None)
        .map_err(|e| anyhow!("Resampling failed: {:?}", e))?;

    Ok(waves_out.into_iter().next().unwrap_or_default())
}

/// Get the models directory path
pub fn get_models_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "sidecar", "Sidecar")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;

    let models_dir = dirs.data_dir().join("models");
    std::fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// Check if a model is downloaded
pub fn is_model_downloaded(model: WhisperModel) -> Result<bool> {
    let model_path = get_models_dir()?.join(model.filename());
    Ok(model_path.exists())
}

/// Get the full path to a model file
pub fn get_model_path(model: WhisperModel) -> Result<PathBuf> {
    Ok(get_models_dir()?.join(model.filename()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info() {
        assert_eq!(WhisperModel::Base.filename(), "ggml-base.bin");
        assert_eq!(WhisperModel::Base.size_mb(), 142);
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!("base".parse::<WhisperModel>().unwrap(), WhisperModel::Base);
        assert_eq!("TINY".parse::<WhisperModel>().unwrap(), WhisperModel::Tiny);
    }
}
