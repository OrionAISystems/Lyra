use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    audio::{AudioCapture, CaptureControl, CaptureOptions, CapturedAudio},
    clipboard::{ClipboardService, TextInjector},
    domain::{
        ActivationAction, ActivationController, ActivationSource, BackendError, CaptureEndpoint,
        DictationState, DictationStateMachine, SessionEvent, SessionSnapshot, ShortcutEdge,
        TextProcessingOptions, TranscriptionRequest, TranscriptionResult,
    },
    runtime::{RuntimeManager, RuntimeStatus, SetupArtifact, SetupArtifactKind, SetupProgress},
    settings::{BarPlacement, Settings, SettingsStore},
    transcription::{ModelRuntimeStatus, TranscriptionRouter},
    tray::TrayController,
    updater::{UpdateManager, UpdateStatus},
    usage::{UsageStats, UsageStatsStore},
    windows::{
        capture_foreground_target, list_displays as enumerate_displays, place_bar,
        BarPlacementStatus, DisplayInfo, TargetContext,
    },
};

const TRANSIENT_ERROR_RESET_SECONDS: u64 = 4;

pub struct AppState {
    pub settings: SettingsStore,
    pub runtime: RuntimeManager,
    pub audio: Arc<dyn AudioCapture>,
    pub router: TranscriptionRouter,
    pub updater: UpdateManager,
    pub injector: Arc<dyn TextInjector>,
    pub session: DictationStateMachine,
    pub snapshot: SessionSnapshot,
    pub activation: ActivationController,
    pub active_capture: Option<CaptureControl>,
    pub session_settings: Option<Settings>,
    pub target: Option<TargetContext>,
    pub last_transcript: Option<String>,
    pub last_result: Option<TranscriptionResult>,
    pub hotkey_status: HotkeyStatus,
    pub app: AppHandle,
    pub started_at: Option<Instant>,
    pub usage: UsageStatsStore,
    pub placement_status: Mutex<BarPlacementStatus>,
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
        updater: UpdateManager,
        app: AppHandle,
        usage: UsageStatsStore,
    ) -> Self {
        let settings_value = settings.get();
        let model_status_app = app.clone();
        router.set_model_status_sink(Arc::new(move |status| {
            let _ = model_status_app.emit("lyra://model-status", status);
        }));
        Self {
            settings,
            runtime,
            audio,
            router,
            updater,
            injector: Arc::new(ClipboardService),
            session: DictationStateMachine::default(),
            snapshot: SessionSnapshot::default(),
            activation: ActivationController::new(settings_value.activation_mode),
            active_capture: None,
            session_settings: None,
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
            usage,
            placement_status: Mutex::new(BarPlacementStatus {
                placement: settings_value.bar_placement,
                monitor_id: settings_value.bar_monitor_id,
                monitor_index: settings_value.bar_monitor_index,
                fallback: false,
                message: None,
            }),
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

    pub fn emit_settings(&self, settings: &Settings) {
        let _ = self.app.emit("lyra://settings", settings.clone());
    }

    pub fn emit_model_status(&self, status: ModelRuntimeStatus) {
        let _ = self.app.emit("lyra://model-status", status);
    }

    pub fn emit_placement_status(&self, status: BarPlacementStatus) {
        if let Ok(mut current) = self.placement_status.lock() {
            *current = status.clone();
        }
        let _ = self.app.emit("lyra://placement-status", status);
    }

    pub fn emit_usage(&self, stats: UsageStats) {
        let _ = self.app.emit("lyra://usage", stats);
    }

    pub fn emit_usage_from_settings(&self, settings: &Settings) {
        self.emit_usage(self.usage.snapshot(
            settings.usage_cost_estimates_enabled,
            settings.cloud_cost_per_minute_usd,
        ));
    }

    pub fn emit_usage_from_current_settings(&self) {
        let settings = self.settings.get();
        self.emit_usage_from_settings(&settings);
    }

    pub fn duration_ms(&self) -> u64 {
        self.started_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(self.snapshot.duration_ms)
    }

    pub fn shutdown(&mut self) {
        if let Some(capture) = self.active_capture.take() {
            capture.cancel();
        }
        info!("Lyra shutdown requested");
    }
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = app.state::<Arc<Mutex<AppState>>>().inner().clone();
    let hotkey_status = try_register_hotkey(app, settings);
    let tray = create_tray(app)?;
    app.manage(tray);
    let show_bar = app
        .state::<Arc<Mutex<AppState>>>()
        .inner()
        .lock()
        .map(|state| state.settings.get().show_bar)
        .unwrap_or(true);
    if let Some(bar) = app.get_webview_window("bar") {
        let startup_target = capture_foreground_target().ok();
        let (placement, monitor_id, monitor_index) = app
            .state::<Arc<Mutex<AppState>>>()
            .inner()
            .lock()
            .map(|state| {
                let settings = state.settings.get();
                (
                    settings.bar_placement,
                    settings.bar_monitor_id,
                    settings.bar_monitor_index,
                )
            })
            .unwrap_or((BarPlacement::FollowActiveWindow, None, None));
        match place_bar(
            &bar,
            placement,
            monitor_id.as_deref(),
            monitor_index,
            startup_target,
        ) {
            Ok(status) => {
                if let Ok(state) = app.state::<Arc<Mutex<AppState>>>().inner().lock() {
                    state.emit_placement_status(status);
                }
            }
            Err(error) => warn!(error = %error, "could not place dictation bar at startup"),
        }
        if show_bar {
            bar.show()?;
        }
    }
    let runtime = app.state::<Arc<Mutex<AppState>>>().inner().clone();
    if let Ok(mut state) = runtime.lock() {
        state.hotkey_status = hotkey_status;
        state.emit_hotkey_status();
        state.emit_runtime_status(state.runtime.status());
        let settings = state.settings.get();
        state
            .router
            .reconcile_model_policy(settings.model_warm_policy);
        state.emit_model_status(state.router.model_status());
        state.emit_usage(state.usage.snapshot(
            settings.usage_cost_estimates_enabled,
            settings.cloud_cost_per_minute_usd,
        ));
        state.emit_state();
    }
    schedule_update_checks(app);
    Ok(())
}

fn current_hotkey_label(state: &Arc<Mutex<AppState>>) -> String {
    state
        .lock()
        .map(|state| state.settings.get().hotkey)
        .unwrap_or_else(|_| "Ctrl+Alt+Space".to_owned())
}

fn try_register_hotkey(app: &AppHandle, state: Arc<Mutex<AppState>>) -> HotkeyStatus {
    let hotkey_label = current_hotkey_label(&state);
    match parse_shortcut(&hotkey_label) {
        Ok(shortcut) => try_register_hotkey_with_shortcut(app, state, shortcut),
        Err(error) => HotkeyStatus {
            hotkey: hotkey_label,
            registered: false,
            error: Some(error.to_string()),
        },
    }
}

fn try_register_hotkey_with_shortcut(
    app: &AppHandle,
    state: Arc<Mutex<AppState>>,
    shortcut: Shortcut,
) -> HotkeyStatus {
    let hotkey_label = current_hotkey_label(&state);
    match register_hotkey(app, state, shortcut) {
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
    }
}

fn register_hotkey(
    app: &AppHandle,
    state: Arc<Mutex<AppState>>,
    shortcut: Shortcut,
) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

fn parse_shortcut(value: &str) -> Result<Shortcut, Box<dyn std::error::Error>> {
    let shortcut = Shortcut::from_str(value.trim())?;
    if shortcut.mods.is_empty() {
        return Err("a shortcut must include at least one modifier".into());
    }
    Ok(shortcut)
}

fn canonical_shortcut(shortcut: Shortcut) -> String {
    let mut parts = Vec::new();
    if shortcut.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_owned());
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_owned());
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_owned());
    }
    if shortcut.mods.contains(Modifiers::SUPER) {
        parts.push("Super".to_owned());
    }
    parts.push(shortcut.key.to_string());
    parts.join("+")
}

