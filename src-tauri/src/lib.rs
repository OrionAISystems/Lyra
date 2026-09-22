mod audio;
mod benchmark;
mod clipboard;
mod commands;
mod domain;
mod native_nemo;
mod personalization;
mod runtime;
mod settings;
pub(crate) mod streaming;
mod transcription;
mod tray;
mod updater;
mod usage;
mod windows;

use std::sync::{Arc, Mutex};

use tauri::{Manager, RunEvent, WindowEvent};
use tracing_subscriber::EnvFilter;

use crate::audio::CpalAudioCapture;
use crate::commands::AppState;
use crate::personalization::PersonalizationStore;
use crate::runtime::RuntimeManager;
use crate::settings::SettingsStore;
use crate::transcription::TranscriptionRouter;
use crate::updater::UpdateManager;
use crate::usage::UsageStatsStore;

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("lyra=info")),
        )
        .compact()
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(settings) = app.get_webview_window("settings") {
                let _ = settings.show();
                let _ = settings.set_focus();
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            let personalization =
                PersonalizationStore::load(app.handle(), settings.get().corrections);
            let audio = Arc::new(CpalAudioCapture::new());
            let router = TranscriptionRouter::new(runtime.clone());
            let updater = UpdateManager::new(app.handle());
            let usage = UsageStatsStore::new(app.handle());
            let state = AppState::new(
                settings,
                runtime,
                audio,
                router,
                updater,
                app.handle().clone(),
                usage,
                personalization,
            );
            app.manage(Arc::new(Mutex::new(state)));
            commands::initialize(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::get_hotkey_status,
            commands::retry_hotkey,
            commands::save_settings,
            commands::get_runtime_status,
            commands::get_model_runtime_status,
            commands::list_displays,
            commands::get_bar_placement_status,
            commands::get_usage_stats,
            commands::reset_usage_stats,
            commands::get_personalization,
            commands::save_personalization,
            commands::get_setup_artifacts,
            commands::get_setup_progress,
            commands::start_setup_artifact,
            commands::cancel_setup_artifact,
            commands::get_update_status,
            commands::check_for_updates,
            commands::install_update,
            commands::defer_update,
            commands::get_session_snapshot,
            commands::list_audio_devices,
            commands::get_benchmark_result,
            commands::run_benchmark,
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
