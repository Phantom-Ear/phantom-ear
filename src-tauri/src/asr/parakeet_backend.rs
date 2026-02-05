// Parakeet ASR Backend
// Implementation of AsrBackend for NVIDIA Parakeet via ONNX Runtime
//
// Note: Parakeet models are English-only but offer fast inference.
// This is a placeholder implementation that can be expanded when ONNX Runtime
// is integrated. For now, it returns an error indicating Parakeet is not yet available.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::backend::{AsrBackend, TranscriptionResult, TranscriptionSegment};

/// Parakeet model variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParakeetModel {
    /// Parakeet CTC 0.6B - fast inference
    Ctc06b,
    /// Parakeet CTC 1.1B - better accuracy
    Ctc11b,
    /// Parakeet RNNT 0.6B - streaming capable
    Rnnt06b,
    /// Parakeet RNNT 1.1B - best accuracy
    Rnnt11b,
}

impl ParakeetModel {
    pub fn filename(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => "parakeet-ctc-0.6b.onnx",
            ParakeetModel::Ctc11b => "parakeet-ctc-1.1b.onnx",
            ParakeetModel::Rnnt06b => "parakeet-rnnt-0.6b.onnx",
            ParakeetModel::Rnnt11b => "parakeet-rnnt-1.1b.onnx",
        }
    }

    pub fn size_mb(&self) -> u64 {
        match self {
            ParakeetModel::Ctc06b => 600,
            ParakeetModel::Ctc11b => 1100,
            ParakeetModel::Rnnt06b => 650,
            ParakeetModel::Rnnt11b => 1200,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ParakeetModel::Ctc06b => "Fast inference, good accuracy",
            ParakeetModel::Ctc11b => "Better accuracy, larger model",
            ParakeetModel::Rnnt06b => "Streaming capable, fast",
            ParakeetModel::Rnnt11b => "Best accuracy, streaming",
        }
    }
}

impl std::str::FromStr for ParakeetModel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ctc-0.6b" | "ctc06b" => Ok(ParakeetModel::Ctc06b),
            "ctc-1.1b" | "ctc11b" => Ok(ParakeetModel::Ctc11b),
            "rnnt-0.6b" | "rnnt06b" => Ok(ParakeetModel::Rnnt06b),
            "rnnt-1.1b" | "rnnt11b" => Ok(ParakeetModel::Rnnt11b),
            _ => Err(anyhow!("Unknown Parakeet model: {}", s)),
        }
    }
}

pub struct ParakeetBackend {
    model_path: Option<PathBuf>,
    language: String,
    is_loaded: bool,
}

impl ParakeetBackend {
    pub fn new() -> Self {
        Self {
            model_path: None,
            language: "en".to_string(), // Parakeet is English-only
            is_loaded: false,
        }
    }

    pub fn model_path(&self) -> Option<&PathBuf> {
        self.model_path.as_ref()
    }
}

impl Default for ParakeetBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsrBackend for ParakeetBackend {
    fn name(&self) -> &str {
        "Parakeet"
    }

    fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    fn load_model(&mut self, path: &Path) -> Result<()> {
        log::info!("Loading Parakeet model from: {:?}", path);

        if !path.exists() {
            return Err(anyhow!("Model file not found: {:?}", path));
        }

        // Check file extension
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if extension != "onnx" {
            return Err(anyhow!("Parakeet requires an ONNX model file (.onnx)"));
        }

        // TODO: Integrate ONNX Runtime when available
        // For now, we'll return an error indicating Parakeet support is coming
        return Err(anyhow!(
            "Parakeet ONNX support is not yet implemented. \
             Please use Whisper backend for now. \
             Parakeet integration requires ONNX Runtime which will be added in a future update."
        ));

        // Future implementation will:
        // 1. Load ONNX model using `ort` crate
        // 2. Initialize tokenizer/vocabulary
        // 3. Set up inference session

        #[allow(unreachable_code)]
        {
            self.model_path = Some(path.to_path_buf());
            self.is_loaded = true;
            log::info!("Parakeet model loaded successfully");
            Ok(())
        }
    }

    fn set_language(&mut self, lang: &str) {
        // Parakeet only supports English
        if lang != "en" && lang != "auto" {
            log::warn!("Parakeet only supports English. Ignoring language setting: {}", lang);
        }
        self.language = "en".to_string();
    }

    fn language(&self) -> &str {
        &self.language
    }

    async fn transcribe(&self, _samples: &[f32]) -> Result<TranscriptionResult> {
        if !self.is_loaded {
            return Err(anyhow!("No model loaded"));
        }

        // TODO: Implement ONNX Runtime inference
        // For now, return placeholder
        Err(anyhow!(
            "Parakeet transcription not yet implemented. \
             Please use Whisper backend for transcription."
        ))

        // Future implementation will:
        // 1. Preprocess audio (mel spectrogram)
        // 2. Run ONNX inference
        // 3. Decode output tokens to text
        // 4. Return TranscriptionResult with segments
    }
}

/// Get the models directory for Parakeet
pub fn get_parakeet_models_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "sidecar", "Sidecar")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;

    let models_dir = dirs.data_dir().join("models").join("parakeet");
    std::fs::create_dir_all(&models_dir)?;

    Ok(models_dir)
}

/// Check if a Parakeet model is downloaded
pub fn is_parakeet_model_downloaded(model: ParakeetModel) -> Result<bool> {
    let model_path = get_parakeet_models_dir()?.join(model.filename());
    Ok(model_path.exists())
}

/// Get the full path to a Parakeet model file
pub fn get_parakeet_model_path(model: ParakeetModel) -> Result<PathBuf> {
    Ok(get_parakeet_models_dir()?.join(model.filename()))
}