fn configured_shortcut(
    state: &Arc<Mutex<AppState>>,
) -> Result<Shortcut, Box<dyn std::error::Error>> {
    let label = current_hotkey_label(state);
    parse_shortcut(&label)
}

fn create_tray(app: &AppHandle) -> Result<TrayController, Box<dyn std::error::Error>> {
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
    let update = MenuItem::with_id(app, "update", "Check for updates", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Lyra", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &copy, &separator, &update, &quit])?;
    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Lyra application icon is unavailable",
        )
    })?;
    TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
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
            "update" => {
                let available = app
                    .try_state::<Arc<Mutex<AppState>>>()
                    .and_then(|state| state.lock().ok().map(|state| state.updater.status()))
                    .is_some_and(|status| {
                        matches!(status.state, crate::updater::UpdateState::Available)
                    });
                if available {
                    let _ = show_settings_window(app);
                } else {
                    spawn_update_check(app);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(TrayController::new(update))
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
pub fn retry_hotkey(state: State<'_, Arc<Mutex<AppState>>>) -> Result<HotkeyStatus, String> {
    let shared_state = state.inner().clone();
    let app = shared_state
        .lock()
        .map(|state| state.app.clone())
        .map_err(|_| "state lock poisoned".to_owned())?;
    let status = match configured_shortcut(&shared_state) {
        Ok(shortcut) if app.global_shortcut().is_registered(shortcut) => HotkeyStatus {
            hotkey: current_hotkey_label(&shared_state),
            registered: true,
            error: None,
        },
        Ok(shortcut) => try_register_hotkey_with_shortcut(&app, shared_state.clone(), shortcut),
        Err(error) => HotkeyStatus {
            hotkey: current_hotkey_label(&shared_state),
            registered: false,
            error: Some(error.to_string()),
        },
    };
    let mut state = shared_state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?;
    state.hotkey_status = status.clone();
    state.emit_hotkey_status();
    Ok(status)
}

#[tauri::command]
pub fn save_settings(
    mut settings: Settings,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Settings, String> {
    let shared_state = state.inner().clone();
    let mut state = shared_state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?;
    let previous = state.settings.get();
    let previous_hotkey_registered = parse_shortcut(&previous.hotkey)
        .ok()
        .is_some_and(|shortcut| state.app.global_shortcut().is_registered(shortcut));
    let candidate_shortcut = if settings.hotkey.trim() != previous.hotkey.trim() {
        let candidate = parse_shortcut(&settings.hotkey).map_err(|error| {
            let message = error.to_string();
            state.hotkey_status = HotkeyStatus {
                hotkey: previous.hotkey.clone(),
                registered: previous_hotkey_registered,
                error: Some(message.clone()),
            };
            state.emit_hotkey_status();
            message
        })?;
        if let Err(error) = register_hotkey(&state.app, shared_state.clone(), candidate) {
            let message = error.to_string();
            state.hotkey_status = HotkeyStatus {
                hotkey: previous.hotkey.clone(),
                registered: configured_shortcut(&shared_state)
                    .ok()
                    .is_some_and(|old| state.app.global_shortcut().is_registered(old)),
                error: Some(message.clone()),
            };
            state.emit_hotkey_status();
            return Err(message);
        }
        Some(candidate)
    } else {
        None
    };
    if let Some(candidate) = candidate_shortcut {
        settings.hotkey = canonical_shortcut(candidate);
    }
    let saved = match state.settings.set(settings) {
        Ok(saved) => saved,
        Err(error) => {
            if let Some(candidate) = candidate_shortcut {
                let _ = state.app.global_shortcut().unregister(candidate);
                if let Ok(old) = configured_shortcut(&shared_state) {
                    let _ = register_hotkey(&state.app, shared_state.clone(), old);
                }
            }
            return Err(error.to_string());
        }
    };
    if previous.activation_mode != saved.activation_mode {
        state.activation.set_mode(saved.activation_mode);
    }
    if let Some(candidate) = candidate_shortcut {
        if let Ok(old) = parse_shortcut(&previous.hotkey) {
            if old != candidate {
                let _ = state.app.global_shortcut().unregister(old);
            }
        }
        state.hotkey_status = HotkeyStatus {
            hotkey: saved.hotkey.clone(),
            registered: true,
            error: None,
        };
        state.emit_hotkey_status();
    }
    if previous.max_duration_seconds != saved.max_duration_seconds {
        if let Some(control) = &state.active_capture {
            control.update_max_duration(saved.max_duration_seconds);
        }
    }
    state.router.reconcile_model_policy(saved.model_warm_policy);
    state.emit_model_status(state.router.model_status());
    state.emit_settings(&saved);
    state.emit_usage_from_settings(&saved);
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
pub fn get_model_runtime_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ModelRuntimeStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let status = state.router.model_status();
    state.emit_model_status(status.clone());
    Ok(status)
}

#[tauri::command]
pub fn list_displays(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<DisplayInfo>, String> {
    let bar = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .app
        .get_webview_window("bar")
        .ok_or_else(|| "dictation bar window is unavailable".to_owned())?;
    enumerate_displays(&bar).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_bar_placement_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BarPlacementStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    state
        .placement_status
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "placement status lock poisoned".to_owned())
}

#[tauri::command]
pub fn get_usage_stats(state: State<'_, Arc<Mutex<AppState>>>) -> Result<UsageStats, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let settings = state.settings.get();
    Ok(state.usage.snapshot(
        settings.usage_cost_estimates_enabled,
        settings.cloud_cost_per_minute_usd,
    ))
}

#[tauri::command]
pub fn reset_usage_stats(state: State<'_, Arc<Mutex<AppState>>>) -> Result<UsageStats, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let _ = state.usage.reset();
    let settings = state.settings.get();
    let stats = state.usage.snapshot(
        settings.usage_cost_estimates_enabled,
        settings.cloud_cost_per_minute_usd,
    );
    state.emit_usage(stats.clone());
    Ok(stats)
}

#[tauri::command]
pub fn get_setup_artifacts(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SetupArtifact>, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.runtime.setup_artifacts())
}

#[tauri::command]
pub fn get_setup_progress(state: State<'_, Arc<Mutex<AppState>>>) -> Result<SetupProgress, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.runtime.setup_progress())
}

#[tauri::command]
pub fn start_setup_artifact(
    kind: String,
    variant: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let shared_state = state.inner().clone();
    let (runtime, router, app) = {
        let state = shared_state
            .lock()
            .map_err(|_| "state lock poisoned".to_owned())?;
        (
            state.runtime.clone(),
            state.router.clone(),
            state.app.clone(),
        )
    };
    let kind = SetupArtifactKind::parse_with_variant(&kind, variant.as_deref())
        .map_err(|error| error.to_string())?;
    runtime
        .start_setup(&app, kind)
        .map_err(|error| error.to_string())?;
    let runtime_for_watch = runtime.clone();
    let app_for_watch = app.clone();
    thread::Builder::new()
        .name("lyra-model-setup-reconcile".to_owned())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(250));
            let progress = runtime_for_watch.setup_progress();
            if matches!(
                progress.state.as_str(),
                "complete" | "cancelled" | "error" | "idle"
            ) {
                if let Ok(state) = shared_state.lock() {
                    let settings = state.settings.get();
                    router.reconcile_model_policy(settings.model_warm_policy);
                    state.emit_runtime_status(runtime_for_watch.status());
                    state.emit_model_status(router.model_status());
                }
                let _ = app_for_watch.emit("lyra://setup-reconciled", true);
                break;
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn cancel_setup_artifact(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.runtime.cancel_setup())
}

#[tauri::command]
pub fn get_update_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<UpdateStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.updater.status())
}

