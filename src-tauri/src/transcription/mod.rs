// Real-time transcription pipeline
// Processes audio chunks and emits transcription results

use serde::Serialize;

/// Transcription segment emitted to frontend
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionEvent {
    pub id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub is_partial: bool,
}

/// Configuration for the transcription pipeline
pub struct TranscriptionConfig {
    /// Duration of audio chunks to process (in seconds)
    pub chunk_duration_secs: f32,
    /// Overlap between chunks (in seconds) for better continuity
    pub overlap_secs: f32,
    /// Minimum audio level to trigger transcription
    pub silence_threshold: f32,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            chunk_duration_secs: 5.0, // Process every 5 seconds for faster feedback
            overlap_secs: 0.5,
            silence_threshold: 0.01,
        }
    }
}

