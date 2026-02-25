// Tauri IPC commands - bridge between frontend and Rust backend
#![allow(
    clippy::wildcard_in_or_patterns,
    clippy::needless_borrows_for_generic_args
)]

#[cfg(feature = "parakeet")]
use crate::asr::parakeet_backend::ParakeetModel;
#[cfg(feature = "parakeet")]
use crate::asr::AsrBackendType;
use crate::asr::{self, TranscriptionEngine, WhisperModel};
use crate::audio::AudioCapture;
#[cfg(target_os = "macos")]
use crate::audio::SystemAudioCapture;
use crate::detection::MeetingDetector;
use crate::embeddings::{self, EmbeddingModel};
use crate::llm::{LlmClient, LlmProvider};
use crate::models::{self, ModelInfo};
use crate::storage::{
    Database, MeetingListItem, SearchResult, SegmentRow, SemanticSearchResult, Speaker,
};
use crate::transcription::TranscriptionConfig;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptSegment {
    pub id: String,
    pub time: String,
    pub text: String,
    pub timestamp_ms: u64,
    /// "mic" = local user, "system" = remote participants via SCK
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub llm_provider: String,
    pub openai_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
    pub auto_detect_meetings: bool,
    #[serde(default = "default_true")]
    pub show_system_notifications: bool,
    #[serde(default)]
    pub onboarding_completed: bool,
    pub whisper_model: String,
    pub language: String,
    #[serde(default = "default_asr_backend")]
    pub asr_backend: String,
    #[serde(default)]
    pub audio_device: Option<String>,
    // AI Features
    #[serde(default)]
    pub enhance_transcripts: bool,
    #[serde(default)]
    pub detect_questions: bool,
}

fn default_true() -> bool {
    true
}

