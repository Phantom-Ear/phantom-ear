// Sidecar - Privacy-first desktop meeting assistant
// Main library entry point

pub mod audio;
pub mod asr;
pub mod commands;
pub mod detection;
pub mod embeddings;
pub mod llm;
pub mod models;
pub mod specs;
pub mod storage;
pub mod transcription;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::ask_question,
            commands::get_transcript,
            commands::generate_summary,
            commands::get_settings,
            commands::save_settings,
            commands::check_model_status,
            commands::download_model,
            commands::load_model,
            commands::get_models_info,
            commands::list_audio_devices,
            commands::get_device_specs,
            commands::get_model_recommendation,
            commands::get_asr_backends,
        ])
        .setup(|app| {
            // Initialize storage on app start
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = storage::init_database(&app_handle).await {
                    log::error!("Failed to initialize database: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