#[tauri::command]
pub async fn check_for_updates(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<UpdateStatus, String> {
    let updater = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .updater
        .clone();
    Ok(updater.check_for_updates().await)
}

#[tauri::command]
pub async fn install_update(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<UpdateStatus, String> {
    let (updater, reservation) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        let updater = state.updater.clone();
        let reservation = updater.reserve_installation(state.session.state());
        (updater, reservation)
    };
    if let Err(status) = reservation {
        return Ok(status);
    }
    Ok(updater.install_reserved().await)
}

#[tauri::command]
pub fn defer_update(state: State<'_, Arc<Mutex<AppState>>>) -> Result<UpdateStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    Ok(state.updater.defer_update())
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

pub fn spawn_update_check(app: &AppHandle) {
    let updater = app
        .try_state::<Arc<Mutex<AppState>>>()
        .and_then(|state| state.lock().ok().map(|state| state.updater.clone()));
    if let Some(updater) = updater {
        tauri::async_runtime::spawn(async move {
            let _ = updater.check_for_updates().await;
        });
    }
}

fn schedule_update_checks(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        loop {
            spawn_update_check(&app);
            tokio::time::sleep(Duration::from_secs(60 * 60)).await;
        }
    });
}

fn start_session(state: Arc<Mutex<AppState>>, _source: ActivationSource) -> Result<(), String> {
    let state_for_error_reset = state.clone();
    let target = capture_foreground_target().map_err(|error| error.to_string())?;
    let (active_capture, app, request_id) = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.updater.installation_blocks_sessions() {
            return Err(
                "An update is being installed; new dictations are temporarily blocked".to_owned(),
            );
        }
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
        let request_id = Uuid::new_v4().to_string();
        state.snapshot = SessionSnapshot {
            state: DictationState::Listening,
            duration_ms: 0,
            input_level: 0.0,
            request_id: Some(request_id.clone()),
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
        let settings = state.settings.get();
        let capture = match state.audio.start(
            CaptureOptions {
                device_id: settings.audio_device_id.clone(),
                max_duration_seconds: settings.max_duration_seconds,
                silence_after_ms: (settings.activation_mode
                    == crate::domain::ActivationMode::Toggle)
                    .then_some(settings.auto_stop_after_silence_ms)
                    .flatten(),
            },
            on_level,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                state.session.transition(SessionEvent::Failure).ok();
                state.snapshot.state = DictationState::Error;
                state.snapshot.error = Some(error.to_string());
                state.started_at = None;
                state.session_settings = None;
                state.usage.record_failure();
                state.emit_usage_from_current_settings();
                state.emit_state();
                if let Some(request_id) = state.snapshot.request_id.clone() {
                    schedule_error_reset(&state_for_error_reset, &request_id);
                }
                return Err(error.to_string());
            }
        };
        state.session_settings = Some(settings.clone());
        state.active_capture = Some(capture.control());
        state.target = Some(target);
        state.usage.record_started();
        state.emit_usage_from_current_settings();
        place_bar_for_session(&state, target);
        state.emit_state();
        (capture, app, request_id)
    };

    let state_for_worker = state.clone();
    let capture_control = active_capture.control();
    let request_id_for_worker = request_id.clone();
    let worker = thread::Builder::new()
        .name("lyra-capture-coordinator".to_owned())
        .spawn(move || match active_capture.wait() {
            Ok(captured) => complete_capture(
                state_for_worker.clone(),
                request_id_for_worker.clone(),
                captured,
            ),
            Err(BackendError::Cancellation) => {}
            Err(error) => finish_error(&state_for_worker, &request_id_for_worker, error),
        });
    if let Err(error) = worker {
        capture_control.cancel();
        finish_error(
            &state,
            &request_id,
            BackendError::StreamFailure(error.to_string()),
        );
        return Err(error.to_string());
    }
    info!("dictation session started");
    let _ = app.emit("lyra://session-started", true);
    Ok(())
}

