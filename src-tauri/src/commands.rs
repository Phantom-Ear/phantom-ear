// Tauri IPC commands - bridge between frontend and Rust backend

use crate::asr::{self, TranscriptionEngine, WhisperModel};
#[cfg(feature = "parakeet")]
use crate::asr::AsrBackendType;
#[cfg(feature = "parakeet")]
use crate::asr::parakeet_backend::ParakeetModel;
use crate::audio::AudioCapture;
use crate::detection::MeetingDetector;
use crate::embeddings::{self, EmbeddingModel};
use crate::llm::{LlmClient, LlmProvider};
use crate::models::{self, ModelInfo};
use crate::storage::{Database, MeetingListItem, SearchResult, SemanticSearchResult, SegmentRow, Speaker};
use crate::transcription::TranscriptionConfig;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub llm_provider: String,
    pub openai_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
    pub auto_detect_meetings: bool,
    pub whisper_model: String,
    pub language: String,
    #[serde(default = "default_asr_backend")]
    pub asr_backend: String,
    #[serde(default)]
    pub audio_device: Option<String>,
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
            whisper_model: "small".to_string(),
            language: "en".to_string(),
            asr_backend: "whisper".to_string(),
            audio_device: None,
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
    now.format("%a %d/%m/%y \u{00b7} %l:%M %p").to_string().trim().to_string()
}

/// Start audio recording and transcription
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut is_recording = state.is_recording.lock().await;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Check if model is loaded
    {
        let engine = state.transcription_engine.lock().await;
        if engine.is_none() {
            return Err("Transcription model not loaded. Please download a model first.".to_string());
        }
    }

    // Initialize audio capture with selected device from settings
    let mut audio_capture = AudioCapture::new()
        .map_err(|e| format!("Failed to initialize audio: {}", e))?;

    // Apply selected audio device if configured
    {
        let settings = state.settings.lock().await;
        if let Some(ref device_name) = settings.audio_device {
            audio_capture.select_device(Some(device_name.as_str()))
                .map_err(|e| format!("Failed to select audio device '{}': {}", device_name, e))?;
        }
    }

    audio_capture.start()
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    // Create meeting in DB
    let meeting_id = format!("meeting-{}", Utc::now().timestamp_millis());
    let title = format_meeting_title();
    let created_at = Utc::now().to_rfc3339();

    state.db.create_meeting(&meeting_id, &title, &created_at)
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    *state.active_meeting_id.lock().await = Some(meeting_id.clone());

    // Store in state
    *state.audio_capture.lock().await = Some(audio_capture);
    *state.transcript.lock().await = Vec::new();
    *state.is_paused.lock().await = false;
    *is_recording = true;

    // Start transcription loop in background
    let audio_capture_arc = state.audio_capture.clone();
    let engine_arc = state.transcription_engine.clone();
    let is_recording_arc = state.is_recording.clone();
    let is_paused_arc = state.is_paused.clone();
    let transcript_arc = state.transcript.clone();
    let db_arc = state.db.clone();
    let emb_model_arc = state.embedding_model.clone();
    let mid = meeting_id.clone();
    let meeting_title = title.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        run_transcription_loop_with_storage(
            app_clone,
            audio_capture_arc,
            engine_arc,
            is_recording_arc,
            is_paused_arc,
            transcript_arc,
            TranscriptionConfig::default(),
            db_arc,
            mid,
            emb_model_arc,
            meeting_title,
        ).await;
    });

    log::info!("Recording started with meeting {}", meeting_id);
    Ok(meeting_id)
}

