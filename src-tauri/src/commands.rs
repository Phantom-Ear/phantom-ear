// Tauri IPC commands - bridge between frontend and Rust backend

use crate::asr::{self, TranscriptionEngine, WhisperModel};
#[cfg(feature = "parakeet")]
use crate::asr::AsrBackendType;
#[cfg(feature = "parakeet")]
use crate::asr::parakeet_backend::ParakeetModel;
use crate::audio::AudioCapture;
use crate::llm::{LlmClient, LlmProvider};
use crate::models::{self, ModelInfo};
use crate::storage::{Database, MeetingListItem, SearchResult, SegmentRow};
use crate::transcription::TranscriptionConfig;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
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

    // Initialize audio capture
    let mut audio_capture = AudioCapture::new()
        .map_err(|e| format!("Failed to initialize audio: {}", e))?;

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
    let mid = meeting_id.clone();
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

                                let seg_id = format!("seg-{}", segment_counter);
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
                                if let Err(e) = db.insert_segment(&SegmentRow {
                                    id: seg_id,
                                    meeting_id: meeting_id.clone(),
                                    time_label,
                                    text: text.to_string(),
                                    timestamp_ms: total_duration_ms,
                                }) {
                                    log::error!("Failed to persist segment: {}", e);
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
// Q&A Commands
// ============================================================================

/// Ask a question about the current meeting using RAG
#[tauri::command]
pub async fn ask_question(
    question: String,
    meeting_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Build context: from DB if meeting_id provided, otherwise from live transcript
    let context: String = if let Some(ref mid) = meeting_id {
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
#[tauri::command]
pub async fn check_model_status() -> Result<ModelStatus, String> {
    let model = WhisperModel::Small;
    let downloaded = asr::is_model_downloaded(model)
        .map_err(|e| format!("Failed to check model: {}", e))?;

    let models_dir = asr::get_models_dir()
        .map_err(|e| format!("Failed to get models dir: {}", e))?;

    Ok(ModelStatus {
        whisper_downloaded: downloaded,
        whisper_model: "small".to_string(),
        whisper_size_mb: model.size_mb(),
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