fn default_asr_backend() -> String {
    "whisper".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm_provider: "ollama".to_string(),
            openai_api_key: None,
            ollama_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3.2".to_string()),
            auto_detect_meetings: false,
            show_system_notifications: true,
            onboarding_completed: false,
            whisper_model: "small".to_string(),
            language: "en".to_string(),
            asr_backend: "whisper".to_string(),
            audio_device: None,
            // AI Features (default off)
            enhance_transcripts: true,
            detect_questions: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub whisper_downloaded: bool,
    pub whisper_model: String,
    pub whisper_size_mb: u64,
    pub models_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub overview: String,
    pub action_items: Vec<String>,
    pub key_points: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingWithTranscript {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub ended_at: Option<String>,
    pub pinned: bool,
    pub duration_ms: i64,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingStatus {
    pub model_loaded: bool,
    pub embedded_count: u64,
    pub total_segments: u64,
}

/// Event emitted when a meeting app is detected
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingDetectedEvent {
    pub app_name: String,
    pub message: String,
}

/// Event emitted when all meeting apps have closed
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeetingEndedEvent {
    pub message: String,
}

// ============================================================================
// App State
// ============================================================================

pub struct AppState {
    pub audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    pub transcription_engine: Arc<Mutex<Option<TranscriptionEngine>>>,
    pub transcript: Arc<Mutex<Vec<TranscriptSegment>>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub is_paused: Arc<Mutex<bool>>,
    pub settings: Arc<Mutex<Settings>>,
    pub db: Arc<Database>,
    pub active_meeting_id: Arc<Mutex<Option<String>>>,
    pub embedding_model: Arc<Mutex<Option<EmbeddingModel>>>,
    // Meeting detection
    pub meeting_detector: Arc<Mutex<MeetingDetector>>,
    pub detection_running: Arc<AtomicBool>,
    // Transcription queue depth (number of chunks waiting to be transcribed)
    pub pending_chunks: Arc<AtomicUsize>,
}

/// A captured and resampled audio chunk ready for transcription.
/// Passed from the audio producer to the transcription consumer via mpsc channel.
/// Which physical source the audio came from.
#[derive(Debug, Clone, PartialEq)]
enum AudioSource {
    /// Microphone — the local user's voice.
    Mic,
    /// System audio via ScreenCaptureKit — remote participants / any app audio.
    #[cfg(target_os = "macos")]
    System,
}

impl AudioSource {
    fn as_str(&self) -> &'static str {
        match self {
            AudioSource::Mic => "mic",
            #[cfg(target_os = "macos")]
            AudioSource::System => "system",
        }
    }
}

#[derive(Debug)]
struct AudioChunk {
    /// 16kHz mono f32 PCM samples, already resampled.
    samples: Vec<f32>,
    /// Absolute start position in the recording timeline (milliseconds).
    start_ms: i64,
    /// Duration of this chunk in milliseconds.
    duration_ms: i64,
    /// Monotonic index of this chunk (producer-assigned, used for segment IDs).
    chunk_index: u64,
    /// Which capture source produced this chunk.
    source: AudioSource,
}

// ============================================================================
// Recording Commands
// ============================================================================

/// Format milliseconds to MM:SS string
fn format_time(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

/// Format meeting title from current time
fn format_meeting_title() -> String {
    let now = Utc::now();
    now.format("%a %d/%m/%y \u{00b7} %l:%M %p")
        .to_string()
        .trim()
        .to_string()
}

/// Start audio recording and transcription
#[tauri::command]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let mut is_recording = state.is_recording.lock().await;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Check if model is loaded
    {
        let engine = state.transcription_engine.lock().await;
        if engine.is_none() {
            return Err(
                "Transcription model not loaded. Please download a model first.".to_string(),
            );
        }
    }

    // Initialize audio capture with selected device from settings
    let mut audio_capture =
        AudioCapture::new().map_err(|e| format!("Failed to initialize audio: {}", e))?;

    // Apply selected audio device if configured
    {
        let settings = state.settings.lock().await;
        if let Some(ref device_name) = settings.audio_device {
            audio_capture
                .select_device(Some(device_name.as_str()))
                .map_err(|e| format!("Failed to select audio device '{}': {}", device_name, e))?;
        }
    }

    audio_capture
        .start()
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    // Create meeting in DB
    let meeting_id = format!("meeting-{}", Utc::now().timestamp_millis());
    let title = format_meeting_title();
    let created_at = Utc::now().to_rfc3339();

    state
        .db
        .create_meeting(&meeting_id, &title, &created_at)
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    *state.active_meeting_id.lock().await = Some(meeting_id.clone());

    // Store in state
    *state.audio_capture.lock().await = Some(audio_capture);
    *state.transcript.lock().await = Vec::new();
    *state.is_paused.lock().await = false;
    *is_recording = true;

    // Start producer-consumer transcription pipeline
    let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<AudioChunk>(32);
    state.pending_chunks.store(0, Ordering::SeqCst);

    // Audio producer: captures audio and sends chunks independently of transcription speed
    let audio_capture_arc = state.audio_capture.clone();
    let is_recording_arc = state.is_recording.clone();
    let is_paused_arc = state.is_paused.clone();
    let app_producer = app.clone();
    let pending_prod = state.pending_chunks.clone();

    // Clone tx so the system audio producer can share the same consumer channel.
    #[cfg(target_os = "macos")]
    let system_chunk_tx = chunk_tx.clone();

    tauri::async_runtime::spawn(run_audio_producer(
        app_producer,
        audio_capture_arc,
        is_recording_arc.clone(),
        is_paused_arc.clone(),
        TranscriptionConfig::default(),
        chunk_tx,
        pending_prod.clone(),
    ));

    // System audio producer (macOS only): captures all app output via ScreenCaptureKit.
    // Shares the same consumer channel; chunks are tagged AudioSource::System.
    #[cfg(target_os = "macos")]
    {
        let app_sys = app.clone();
        let is_recording_sys = is_recording_arc.clone();
        let is_paused_sys = is_paused_arc.clone();
        let pending_sys = pending_prod.clone();
        tauri::async_runtime::spawn(run_system_audio_producer(
            app_sys,
            is_recording_sys,
            is_paused_sys,
            TranscriptionConfig::default(),
            system_chunk_tx,
            pending_sys,
        ));
    }

    // Transcription consumer: processes chunks from queue without blocking audio capture
    let engine_arc = state.transcription_engine.clone();
    let transcript_arc = state.transcript.clone();
    let db_arc = state.db.clone();
    let emb_model_arc = state.embedding_model.clone();
    let mid = meeting_id.clone();
    let meeting_title = title.clone();
    let settings_arc = state.settings.clone();
    let app_consumer = app.clone();
    let pending_cons = state.pending_chunks.clone();

    tauri::async_runtime::spawn(run_transcription_consumer(
        app_consumer,
        engine_arc,
        transcript_arc,
        db_arc,
        mid,
        emb_model_arc,
        meeting_title,
        settings_arc,
        chunk_rx,
        pending_cons,
    ));

    log::info!("Recording started with meeting {}", meeting_id);
    Ok(meeting_id)
}

/// Audio producer: captures audio, accumulates chunks, and sends them to the transcription channel.
/// Runs independently so audio is never dropped while transcription is busy.
async fn run_audio_producer(
    app: AppHandle,
    audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    is_recording: Arc<Mutex<bool>>,
    is_paused: Arc<Mutex<bool>>,
    config: TranscriptionConfig,
    chunk_tx: tokio::sync::mpsc::Sender<AudioChunk>,
    pending_chunks: Arc<AtomicUsize>,
) {
    use crate::asr::resample_to_16khz;

    let chunk_samples = (config.chunk_duration_secs * 16000.0) as usize;
    let mut accumulated_samples: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    let mut chunk_index: u64 = 0;
    let mut total_duration_ms: i64 = 0;

    log::info!(
        "Audio producer started, chunk size: {} samples",
        chunk_samples
    );

    loop {
        // Check if still recording
        {
            let recording = is_recording.lock().await;
            if !*recording {
                break;
            }
        }

        // Check if paused - drain buffer to prevent buildup
        {
            let paused = is_paused.lock().await;
            if *paused {
                let capture_guard = audio_capture.lock().await;
                if let Some(ref capture) = *capture_guard {
                    let _ = capture.get_samples();
                }
                drop(capture_guard);
                accumulated_samples.clear();
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                continue;
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
            let samples_16k = if sample_rate != 16000 {
                match resample_to_16khz(&samples, sample_rate) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Resampling failed: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }
                }
            } else {
                samples
            };
            accumulated_samples.extend(samples_16k);
        }

        // When we have a full chunk, check silence and send to consumer
        if accumulated_samples.len() >= chunk_samples {
            let rms: f32 = {
                let sum_squares: f32 = accumulated_samples[..chunk_samples]
                    .iter()
                    .map(|s| s * s)
                    .sum();
                (sum_squares / chunk_samples as f32).sqrt()
            };

            if rms >= config.silence_threshold {
                let chunk_data: Vec<f32> = accumulated_samples.drain(..chunk_samples).collect();
                let duration_ms = (chunk_data.len() as f32 / 16.0) as i64;
                chunk_index += 1;

                let chunk = AudioChunk {
                    samples: chunk_data,
                    start_ms: total_duration_ms,
                    duration_ms,
                    chunk_index,
                    source: AudioSource::Mic,
                };

                total_duration_ms += duration_ms;

                // Increment pending count and notify frontend
                let count = pending_chunks.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "transcription-status",
                    serde_json::json!({ "status": "queued", "pending_chunks": count }),
                );

                // Send to consumer - if channel is closed, stop
                if chunk_tx.send(chunk).await.is_err() {
                    log::info!("Chunk channel closed, stopping audio producer");
                    break;
                }
            } else {
                // Silence: discard chunk but keep overlap for continuity
                let keep = (config.overlap_secs * 16000.0) as usize;
                if accumulated_samples.len() > keep {
                    accumulated_samples.drain(..(accumulated_samples.len() - keep));
                }
                total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    log::info!("Audio producer stopped after {} chunks", chunk_index);
    // Dropping chunk_tx here closes the channel, signalling the consumer to drain and stop
}

/// System audio producer (macOS only): captures all application output via ScreenCaptureKit.
/// Shares the same mpsc channel as the mic producer; chunks are tagged AudioSource::System.
#[cfg(target_os = "macos")]
async fn run_system_audio_producer(
    app: AppHandle,
    is_recording: Arc<Mutex<bool>>,
    is_paused: Arc<Mutex<bool>>,
    config: TranscriptionConfig,
    chunk_tx: tokio::sync::mpsc::Sender<AudioChunk>,
    pending_chunks: Arc<AtomicUsize>,
) {
    let mut capture = SystemAudioCapture::new();

    if let Err(e) = capture.start() {
        log::warn!("System audio capture unavailable: {e}. Remote audio will not be transcribed.");
        return;
    }

    let chunk_samples = (config.chunk_duration_secs * 16000.0) as usize;
    let mut accumulated_samples: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    let mut chunk_index: u64 = 0;
    let mut total_duration_ms: i64 = 0;

    log::info!("System audio producer started");

    loop {
        {
            let recording = is_recording.lock().await;
            if !*recording {
                break;
            }
        }

        {
            let paused = is_paused.lock().await;
            if *paused {
                let _ = capture.get_samples(); // drain to avoid stale audio on resume
                accumulated_samples.clear();
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                continue;
            }
        }

        let samples = capture.get_samples();
        if !samples.is_empty() {
            accumulated_samples.extend(samples);
        }

        if accumulated_samples.len() >= chunk_samples {
            let rms: f32 = {
                let sum: f32 = accumulated_samples[..chunk_samples]
                    .iter()
                    .map(|s| s * s)
                    .sum();
                (sum / chunk_samples as f32).sqrt()
            };

            if rms >= config.silence_threshold {
                let chunk_data: Vec<f32> = accumulated_samples.drain(..chunk_samples).collect();
                let duration_ms = (chunk_data.len() as f32 / 16.0) as i64;
                chunk_index += 1;

                let chunk = AudioChunk {
                    samples: chunk_data,
                    start_ms: total_duration_ms,
                    duration_ms,
                    chunk_index,
                    source: AudioSource::System,
                };

                total_duration_ms += duration_ms;

                let count = pending_chunks.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "transcription-status",
                    serde_json::json!({ "status": "queued", "pending_chunks": count }),
                );

                if chunk_tx.send(chunk).await.is_err() {
                    log::info!("Chunk channel closed, stopping system audio producer");
                    break;
                }
            } else {
                let keep = (config.overlap_secs * 16000.0) as usize;
                if accumulated_samples.len() > keep {
                    accumulated_samples.drain(..(accumulated_samples.len() - keep));
                }
                total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    capture.stop();
    log::info!("System audio producer stopped after {} chunks", chunk_index);
}

/// Transcription consumer: receives audio chunks from the channel and runs Whisper inference.
/// Processes all queued chunks even after recording stops (drain-not-discard).
#[allow(clippy::too_many_arguments)]
async fn run_transcription_consumer(
    app: AppHandle,
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
    transcript: Arc<Mutex<Vec<TranscriptSegment>>>,
    db: Arc<Database>,
    meeting_id: String,
    embedding_model: Arc<Mutex<Option<EmbeddingModel>>>,
    meeting_title: String,
    settings: Arc<Mutex<Settings>>,
    mut chunk_rx: tokio::sync::mpsc::Receiver<AudioChunk>,
    pending_chunks: Arc<AtomicUsize>,
) {
    use crate::transcription::TranscriptionEvent;

    let mut segment_counter: u64 = 0;

    log::info!("Transcription consumer started");

    while let Some(chunk) = chunk_rx.recv().await {
        // Atomically decrement so the producer can never observe a torn value
        // between load and store. fetch_sub returns the old value, so subtract 1.
        let new_count = pending_chunks
            .fetch_sub(1, Ordering::SeqCst)
            .saturating_sub(1);

        let _ = app.emit(
            "transcription-status",
            serde_json::json!({ "status": "processing", "pending_chunks": new_count }),
        );

        // Run Whisper inference
        let transcription_result = {
            let engine_guard = engine.lock().await;
            if let Some(ref eng) = *engine_guard {
                eng.transcribe(&chunk.samples).await
            } else {
                log::warn!(
                    "No transcription engine, dropping chunk {}",
                    chunk.chunk_index
                );
                let _ = app.emit(
                    "transcription-status",
                    serde_json::json!({ "status": "idle", "pending_chunks": new_count }),
                );
                continue;
            }
        };

        let _ = app.emit(
            "transcription-status",
            serde_json::json!({ "status": "idle", "pending_chunks": new_count }),
        );

        match transcription_result {
            Ok(result) => {
                let text = result.full_text.trim().to_string();
                if !text.is_empty() {
                    segment_counter += 1;

                    let seg_id = format!("{}-seg-{}", meeting_id, segment_counter);
                    let time_label = format_time(chunk.start_ms as u64);

                    let source_str = chunk.source.as_str().to_string();

                    let event = TranscriptionEvent {
                        id: seg_id.clone(),
                        text: text.clone(),
                        start_ms: chunk.start_ms,
                        end_ms: chunk.start_ms + chunk.duration_ms,
                        is_partial: false,
                        source: source_str.clone(),
                    };

                    // Store in in-memory transcript
                    {
                        let mut transcript_guard = transcript.lock().await;
                        transcript_guard.push(TranscriptSegment {
                            id: event.id.clone(),
                            time: time_label.clone(),
                            text: event.text.clone(),
                            timestamp_ms: chunk.start_ms as u64,
                            source: source_str.clone(),
                        });
                    }

                    // Persist segment to DB
                    let seg_id_for_emb = seg_id.clone();
                    let time_label_for_emb = time_label.clone();
                    let text_for_emb = text.clone();
                    if let Err(e) = db.insert_segment(&SegmentRow {
                        id: seg_id,
                        meeting_id: meeting_id.clone(),
                        time_label,
                        text: text.clone(),
                        timestamp_ms: chunk.start_ms,
                        speaker_id: None,
                        source: Some(source_str.clone()),
                        enhanced_text: None,
                        is_question: false,
                        question_answer: None,
                    }) {
                        log::error!("Failed to persist segment: {}", e);
                    }

                    // Generate embedding in background (non-blocking)
                    {
                        let emb_model = embedding_model.clone();
                        let emb_db = db.clone();
                        let emb_title = meeting_title.clone();
                        tokio::spawn(async move {
                            let model_guard = emb_model.lock().await;
                            if let Some(ref m) = *model_guard {
                                let enriched = embeddings::enrich_segment(
                                    &emb_title,
                                    &time_label_for_emb,
                                    &text_for_emb,
                                );
                                match m.embed(&enriched) {
                                    Ok(emb) => {
                                        if let Err(e) =
                                            emb_db.insert_embedding(&seg_id_for_emb, &emb)
                                        {
                                            log::error!("Failed to store embedding: {}", e);
                                        }
                                    }
                                    Err(e) => log::error!("Embedding failed: {}", e),
                                }
                            }
                        });
                    }

                    // AI processing: auto-title, transcript enhancement, question detection
                    let current_seg_counter = segment_counter;
                    let settings_for_ai = settings.clone();
                    let db_for_ai = db.clone();
                    let mid_for_ai = meeting_id.clone();
                    let app_for_ai = app.clone();
                    let transcript_for_ai = transcript.clone();
                    tokio::spawn(async move {
                        let settings = settings_for_ai.lock().await;
                        let enhance_transcripts = settings.enhance_transcripts;
                        let detect_questions = settings.detect_questions;
                        log::info!(
                            "AI Processing check - enhance: {}, detect_questions: {}",
                            enhance_transcripts,
                            detect_questions
                        );
                        drop(settings);

                        let llm_provider = {
                            let settings = settings_for_ai.lock().await;
                            match settings.llm_provider.as_str() {
                                "openai" => settings
                                    .openai_api_key
                                    .clone()
                                    .map(|key| LlmProvider::OpenAI { api_key: key }),
                                _ => {
                                    let url = settings
                                        .ollama_url
                                        .clone()
                                        .unwrap_or_else(|| "http://localhost:11434".to_string());
                                    let model = settings
                                        .ollama_model
                                        .clone()
                                        .unwrap_or_else(|| "llama3.2".to_string());
                                    Some(LlmProvider::Ollama { url, model })
                                }
                            }
                        };

                        if let Some(provider) = llm_provider {
                            let client = LlmClient::new(provider);

                            // Auto-title exactly once at segment 10
                            if current_seg_counter == 10 {
                                let transcript = transcript_for_ai.lock().await;
                                let title_transcript: String = transcript
                                    .iter()
                                    .take(10)
                                    .map(|s| s.text.clone())
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                drop(transcript);

                                if let Ok(title) = client.generate_title(&title_transcript).await {
                                    let title = title
                                        .trim()
                                        .trim_matches('"')
                                        .trim_matches('\'')
                                        .to_string();
                                    if !title.is_empty() {
                                        if let Err(e) =
                                            db_for_ai.update_meeting_title(&mid_for_ai, &title)
                                        {
                                            log::error!(
                                                "Failed to auto-update meeting title: {}",
                                                e
                                            );
                                        } else {
                                            log::info!("Auto-title saved: {}", title);
                                            let _ = app_for_ai.emit(
                                                "meeting-title-updated",
                                                serde_json::json!({
                                                    "meeting_id": mid_for_ai,
                                                    "title": title
                                                }),
                                            );
                                        }
                                    }
                                }
                            }

                            // Transcript enhancement - batch 5 segments together for better context
                            if enhance_transcripts && current_seg_counter.is_multiple_of(5) {
                                let transcript = transcript_for_ai.lock().await;
                                let start_idx = (current_seg_counter as usize).saturating_sub(5);
                                let segments: Vec<String> = transcript
                                    [start_idx..current_seg_counter as usize]
                                    .iter()
                                    .map(|s| s.text.clone())
                                    .collect();
                                let segment_ids: Vec<String> = (start_idx
                                    ..current_seg_counter as usize)
                                    .map(|i| format!("{}-seg-{}", mid_for_ai, i + 1))
                                    .collect();
                                drop(transcript);

                                if segments.len() >= 3 {
                                    if let Ok(enhanced_segments) =
                                        client.enhance_batch(&segments).await
                                    {
                                        let combined_text = enhanced_segments.join("\n\n");
                                        for seg_id in &segment_ids {
                                            let _ = db_for_ai.update_segment_enhanced_text(
                                                seg_id,
                                                Some(&combined_text),
                                            );
                                        }
                                        let _ = app_for_ai.emit(
                                            "segment-enhanced",
                                            serde_json::json!({
                                                "segment_ids": segment_ids,
                                                "enhanced_text": combined_text
                                            }),
                                        );
                                        log::info!(
                                            "Emitted semantic reconstruction for segments {}-{}",
                                            segment_ids.first().unwrap_or(&String::new()),
                                            segment_ids.last().unwrap_or(&String::new())
                                        );
                                    }
                                }
                            }

                            // Question detection - check every segment
                            if detect_questions {
                                let transcript = transcript_for_ai.lock().await;
                                let idx = (current_seg_counter as usize).saturating_sub(1);
                                let curr_text = transcript
                                    .get(idx)
                                    .map(|s| s.text.clone())
                                    .unwrap_or_default();
                                drop(transcript);

                                log::info!(
                                    "Checking for question in segment {}",
                                    current_seg_counter
                                );

                                if let Ok(is_question) = client.detect_question(&curr_text).await {
                                    log::info!("Question detection result: {}", is_question);
                                    if is_question {
                                        let transcript = transcript_for_ai.lock().await;
                                        let ctx_start = idx.saturating_sub(5);
                                        let ctx_end = (idx + 6).min(transcript.len());
                                        let context: String = transcript[ctx_start..ctx_end]
                                            .iter()
                                            .map(|s| s.text.clone())
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        drop(transcript);

                                        if let Ok(answer) =
                                            client.answer_question(&context, &curr_text).await
                                        {
                                            let seg_id = format!(
                                                "{}-seg-{}",
                                                mid_for_ai, current_seg_counter
                                            );
                                            let _ = db_for_ai.update_segment_question(
                                                &seg_id,
                                                true,
                                                Some(&answer),
                                            );
                                            let _ = app_for_ai.emit(
                                                "question-detected",
                                                serde_json::json!({
                                                    "segment_id": seg_id,
                                                    "question": curr_text,
                                                    "answer": answer
                                                }),
                                            );
                                            log::info!(
                                                "Question detected and emitted: {}",
                                                curr_text
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    });

                    // Emit transcription segment to frontend
                    if let Err(e) = app.emit("transcription", &event) {
                        log::error!("Failed to emit transcription: {}", e);
                    }
                    log::info!("[{}] {}", format_time(chunk.start_ms as u64), text);
                }
            }
            Err(e) => {
                log::error!("Transcription error for chunk {}: {}", chunk.chunk_index, e);
            }
        }
    }

    log::info!(
        "Transcription consumer finished, processed {} segments",
        segment_counter
    );
}

/// Stop recording and finalize transcript
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<Vec<TranscriptSegment>, String> {
    let mut is_recording = state.is_recording.lock().await;
    if !*is_recording {
        return Err("Not recording".to_string());
    }

    // Signal to stop (this will stop the transcription loop)
    *is_recording = false;

    // Give the transcription loop a moment to finish
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Stop audio capture
    let mut capture_guard = state.audio_capture.lock().await;
    if let Some(ref mut capture) = *capture_guard {
        capture
            .stop()
            .map_err(|e| format!("Failed to stop: {}", e))?;
    }
    *capture_guard = None;

    let transcript = state.transcript.lock().await.clone();

    // Update meeting ended_at and duration
    let meeting_id = state.active_meeting_id.lock().await.clone();
    if let Some(mid) = &meeting_id {
        let ended_at = Utc::now().to_rfc3339();
        let duration_ms = transcript
            .last()
            .map(|s| s.timestamp_ms as i64)
            .unwrap_or(0);
        if let Err(e) = state.db.update_meeting_ended(mid, &ended_at, duration_ms) {
            log::error!("Failed to update meeting ended: {}", e);
        }
    }

    log::info!("Recording stopped, {} segments", transcript.len());

    // Auto-generate summary in background (non-blocking)
    if !transcript.is_empty() {
        if let Some(mid) = &meeting_id {
            let db_clone = state.db.clone();
            let settings_clone = state.settings.clone();
            let mid_clone = mid.clone();
            let transcript_text: String = transcript
                .iter()
                .map(|s| format!("[{}] {}", s.time, s.text))
                .collect::<Vec<_>>()
                .join("\n");

            tauri::async_runtime::spawn(async move {
                let settings = settings_clone.lock().await;
                let provider = match settings.llm_provider.as_str() {
                    "openai" => match settings.openai_api_key.clone() {
                        Some(key) if !key.is_empty() => LlmProvider::OpenAI { api_key: key },
                        _ => return,
                    },
                    _ => {
                        let url = settings
                            .ollama_url
                            .clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        let model = settings
                            .ollama_model
                            .clone()
                            .unwrap_or_else(|| "llama3.2".to_string());
                        LlmProvider::Ollama { url, model }
                    }
                };
                drop(settings);

                let client = LlmClient::new(provider);

                // Auto-generate title first
                let title_transcript: String = transcript_text
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join(" ");
                match client.generate_title(&title_transcript).await {
                    Ok(title) => {
                        let title = title
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        if !title.is_empty() {
                            if let Err(e) = db_clone.update_meeting_title(&mid_clone, &title) {
                                log::error!("Failed to auto-update meeting title: {}", e);
                            } else {
                                log::info!("Auto-title saved for meeting {}: {}", mid_clone, title);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Auto-title generation failed (non-critical): {}", e);
                    }
                }

                // Then generate summary
                match client.summarize(&transcript_text).await {
                    Ok(summary_text) => {
                        if let Err(e) = db_clone.save_meeting_summary(&mid_clone, &summary_text) {
                            log::error!("Failed to save auto-summary: {}", e);
                        } else {
                            log::info!("Auto-summary saved for meeting {}", mid_clone);
                        }
                    }
                    Err(e) => {
                        log::warn!("Auto-summary generation failed (non-critical): {}", e);
                    }
                }
            });
        }
    }

    Ok(transcript)
}

/// Pause recording (stops transcription but keeps session active)
#[tauri::command]
pub async fn pause_recording(state: State<'_, AppState>) -> Result<(), String> {
    let is_recording = state.is_recording.lock().await;
    if !*is_recording {
        return Err("Not recording".to_string());
    }
    *state.is_paused.lock().await = true;
    log::info!("Recording paused");
    Ok(())
}

/// Resume recording after pause
#[tauri::command]
pub async fn resume_recording(state: State<'_, AppState>) -> Result<(), String> {
    let is_recording = state.is_recording.lock().await;
    if !*is_recording {
        return Err("Not recording".to_string());
    }
    *state.is_paused.lock().await = false;
    log::info!("Recording resumed");
    Ok(())
}

/// Get current transcript segments
#[tauri::command]
pub async fn get_transcript(state: State<'_, AppState>) -> Result<Vec<TranscriptSegment>, String> {
    let transcript = state.transcript.lock().await.clone();
    Ok(transcript)
}

// ============================================================================
// Meeting Commands
// ============================================================================

#[tauri::command]
pub async fn list_meetings(state: State<'_, AppState>) -> Result<Vec<MeetingListItem>, String> {
    state
        .db
        .list_meetings()
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn get_meeting(
    id: String,
    state: State<'_, AppState>,
) -> Result<MeetingWithTranscript, String> {
    let meeting = state
        .db
        .get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    let segments = state
        .db
        .get_segments(&id)
        .map_err(|e| format!("DB error: {}", e))?;

    let transcript_segments: Vec<TranscriptSegment> = segments
        .into_iter()
        .map(|s| TranscriptSegment {
            id: s.id,
            time: s.time_label,
            text: s.text,
            timestamp_ms: s.timestamp_ms as u64,
            source: s.source.unwrap_or_else(|| "mic".to_string()),
        })
        .collect();

    Ok(MeetingWithTranscript {
        id: meeting.id,
        title: meeting.title,
        created_at: meeting.created_at,
        ended_at: meeting.ended_at,
        pinned: meeting.pinned,
        duration_ms: meeting.duration_ms,
        segments: transcript_segments,
    })
}

#[tauri::command]
pub async fn rename_meeting(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_meeting_title(&id, &title)
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn update_meeting_tags(
    id: String,
    tags: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_meeting_tags(&id, tags.as_deref())
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn toggle_pin_meeting(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let meeting = state
        .db
        .get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;
    state
        .db
        .set_meeting_pinned(&id, !meeting.pinned)
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn delete_meeting(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .delete_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn search_meetings(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    state
        .db
        .search_transcripts(&query, 50)
        .map_err(|e| format!("Search error: {}", e))
}

#[tauri::command]
pub async fn export_meeting(
    id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let meeting = state
        .db
        .get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    let segments = state
        .db
        .get_segments(&id)
        .map_err(|e| format!("DB error: {}", e))?;

    match format.as_str() {
        "markdown" => {
            let mut md = format!("# {}\n\n", meeting.title);
            md.push_str(&format!("**Date:** {}\n\n", meeting.created_at));
            md.push_str("## Transcript\n\n");
            for seg in &segments {
                md.push_str(&format!("**[{}]** {}\n\n", seg.time_label, seg.text));
            }
            Ok(md)
        }
        "srt" => {
            // SRT subtitle format
            let mut srt = String::new();
            for (i, seg) in segments.iter().enumerate() {
                // Convert timestamp_ms to SRT format (HH:MM:SS,mmm)
                let start_time = format_srt_timestamp(seg.timestamp_ms);
                let end_time = format_srt_timestamp(seg.timestamp_ms + 5000); // Assume 5s duration
                srt.push_str(&format!("{}\n", i + 1));
                srt.push_str(&format!("{} --> {}\n", start_time, end_time));
                srt.push_str(&format!("{}\n\n", seg.text));
            }
            Ok(srt)
        }
        _ => {
            // Default: plain text
            let mut txt = format!("{}\n{}\n\n", meeting.title, meeting.created_at);
            for seg in &segments {
                txt.push_str(&format!("[{}] {}\n", seg.time_label, seg.text));
            }
            Ok(txt)
        }
    }
}

/// Format milliseconds to SRT timestamp format (HH:MM:SS,mmm)
fn format_srt_timestamp(ms: i64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    let millis = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

/// Export meeting to a file with Save As dialog
#[tauri::command]
pub async fn export_meeting_to_file(
    app: AppHandle,
    id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    // First get the content (clone values to avoid borrow issues)
    let content = export_meeting(id.clone(), format.clone(), State::clone(&state)).await?;

    // Determine file extension and filter
    let (extension, filter_name) = match format.as_str() {
        "markdown" => ("md", "Markdown"),
        "srt" => ("srt", "Subtitles"),
        _ => ("txt", "Text"),
    };

    // Get meeting title for default filename
    let meeting = state
        .db
        .get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    // Sanitize filename
    let safe_title = meeting
        .title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect::<String>()
        .trim()
        .replace(' ', "_");
    let default_name = if safe_title.is_empty() {
        format!("meeting_{}", chrono::Utc::now().format("%Y%m%d"))
    } else {
        safe_title
    };

    // Use Tauri dialog to show Save As
    use tauri_plugin_dialog::DialogExt;
    let file_path = app
        .dialog()
        .file()
        .set_file_name(format!("{}.{}", default_name, extension))
        .add_filter(filter_name, &[extension])
        .blocking_save_file();

    if let Some(path) = file_path {
        // Write the file
        std::fs::write(path.as_path().unwrap(), content)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        log::info!("Exported meeting to: {:?}", path);
        Ok(true)
    } else {
        // User cancelled
        Ok(false)
    }
}

// ============================================================================
// Segment Editing Commands
// ============================================================================

/// Update a segment's text and/or speaker
#[tauri::command]
pub async fn update_segment(
    segment_id: String,
    text: Option<String>,
    speaker_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(ref new_text) = text {
        state
            .db
            .update_segment_text(&segment_id, new_text)
            .map_err(|e| format!("Failed to update segment text: {}", e))?;
    }
    if speaker_id.is_some() || text.is_some() {
        // Only update speaker if explicitly passed (even if None to clear it)
        if let Some(ref speaker) = speaker_id {
            state
                .db
                .update_segment_speaker(&segment_id, Some(speaker.as_str()))
                .map_err(|e| format!("Failed to update segment speaker: {}", e))?;
        }
    }
    Ok(())
}

/// Delete a segment
#[tauri::command]
pub async fn delete_segment(segment_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .delete_segment(&segment_id)
        .map_err(|e| format!("Failed to delete segment: {}", e))
}

// ============================================================================
// Speaker Commands
// ============================================================================

/// List all speakers
#[tauri::command]
pub async fn list_speakers(state: State<'_, AppState>) -> Result<Vec<Speaker>, String> {
    state
        .db
        .list_speakers()
        .map_err(|e| format!("Failed to list speakers: {}", e))
}

/// Create a new speaker
#[tauri::command]
pub async fn create_speaker(
    name: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = format!("speaker-{}", Utc::now().timestamp_millis());
    let created_at = Utc::now().to_rfc3339();
    state
        .db
        .create_speaker(&id, &name, &color, &created_at)
        .map_err(|e| format!("Failed to create speaker: {}", e))?;
    Ok(id)
}

/// Update a speaker
#[tauri::command]
pub async fn update_speaker(
    id: String,
    name: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .update_speaker(&id, &name, &color)
        .map_err(|e| format!("Failed to update speaker: {}", e))
}

/// Delete a speaker
#[tauri::command]
pub async fn delete_speaker(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .delete_speaker(&id)
        .map_err(|e| format!("Failed to delete speaker: {}", e))
}

// ============================================================================
// Q&A Commands
// ============================================================================

/// Ask a question about the current meeting using RAG
#[tauri::command]
pub async fn ask_question(
    question: String,
    meeting_id: Option<String>,
    context_limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let limit = context_limit.unwrap_or(10);

    // Resolve the meeting scope: explicit meeting_id, or active meeting for live recording
    let effective_meeting_id: Option<String> = if meeting_id.is_some() {
        meeting_id.clone()
    } else {
        state.active_meeting_id.lock().await.clone()
    };

    // Try semantic search first if embedding model is loaded
    let semantic_context: Option<String> = {
        let query_emb = {
            let model_guard = state.embedding_model.lock().await;
            match model_guard.as_ref() {
                Some(model) => model.embed(&question).ok(),
                None => None,
            }
        };
        if let Some(emb) = query_emb {
            match state
                .db
                .search_semantic(&emb, limit, effective_meeting_id.as_deref())
            {
                Ok(results) if !results.is_empty() => Some(
                    results
                        .iter()
                        .map(|r| format!("[{}] {}", r.time_label, r.text))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                _ => None,
            }
        } else {
            None
        }
    };

    // Use semantic context if available, otherwise fall back to full transcript
    let context: String = if let Some(ctx) = semantic_context {
        ctx
    } else if let Some(ref mid) = meeting_id {
        let segments = state
            .db
            .get_segments(mid)
            .map_err(|e| format!("DB error: {}", e))?;
        if segments.is_empty() {
            return Err("No transcript available for this meeting".to_string());
        }
        segments
            .iter()
            .map(|s| format!("[{}] {}", s.time_label, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let transcript = state.transcript.lock().await;
        if transcript.is_empty() {
            return Err("No transcript available".to_string());
        }
        transcript
            .iter()
            .map(|s| format!("[{}] {}", s.time, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings
                .openai_api_key
                .clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err(
                    "OpenAI API key is empty. Please add your API key in Settings.".to_string(),
                );
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings
                .ollama_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings
                .ollama_model
                .clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings); // Release lock before async call

    let client = LlmClient::new(provider);
    log::info!("Asking question: {}", question);

    client
        .answer_question(&context, &question)
        .await
        .map_err(|e| format!("LLM error: {}", e))
}

/// Check if user notes are mentioned in the transcript
/// Returns which notes were mentioned with briefings
#[tauri::command]
pub async fn check_notes_in_transcript(
    notes: Vec<NoteInput>,
    transcript_context: String,
    state: State<'_, AppState>,
) -> Result<Vec<NoteCheckResult>, String> {
    if notes.is_empty() || transcript_context.is_empty() {
        return Ok(vec![]);
    }

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings
                .openai_api_key
                .clone()
                .ok_or("OpenAI API key not configured")?;
            if api_key.is_empty() {
                return Err("OpenAI API key is empty".to_string());
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings
                .ollama_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings
                .ollama_model
                .clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings);

    let client = LlmClient::new(provider);

    // Extract just the text from notes
    let note_texts: Vec<String> = notes.iter().map(|n| n.text.clone()).collect();

    // Call LLM to check notes
    let matches = client
        .check_notes(&note_texts, &transcript_context)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    // Convert matches to results with note IDs
    let results: Vec<NoteCheckResult> = matches
        .into_iter()
        .filter_map(|m| {
            notes.get(m.index).map(|note| NoteCheckResult {
                note_id: note.id.clone(),
                note_text: note.text.clone(),
                mentioned: m.mentioned,
                briefing: m.briefing,
            })
        })
        .collect();

    Ok(results)
}

/// Input structure for note checking
#[derive(Debug, Deserialize)]
pub struct NoteInput {
    pub id: String,
    pub text: String,
}

/// Result of checking a note against transcript
#[derive(Debug, Serialize)]
pub struct NoteCheckResult {
    pub note_id: String,
    pub note_text: String,
    pub mentioned: bool,
    pub briefing: Option<String>,
}

/// Phomy: intelligent assistant that routes queries appropriately
/// Handles recency, time-based, meeting recall, and global summary queries
#[tauri::command]
pub async fn phomy_ask(question: String, state: State<'_, AppState>) -> Result<String, String> {
    // Helper to get display text (prefer enhanced text when available)
    let _get_display_text =
        |s: &SegmentRow| -> String { s.enhanced_text.as_ref().unwrap_or(&s.text).clone() };

    let q = question.to_lowercase();

    // Build LLM client from settings
    let provider = {
        let settings = state.settings.lock().await;
        match settings.llm_provider.as_str() {
            "openai" => {
                let api_key = settings
                    .openai_api_key
                    .clone()
                    .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
                if api_key.is_empty() {
                    return Err(
                        "OpenAI API key is empty. Please add your API key in Settings.".to_string(),
                    );
                }
                LlmProvider::OpenAI { api_key }
            }
            _ => {
                let url = settings
                    .ollama_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = settings
                    .ollama_model
                    .clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                LlmProvider::Ollama { url, model }
            }
        }
    };
    let client = LlmClient::new(provider);

    // ---- Route 1: "What did they just say?" / recency queries ----
    let is_recency = q.contains("just said")
        || q.contains("just say")
        || q.contains("they just")
        || q.contains("last thing")
        || q.contains("just now")
        || q.contains("right now")
        || (q.contains("what") && q.contains("just"));

    if is_recency {
        // Use live transcript (last ~10 chunks) or active meeting
        let context = {
            let is_recording = *state.is_recording.lock().await;
            let active_mid = state.active_meeting_id.lock().await.clone();

            if is_recording {
                let transcript = state.transcript.lock().await;
                let recent: Vec<_> = transcript
                    .iter()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                recent
                    .iter()
                    .map(|s| format!("[{}] {}", s.time, s.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else if let Some(mid) = active_mid {
                let segs = state
                    .db
                    .get_last_segments(&mid, 10)
                    .map_err(|e| format!("DB error: {}", e))?;
                // Use enhanced text when available
                segs.iter()
                    .map(|s| {
                        let text = s.enhanced_text.as_ref().unwrap_or(&s.text);
                        format!("[{}] {}", s.time_label, text)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                return Err("No active meeting to reference.".to_string());
            }
        };

        if context.is_empty() {
            return Err("No recent transcript available.".to_string());
        }

        let system = "You are Phomy, a calm meeting assistant. Summarize what was just said based on the most recent transcript chunks. Be brief and direct.";
        let user = format!("Recent transcript:\n{}\n\nQuestion: {}", context, question);
        return client
            .complete(system, &user)
            .await
            .map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Route 2: Time-based queries ("last 5 minutes", "past 10 minutes") ----
    let time_minutes = extract_time_minutes(&q);
    if let Some(mins) = time_minutes {
        let context = {
            let is_recording = *state.is_recording.lock().await;
            let active_mid = state.active_meeting_id.lock().await.clone();

            if is_recording {
                let transcript = state.transcript.lock().await;
                if let Some(latest) = transcript.last() {
                    let cutoff = (latest.timestamp_ms as i64) - (mins * 60 * 1000);
                    let filtered: Vec<_> = transcript
                        .iter()
                        .filter(|s| s.timestamp_ms as i64 >= cutoff)
                        .collect();
                    filtered
                        .iter()
                        .map(|s| format!("[{}] {}", s.time, s.text))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    String::new()
                }
            } else if let Some(mid) = active_mid {
                let segs = state
                    .db
                    .get_segments(&mid)
                    .map_err(|e| format!("DB error: {}", e))?;
                if let Some(latest) = segs.last() {
                    let cutoff = latest.timestamp_ms - (mins * 60 * 1000);
                    let filtered: Vec<_> =
                        segs.iter().filter(|s| s.timestamp_ms >= cutoff).collect();
                    filtered
                        .iter()
                        .map(|s| format!("[{}] {}", s.time_label, s.text))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    String::new()
                }
            } else {
                return Err("No active meeting to reference.".to_string());
            }
        };

        if context.is_empty() {
            return Err("No transcript in that time range.".to_string());
        }

        let system = "You are Phomy, a calm meeting assistant. Summarize the transcript from the requested time window. Be concise.";
        let user = format!(
            "Transcript from the last {} minutes:\n{}\n\nQuestion: {}",
            mins, context, question
        );
        return client
            .complete(system, &user)
            .await
            .map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Route 3: Meeting recall ("last meeting", "previous meeting") ----
    let is_recall = q.contains("last meeting")
        || q.contains("previous meeting")
        || q.contains("most recent meeting");

    if is_recall {
        let meetings = state
            .db
            .get_recent_meetings_with_summaries(1)
            .map_err(|e| format!("DB error: {}", e))?;

        if meetings.is_empty() {
            return Err("No completed meetings found.".to_string());
        }

        let (mid, title, created_at, summary) = &meetings[0];
        let context = if let Some(s) = summary {
            format!("Meeting: {} ({})\nSummary:\n{}", title, created_at, s)
        } else {
            // Fall back to transcript chunks
            let segs = state
                .db
                .get_segments(mid)
                .map_err(|e| format!("DB error: {}", e))?;
            let transcript: String = segs
                .iter()
                .map(|s| format!("[{}] {}", s.time_label, s.text))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Meeting: {} ({})\nTranscript:\n{}",
                title, created_at, transcript
            )
        };

        let system = "You are Phomy, a calm meeting assistant. Answer the question using the meeting context provided. Be helpful and concise.";
        let user = format!("{}\n\nQuestion: {}", context, question);
        return client
            .complete(system, &user)
            .await
            .map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Route 4: Global/weekly summaries ("this week", "all meetings", "overall") ----
    let is_global = q.contains("this week")
        || q.contains("all meetings")
        || q.contains("overall")
        || q.contains("week cover")
        || q.contains("meetings about")
        || q.contains("my meetings");

    if is_global {
        let meetings = state
            .db
            .get_recent_meetings_with_summaries(10)
            .map_err(|e| format!("DB error: {}", e))?;

        if meetings.is_empty() {
            return Err("No completed meetings found.".to_string());
        }

        let context: String = meetings
            .iter()
            .map(|(_, title, created_at, summary)| {
                if let Some(s) = summary {
                    format!("--- {} ({}) ---\n{}\n", title, created_at, s)
                } else {
                    format!(
                        "--- {} ({}) ---\n(no summary available)\n",
                        title, created_at
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are Phomy, a calm meeting assistant. Provide a high-level overview across the meetings described. Be concise and organized.";
        let user = format!("Meeting summaries:\n{}\n\nQuestion: {}", context, question);
        return client
            .complete(system, &user)
            .await
            .map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Default: semantic search across all meetings (existing behavior) ----
    let limit = 10;
    let semantic_context: Option<String> = {
        let query_emb = {
            let model_guard = state.embedding_model.lock().await;
            match model_guard.as_ref() {
                Some(model) => model.embed(&question).ok(),
                None => None,
            }
        };
        if let Some(emb) = query_emb {
            match state.db.search_semantic(&emb, limit, None) {
                Ok(results) if !results.is_empty() => Some(
                    results
                        .iter()
                        .map(|r| format!("[{} - {}] {}", r.meeting_title, r.time_label, r.text))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                _ => None,
            }
        } else {
            None
        }
    };

    let context = match semantic_context {
        Some(ctx) => ctx,
        None => {
            return Err(
                "No relevant context found. Try asking about a specific meeting.".to_string(),
            )
        }
    };

    let system = "You are Phomy, a calm meeting assistant. Answer the question using the provided meeting context. Be helpful and concise. If the answer isn't in the context, say so.";
    let user = format!("Context:\n{}\n\nQuestion: {}", context, question);
    client
        .complete(system, &user)
        .await
        .map_err(|e| format!("LLM error: {}", e))
}

/// Extract time in minutes from natural language (e.g., "last 5 minutes" → 5)
fn extract_time_minutes(q: &str) -> Option<i64> {
    use std::str::FromStr;
    // Patterns: "last N minutes", "past N minutes", "last N mins"
    let patterns = ["last ", "past "];
    for pat in &patterns {
        if let Some(idx) = q.find(pat) {
            let after = &q[idx + pat.len()..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = i64::from_str(&num_str) {
                if after.contains("minute") || after.contains("min") {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Generate meeting summary using LLM
#[tauri::command]
pub async fn generate_summary(
    meeting_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Summary, String> {
    // Build transcript text: from DB if meeting_id provided, otherwise from live transcript
    let transcript_text: String = if let Some(ref mid) = meeting_id {
        let segments = state
            .db
            .get_segments(mid)
            .map_err(|e| format!("DB error: {}", e))?;
        if segments.is_empty() {
            return Err("No transcript available for this meeting".to_string());
        }
        segments
            .iter()
            .map(|s| format!("[{}] {}", s.time_label, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let transcript = state.transcript.lock().await;
        if transcript.is_empty() {
            return Err("No transcript available".to_string());
        }
        transcript
            .iter()
            .map(|s| format!("[{}] {}", s.time, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings
                .openai_api_key
                .clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err(
                    "OpenAI API key is empty. Please add your API key in Settings.".to_string(),
                );
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings
                .ollama_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings
                .ollama_model
                .clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings); // Release lock before async call

    let client = LlmClient::new(provider);
    log::info!("Generating meeting summary");

    let summary_text = client
        .summarize(&transcript_text)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    // Parse the summary into structured format
    let lines: Vec<&str> = summary_text.lines().collect();
    let mut overview = String::new();
    let mut action_items = Vec::new();
    let mut key_points = Vec::new();
    let mut current_section = "overview";

    for line in lines {
        let line_lower = line.to_lowercase();
        if line_lower.contains("action item")
            || line_lower.contains("todo")
            || line_lower.contains("next step")
        {
            current_section = "actions";
            continue;
        } else if line_lower.contains("key point")
            || line_lower.contains("highlight")
            || line_lower.contains("important")
        {
            current_section = "points";
            continue;
        }

        let trimmed = line
            .trim()
            .trim_start_matches(
                &[
                    '-', '*', '\u{2022}', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.',
                    ')',
                ][..],
            )
            .trim();
        if trimmed.is_empty() {
            continue;
        }

        match current_section {
            "actions" => action_items.push(trimmed.to_string()),
            "points" => key_points.push(trimmed.to_string()),
            _ => {
                if !overview.is_empty() {
                    overview.push(' ');
                }
                overview.push_str(trimmed);
            }
        }
    }

    // If no structured parsing worked, put everything in overview
    if overview.is_empty() && action_items.is_empty() && key_points.is_empty() {
        overview = summary_text.clone();
    }

    let summary = Summary {
        overview: overview.clone(),
        action_items: action_items.clone(),
        key_points: key_points.clone(),
    };

    // Save summary to database if we have a meeting_id
    let effective_meeting_id = if meeting_id.is_some() {
        meeting_id.clone()
    } else {
        state.active_meeting_id.lock().await.clone()
    };

    if let Some(mid) = effective_meeting_id {
        // Serialize the summary as JSON for storage
        if let Ok(summary_json) = serde_json::to_string(&summary) {
            if let Err(e) = state.db.save_meeting_summary(&mid, &summary_json) {
                log::warn!("Failed to save summary to DB: {}", e);
            } else {
                log::info!("Summary saved to database for meeting {}", mid);
            }
        }
    }

    Ok(summary)
}

/// Get saved summary for a meeting
#[tauri::command]
pub async fn get_saved_summary(
    meeting_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Summary>, String> {
    let summary_json = state
        .db
        .get_meeting_summary(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))?;

    match summary_json {
        Some(json) => {
            let summary: Summary =
                serde_json::from_str(&json).map_err(|e| format!("Parse error: {}", e))?;
            Ok(Some(summary))
        }
        None => Ok(None),
    }
}

/// Save a conversation item (Q&A) for a meeting
#[tauri::command]
pub async fn save_conversation_item(
    meeting_id: String,
    question: String,
    answer: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .save_conversation_item(&meeting_id, &question, &answer)
        .map_err(|e| format!("DB error: {}", e))
}

/// Get saved conversations for a meeting
#[tauri::command]
pub async fn get_meeting_conversations(
    meeting_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::storage::ConversationItem>, String> {
    state
        .db
        .get_meeting_conversations(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))
}

/// Generate meeting title using LLM
#[tauri::command]
pub async fn generate_title(
    meeting_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get transcript segments (first few for context)
    let segments = state
        .db
        .get_segments(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))?;

    if segments.is_empty() {
        return Err("No transcript available for this meeting".to_string());
    }

    // Use first 10 segments for title generation (to get context without too much text)
    let context_segments: Vec<_> = segments.iter().take(10).collect();
    let transcript_text: String = context_segments
        .iter()
        .map(|s| s.text.clone())
        .collect::<Vec<_>>()
        .join(" ");

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings
                .openai_api_key
                .clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err(
                    "OpenAI API key is empty. Please add your API key in Settings.".to_string(),
                );
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings
                .ollama_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings
                .ollama_model
                .clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings);

    let client = LlmClient::new(provider);
    log::info!("Generating meeting title for {}", meeting_id);

    let title = client
        .generate_title(&transcript_text)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    // Clean up the title (remove quotes if present)
    let title = title
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();

    // Update the meeting title in the database
    state
        .db
        .update_meeting_title(&meeting_id, &title)
        .map_err(|e| format!("Failed to update title: {}", e))?;

    log::info!("Generated title: {}", title);
    Ok(title)
}

/// Generate suggested questions based on transcript context
#[tauri::command]
pub async fn generate_suggested_questions(
    transcript_context: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings
                .openai_api_key
                .clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err(
                    "OpenAI API key is empty. Please add your API key in Settings.".to_string(),
                );
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings
                .ollama_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings
                .ollama_model
                .clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings);

    let client = LlmClient::new(provider);
    log::info!("Generating suggested questions from transcript context");

    let questions = client
        .generate_suggested_questions(&transcript_context)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    log::info!("Generated {} suggested questions", questions.len());
    Ok(questions)
}

// ============================================================================
// Settings Commands
// ============================================================================

/// Get current settings
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().await.clone();
    Ok(settings)
}

/// Save settings (also persists to DB)
#[tauri::command]
pub async fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    // Persist to DB
    let json = serde_json::to_string(&settings).map_err(|e| format!("Serialize error: {}", e))?;
    state
        .db
        .save_settings_json(&json)
        .map_err(|e| format!("DB error: {}", e))?;

    *state.settings.lock().await = settings;
    log::info!("Settings saved to DB");
    Ok(())
}

// ============================================================================
// Model Commands
// ============================================================================

/// Check if required ML models are downloaded
/// Checks ALL whisper models — returns the first downloaded one found
#[tauri::command]
pub async fn check_model_status(state: State<'_, AppState>) -> Result<ModelStatus, String> {
    let settings = state.settings.lock().await;
    let configured_model_name = settings.whisper_model.clone();
    drop(settings);

    // First check the configured model
    let configured: WhisperModel = configured_model_name.parse().unwrap_or(WhisperModel::Small);

    let models_dir =
        asr::get_models_dir().map_err(|e| format!("Failed to get models dir: {}", e))?;

    if asr::is_model_downloaded(configured).unwrap_or(false) {
        return Ok(ModelStatus {
            whisper_downloaded: true,
            whisper_model: configured_model_name,
            whisper_size_mb: configured.size_mb(),
            models_dir: models_dir.to_string_lossy().to_string(),
        });
    }

    // Fallback: check if ANY model is downloaded
    let all_models = [
        WhisperModel::Tiny,
        WhisperModel::Base,
        WhisperModel::Small,
        WhisperModel::Medium,
        WhisperModel::Large,
    ];
    for model in all_models {
        if asr::is_model_downloaded(model).unwrap_or(false) {
            let name = format!("{:?}", model).to_lowercase();
            return Ok(ModelStatus {
                whisper_downloaded: true,
                whisper_model: name,
                whisper_size_mb: model.size_mb(),
                models_dir: models_dir.to_string_lossy().to_string(),
            });
        }
    }

    Ok(ModelStatus {
        whisper_downloaded: false,
        whisper_model: configured_model_name,
        whisper_size_mb: configured.size_mb(),
        models_dir: models_dir.to_string_lossy().to_string(),
    })
}

/// Get info about all available models
#[tauri::command]
pub async fn get_models_info() -> Result<Vec<ModelInfo>, String> {
    models::get_all_models_status().map_err(|e| format!("Failed to get models info: {}", e))
}

/// Download a specific model (Whisper or Parakeet)
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let language = {
        let settings = state.settings.lock().await;
        settings.language.clone()
    };

    // Parakeet models (requires `parakeet` feature)
    #[cfg(feature = "parakeet")]
    if model_name.starts_with("parakeet-") {
        let parakeet_name = model_name.trim_start_matches("parakeet-");
        let model: ParakeetModel = parakeet_name
            .parse()
            .map_err(|e: anyhow::Error| e.to_string())?;

        if !model.is_ctc() {
            return Err("Only CTC Parakeet models are currently supported".to_string());
        }

        let model_path = models::download_parakeet_model(&app, model)
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        let mut engine = TranscriptionEngine::with_backend(AsrBackendType::Parakeet)
            .map_err(|e| e.to_string())?;
        engine.set_language(&language);
        engine
            .load_model(&model_path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        *state.transcription_engine.lock().await = Some(engine);
        log::info!("Parakeet model {} loaded and ready", model_name);
        return Ok(());
    }

    #[cfg(not(feature = "parakeet"))]
    if model_name.starts_with("parakeet-") {
        return Err(
            "Parakeet backend is not available. Rebuild with --features parakeet to enable it."
                .to_string(),
        );
    }

    // Whisper models (default)
    let model: WhisperModel = model_name
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let model_path = models::download_model(&app, model)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let mut engine = TranscriptionEngine::new();
    engine.set_language(&language);
    engine
        .load_model(&model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    *state.transcription_engine.lock().await = Some(engine);
    log::info!(
        "Whisper model {} loaded and ready with language '{}'",
        model_name,
        language
    );
    Ok(())
}

/// Load an already-downloaded model into memory (Whisper or Parakeet)
#[tauri::command]
pub async fn load_model(model_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let language = {
        let settings = state.settings.lock().await;
        settings.language.clone()
    };

    // Parakeet models (requires `parakeet` feature)
    #[cfg(feature = "parakeet")]
    if model_name.starts_with("parakeet-") {
        let parakeet_name = model_name.trim_start_matches("parakeet-");
        let model: ParakeetModel = parakeet_name
            .parse()
            .map_err(|e: anyhow::Error| e.to_string())?;

        let model_path = crate::asr::parakeet_backend::get_parakeet_model_path(model)
            .map_err(|e| format!("Failed to get model path: {}", e))?;

        if !model_path.exists() {
            return Err(format!("Parakeet model {} is not downloaded", model_name));
        }

        log::info!(
            "Loading Parakeet model {} from {:?}",
            model_name,
            model_path
        );
        let mut engine = TranscriptionEngine::with_backend(AsrBackendType::Parakeet)
            .map_err(|e| e.to_string())?;
        engine.set_language(&language);
        engine
            .load_model(&model_path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        *state.transcription_engine.lock().await = Some(engine);
        log::info!("Parakeet model {} loaded and ready", model_name);
        return Ok(());
    }

    #[cfg(not(feature = "parakeet"))]
    if model_name.starts_with("parakeet-") {
        return Err(
            "Parakeet backend is not available. Rebuild with --features parakeet to enable it."
                .to_string(),
        );
    }

    // Whisper models (default)
    let model: WhisperModel = model_name
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let model_path =
        asr::get_model_path(model).map_err(|e| format!("Failed to get model path: {}", e))?;

    if !model_path.exists() {
        return Err(format!("Model {} is not downloaded", model_name));
    }

    log::info!(
        "Loading model {} with language '{}' from {:?}",
        model_name,
        language,
        model_path
    );
    let mut engine = TranscriptionEngine::new();
    engine.set_language(&language);
    engine
        .load_model(&model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    *state.transcription_engine.lock().await = Some(engine);
    log::info!(
        "Model {} loaded and ready with language '{}'",
        model_name,
        language
    );
    Ok(())
}

/// Import a model file from user's filesystem (manual download fallback)
/// Copies the file to the correct models directory and loads it
#[tauri::command]
pub async fn import_model(
    file_path: String,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = std::path::PathBuf::from(&file_path);
    if !source.exists() {
        return Err("File not found".to_string());
    }

    // Validate file size (at least 10MB for any model)
    let file_size = std::fs::metadata(&source)
        .map(|m| m.len())
        .map_err(|e| format!("Failed to read file: {}", e))?;
    if file_size < 10 * 1024 * 1024 {
        return Err(format!(
            "File is too small ({:.1} MB). This doesn't appear to be a valid model file.",
            file_size as f64 / (1024.0 * 1024.0)
        ));
    }

    let model: WhisperModel = model_name
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let target_path =
        asr::get_model_path(model).map_err(|e| format!("Failed to get model path: {}", e))?;

    // Handle .zip files by extracting the .bin inside
    let is_zip = source.extension().and_then(|e| e.to_str()) == Some("zip");

    if is_zip {
        log::info!(
            "Extracting model from zip {:?} to {:?}",
            source,
            target_path
        );
        crate::models::extract_bin_from_zip(&source, &target_path)
            .map_err(|e| format!("Failed to extract model from zip: {}", e))?;
    } else {
        log::info!(
            "Importing model from {:?} to {:?} ({} bytes)",
            source,
            target_path,
            file_size
        );
        std::fs::copy(&source, &target_path)
            .map_err(|e| format!("Failed to copy model file: {}", e))?;
    }

    // Load the model
    let language = {
        let settings = state.settings.lock().await;
        settings.language.clone()
    };

    let mut engine = TranscriptionEngine::new();
    engine.set_language(&language);
    engine.load_model(&target_path).map_err(|e| {
        // Clean up if loading fails
        let _ = std::fs::remove_file(&target_path);
        format!("File copied but failed to load model: {}", e)
    })?;

    *state.transcription_engine.lock().await = Some(engine);
    log::info!("Model {} imported and loaded successfully", model_name);
    Ok(())
}

/// Get the download URL for a whisper model (for manual download)
#[tauri::command]
pub async fn get_model_download_url(model_name: String) -> Result<serde_json::Value, String> {
    let model: WhisperModel = model_name
        .parse()
        .map_err(|e: anyhow::Error| e.to_string())?;
    Ok(serde_json::json!({
        "huggingface": model.download_url(),
        "github": model.github_release_url(),
    }))
}

// ============================================================================
// Audio Device Commands
// ============================================================================

/// List available audio input devices
#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let capture = AudioCapture::new().map_err(|e| format!("Failed to init audio: {}", e))?;

    let devices = capture
        .list_devices()
        .map_err(|e| format!("Failed to list devices: {}", e))?;

    Ok(devices
        .into_iter()
        .map(|d| AudioDeviceInfo {
            name: d.name,
            is_default: d.is_default,
        })
        .collect())
}

// ============================================================================
// Device Specs Commands
// ============================================================================

use crate::asr::{get_available_backends, BackendInfo};
use crate::specs::{DeviceSpecs, ModelRecommendation};

/// Get device specifications (CPU, RAM, GPU)
#[tauri::command]
pub async fn get_device_specs() -> Result<DeviceSpecs, String> {
    Ok(DeviceSpecs::detect())
}

/// Get model recommendation based on device specs
#[tauri::command]
pub async fn get_model_recommendation() -> Result<ModelRecommendation, String> {
    let specs = DeviceSpecs::detect();
    Ok(ModelRecommendation::from_specs(&specs))
}

// ============================================================================
// ASR Backend Commands
// ============================================================================

/// Get available ASR backends
#[tauri::command]
pub async fn get_asr_backends() -> Result<Vec<BackendInfo>, String> {
    Ok(get_available_backends())
}

// ============================================================================
// Embedding Commands
// ============================================================================

/// Download and load the BGE-small embedding model
#[tauri::command]
pub async fn download_embedding_model_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let model_dir = models::download_embedding_model(&app)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let model = EmbeddingModel::load(&model_dir)
        .map_err(|e| format!("Failed to load embedding model: {}", e))?;

    *state.embedding_model.lock().await = Some(model);
    log::info!("Embedding model loaded and ready");
    Ok(())
}

/// Load an already-downloaded embedding model
#[tauri::command]
pub async fn load_embedding_model(state: State<'_, AppState>) -> Result<(), String> {
    let model_dir =
        models::get_embedding_model_dir().map_err(|e| format!("Failed to get model dir: {}", e))?;

    if !model_dir.join("model.onnx").exists() {
        return Err("Embedding model not downloaded".to_string());
    }

    let model = EmbeddingModel::load(&model_dir)
        .map_err(|e| format!("Failed to load embedding model: {}", e))?;

    *state.embedding_model.lock().await = Some(model);
    log::info!("Embedding model loaded");
    Ok(())
}

/// Semantic search across embeddings
#[tauri::command]
pub async fn semantic_search(
    query: String,
    meeting_id: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SemanticSearchResult>, String> {
    let lim = limit.unwrap_or(10);

    let model_guard = state.embedding_model.lock().await;
    let model = model_guard.as_ref().ok_or("Embedding model not loaded")?;

    let query_emb = model
        .embed(&query)
        .map_err(|e| format!("Embedding failed: {}", e))?;
    drop(model_guard);

    state
        .db
        .search_semantic(&query_emb, lim, meeting_id.as_deref())
        .map_err(|e| format!("Search error: {}", e))
}

/// Batch embed all unembedded segments in a meeting
#[tauri::command]
pub async fn embed_meeting(meeting_id: String, state: State<'_, AppState>) -> Result<u64, String> {
    let model_guard = state.embedding_model.lock().await;
    let model = model_guard.as_ref().ok_or("Embedding model not loaded")?;

    // Get meeting title for enrichment
    let meeting = state
        .db
        .get_meeting(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or("Meeting not found")?;

    let unembedded = state
        .db
        .get_unembedded_segment_ids(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))?;

    let mut count = 0u64;
    for (seg_id, time_label, text) in &unembedded {
        let enriched = embeddings::enrich_segment(&meeting.title, time_label, text);
        match model.embed(&enriched) {
            Ok(emb) => {
                if let Err(e) = state.db.insert_embedding(seg_id, &emb) {
                    log::error!("Failed to store embedding for {}: {}", seg_id, e);
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                log::error!("Embedding failed for {}: {}", seg_id, e);
            }
        }
    }

    log::info!("Embedded {} segments for meeting {}", count, meeting_id);
    Ok(count)
}

/// Get embedding status (model loaded, counts)
#[tauri::command]
pub async fn get_embedding_status(state: State<'_, AppState>) -> Result<EmbeddingStatus, String> {
    let model_loaded = state.embedding_model.lock().await.is_some();
    let (embedded_count, total_segments) = state
        .db
        .count_embeddings()
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(EmbeddingStatus {
        model_loaded,
        embedded_count,
        total_segments,
    })
}

/// Check if embedding model is downloaded
#[tauri::command]
pub async fn is_embedding_model_downloaded() -> Result<bool, String> {
    Ok(models::is_embedding_model_downloaded())
}

/// Get download URLs for manual embedding model download
#[tauri::command]
pub fn get_embedding_model_download_urls() -> Result<EmbeddingModelUrls, String> {
    let (model_url, tokenizer_url) = models::get_embedding_model_download_urls();
    Ok(EmbeddingModelUrls {
        model_url,
        tokenizer_url,
    })
}

/// URLs for manual embedding model download
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingModelUrls {
    pub model_url: String,
    pub tokenizer_url: String,
}

/// Import embedding model from manually downloaded files
#[tauri::command]
pub async fn import_embedding_model(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source_path = std::path::PathBuf::from(&file_path);

    let model_dir = models::import_embedding_model(&source_path)
        .map_err(|e| format!("Import failed: {}", e))?;

    // Load the model after successful import
    let model = EmbeddingModel::load(&model_dir)
        .map_err(|e| format!("Failed to load embedding model: {}", e))?;

    *state.embedding_model.lock().await = Some(model);
    log::info!("Embedding model imported and loaded successfully");
    Ok(())
}

/// Get current audio level (RMS) for visualization
#[tauri::command]
pub async fn get_audio_level(state: State<'_, AppState>) -> Result<f32, String> {
    let audio = state.audio_capture.lock().await;
    match audio.as_ref() {
        Some(capture) => Ok(capture.get_rms_level()),
        None => Ok(0.0),
    }
}

// ============================================================================
// Meeting Detection Commands
// ============================================================================

/// Start automatic meeting detection (polls every 5 seconds)
#[tauri::command]
pub async fn start_meeting_detection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check if detection is already running
    if state.detection_running.load(Ordering::SeqCst) {
        return Ok(()); // Already running, no-op
    }

    // Check if auto-detect is enabled in settings
    {
        let settings = state.settings.lock().await;
        if !settings.auto_detect_meetings {
            return Err("Meeting detection is disabled in settings".to_string());
        }
    }

    state.detection_running.store(true, Ordering::SeqCst);
    log::info!("Meeting detection started");

    let detector_arc = state.meeting_detector.clone();
    let detection_running = state.detection_running.clone();
    let is_recording = state.is_recording.clone();
    let settings_arc = state.settings.clone();

    // Spawn background detection loop
    tauri::async_runtime::spawn(async move {
        let mut last_meeting_active = false;

        loop {
            // Check if we should stop
            if !detection_running.load(Ordering::SeqCst) {
                log::info!("Meeting detection stopped");
                break;
            }

            // Skip detection if already recording
            let recording = *is_recording.lock().await;
            if recording {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }

            // Detect meetings
            let mut detector = detector_arc.lock().await;

            // Check for new meeting detected
            if let Some(detected) = detector.detect_meeting() {
                log::info!(
                    "Meeting detected: {} ({})",
                    detected.app_name,
                    detected.process_name
                );

                // Check if system notifications are enabled
                let show_system_notifications = {
                    let settings = settings_arc.lock().await;
                    settings.show_system_notifications
                };

                // Send native OS notification (if enabled)
                #[cfg(desktop)]
                if show_system_notifications {
                    use tauri_plugin_notification::NotificationExt;
                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Meeting Detected")
                        .body(&format!(
                            "{} is running. Would you like to start recording?",
                            detected.app_name
                        ))
                        .show()
                    {
                        log::warn!("Failed to send native notification: {}", e);
                    }
                }

                // Also emit in-app notification event (always)
                let event = MeetingDetectedEvent {
                    app_name: detected.app_name.clone(),
                    message: format!(
                        "{} detected! Would you like to start recording?",
                        detected.app_name
                    ),
                };

                if let Err(e) = app.emit("meeting-detected", &event) {
                    log::error!("Failed to emit meeting-detected event: {}", e);
                }

                last_meeting_active = true;
            } else if last_meeting_active {
                // Check if meeting is still running
                if detector.is_meeting_running().is_none() {
                    log::info!("Meeting ended");

                    let event = MeetingEndedEvent {
                        message: "Meeting app closed".to_string(),
                    };

                    if let Err(e) = app.emit("meeting-ended", &event) {
                        log::error!("Failed to emit meeting-ended event: {}", e);
                    }

                    last_meeting_active = false;
                }
            }

            drop(detector);

            // Poll every 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

/// Stop automatic meeting detection
#[tauri::command]
pub async fn stop_meeting_detection(state: State<'_, AppState>) -> Result<(), String> {
    state.detection_running.store(false, Ordering::SeqCst);

    // Clear notification tracking
    let mut detector = state.meeting_detector.lock().await;
    detector.clear_notifications();

    log::info!("Meeting detection stopped");
    Ok(())
}

/// Check if meeting detection is currently running
#[tauri::command]
pub async fn is_meeting_detection_running(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.detection_running.load(Ordering::SeqCst))
}

/// Dismiss the current meeting detection notification
/// Keeps the meeting in "dismissed" state until it actually ends,
/// preventing repeated notifications for the same meeting
#[tauri::command]
pub async fn dismiss_meeting_notification(state: State<'_, AppState>) -> Result<(), String> {
    let mut detector = state.meeting_detector.lock().await;
    detector.dismiss_notification(); // Mark as dismissed, not cleared
    log::info!("Meeting notification dismissed (will not re-notify until meeting ends)");
    Ok(())
}

/// Check if a meeting app is currently running (one-shot check)
#[tauri::command]
pub async fn check_meeting_running(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut detector = state.meeting_detector.lock().await;
    Ok(detector.is_meeting_running().map(|d| d.app_name))
}

/// Check if Screen Recording permission is granted (macOS only)
/// Returns true if we can read OTHER apps' window titles, false otherwise
#[tauri::command]
pub async fn check_screen_recording_permission() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        // Use the improved permission check that tests reading OTHER apps' window titles
        Ok(MeetingDetector::has_screen_recording_permission())
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Windows/Linux, no special permission needed
        Ok(true)
    }
}

/// Open Screen Recording settings (macOS) or return info for other platforms
#[tauri::command]
pub async fn open_screen_recording_settings() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Open System Preferences > Privacy & Security > Screen Recording
        let result = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();

        match result {
            Ok(_) => {
                log::info!("Opened Screen Recording settings");
                Ok("opened".to_string())
            }
            Err(e) => {
                log::error!("Failed to open Screen Recording settings: {}", e);
                Err(format!("Failed to open settings: {}", e))
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        Ok("not_required".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        Ok("not_required".to_string())
    }
}

// ============================================================================
// AI-Powered Features: Analytics, Enhancement, Web Search
// ============================================================================

/// Get meeting statistics for a date range
#[tauri::command]
pub async fn get_meeting_stats(
    from_date: Option<String>,
    to_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<MeetingStats, String> {
    let (count, duration_ms) = state
        .db
        .get_meeting_stats(from_date.as_deref(), to_date.as_deref())
        .map_err(|e| format!("Failed to get stats: {}", e))?;

    Ok(MeetingStats {
        meeting_count: count,
        total_duration_ms: duration_ms,
    })
}

#[derive(Debug, Serialize)]
pub struct MeetingStats {
    pub meeting_count: i64,
    pub total_duration_ms: i64,
}

/// Extract metadata from a meeting (topics, action items, decisions)
#[tauri::command]
pub async fn extract_meeting_metadata(
    meeting_id: String,
    state: State<'_, AppState>,
) -> Result<ExtractedMetadata, String> {
    // Get full transcript
    let segments = state
        .db
        .get_segments(&meeting_id)
        .map_err(|e| format!("Failed to get segments: {}", e))?;

    let transcript = segments
        .iter()
        .map(|s| s.text.clone())
        .collect::<Vec<_>>()
        .join("\n");

    if transcript.is_empty() {
        return Err("No transcript found for this meeting".to_string());
    }

    // Build LLM client
    let provider = {
        let settings = state.settings.lock().await;
        match settings.llm_provider.as_str() {
            "openai" => {
                let api_key = settings
                    .openai_api_key
                    .clone()
                    .ok_or("OpenAI API key not configured")?;
                LlmProvider::OpenAI { api_key }
            }
            _ => {
                let url = settings
                    .ollama_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = settings
                    .ollama_model
                    .clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                LlmProvider::Ollama { url, model }
            }
        }
    };
    let client = LlmClient::new(provider);

    // Extract metadata using LLM
    let metadata = client
        .extract_metadata(&transcript)
        .await
        .map_err(|e| format!("Failed to extract metadata: {}", e))?;

    // Save to database
    let topics = serde_json::to_string(&metadata.topics).ok();
    let action_items = serde_json::to_string(&metadata.action_items).ok();
    let decisions = serde_json::to_string(&metadata.decisions).ok();

    state
        .db
        .update_meeting_metadata(
            &meeting_id,
            topics.as_deref(),
            action_items.as_deref(),
            decisions.as_deref(),
            metadata.participant_count_estimate,
        )
        .map_err(|e| format!("Failed to save metadata: {}", e))?;

    Ok(ExtractedMetadata {
        topics: metadata.topics,
        action_items: metadata.action_items,
        decisions: metadata.decisions,
        participant_count: metadata.participant_count_estimate,
    })
}

#[derive(Debug, Serialize)]
pub struct ExtractedMetadata {
    pub topics: Vec<String>,
    pub action_items: Vec<String>,
    pub decisions: Vec<String>,
    pub participant_count: i32,
}

/// Enhance a transcript segment using AI
#[tauri::command]
pub async fn enhance_transcript_segment(
    meeting_id: String,
    segment_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get segments
    let segments = state
        .db
        .get_segments(&meeting_id)
        .map_err(|e| format!("Failed to get segments: {}", e))?;

    // Find the target segment and its neighbors
    let idx = segments
        .iter()
        .position(|s| s.id == segment_id)
        .ok_or("Segment not found")?;

    let prev_text = if idx > 0 {
        Some(segments[idx - 1].text.as_str())
    } else {
        None
    };
    let current_text = &segments[idx].text;
    let next_text = if idx + 1 < segments.len() {
        Some(segments[idx + 1].text.as_str())
    } else {
        None
    };

    // Build LLM client
    let provider = {
        let settings = state.settings.lock().await;
        match settings.llm_provider.as_str() {
            "openai" => {
                let api_key = settings
                    .openai_api_key
                    .clone()
                    .ok_or("OpenAI API key not configured")?;
                LlmProvider::OpenAI { api_key }
            }
            _ => {
                let url = settings
                    .ollama_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = settings
                    .ollama_model
                    .clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                LlmProvider::Ollama { url, model }
            }
        }
    };
    let client = LlmClient::new(provider);

    // Enhance segment
    let enhanced = client
        .enhance_segment(prev_text, current_text, next_text)
        .await
        .map_err(|e| format!("Failed to enhance segment: {}", e))?;

    // Save to database
    state
        .db
        .update_segment_enhanced_text(&segment_id, Some(&enhanced))
        .map_err(|e| format!("Failed to save enhanced text: {}", e))?;

    Ok(enhanced)
}

/// Detect if a segment contains a question and generate an answer
#[tauri::command]
pub async fn detect_and_answer_question(
    meeting_id: String,
    segment_id: String,
    state: State<'_, AppState>,
) -> Result<QuestionResult, String> {
    // Get segments
    let segments = state
        .db
        .get_segments(&meeting_id)
        .map_err(|e| format!("Failed to get segments: {}", e))?;

    let idx = segments
        .iter()
        .position(|s| s.id == segment_id)
        .ok_or("Segment not found")?;

    let current_text = &segments[idx].text;

    // Build LLM client
    let provider = {
        let settings = state.settings.lock().await;
        match settings.llm_provider.as_str() {
            "openai" => {
                let api_key = settings
                    .openai_api_key
                    .clone()
                    .ok_or("OpenAI API key not configured")?;
                LlmProvider::OpenAI { api_key }
            }
            _ => {
                let url = settings
                    .ollama_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = settings
                    .ollama_model
                    .clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                LlmProvider::Ollama { url, model }
            }
        }
    };
    let client = LlmClient::new(provider);

    // Detect if it's a question
    let is_question = client
        .detect_question(current_text)
        .await
        .map_err(|e| format!("Failed to detect question: {}", e))?;

    let answer = if is_question {
        // Get context from surrounding segments
        let context_start = idx.saturating_sub(5);
        let context_end = (idx + 6).min(segments.len());
        let context: String = segments[context_start..context_end]
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let ans = client
            .answer_question(current_text, &context)
            .await
            .map_err(|e| format!("Failed to generate answer: {}", e))?;

        // Save to database
        state
            .db
            .update_segment_question(&segment_id, true, Some(&ans))
            .map_err(|e| format!("Failed to save question: {}", e))?;

        Some(ans)
    } else {
        None
    };

    Ok(QuestionResult {
        is_question,
        answer,
    })
}

#[derive(Debug, Serialize)]
pub struct QuestionResult {
    pub is_question: bool,
    pub answer: Option<String>,
}

/// Web search command for Phomy
#[tauri::command]
pub async fn web_search(query: String) -> Result<Vec<WebSearchResult>, String> {
    let client = crate::websearch::WebSearchClient::new();
    let results = client
        .search(&query, 5)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| WebSearchResult {
            title: r.title,
            url: r.url,
            snippet: r.snippet,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Ask Phomy with optional web search fallback
#[tauri::command]
pub async fn phomy_ask_with_search(
    question: String,
    use_web_search: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // First try to answer from meeting data
    let answer = phomy_ask(question.clone(), state.clone()).await;

    // If answer fails or indicates no context, try web search
    if use_web_search
        && (answer.is_err()
            || answer
                .as_ref()
                .map(|a| a.contains("No meeting"))
                .unwrap_or(false))
    {
        // Perform web search
        let search_results = web_search(question.clone()).await?;

        if !search_results.is_empty() {
            // Use LLM to synthesize answer from web results
            let provider = {
                let settings = state.settings.lock().await;
                match settings.llm_provider.as_str() {
                    "openai" => {
                        let api_key = settings
                            .openai_api_key
                            .clone()
                            .ok_or("OpenAI API key not configured")?;
                        LlmProvider::OpenAI { api_key }
                    }
                    _ => {
                        let url = settings
                            .ollama_url
                            .clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        let model = settings
                            .ollama_model
                            .clone()
                            .unwrap_or_else(|| "llama3.2".to_string());
                        LlmProvider::Ollama { url, model }
                    }
                }
            };
            let client = LlmClient::new(provider);

            let web_context = search_results
                .iter()
                .map(|r| format!("{} - {}\n{}", r.title, r.url, r.snippet))
                .collect::<Vec<_>>()
                .join("\n\n");

            let system = "You are Phomy, a helpful assistant. Answer the user's question based on the web search results provided. Be accurate and cite sources when possible.";
            let user = format!(
                "Web search results:\n{}\n\nQuestion: {}",
                web_context, question
            );

            return client
                .complete(system, &user)
                .await
                .map_err(|e| format!("LLM error: {}", e));
        }
    }

    answer
}
