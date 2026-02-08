// Automatic Speech Recognition module
// Handles transcription using Whisper models via whisper-rs
// Parakeet backend available behind `parakeet` feature flag

pub mod backend;
#[cfg(feature = "parakeet")]
pub mod mel;
#[cfg(feature = "parakeet")]
pub mod parakeet_backend;
pub mod whisper_backend;

use anyhow::{anyhow, Result};
use std::path::PathBuf;

// Re-export backend types
pub use backend::{AsrBackend, AsrBackendType, BackendInfo, TranscriptionResult, TranscriptionSegment};
#[cfg(feature = "parakeet")]
pub use parakeet_backend::{ParakeetBackend, ParakeetModel};
pub use whisper_backend::WhisperBackend;

/// Available Whisper model sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhisperModel {
    Tiny,    // ~75MB, fastest, lowest quality
    Base,    // ~142MB, good balance for real-time
    Small,   // ~466MB, recommended default
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

/// Transcription engine that wraps an ASR backend
/// Default backend is Whisper. Parakeet available with `parakeet` feature.
pub struct TranscriptionEngine {
    backend: Box<dyn AsrBackend>,
    backend_type: AsrBackendType,
}

impl TranscriptionEngine {
    /// Create a new TranscriptionEngine with Whisper backend (default)
    pub fn new() -> Self {
        Self {
            backend: Box::new(WhisperBackend::new()),
            backend_type: AsrBackendType::Whisper,
        }
    }

    /// Create a new TranscriptionEngine with a specific backend
    pub fn with_backend(backend_type: AsrBackendType) -> Result<Self> {
        let backend: Box<dyn AsrBackend> = match backend_type {
            AsrBackendType::Whisper => Box::new(WhisperBackend::new()),
            AsrBackendType::Parakeet => {
                #[cfg(feature = "parakeet")]
                {
                    Box::new(ParakeetBackend::new())
                }
                #[cfg(not(feature = "parakeet"))]
                {
                    return Err(anyhow!(
                        "Parakeet backend is not available. Rebuild with `--features parakeet` to enable it."
                    ));
                }
            }
        };
        Ok(Self {
            backend,
            backend_type,
        })
    }

    /// Get the current backend type
    pub fn backend_type(&self) -> AsrBackendType {
        self.backend_type
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Set the language for transcription (e.g., "en", "es", "fr", "auto")
    pub fn set_language(&mut self, lang: &str) {
        self.backend.set_language(lang);
    }

    /// Get the current language
    pub fn language(&self) -> &str {
        self.backend.language()
    }

    /// Load a model from a file path
    pub fn load_model(&mut self, model_path: &PathBuf) -> Result<()> {
        self.backend.load_model(model_path)
    }

    /// Check if a model is loaded
    pub fn is_loaded(&self) -> bool {
        self.backend.is_loaded()
    }

    /// Transcribe audio samples (must be 16kHz mono f32)
    pub async fn transcribe(&self, audio_samples: &[f32]) -> Result<TranscriptionResult> {
        self.backend.transcribe(audio_samples).await
    }

    /// Transcribe with a time offset (for streaming transcription)
    pub async fn transcribe_with_offset(
        &self,
        audio_samples: &[f32],
        offset_ms: i64,
    ) -> Result<TranscriptionResult> {
        self.backend.transcribe_with_offset(audio_samples, offset_ms).await
    }
}

impl Default for TranscriptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Get available ASR backends info
pub fn get_available_backends() -> Vec<BackendInfo> {
    #[cfg(not(feature = "parakeet"))]
    {
        vec![BackendInfo::whisper()]
    }
    #[cfg(feature = "parakeet")]
    {
        let mut backends = vec![BackendInfo::whisper()];
        backends.push(BackendInfo::parakeet());
        backends
    }
}

/// Resample audio from source sample rate to 16kHz (required by ASR engines)
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
    let dirs = directories::ProjectDirs::from("com", "phantomear", "PhantomEar")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;

    let models_dir = dirs.data_dir().join("models");
    std::fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// Check if a model is downloaded (with size validation)
pub fn is_model_downloaded(model: WhisperModel) -> Result<bool> {
    let model_path = get_models_dir()?.join(model.filename());
    if !model_path.exists() {
        return Ok(false);
    }
    // Validate file size — at least 80% of expected to catch corrupt/proxy files
    let file_size = std::fs::metadata(&model_path).map(|m| m.len()).unwrap_or(0);
    let expected_min = model.size_mb() * 1024 * 1024 * 8 / 10;
    Ok(file_size >= expected_min)
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