/// Transcription loop that also stores segments
async fn run_transcription_loop_with_storage(
    app: AppHandle,
    audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    engine: Arc<Mutex<Option<TranscriptionEngine>>>,
    is_recording: Arc<Mutex<bool>>,
    is_paused: Arc<Mutex<bool>>,
    transcript: Arc<Mutex<Vec<TranscriptSegment>>>,
    config: TranscriptionConfig,
    db: Arc<Database>,
    meeting_id: String,
    embedding_model: Arc<Mutex<Option<EmbeddingModel>>>,
    meeting_title: String,
) {
    use crate::asr::resample_to_16khz;
    use crate::transcription::TranscriptionEvent;
    use tauri::Emitter;

    let chunk_samples = (config.chunk_duration_secs * 16000.0) as usize;
    let mut accumulated_samples: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    let mut segment_counter: u64 = 0;
    let mut total_duration_ms: i64 = 0;

    log::info!("Transcription loop started, chunk size: {} samples", chunk_samples);

    loop {
        // Check if still recording
        {
            let recording = is_recording.lock().await;
            if !*recording {
                break;
            }
        }

        // Check if paused - skip processing but keep loop alive
        {
            let paused = is_paused.lock().await;
            if *paused {
                // Drain audio buffer to prevent buildup
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
            // Calculate RMS to check for silence
            let rms: f32 = {
                let sum_squares: f32 = accumulated_samples[..chunk_samples]
                    .iter()
                    .map(|s| s * s)
                    .sum();
                (sum_squares / chunk_samples as f32).sqrt()
            };

            if rms >= config.silence_threshold {
                // Get the engine
                let engine_guard = engine.lock().await;
                if let Some(ref eng) = *engine_guard {
                    // Process chunk
                    let chunk: Vec<f32> = accumulated_samples.drain(..chunk_samples).collect();
                    let duration_ms = (chunk.len() as f32 / 16.0) as i64;

                    match eng.transcribe(&chunk).await {
                        Ok(result) => {
                            let text = result.full_text.trim();
                            if !text.is_empty() {
                                segment_counter += 1;

                                let seg_id = format!("{}-seg-{}", meeting_id, segment_counter);
                                let time_label = format_time(total_duration_ms as u64);

                                let event = TranscriptionEvent {
                                    id: seg_id.clone(),
                                    text: text.to_string(),
                                    start_ms: total_duration_ms,
                                    end_ms: total_duration_ms + duration_ms,
                                    is_partial: false,
                                };

                                // Store in transcript
                                {
                                    let mut transcript_guard = transcript.lock().await;
                                    transcript_guard.push(TranscriptSegment {
                                        id: event.id.clone(),
                                        time: time_label.clone(),
                                        text: event.text.clone(),
                                        timestamp_ms: total_duration_ms as u64,
                                    });
                                }

                                // Persist segment to DB
                                let seg_id_for_emb = seg_id.clone();
                                let time_label_for_emb = time_label.clone();
                                let text_for_emb = text.to_string();
                                if let Err(e) = db.insert_segment(&SegmentRow {
                                    id: seg_id,
                                    meeting_id: meeting_id.clone(),
                                    time_label,
                                    text: text.to_string(),
                                    timestamp_ms: total_duration_ms,
                                    speaker_id: None,
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
                                                &emb_title, &time_label_for_emb, &text_for_emb,
                                            );
                                            match m.embed(&enriched) {
                                                Ok(emb) => {
                                                    if let Err(e) = emb_db.insert_embedding(&seg_id_for_emb, &emb) {
                                                        log::error!("Failed to store embedding: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    log::error!("Embedding failed: {}", e);
                                                }
                                            }
                                        }
                                    });
                                }

                                total_duration_ms += duration_ms;

                                // Emit to frontend
                                if let Err(e) = app.emit("transcription", &event) {
                                    log::error!("Failed to emit transcription: {}", e);
                                }

                                log::info!("[{}] {}", format_time(event.start_ms as u64), text);
                            } else {
                                total_duration_ms += duration_ms;
                            }
                        }
                        Err(e) => {
                            log::error!("Transcription error: {}", e);
                            total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
                        }
                    }
                } else {
                    // No engine, discard samples
                    accumulated_samples.drain(..chunk_samples);
                    total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
                }
            } else {
                // Silence - discard but keep some overlap
                let keep = (config.overlap_secs * 16000.0) as usize;
                if accumulated_samples.len() > keep {
                    let drain_count = accumulated_samples.len() - keep;
                    accumulated_samples.drain(..drain_count);
                }
                total_duration_ms += (chunk_samples as f32 / 16.0) as i64;
            }
        }

        // Small delay to prevent busy loop
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    log::info!("Transcription loop ended, processed {} segments", segment_counter);
}

/// Stop recording and finalize transcript
#[tauri::command]
pub async fn stop_recording(
    state: State<'_, AppState>,
) -> Result<Vec<TranscriptSegment>, String> {
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
        capture.stop().map_err(|e| format!("Failed to stop: {}", e))?;
    }
    *capture_guard = None;

    let transcript = state.transcript.lock().await.clone();

    // Update meeting ended_at and duration
    let meeting_id = state.active_meeting_id.lock().await.clone();
    if let Some(mid) = &meeting_id {
        let ended_at = Utc::now().to_rfc3339();
        let duration_ms = transcript.last().map(|s| s.timestamp_ms as i64).unwrap_or(0);
        if let Err(e) = state.db.update_meeting_ended(mid, &ended_at, duration_ms) {
            log::error!("Failed to update meeting ended: {}", e);
        }
    }

    log::info!("Recording stopped, {} segments", transcript.len());

    // Auto-generate summary in background (non-blocking)
    if transcript.len() > 0 {
        if let Some(mid) = &meeting_id {
            let db_clone = state.db.clone();
            let settings_clone = state.settings.clone();
            let mid_clone = mid.clone();
            let transcript_text: String = transcript.iter()
                .map(|s| format!("[{}] {}", s.time, s.text))
                .collect::<Vec<_>>()
                .join("\n");

            tauri::async_runtime::spawn(async move {
                let settings = settings_clone.lock().await;
                let provider = match settings.llm_provider.as_str() {
                    "openai" => {
                        match settings.openai_api_key.clone() {
                            Some(key) if !key.is_empty() => LlmProvider::OpenAI { api_key: key },
                            _ => return,
                        }
                    }
                    _ => {
                        let url = settings.ollama_url.clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        let model = settings.ollama_model.clone()
                            .unwrap_or_else(|| "llama3.2".to_string());
                        LlmProvider::Ollama { url, model }
                    }
                };
                drop(settings);

                let client = LlmClient::new(provider);
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
pub async fn pause_recording(
    state: State<'_, AppState>,
) -> Result<(), String> {
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
pub async fn resume_recording(
    state: State<'_, AppState>,
) -> Result<(), String> {
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
pub async fn get_transcript(
    state: State<'_, AppState>,
) -> Result<Vec<TranscriptSegment>, String> {
    let transcript = state.transcript.lock().await.clone();
    Ok(transcript)
}

// ============================================================================
// Meeting Commands
// ============================================================================

#[tauri::command]
pub async fn list_meetings(
    state: State<'_, AppState>,
) -> Result<Vec<MeetingListItem>, String> {
    state.db.list_meetings().map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn get_meeting(
    id: String,
    state: State<'_, AppState>,
) -> Result<MeetingWithTranscript, String> {
    let meeting = state.db.get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    let segments = state.db.get_segments(&id)
        .map_err(|e| format!("DB error: {}", e))?;

    let transcript_segments: Vec<TranscriptSegment> = segments
        .into_iter()
        .map(|s| TranscriptSegment {
            id: s.id,
            time: s.time_label,
            text: s.text,
            timestamp_ms: s.timestamp_ms as u64,
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
    state.db.update_meeting_title(&id, &title)
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn toggle_pin_meeting(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meeting = state.db.get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;
    state.db.set_meeting_pinned(&id, !meeting.pinned)
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn delete_meeting(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.delete_meeting(&id)
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
    state.db.search_transcripts(&query, 50)
        .map_err(|e| format!("Search error: {}", e))
}

#[tauri::command]
pub async fn export_meeting(
    id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let meeting = state.db.get_meeting(&id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    let segments = state.db.get_segments(&id)
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
        state.db.update_segment_text(&segment_id, new_text)
            .map_err(|e| format!("Failed to update segment text: {}", e))?;
    }
    if speaker_id.is_some() || text.is_some() {
        // Only update speaker if explicitly passed (even if None to clear it)
        if let Some(ref speaker) = speaker_id {
            state.db.update_segment_speaker(&segment_id, Some(speaker.as_str()))
                .map_err(|e| format!("Failed to update segment speaker: {}", e))?;
        }
    }
    Ok(())
}

/// Delete a segment
#[tauri::command]
pub async fn delete_segment(
    segment_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.delete_segment(&segment_id)
        .map_err(|e| format!("Failed to delete segment: {}", e))
}

// ============================================================================
// Speaker Commands
// ============================================================================

/// List all speakers
#[tauri::command]
pub async fn list_speakers(
    state: State<'_, AppState>,
) -> Result<Vec<Speaker>, String> {
    state.db.list_speakers()
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
    state.db.create_speaker(&id, &name, &color, &created_at)
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
    state.db.update_speaker(&id, &name, &color)
        .map_err(|e| format!("Failed to update speaker: {}", e))
}

/// Delete a speaker
#[tauri::command]
pub async fn delete_speaker(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.delete_speaker(&id)
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
            match state.db.search_semantic(&emb, limit, effective_meeting_id.as_deref()) {
                Ok(results) if !results.is_empty() => {
                    Some(results.iter()
                        .map(|r| format!("[{}] {}", r.time_label, r.text))
                        .collect::<Vec<_>>()
                        .join("\n"))
                }
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
        let segments = state.db.get_segments(mid)
            .map_err(|e| format!("DB error: {}", e))?;
        if segments.is_empty() {
            return Err("No transcript available for this meeting".to_string());
        }
        segments.iter()
            .map(|s| format!("[{}] {}", s.time_label, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let transcript = state.transcript.lock().await;
        if transcript.is_empty() {
            return Err("No transcript available".to_string());
        }
        transcript.iter()
            .map(|s| format!("[{}] {}", s.time, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings.openai_api_key.clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err("OpenAI API key is empty. Please add your API key in Settings.".to_string());
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings.ollama_url.clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings.ollama_model.clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings); // Release lock before async call

    let client = LlmClient::new(provider);
    log::info!("Asking question: {}", question);

    client.answer_question(&context, &question)
        .await
        .map_err(|e| format!("LLM error: {}", e))
}

/// Phomy: intelligent assistant that routes queries appropriately
/// Handles recency, time-based, meeting recall, and global summary queries
#[tauri::command]
pub async fn phomy_ask(
    question: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let q = question.to_lowercase();

    // Build LLM client from settings
    let provider = {
        let settings = state.settings.lock().await;
        match settings.llm_provider.as_str() {
            "openai" => {
                let api_key = settings.openai_api_key.clone()
                    .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
                if api_key.is_empty() {
                    return Err("OpenAI API key is empty. Please add your API key in Settings.".to_string());
                }
                LlmProvider::OpenAI { api_key }
            }
            _ => {
                let url = settings.ollama_url.clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let model = settings.ollama_model.clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                LlmProvider::Ollama { url, model }
            }
        }
    };
    let client = LlmClient::new(provider);

    // ---- Route 1: "What did they just say?" / recency queries ----
    let is_recency = q.contains("just said") || q.contains("just say")
        || q.contains("they just") || q.contains("last thing")
        || q.contains("just now") || q.contains("right now")
        || (q.contains("what") && q.contains("just"));

    if is_recency {
        // Use live transcript (last ~10 chunks) or active meeting
        let context = {
            let is_recording = *state.is_recording.lock().await;
            let active_mid = state.active_meeting_id.lock().await.clone();

            if is_recording {
                let transcript = state.transcript.lock().await;
                let recent: Vec<_> = transcript.iter().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect();
                recent.iter().map(|s| format!("[{}] {}", s.time, s.text)).collect::<Vec<_>>().join("\n")
            } else if let Some(mid) = active_mid {
                let segs = state.db.get_last_segments(&mid, 10).map_err(|e| format!("DB error: {}", e))?;
                segs.iter().map(|s| format!("[{}] {}", s.time_label, s.text)).collect::<Vec<_>>().join("\n")
            } else {
                return Err("No active meeting to reference.".to_string());
            }
        };

        if context.is_empty() {
            return Err("No recent transcript available.".to_string());
        }

        let system = "You are Phomy, a calm meeting assistant. Summarize what was just said based on the most recent transcript chunks. Be brief and direct.";
        let user = format!("Recent transcript:\n{}\n\nQuestion: {}", context, question);
        return client.complete(system, &user).await.map_err(|e| format!("LLM error: {}", e));
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
                    let filtered: Vec<_> = transcript.iter()
                        .filter(|s| s.timestamp_ms as i64 >= cutoff)
                        .collect();
                    filtered.iter().map(|s| format!("[{}] {}", s.time, s.text)).collect::<Vec<_>>().join("\n")
                } else {
                    String::new()
                }
            } else if let Some(mid) = active_mid {
                let segs = state.db.get_segments(&mid).map_err(|e| format!("DB error: {}", e))?;
                if let Some(latest) = segs.last() {
                    let cutoff = latest.timestamp_ms - (mins * 60 * 1000);
                    let filtered: Vec<_> = segs.iter().filter(|s| s.timestamp_ms >= cutoff).collect();
                    filtered.iter().map(|s| format!("[{}] {}", s.time_label, s.text)).collect::<Vec<_>>().join("\n")
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
        let user = format!("Transcript from the last {} minutes:\n{}\n\nQuestion: {}", mins, context, question);
        return client.complete(system, &user).await.map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Route 3: Meeting recall ("last meeting", "previous meeting") ----
    let is_recall = q.contains("last meeting") || q.contains("previous meeting")
        || q.contains("most recent meeting");

    if is_recall {
        let meetings = state.db.get_recent_meetings_with_summaries(1)
            .map_err(|e| format!("DB error: {}", e))?;

        if meetings.is_empty() {
            return Err("No completed meetings found.".to_string());
        }

        let (mid, title, created_at, summary) = &meetings[0];
        let context = if let Some(s) = summary {
            format!("Meeting: {} ({})\nSummary:\n{}", title, created_at, s)
        } else {
            // Fall back to transcript chunks
            let segs = state.db.get_segments(mid).map_err(|e| format!("DB error: {}", e))?;
            let transcript: String = segs.iter()
                .map(|s| format!("[{}] {}", s.time_label, s.text))
                .collect::<Vec<_>>()
                .join("\n");
            format!("Meeting: {} ({})\nTranscript:\n{}", title, created_at, transcript)
        };

        let system = "You are Phomy, a calm meeting assistant. Answer the question using the meeting context provided. Be helpful and concise.";
        let user = format!("{}\n\nQuestion: {}", context, question);
        return client.complete(system, &user).await.map_err(|e| format!("LLM error: {}", e));
    }

    // ---- Route 4: Global/weekly summaries ("this week", "all meetings", "overall") ----
    let is_global = q.contains("this week") || q.contains("all meetings")
        || q.contains("overall") || q.contains("week cover")
        || q.contains("meetings about") || q.contains("my meetings");

    if is_global {
        let meetings = state.db.get_recent_meetings_with_summaries(10)
            .map_err(|e| format!("DB error: {}", e))?;

        if meetings.is_empty() {
            return Err("No completed meetings found.".to_string());
        }

        let context: String = meetings.iter().map(|(_, title, created_at, summary)| {
            if let Some(s) = summary {
                format!("--- {} ({}) ---\n{}\n", title, created_at, s)
            } else {
                format!("--- {} ({}) ---\n(no summary available)\n", title, created_at)
            }
        }).collect::<Vec<_>>().join("\n");

        let system = "You are Phomy, a calm meeting assistant. Provide a high-level overview across the meetings described. Be concise and organized.";
        let user = format!("Meeting summaries:\n{}\n\nQuestion: {}", context, question);
        return client.complete(system, &user).await.map_err(|e| format!("LLM error: {}", e));
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
                Ok(results) if !results.is_empty() => {
                    Some(results.iter()
                        .map(|r| format!("[{} - {}] {}", r.meeting_title, r.time_label, r.text))
                        .collect::<Vec<_>>()
                        .join("\n"))
                }
                _ => None,
            }
        } else {
            None
        }
    };

    let context = match semantic_context {
        Some(ctx) => ctx,
        None => return Err("No relevant context found. Try asking about a specific meeting.".to_string()),
    };

    let system = "You are Phomy, a calm meeting assistant. Answer the question using the provided meeting context. Be helpful and concise. If the answer isn't in the context, say so.";
    let user = format!("Context:\n{}\n\nQuestion: {}", context, question);
    client.complete(system, &user).await.map_err(|e| format!("LLM error: {}", e))
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
        let segments = state.db.get_segments(mid)
            .map_err(|e| format!("DB error: {}", e))?;
        if segments.is_empty() {
            return Err("No transcript available for this meeting".to_string());
        }
        segments.iter()
            .map(|s| format!("[{}] {}", s.time_label, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let transcript = state.transcript.lock().await;
        if transcript.is_empty() {
            return Err("No transcript available".to_string());
        }
        transcript.iter()
            .map(|s| format!("[{}] {}", s.time, s.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Get settings and create LLM client
    let settings = state.settings.lock().await;
    let provider = match settings.llm_provider.as_str() {
        "openai" => {
            let api_key = settings.openai_api_key.clone()
                .ok_or("OpenAI API key not configured. Please add your API key in Settings.")?;
            if api_key.is_empty() {
                return Err("OpenAI API key is empty. Please add your API key in Settings.".to_string());
            }
            LlmProvider::OpenAI { api_key }
        }
        "ollama" | _ => {
            let url = settings.ollama_url.clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = settings.ollama_model.clone()
                .unwrap_or_else(|| "llama3.2".to_string());
            LlmProvider::Ollama { url, model }
        }
    };
    drop(settings); // Release lock before async call

    let client = LlmClient::new(provider);
    log::info!("Generating meeting summary");

    let summary_text = client.summarize(&transcript_text)
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
        if line_lower.contains("action item") || line_lower.contains("todo") || line_lower.contains("next step") {
            current_section = "actions";
            continue;
        } else if line_lower.contains("key point") || line_lower.contains("highlight") || line_lower.contains("important") {
            current_section = "points";
            continue;
        }

        let trimmed = line.trim().trim_start_matches(&['-', '*', '\u{2022}', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', ')'][..]).trim();
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
        overview = summary_text;
    }

    Ok(Summary {
        overview,
        action_items,
        key_points,
    })
}

// ============================================================================
// Settings Commands
// ============================================================================

/// Get current settings
#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    let settings = state.settings.lock().await.clone();
    Ok(settings)
}

/// Save settings (also persists to DB)
#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Persist to DB
    let json = serde_json::to_string(&settings)
        .map_err(|e| format!("Serialize error: {}", e))?;
    state.db.save_settings_json(&json)
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
pub async fn check_model_status(
    state: State<'_, AppState>,
) -> Result<ModelStatus, String> {
    let settings = state.settings.lock().await;
    let configured_model_name = settings.whisper_model.clone();
    drop(settings);

    // First check the configured model
    let configured: WhisperModel = configured_model_name.parse()
        .unwrap_or(WhisperModel::Small);

    let models_dir = asr::get_models_dir()
        .map_err(|e| format!("Failed to get models dir: {}", e))?;

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
        WhisperModel::Tiny, WhisperModel::Base, WhisperModel::Small,
        WhisperModel::Medium, WhisperModel::Large,
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
    models::get_all_models_status()
        .map_err(|e| format!("Failed to get models info: {}", e))
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
        let model: ParakeetModel = parakeet_name.parse()
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
        engine.load_model(&model_path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        *state.transcription_engine.lock().await = Some(engine);
        log::info!("Parakeet model {} loaded and ready", model_name);
        return Ok(());
    }

    #[cfg(not(feature = "parakeet"))]
    if model_name.starts_with("parakeet-") {
        return Err("Parakeet backend is not available. Rebuild with --features parakeet to enable it.".to_string());
    }

    // Whisper models (default)
    let model: WhisperModel = model_name.parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let model_path = models::download_model(&app, model)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let mut engine = TranscriptionEngine::new();
    engine.set_language(&language);
    engine.load_model(&model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    *state.transcription_engine.lock().await = Some(engine);
    log::info!("Whisper model {} loaded and ready with language '{}'", model_name, language);
    Ok(())
}

/// Load an already-downloaded model into memory (Whisper or Parakeet)
#[tauri::command]
pub async fn load_model(
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
        let model: ParakeetModel = parakeet_name.parse()
            .map_err(|e: anyhow::Error| e.to_string())?;

        let model_path = crate::asr::parakeet_backend::get_parakeet_model_path(model)
            .map_err(|e| format!("Failed to get model path: {}", e))?;

        if !model_path.exists() {
            return Err(format!("Parakeet model {} is not downloaded", model_name));
        }

        log::info!("Loading Parakeet model {} from {:?}", model_name, model_path);
        let mut engine = TranscriptionEngine::with_backend(AsrBackendType::Parakeet)
            .map_err(|e| e.to_string())?;
        engine.set_language(&language);
        engine.load_model(&model_path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        *state.transcription_engine.lock().await = Some(engine);
        log::info!("Parakeet model {} loaded and ready", model_name);
        return Ok(());
    }

    #[cfg(not(feature = "parakeet"))]
    if model_name.starts_with("parakeet-") {
        return Err("Parakeet backend is not available. Rebuild with --features parakeet to enable it.".to_string());
    }

    // Whisper models (default)
    let model: WhisperModel = model_name.parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let model_path = asr::get_model_path(model)
        .map_err(|e| format!("Failed to get model path: {}", e))?;

    if !model_path.exists() {
        return Err(format!("Model {} is not downloaded", model_name));
    }

    log::info!("Loading model {} with language '{}' from {:?}", model_name, language, model_path);
    let mut engine = TranscriptionEngine::new();
    engine.set_language(&language);
    engine.load_model(&model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    *state.transcription_engine.lock().await = Some(engine);
    log::info!("Model {} loaded and ready with language '{}'", model_name, language);
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

    let model: WhisperModel = model_name.parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    let target_path = asr::get_model_path(model)
        .map_err(|e| format!("Failed to get model path: {}", e))?;

    // Handle .zip files by extracting the .bin inside
    let is_zip = source.extension().and_then(|e| e.to_str()) == Some("zip");

    if is_zip {
        log::info!("Extracting model from zip {:?} to {:?}", source, target_path);
        crate::models::extract_bin_from_zip(&source, &target_path)
            .map_err(|e| format!("Failed to extract model from zip: {}", e))?;
    } else {
        log::info!("Importing model from {:?} to {:?} ({} bytes)", source, target_path, file_size);
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
    engine.load_model(&target_path)
        .map_err(|e| {
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
    let model: WhisperModel = model_name.parse()
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
    let capture = AudioCapture::new()
        .map_err(|e| format!("Failed to init audio: {}", e))?;

    let devices = capture.list_devices()
        .map_err(|e| format!("Failed to list devices: {}", e))?;

    Ok(devices.into_iter().map(|d| AudioDeviceInfo {
        name: d.name,
        is_default: d.is_default,
    }).collect())
}

// ============================================================================
// Device Specs Commands
// ============================================================================

use crate::specs::{DeviceSpecs, ModelRecommendation};
use crate::asr::{BackendInfo, get_available_backends};

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
pub async fn load_embedding_model(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let model_dir = models::get_embedding_model_dir()
        .map_err(|e| format!("Failed to get model dir: {}", e))?;

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
    let model = model_guard.as_ref()
        .ok_or("Embedding model not loaded")?;

    let query_emb = model.embed(&query)
        .map_err(|e| format!("Embedding failed: {}", e))?;
    drop(model_guard);

    state.db.search_semantic(&query_emb, lim, meeting_id.as_deref())
        .map_err(|e| format!("Search error: {}", e))
}

/// Batch embed all unembedded segments in a meeting
#[tauri::command]
pub async fn embed_meeting(
    meeting_id: String,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let model_guard = state.embedding_model.lock().await;
    let model = model_guard.as_ref()
        .ok_or("Embedding model not loaded")?;

    // Get meeting title for enrichment
    let meeting = state.db.get_meeting(&meeting_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or("Meeting not found")?;

    let unembedded = state.db.get_unembedded_segment_ids(&meeting_id)
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
pub async fn get_embedding_status(
    state: State<'_, AppState>,
) -> Result<EmbeddingStatus, String> {
    let model_loaded = state.embedding_model.lock().await.is_some();
    let (embedded_count, total_segments) = state.db.count_embeddings()
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
                log::info!("Meeting detected: {} ({})", detected.app_name, detected.process_name);

                let event = MeetingDetectedEvent {
                    app_name: detected.app_name.clone(),
                    message: format!("{} detected! Would you like to start recording?", detected.app_name),
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
pub async fn stop_meeting_detection(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.detection_running.store(false, Ordering::SeqCst);

    // Clear notification tracking
    let mut detector = state.meeting_detector.lock().await;
    detector.clear_notifications();

    log::info!("Meeting detection stopped");
    Ok(())
}

/// Check if meeting detection is currently running
#[tauri::command]
pub async fn is_meeting_detection_running(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(state.detection_running.load(Ordering::SeqCst))
}

/// Dismiss the current meeting detection notification
/// (allows same app to be detected again later)
#[tauri::command]
pub async fn dismiss_meeting_notification(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut detector = state.meeting_detector.lock().await;
    detector.clear_notifications();
    log::info!("Meeting notification dismissed");
    Ok(())
}

/// Check if a meeting app is currently running (one-shot check)
#[tauri::command]
pub async fn check_meeting_running(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let mut detector = state.meeting_detector.lock().await;
    Ok(detector.is_meeting_running().map(|d| d.app_name))
}
