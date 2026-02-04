// Tauri IPC commands - bridge between frontend and Rust backend

use crate::asr::{self, TranscriptionEngine, WhisperModel};
use crate::audio::AudioCapture;
use crate::models::{self, ModelInfo};
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm_provider: "ollama".to_string(),
            openai_api_key: None,
            ollama_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3.2".to_string()),
            auto_detect_meetings: false,
            whisper_model: "base".to_string(),
            language: "en".to_string(),
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

// ============================================================================
// App State
// ============================================================================

pub struct AppState {
    pub audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    pub transcription_engine: Arc<Mutex<Option<TranscriptionEngine>>>,
    pub transcript: Arc<Mutex<Vec<TranscriptSegment>>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub settings: Arc<Mutex<Settings>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            audio_capture: Arc::new(Mutex::new(None)),
            transcription_engine: Arc::new(Mutex::new(None)),
            transcript: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }
}

// ============================================================================
// Recording Commands
// ============================================================================

/// Start audio recording and transcription
#[tauri::command]
pub async fn start_recording(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut is_recording = state.is_recording.lock().await;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    // Initialize audio capture
    let mut audio_capture = AudioCapture::new()
        .map_err(|e| format!("Failed to initialize audio: {}", e))?;

    audio_capture.start()
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    // Store in state
    *state.audio_capture.lock().await = Some(audio_capture);
    *state.transcript.lock().await = Vec::new();
    *is_recording = true;

    // Check if model is loaded
    let engine = state.transcription_engine.lock().await;
    if engine.is_none() {
        log::warn!("Transcription engine not loaded - transcription will not work");
    }

    log::info!("Recording started");
    Ok(())
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

    // Stop audio capture
    let mut capture_guard = state.audio_capture.lock().await;
    if let Some(ref mut capture) = *capture_guard {
        capture.stop().map_err(|e| format!("Failed to stop: {}", e))?;
    }

    *is_recording = false;
    *capture_guard = None;

    let transcript = state.transcript.lock().await.clone();
    log::info!("Recording stopped, {} segments", transcript.len());

    Ok(transcript)
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
// Q&A Commands
// ============================================================================

/// Ask a question about the current meeting using RAG
#[tauri::command]
pub async fn ask_question(
    question: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let transcript = state.transcript.lock().await;

    if transcript.is_empty() {
        return Err("No transcript available".to_string());
    }

    // Build context from transcript
    let _context: String = transcript
        .iter()
        .map(|s| format!("[{}] {}", s.time, s.text))
        .collect::<Vec<_>>()
        .join("\n");

    // TODO: Use LLM to answer question
    log::info!("Question received: {}", question);
    Ok(format!("Question: {}\n\nThis feature requires LLM configuration. Please set up OpenAI or Ollama in settings.", question))
}

/// Generate meeting summary using LLM
#[tauri::command]
pub async fn generate_summary(
    state: State<'_, AppState>,
) -> Result<Summary, String> {
    let transcript = state.transcript.lock().await;

    if transcript.is_empty() {
        return Err("No transcript available".to_string());
    }

    // TODO: Use LLM to generate summary
    Ok(Summary {
        overview: "Summary generation requires LLM configuration.".to_string(),
        action_items: vec![],
        key_points: transcript.iter().map(|s| s.text.clone()).take(5).collect(),
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

/// Save settings
#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.settings.lock().await = settings;
    log::info!("Settings saved");
    Ok(())
}

// ============================================================================
// Model Commands
// ============================================================================

/// Check if required ML models are downloaded
#[tauri::command]
pub async fn check_model_status() -> Result<ModelStatus, String> {
    let model = WhisperModel::Base;
    let downloaded = asr::is_model_downloaded(model)
        .map_err(|e| format!("Failed to check model: {}", e))?;

    let models_dir = asr::get_models_dir()
        .map_err(|e| format!("Failed to get models dir: {}", e))?;

    Ok(ModelStatus {
        whisper_downloaded: downloaded,
        whisper_model: "base".to_string(),
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

/// Download a specific model
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let model: WhisperModel = model_name.parse()
        .map_err(|e: anyhow::Error| e.to_string())?;

    // Download the model
    let model_path = models::download_model(&app, model)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    // Load the model into the transcription engine
    let mut engine = TranscriptionEngine::new();
    engine.load_model(&model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;

    *state.transcription_engine.lock().await = Some(engine);

    log::info!("Model {} loaded and ready", model_name);
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
