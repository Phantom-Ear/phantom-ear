// PhantomEar - Privacy-first desktop meeting assistant
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

use commands::{AppState, Settings};
use std::sync::Arc;
use storage::Database;
use tauri::{
    Emitter, Manager, RunEvent, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::ask_question,
            commands::get_transcript,
            commands::generate_summary,
            commands::get_settings,
            commands::save_settings,
            commands::check_model_status,
            commands::download_model,
            commands::load_model,
            commands::import_model,
            commands::get_model_download_url,
            commands::get_models_info,
            commands::list_audio_devices,
            commands::get_device_specs,
            commands::get_model_recommendation,
            commands::get_asr_backends,
            // Meeting persistence commands
            commands::list_meetings,
            commands::get_meeting,
            commands::rename_meeting,
            commands::toggle_pin_meeting,
            commands::delete_meeting,
            commands::search_meetings,
            commands::export_meeting,
            // Segment editing commands
            commands::update_segment,
            commands::delete_segment,
            // Speaker commands
            commands::list_speakers,
            commands::create_speaker,
            commands::update_speaker,
            commands::delete_speaker,
            // Phomy assistant
            commands::phomy_ask,
            // Embedding commands
            commands::download_embedding_model_cmd,
            commands::load_embedding_model,
            commands::semantic_search,
            commands::embed_meeting,
            commands::get_embedding_status,
            commands::is_embedding_model_downloaded,
            commands::get_embedding_model_download_urls,
            commands::import_embedding_model,
            commands::get_audio_level,
        ])
        .setup(|app| {
            // Get app data directory and create DB synchronously
            let app_dir = app.path().app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            std::fs::create_dir_all(&app_dir)?;

            let db_path = app_dir.join("phantomear.db");
            log::info!("Opening database at: {:?}", db_path);

            let db = Database::new(&db_path)
                .map_err(|e| format!("Failed to initialize database: {}", e))?;
            let db = Arc::new(db);

            // Load settings from DB
            let settings = match db.load_settings_json() {
                Ok(Some(json)) => {
                    match serde_json::from_str::<Settings>(&json) {
                        Ok(s) => {
                            log::info!("Settings loaded from DB");
                            s
                        }
                        Err(e) => {
                            log::warn!("Failed to parse settings from DB: {}, using defaults", e);
                            Settings::default()
                        }
                    }
                }
                _ => {
                    log::info!("No settings in DB, using defaults");
                    Settings::default()
                }
            };

            let state = AppState {
                audio_capture: Arc::new(Mutex::new(None)),
                transcription_engine: Arc::new(Mutex::new(None)),
                transcript: Arc::new(Mutex::new(Vec::new())),
                is_recording: Arc::new(Mutex::new(false)),
                is_paused: Arc::new(Mutex::new(false)),
                settings: Arc::new(Mutex::new(settings)),
                db,
                active_meeting_id: Arc::new(Mutex::new(None)),
                embedding_model: Arc::new(Mutex::new(None)),
            };

            app.manage(state);

            // Setup system tray
            let toggle_item = MenuItem::with_id(app, "toggle", "Start Recording", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit PhantomEar", true, None::<&str>)?;

            let tray_menu = Menu::with_items(app, &[&toggle_item, &show_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app_handle: &tauri::AppHandle, event| {
                    match event.id.as_ref() {
                        "toggle" => {
                            // Emit event to frontend to toggle recording
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("tray-toggle-recording", ());
                            }
                        }
                        "show" => {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app_handle.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray_icon: &tauri::tray::TrayIcon, event| {
                    // Show window on left click
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray_icon.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            log::info!("System tray initialized");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Handle window close - minimize to tray instead of quitting
            if let RunEvent::WindowEvent { label, event: WindowEvent::CloseRequested { api, .. }, .. } = event {
                if label == "main" {
                    // Prevent the window from being destroyed
                    api.prevent_close();
                    // Hide the window instead
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.hide();
                    }
                    log::info!("Window hidden to tray");
                }
            }
        });
}