fn place_bar_for_session(state: &AppState, target: TargetContext) {
    let Some(bar) = state.app.get_webview_window("bar") else {
        return;
    };
    let settings = state.settings.get();
    match place_bar(
        &bar,
        settings.bar_placement,
        settings.bar_monitor_id.as_deref(),
        settings.bar_monitor_index,
        Some(target),
    ) {
        Ok(status) => state.emit_placement_status(status),
        Err(error) => warn!(error = %error, "could not place dictation bar for session"),
    }
}

fn stop_session(state: Arc<Mutex<AppState>>, _source: ActivationSource) -> Result<(), String> {
    let control = {
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
        let control = state.active_capture.take();
        state.emit_state();
        control
    };
    if let Some(control) = control {
        control.stop();
    }
    Ok(())
}

fn cancel_session(state: Arc<Mutex<AppState>>) -> Result<(), String> {
    let control = {
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
        state.session_settings = None;
        let control = state.active_capture.take();
        state.emit_state();
        control
    };
    if let Some(control) = control {
        control.cancel();
    }
    Ok(())
}

fn complete_capture(state: Arc<Mutex<AppState>>, request_id: String, captured: CapturedAudio) {
    let context = {
        let mut state = match state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if state.snapshot.request_id.as_deref() != Some(request_id.as_str())
            || !matches!(
                state.session.state(),
                DictationState::Listening | DictationState::Processing
            )
        {
            return;
        }
        if state.session.state() == DictationState::Listening {
            state.session.transition(SessionEvent::Stop).ok();
            state.snapshot.state = DictationState::Processing;
            state.snapshot.input_level = 0.0;
        }
        state.active_capture = None;
        let settings = state
            .session_settings
            .take()
            .unwrap_or_else(|| state.settings.get());
        let target = state.target.take();
        let router = state.router.clone();
        let injector = state.injector.clone();
        let app = state.app.clone();
        state.emit_state();
        (settings, router, injector, target, app)
    };
    if captured.endpoint != CaptureEndpoint::Manual {
        info!(
            request_id = %request_id,
            endpoint = ?captured.endpoint,
            duration_ms = captured.duration_ms,
            limit_reached = captured.limit_reached,
            "capture endpoint reached; processing captured audio"
        );
    }
    process_capture(
        state, request_id, context.0, context.1, context.2, context.3, context.4, captured,
    );
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
    if let Ok(state_guard) = state.lock() {
        state_guard.usage.record_capture(captured_duration_ms);
        state_guard.emit_usage_from_settings(&settings);
    }
    let request = TranscriptionRequest {
        audio_samples: captured.samples,
        sample_rate: captured.sample_rate,
        language: settings.language,
        request_id: request_id.clone(),
    };
    let mut result = match router.transcribe(&request, settings.route) {
        Ok(result) => result,
        Err(error) => {
            finish_error(&state, &request_id, error);
            return;
        }
    };
    result.audio_duration_ms = captured_duration_ms;
    result.text = crate::domain::process_transcription_text(
        &result.text,
        None,
        TextProcessingOptions {
            formatting_commands_enabled: settings.formatting_commands_enabled,
            corrections: &settings.corrections,
        },
    );
    if result.text.is_empty() {
        finish_error(&state, &request_id, BackendError::EmptyRecording);
        return;
    }
    if let Ok(state_guard) = state.lock() {
        state_guard.usage.record_result(&result);
        state_guard.emit_usage_from_settings(&settings);
    }
    let mut injection_error = None;
    let mut paste_attempted = false;
    let mut paste_succeeded = false;
    if settings.auto_paste {
        paste_attempted = true;
        if let Some(target) = target {
            match injector.paste_into_target(target, &result.text) {
                Ok(_) => paste_succeeded = true,
                Err(error) => injection_error = Some(error),
            }
        } else {
            let _ = injector.copy_text(&result.text);
            injection_error = Some(BackendError::DeviceFailure(
                "the original target window was unavailable; transcript copied for manual paste"
                    .to_owned(),
            ));
        }
    } else if let Err(error) = injector.copy_text(&result.text) {
        injection_error = Some(error);
    }
    if paste_attempted {
        if let Ok(state_guard) = state.lock() {
            state_guard.usage.record_paste(paste_succeeded);
            state_guard.emit_usage_from_settings(&settings);
        }
    }
    let has_injection_error = injection_error.is_some();
    let state_updated = if let Ok(mut state) = state.lock() {
        if state.snapshot.request_id.as_deref() != Some(request_id.as_str()) {
            return;
        }
        state.last_transcript = Some(result.text.clone());
        state.last_result = Some(result.clone());
        let _ = app.emit("lyra://transcription-result", result.clone());
        if let Some(error) = injection_error {
            state.usage.record_failure();
            state.emit_usage_from_settings(&settings);
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
        true
    } else {
        false
    };
    if state_updated && has_injection_error {
        schedule_error_reset(&state, &request_id);
    }
}

fn finish_error(state: &Arc<Mutex<AppState>>, request_id: &str, error: BackendError) {
    let state_updated = if let Ok(mut state) = state.lock() {
        if state.snapshot.request_id.as_deref() != Some(request_id) {
            return;
        }
        state.session.transition(SessionEvent::Failure).ok();
        state.snapshot.state = DictationState::Error;
        state.snapshot.error = Some(error.to_string());
        state.snapshot.input_level = 0.0;
        state.started_at = None;
        state.active_capture = None;
        state.target = None;
        state.session_settings = None;
        state.usage.record_failure();
        state.emit_usage_from_current_settings();
        state.emit_state();
        true
    } else {
        false
    };
    if state_updated {
        schedule_error_reset(state, request_id);
    }
}

fn schedule_error_reset(state: &Arc<Mutex<AppState>>, request_id: &str) {
    let state = Arc::clone(state);
    let request_id = request_id.to_owned();
    if let Err(error) = thread::Builder::new()
        .name("lyra-error-reset".to_owned())
        .spawn(move || {
            thread::sleep(Duration::from_secs(TRANSIENT_ERROR_RESET_SECONDS));
            if let Ok(mut state) = state.lock() {
                if state.snapshot.request_id.as_deref() != Some(request_id.as_str())
                    || state.session.state() != DictationState::Error
                {
                    return;
                }
                state.session.transition(SessionEvent::Reset).ok();
                state.snapshot = SessionSnapshot::default();
                state.emit_state();
                info!(request_id = %request_id, "transient error state reset to idle");
            }
        })
    {
        warn!(error = %error, "could not schedule transient error reset");
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_shortcut, parse_shortcut};

    #[test]
    fn shortcuts_require_a_modifier_and_canonicalize_key_order() {
        let shortcut = parse_shortcut("alt + ctrl + Space").expect("valid shortcut");
        assert_eq!(canonical_shortcut(shortcut), "Ctrl+Alt+Space");
        assert!(parse_shortcut("Space").is_err());
        assert!(parse_shortcut("Ctrl+Alt").is_err());
        assert!(parse_shortcut("Ctrl+Space+A").is_err());
    }
}
