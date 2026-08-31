use std::{
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    audio::{ActiveCapture, AudioCapture, CapturedAudio},
    clipboard::{ClipboardService, TextInjector},
    domain::{
        ActivationAction, ActivationController, ActivationSource, BackendError,
        ConservativePostProcessor, DictationState, DictationStateMachine, PostProcessor,
        SessionEvent, SessionSnapshot, ShortcutEdge, TranscriptionRequest, TranscriptionResult,
    },
    runtime::{RuntimeManager, RuntimeStatus, SetupArtifact},
    settings::{Settings, SettingsStore},
    transcription::TranscriptionRouter,
    windows::{capture_foreground_target, place_bar_on_target_monitor, TargetContext},
};

pub struct AppState {
    pub settings: SettingsStore,
    pub runtime: RuntimeManager,
    pub audio: Arc<dyn AudioCapture>,
    pub router: TranscriptionRouter,
    pub injector: Arc<dyn TextInjector>,
    pub session: DictationStateMachine,
    pub snapshot: SessionSnapshot,
    pub activation: ActivationController,
    pub active_capture: Option<ActiveCapture>,
    pub target: Option<TargetContext>,
    pub last_transcript: Option<String>,
    pub last_result: Option<TranscriptionResult>,
    pub hotkey_status: HotkeyStatus,
    pub app: AppHandle,
    pub started_at: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub hotkey: String,
    pub registered: bool,
    pub error: Option<String>,
}

impl AppState {
    pub fn new(
        settings: SettingsStore,
        runtime: RuntimeManager,
        audio: Arc<dyn AudioCapture>,
        router: TranscriptionRouter,
        app: AppHandle,
    ) -> Self {
        let settings_value = settings.get();
        Self {
            settings,
            runtime,
            audio,
            router,
            injector: Arc::new(ClipboardService),
            session: DictationStateMachine::default(),
            snapshot: SessionSnapshot::default(),
            activation: ActivationController::new(settings_value.activation_mode),
            active_capture: None,
            target: None,
            last_transcript: None,
            last_result: None,
            hotkey_status: HotkeyStatus {
                hotkey: settings_value.hotkey,
                registered: false,
                error: Some("Hotkey registration has not completed".to_owned()),
            },
            app,
            started_at: None,
        }
    }

    pub fn emit_state(&self) {
        let mut snapshot = self.snapshot.clone();
        snapshot.duration_ms = self.duration_ms();
        let _ = self.app.emit("lyra://state", snapshot);
    }

    pub fn emit_runtime_status(&self, status: RuntimeStatus) {
        let _ = self.app.emit("lyra://runtime-status", status);
    }

    pub fn emit_hotkey_status(&self) {
        let _ = self
            .app
            .emit("lyra://hotkey-status", self.hotkey_status.clone());
    }

    pub fn duration_ms(&self) -> u64 {
        self.started_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(self.snapshot.duration_ms)
    }

    pub fn shutdown(&mut self) {
        if let Some(capture) = self.active_capture.take() {
            let _ = capture.cancel();
        }
        info!("Lyra shutdown requested");
    }
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = app.state::<Arc<Mutex<AppState>>>().inner().clone();
    let hotkey_label = settings
        .lock()
        .map(|state| state.settings.get().hotkey)
        .unwrap_or_else(|_| "Ctrl+Alt+Space".to_owned());
    let hotkey_status = match register_hotkey(app, settings) {
        Ok(()) => HotkeyStatus {
            hotkey: hotkey_label,
            registered: true,
            error: None,
        },
        Err(error) => {
            let message = error.to_string();
            warn!(hotkey = %hotkey_label, error = %message, "global shortcut unavailable; bar and tray remain available");
            HotkeyStatus {
                hotkey: hotkey_label,
                registered: false,
                error: Some(message),
            }
        }
    };
    create_tray(app)?;
    let runtime = app.state::<Arc<Mutex<AppState>>>().inner().clone();
    if let Ok(mut state) = runtime.lock() {
        state.hotkey_status = hotkey_status;
        state.emit_hotkey_status();
        state.emit_runtime_status(state.runtime.status());
        state.emit_state();
    }
    Ok(())
}

