// Tauri IPC commands - bridge between frontend and Rust backend

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub time: String,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub llm_provider: String, // "openai" or "ollama"
    pub openai_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
    pub auto_detect_meetings: bool,
    pub embedding_model: String, // "bge-small" or "nomic"
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm_provider: "ollama".to_string(),
            openai_api_key: None,
            ollama_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3.2".to_string()),
            auto_detect_meetings: false,
            embedding_model: "bge-small".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub asr_downloaded: bool,
    pub embedding_downloaded: bool,
    pub asr_size_mb: u64,
    pub embedding_size_mb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Summary {
    pub overview: String,
    pub action_items: Vec<String>,
    pub key_points: Vec<String>,
}

/// Start audio recording and transcription
#[tauri::command]
pub async fn start_recording() -> Result<(), String> {
    // TODO: Implement audio capture and ASR pipeline
    log::info!("Starting recording...");
    Ok(())
}

/// Stop recording and finalize transcript
#[tauri::command]
pub async fn stop_recording() -> Result<(), String> {
    // TODO: Stop audio capture, flush remaining audio to ASR
    log::info!("Stopping recording...");
    Ok(())
}

/// Ask a question about the current meeting using RAG
#[tauri::command]
pub async fn ask_question(question: String) -> Result<String, String> {
    // TODO:
    // 1. Generate embedding for question
    // 2. Search similar transcript chunks via sqlite-vec
    // 3. Build prompt with relevant context
    // 4. Send to LLM (OpenAI/Ollama)
    // 5. Return answer
    log::info!("Question received: {}", question);
    Ok("This feature is coming soon!".to_string())
}

/// Get current transcript segments
#[tauri::command]
pub async fn get_transcript() -> Result<Vec<TranscriptSegment>, String> {
    // TODO: Return transcript from current recording session
    Ok(vec![])
}

/// Generate meeting summary using LLM
#[tauri::command]
pub async fn generate_summary() -> Result<Summary, String> {
    // TODO: Send full transcript to LLM for summarization
    Ok(Summary {
        overview: "Summary generation coming soon!".to_string(),
        action_items: vec![],
        key_points: vec![],
    })
}

/// Get current settings
#[tauri::command]
pub async fn get_settings() -> Result<Settings, String> {
    // TODO: Load from storage
    Ok(Settings::default())
}

/// Save settings
#[tauri::command]
pub async fn save_settings(settings: Settings) -> Result<(), String> {
    // TODO: Persist to storage
    log::info!("Saving settings: {:?}", settings);
    Ok(())
}

/// Check if required ML models are downloaded
#[tauri::command]
pub async fn check_model_status() -> Result<ModelStatus, String> {
    // TODO: Check model files exist
    Ok(ModelStatus {
        asr_downloaded: false,
        embedding_downloaded: false,
        asr_size_mb: 500, // Parakeet model size estimate
        embedding_size_mb: 50, // BGE-small size estimate
    })
}

/// Download required ML models
#[tauri::command]
pub async fn download_models() -> Result<(), String> {
    // TODO: Download models with progress events
    log::info!("Starting model download...");
    Ok(())
}
