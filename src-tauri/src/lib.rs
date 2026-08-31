mod audio;
mod clipboard;
mod commands;
mod domain;
mod native_nemo;
mod runtime;
mod settings;
mod transcription;
mod windows;

use std::sync::{Arc, Mutex};

use tauri::{Manager, RunEvent, WindowEvent};
use tracing_subscriber::EnvFilter;

use crate::audio::CpalAudioCapture;
use crate::commands::AppState;
use crate::runtime::RuntimeManager;
use crate::settings::SettingsStore;
use crate::transcription::TranscriptionRouter;

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("lyra=info")),
        )
        .compact()
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let settings = SettingsStore::load(app.handle());
            let runtime = RuntimeManager::new(app.handle());
            let audio = Arc::new(CpalAudioCapture::new());
            let router = TranscriptionRouter::new(runtime.clone());
            let state = AppState::new(settings, runtime, audio, router, app.handle().clone());
            app.manage(Arc::new(Mutex::new(state)));
            commands::initialize(app.handle())?;
            if app
                .state::<Arc<Mutex<AppState>>>()
                .inner()
                .lock()
                .map(|state| state.settings.get().onboarding_complete)
                .unwrap_or(false)
            {
                if let Some(settings_window) = app.get_webview_window("settings") {
                    let _ = settings_window.hide();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_runtime_status,
            commands::get_setup_artifacts,
            commands::get_session_snapshot,
            commands::list_audio_devices,
            commands::start_dictation,
            commands::stop_dictation,
            commands::cancel_dictation,
            commands::copy_last_transcript,
            commands::has_openai_credential,
            commands::save_openai_credential,
            commands::clear_openai_credential,
            commands::show_settings,
            commands::hide_bar,
            commands::quit_lyra
        ])
        .build(tauri::generate_context!())
        .expect("error while building Lyra")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<Arc<Mutex<AppState>>>().inner().clone();
                if let Ok(mut guard) = state.lock() {
                    guard.shutdown();
                };
            }
        });
}
