// ASR Backend trait
// Abstraction layer for different ASR engines (Whisper, Parakeet, etc.)

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Result of a transcription
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptionSegment>,
    pub full_text: String,
}

/// A segment of transcribed text with timestamps
#[derive(Debug, Clone)]
pub struct TranscriptionSegment {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// Available ASR backend types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AsrBackendType {
    Whisper,
    Parakeet,
}

impl std::str::FromStr for AsrBackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "whisper" => Ok(AsrBackendType::Whisper),
            "parakeet" => Ok(AsrBackendType::Parakeet),
            _ => Err(anyhow::anyhow!("Unknown ASR backend: {}", s)),
        }
    }
}

impl std::fmt::Display for AsrBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsrBackendType::Whisper => write!(f, "whisper"),
            AsrBackendType::Parakeet => write!(f, "parakeet"),
        }
    }
}

/// Trait for ASR backends
/// All ASR engines must implement this trait to be used by the transcription engine
#[async_trait]
pub trait AsrBackend: Send + Sync {
    /// Get the name of this backend
    fn name(&self) -> &str;

    /// Check if a model is loaded
    fn is_loaded(&self) -> bool;

    /// Load a model from the given path
    fn load_model(&mut self, path: &Path) -> Result<()>;

    /// Set the language for transcription
    fn set_language(&mut self, lang: &str);

    /// Get the current language setting
    fn language(&self) -> &str;

    /// Transcribe audio samples (must be 16kHz mono f32)
    async fn transcribe(&self, samples: &[f32]) -> Result<TranscriptionResult>;

    /// Transcribe with a time offset (for streaming)
    async fn transcribe_with_offset(
        &self,
        samples: &[f32],
        offset_ms: i64,
    ) -> Result<TranscriptionResult> {
        let mut result = self.transcribe(samples).await?;
        for segment in &mut result.segments {
            segment.start_ms += offset_ms;
            segment.end_ms += offset_ms;
        }
        Ok(result)
    }
}

/// Backend info for display in UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackendInfo {
    pub backend_type: String,
    pub name: String,
    pub description: String,
    pub supported_languages: Vec<String>,
}

impl BackendInfo {
    pub fn whisper() -> Self {
        Self {
            backend_type: "whisper".to_string(),
            name: "Whisper".to_string(),
            description: "OpenAI Whisper - accurate multilingual transcription".to_string(),
            supported_languages: vec![
                "auto".to_string(),
                "en".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "it".to_string(),
                "pt".to_string(),
                "nl".to_string(),
                "pl".to_string(),
                "ru".to_string(),
                "ja".to_string(),
                "ko".to_string(),
                "zh".to_string(),
                "ar".to_string(),
            ],
        }
    }

    pub fn parakeet() -> Self {
        Self {
            backend_type: "parakeet".to_string(),
            name: "Parakeet".to_string(),
            description: "NVIDIA Parakeet - fast English transcription".to_string(),
            supported_languages: vec!["en".to_string()],
        }
    }
}
