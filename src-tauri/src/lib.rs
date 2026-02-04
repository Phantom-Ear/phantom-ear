// Sidecar - Privacy-first desktop meeting assistant
// Main library entry point

mod audio;
mod asr;
mod storage;
mod llm;
mod embeddings;
mod detection;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::ask_question,
            commands::get_transcript,
            commands::generate_summary,
            commands::get_settings,
            commands::save_settings,
            commands::check_model_status,
            commands::download_models,
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
