// Real-time transcription pipeline
// Processes audio chunks and emits transcription results

use crate::asr::{resample_to_16khz, TranscriptionEngine};
use crate::audio::AudioCapture;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

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

/// Real-time transcription worker
pub struct TranscriptionWorker {
    app: AppHandle,
    engine: Arc<Mutex<TranscriptionEngine>>,
    config: TranscriptionConfig,
    segment_counter: u64,
    total_duration_ms: i64,
}

impl TranscriptionWorker {
    pub fn new(
        app: AppHandle,
        engine: Arc<Mutex<TranscriptionEngine>>,
        config: TranscriptionConfig,
    ) -> Self {
        Self {
            app,
            engine,
            config,
            segment_counter: 0,
            total_duration_ms: 0,
        }
    }

    /// Process audio samples and emit transcription
    pub async fn process_audio(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
    ) -> Result<Option<TranscriptionEvent>, String> {
        if samples.is_empty() {
            return Ok(None);
        }

        // Check if audio is mostly silence
        let rms = calculate_rms(&samples);
        if rms < self.config.silence_threshold {
            log::debug!("Audio below silence threshold (RMS: {})", rms);
            return Ok(None);
        }

        // Resample to 16kHz if needed
        let samples_16k = if sample_rate != 16000 {
            resample_to_16khz(&samples, sample_rate)
                .map_err(|e| format!("Resampling failed: {}", e))?
        } else {
            samples
        };

        // Calculate duration
        let duration_ms = (samples_16k.len() as f32 / 16.0) as i64;

        // Run transcription
        let engine = self.engine.lock().await;
        let result = engine
            .transcribe(&samples_16k)
            .await
            .map_err(|e| format!("Transcription failed: {}", e))?;

        if result.full_text.trim().is_empty() {
            return Ok(None);
        }

        // Create event
        self.segment_counter += 1;
        let event = TranscriptionEvent {
            id: format!("seg-{}", self.segment_counter),
            text: result.full_text.trim().to_string(),
            start_ms: self.total_duration_ms,
            end_ms: self.total_duration_ms + duration_ms,
            is_partial: false,
        };

        // Update total duration
        self.total_duration_ms += duration_ms;

        // Emit to frontend
        self.emit_transcription(&event);

        Ok(Some(event))
    }

    /// Emit transcription event to frontend
    fn emit_transcription(&self, event: &TranscriptionEvent) {
        if let Err(e) = self.app.emit("transcription", event) {
            log::error!("Failed to emit transcription event: {}", e);
        }
    }

    /// Reset the worker state (for new recording session)
    pub fn reset(&mut self) {
        self.segment_counter = 0;
        self.total_duration_ms = 0;
    }
}

/// Calculate RMS (root mean square) of audio samples
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Background transcription loop
pub async fn run_transcription_loop(
    app: AppHandle,
    audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
    is_recording: Arc<Mutex<bool>>,
    config: TranscriptionConfig,
) {
    let chunk_samples = (config.chunk_duration_secs * 16000.0) as usize;
    let mut accumulated_samples: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    let mut segment_counter: u64 = 0;
    let mut total_duration_ms: i64 = 0;

    loop {
        // Check if still recording
        {
            let recording = is_recording.lock().await;
            if !*recording {
                break;
            }
        }

        // Get audio samples
        let (samples, sample_rate) = {
            let capture_guard = audio_capture.lock().await;
            if let Some(ref capture) = *capture_guard {
                (capture.get_samples(), capture.sample_rate())
            } else {
                (vec![], 16000)
            }
        };

        if !samples.is_empty() {
            // Resample if needed
            let samples_16k = if sample_rate != 16000 {
                match resample_to_16khz(&samples, sample_rate) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Resampling failed: {}", e);
                        continue;
                    }
                }
            } else {
                samples
            };

            accumulated_samples.extend(samples_16k);
        }

        // Process if we have enough samples
        if accumulated_samples.len() >= chunk_samples {
            // Check if audio is not silence
            let rms = calculate_rms(&accumulated_samples[..chunk_samples]);

            if rms >= config.silence_threshold {
                // Get the engine
                let engine_guard = engine.lock().await;
                if let Some(ref eng) = *engine_guard {
                    // Process chunk
                    let chunk: Vec<f32> = accumulated_samples.drain(..chunk_samples).collect();
                    let duration_ms = (chunk.len() as f32 / 16.0) as i64;

                    // Emit processing status
                    let _ = app.emit(
                        "transcription-status",
                        serde_json::json!({
                            "status": "processing",
                            "chunk_duration_ms": duration_ms
                        }),
                    );

                    match eng.transcribe(&chunk).await {
                        Ok(result) => {
                            if !result.full_text.trim().is_empty() {
                                segment_counter += 1;
                                let event = TranscriptionEvent {
                                    id: format!("seg-{}", segment_counter),
                                    text: result.full_text.trim().to_string(),
                                    start_ms: total_duration_ms,
                                    end_ms: total_duration_ms + duration_ms,
                                    is_partial: false,
                                };

                                total_duration_ms += duration_ms;

                                if let Err(e) = app.emit("transcription", &event) {
                                    log::error!("Failed to emit transcription: {}", e);
                                }

                                log::info!("Transcribed: {}", event.text);
                            }
                        }
                        Err(e) => {
                            log::error!("Transcription error: {}", e);
                        }
                    }

                    // Emit idle status after processing
                    let _ = app.emit(
                        "transcription-status",
                        serde_json::json!({
                            "status": "idle"
                        }),
                    );
                } else {
                    // No engine, just discard samples
                    accumulated_samples.drain(..chunk_samples);
                }
            } else {
                // Silence, discard samples but keep some overlap
                let keep = (config.overlap_secs * 16000.0) as usize;
                if accumulated_samples.len() > keep {
                    accumulated_samples.drain(..(accumulated_samples.len() - keep));
                }
                total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
            }
        }

        // Small delay to prevent busy loop
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    log::info!("Transcription loop ended");
}