fn register_hotkey(
    app: &AppHandle,
    state: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let handler_state = state.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            let edge = match event.state {
                ShortcutState::Pressed => ShortcutEdge::Pressed,
                ShortcutState::Released => ShortcutEdge::Released,
            };
            let action = handler_state
                .lock()
                .ok()
                .map(|mut state| state.activation.handle(edge))
                .unwrap_or(ActivationAction::Ignore);
            match action {
                ActivationAction::Start => {
                    if let Err(error) =
                        start_session(handler_state.clone(), ActivationSource::Hotkey)
                    {
                        warn!(error = %error, "global shortcut could not start dictation");
                    }
                }
                ActivationAction::Stop => {
                    if let Err(error) =
                        stop_session(handler_state.clone(), ActivationSource::Hotkey)
                    {
                        warn!(error = %error, "global shortcut could not stop dictation");
                    }
                }
                ActivationAction::Ignore => {}
            }
            let _ = app.emit("lyra://shortcut", edge);
        })?;
    app.global_shortcut().register(shortcut)?;
    Ok(())
}

fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let copy = MenuItem::with_id(
        app,
        "copy-last",
        "Copy last transcription",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Lyra", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &copy, &separator, &quit])?;
    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Lyra dictation")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = show_settings_window(app);
            }
            "copy-last" => {
                if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                    if let Ok(state) = state.lock() {
                        if let Some(text) = state.last_transcript.as_deref() {
                            let _ = state.injector.copy_text(text);
                        }
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Settings, String> {
    state
        .lock()
        .map(|state| state.settings.get())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn get_hotkey_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<HotkeyStatus, String> {
    state
        .lock()
        .map(|state| state.hotkey_status.clone())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn save_settings(
    settings: Settings,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Settings, String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let saved = state
        .settings
        .set(settings)
        .map_err(|error| error.to_string())?;
    state.activation.set_mode(saved.activation_mode);
    if saved.launch_at_login {
        state
            .app
            .autolaunch()
            .enable()
            .map_err(|error| error.to_string())?;
    } else {
        state
            .app
            .autolaunch()
            .disable()
            .map_err(|error| error.to_string())?;
    }
    if saved.show_bar {
        if let Some(bar) = state.app.get_webview_window("bar") {
            let _ = bar.show();
        }
    } else if let Some(bar) = state.app.get_webview_window("bar") {
        let _ = bar.hide();
    }
    Ok(saved)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<RuntimeStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let status = state.runtime.refresh();
    state.emit_runtime_status(status.clone());
    Ok(status)
}

#[tauri::command]
pub fn get_setup_artifacts(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SetupArtifact>, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.runtime.setup_artifacts())
}

#[tauri::command]
pub fn get_session_snapshot(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SessionSnapshot, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let mut snapshot = state.snapshot.clone();
    snapshot.duration_ms = state.duration_ms();
    Ok(snapshot)
}

#[tauri::command]
pub fn list_audio_devices(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<crate::audio::AudioDeviceInfo>, String> {
    let audio = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .audio
        .clone();
    audio.list_devices().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_dictation(
    source: Option<ActivationSource>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    start_session(
        state.inner().clone(),
        source.unwrap_or(ActivationSource::Bar),
    )
}

#[tauri::command]
pub fn stop_dictation(
    source: Option<ActivationSource>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    stop_session(
        state.inner().clone(),
        source.unwrap_or(ActivationSource::Bar),
    )
}

#[tauri::command]
pub fn cancel_dictation(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    cancel_session(state.inner().clone())
}

#[tauri::command]
pub fn copy_last_transcript(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<String>, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let Some(text) = state.last_transcript.clone() else {
        return Ok(None);
    };
    state
        .injector
        .copy_text(&text)
        .map_err(|error| error.to_string())?;
    Ok(Some(text))
}

#[tauri::command]
pub fn has_openai_credential(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.router.credentials().has_openai_key())
}

#[tauri::command]
pub fn save_openai_credential(
    secret: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    state
        .router
        .credentials()
        .set_openai_key(&secret)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_openai_credential(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    state
        .router
        .credentials()
        .clear_openai_key()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_settings(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    show_settings_window(&state.app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hide_bar(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    if let Some(bar) = state.app.get_webview_window("bar") {
        bar.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn quit_lyra(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let app = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .app
        .clone();
    app.exit(0);
    Ok(())
}

fn show_settings_window(app: &AppHandle) -> Result<(), String> {
    let settings = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_owned())?;
    settings.show().map_err(|error| error.to_string())?;
    settings.set_focus().map_err(|error| error.to_string())?;
    Ok(())
}

fn start_session(state: Arc<Mutex<AppState>>, _source: ActivationSource) -> Result<(), String> {
    let target = capture_foreground_target().map_err(|error| error.to_string())?;
    let app = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if !matches!(
            state.session.state(),
            DictationState::Idle | DictationState::Success | DictationState::Error
        ) {
            return Ok(());
        }
        if state.session.state() != DictationState::Idle {
            state
                .session
                .transition(SessionEvent::Reset)
                .map_err(|error| error.to_string())?;
        }
        state
            .session
            .transition(SessionEvent::Start)
            .map_err(|error| error.to_string())?;
        state.snapshot = SessionSnapshot {
            state: DictationState::Listening,
            duration_ms: 0,
            input_level: 0.0,
            request_id: Some(Uuid::new_v4().to_string()),
            error: None,
            transcript: None,
            backend: None,
            fallback_reason: None,
        };
        state.started_at = Some(Instant::now());
        let app = state.app.clone();
        let level_app = app.clone();
        let on_level = Arc::new(move |level: f32| {
            let _ = level_app.emit("lyra://input-level", level);
        });
        let audio = state.audio.clone();
        let settings = state.settings.get();
        let capture = match audio.start(
            settings.audio_device_id,
            settings.max_duration_seconds,
            on_level,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                state.session.transition(SessionEvent::Failure).ok();
                state.snapshot.state = DictationState::Error;
                state.snapshot.error = Some(error.to_string());
                state.started_at = None;
                state.emit_state();
                return Err(error.to_string());
            }
        };
        state.active_capture = Some(capture);
        state.target = Some(target);
        if let Some(bar) = state.app.get_webview_window("bar") {
            if let Err(error) = place_bar_on_target_monitor(&bar, target) {
                warn!(error = %error, "could not place dictation bar on target monitor");
            }
        }
        state.emit_state();
        app
    };
    info!("dictation session started");
    let _ = app.emit("lyra://session-started", true);
    Ok(())
}

fn stop_session(state: Arc<Mutex<AppState>>, _source: ActivationSource) -> Result<(), String> {
    let (capture, target, request_id, settings, router, injector, app) = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.session.state() != DictationState::Listening {
            return Ok(());
        }
        state
            .session
            .transition(SessionEvent::Stop)
            .map_err(|error| error.to_string())?;
        state.snapshot.state = DictationState::Processing;
        state.snapshot.input_level = 0.0;
        let capture = state
            .active_capture
            .take()
            .ok_or_else(|| "recording handle unavailable".to_owned())?;
        let target = state.target.take();
        let request_id = state
            .snapshot
            .request_id
            .clone()
            .ok_or_else(|| "request id unavailable".to_owned())?;
        let settings = state.settings.get();
        let router = state.router.clone();
        let injector = state.injector.clone();
        let app = state.app.clone();
        state.emit_state();
        (capture, target, request_id, settings, router, injector, app)
    };
    let state_for_worker = state.clone();
    thread::Builder::new()
        .name("lyra-transcription".to_owned())
        .spawn(move || {
            let captured = match capture.stop() {
                Ok(captured) => captured,
                Err(error) => {
                    finish_error(&state_for_worker, &request_id, error);
                    return;
                }
            };
            process_capture(
                state_for_worker,
                request_id,
                settings,
                router,
                injector,
                target,
                app,
                captured,
            );
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn cancel_session(state: Arc<Mutex<AppState>>) -> Result<(), String> {
    let capture = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.session.state() != DictationState::Listening {
            return Ok(());
        }
        state
            .session
            .transition(SessionEvent::Cancel)
            .map_err(|error| error.to_string())?;
        state.snapshot = SessionSnapshot::default();
        state.started_at = None;
        state.target = None;
        let capture = state.active_capture.take();
        state.emit_state();
        capture
    };
    if let Some(capture) = capture {
        thread::spawn(move || {
            let _ = capture.cancel();
        });
    }
    Ok(())
}

fn process_capture(
    state: Arc<Mutex<AppState>>,
    request_id: String,
    settings: Settings,
    router: TranscriptionRouter,
    injector: Arc<dyn TextInjector>,
    target: Option<TargetContext>,
    app: AppHandle,
    captured: CapturedAudio,
) {
    let captured_duration_ms = captured.duration_ms;
    let request = TranscriptionRequest {
        audio_samples: captured.samples,
        sample_rate: captured.sample_rate,
        language: settings.language,
        request_id: request_id.clone(),
    };
    if captured.limit_reached {
        info!(request_id = %request_id, "maximum dictation duration reached; processing captured audio");
    }
    let mut result = match router.transcribe(&request, settings.route) {
        Ok(result) => result,
        Err(error) => {
            finish_error(&state, &request_id, error);
            return;
        }
    };
    result.audio_duration_ms = captured_duration_ms;
    result.text = ConservativePostProcessor.process(&result.text, None);
    if result.text.is_empty() {
        finish_error(&state, &request_id, BackendError::EmptyRecording);
        return;
    }
    let mut injection_error = None;
    if settings.auto_paste {
        match target {
            Some(target) => {
                if let Err(error) = injector.paste_into_target(target, &result.text) {
                    injection_error = Some(error);
                }
            }
            None => injection_error = Some(BackendError::DeviceFailure(
                "the original target window was unavailable; transcript copied for manual paste"
                    .to_owned(),
            )),
        }
    } else if let Err(error) = injector.copy_text(&result.text) {
        injection_error = Some(error);
    }
    if let Ok(mut state) = state.lock() {
        if state.snapshot.request_id.as_deref() != Some(request_id.as_str()) {
            return;
        }
        state.last_transcript = Some(result.text.clone());
        state.last_result = Some(result.clone());
        let _ = app.emit("lyra://transcription-result", result.clone());
        if let Some(error) = injection_error {
            state.session.transition(SessionEvent::Failure).ok();
            state.snapshot.state = DictationState::Error;
            state.snapshot.error = Some(format!("{}; Copied — paste manually", error));
            state.snapshot.transcript = Some(result.text);
            state.snapshot.backend = Some(result.backend);
            state.snapshot.fallback_reason = result.fallback_reason;
        } else {
            state.session.transition(SessionEvent::Success).ok();
            state.snapshot.state = DictationState::Success;
            state.snapshot.error = None;
            state.snapshot.transcript = Some(result.text);
            state.snapshot.backend = Some(result.backend);
            state.snapshot.fallback_reason = result.fallback_reason;
        }
        state.snapshot.duration_ms = state
            .started_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(result.audio_duration_ms);
        state.started_at = None;
        state.emit_state();
    }
}

fn finish_error(state: &Arc<Mutex<AppState>>, request_id: &str, error: BackendError) {
    if let Ok(mut state) = state.lock() {
        if state.snapshot.request_id.as_deref() != Some(request_id) {
            return;
        }
        state.session.transition(SessionEvent::Failure).ok();
        state.snapshot.state = DictationState::Error;
        state.snapshot.error = Some(error.to_string());
        state.snapshot.input_level = 0.0;
        state.started_at = None;
        state.emit_state();
    }
}
