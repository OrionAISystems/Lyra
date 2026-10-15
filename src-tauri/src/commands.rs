use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
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
    audio::{AudioCapture, CaptureControl, CaptureOptions, CapturedAudio, InputLevelMonitor},
    benchmark::{self, BenchmarkResult, BenchmarkScore, BENCHMARK_PHRASE},
    clipboard::{ClipboardService, TextInjector},
    domain::{
        ActivationAction, ActivationController, ActivationSource, BackendError, CaptureEndpoint,
        DictationState, DictationStateMachine, SessionDestination, SessionEvent, SessionSnapshot,
        ShortcutEdge, TextProcessingOptions, TranscriptionRequest, TranscriptionResult,
    },
    personalization::{Personalization, PersonalizationStore},
    runtime::{RuntimeManager, RuntimeStatus, SetupArtifact, SetupArtifactKind, SetupProgress},
    saved_items::{display_app_name, SavedItemPage, SavedItemStore, SavedItemSummary},
    seasonal_themes::{SeasonalThemeManager, ThemeContent, ThemeStatus},
    settings::{BarPlacement, LocalEngine, Settings, SettingsStore},
    streaming::StreamingResult,
    transcription::{ModelRuntimeStatus, TranscriptionRouter},
    tray::TrayController,
    updater::{UpdateManager, UpdateStatus},
    usage::{
        diagnostic_fallback_chain, timestamp_ms, FailureCategory, SessionRecord, UsageStats,
        UsageStatsStore,
    },
    windows::{
        capture_external_foreground_target, capture_foreground_target,
        list_displays as enumerate_displays, place_bar, set_bar_menu_open, target_executable,
        target_is_alive, BarPlacementStatus, DisplayInfo, TargetContext,
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
    shortcut_recording_until: Option<Instant>,
    recorded_held_shortcuts: std::collections::HashSet<String>,
    pub active_capture: Option<CaptureControl>,
    pub microphone_test: Option<InputLevelMonitor>,
    pub benchmark_active: bool,
    pub session_settings: Option<Settings>,
    pub target: Option<TargetContext>,
    pub last_transcript: Option<String>,
    pub last_result: Option<TranscriptionResult>,
    pub hotkey_status: HotkeyStatus,
    pub app: AppHandle,
    pub started_at: Option<Instant>,
    pub usage: UsageStatsStore,
    pub personalization: PersonalizationStore,
    pub saved_items: SavedItemStore,
    pub capture_destination: Option<SessionDestination>,
    pub last_external_target: Option<TargetContext>,
    pub tracking_external_target: Arc<AtomicBool>,
    pub scratchpad_activation: ActivationController,
    pub scratchpad_hotkey_status: HotkeyStatus,
    pub placement_status: Mutex<BarPlacementStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub hotkey: String,
    pub registered: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedItemDetail {
    pub summary: SavedItemSummary,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
enum SavedItemCollection {
    History,
    Scratchpad,
}

impl SavedItemCollection {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "history" => Ok(Self::History),
            "scratchpad" => Ok(Self::Scratchpad),
            _ => Err("saved item collection is invalid".to_owned()),
        }
    }
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
        personalization: PersonalizationStore,
        saved_items: SavedItemStore,
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
            shortcut_recording_until: None,
            recorded_held_shortcuts: std::collections::HashSet::new(),
            scratchpad_activation: ActivationController::new(
                settings_value
                    .scratchpad_activation_mode
                    .unwrap_or(settings_value.activation_mode),
            ),
            active_capture: None,
            microphone_test: None,
            benchmark_active: false,
            session_settings: None,
            target: None,
            last_transcript: None,
            last_result: None,
            hotkey_status: HotkeyStatus {
                hotkey: settings_value.hotkey,
                registered: false,
                error: Some("Hotkey registration has not completed".to_owned()),
            },
            scratchpad_hotkey_status: HotkeyStatus {
                hotkey: settings_value.scratchpad_hotkey,
                registered: false,
                error: Some("Scratchpad shortcut registration has not completed".to_owned()),
            },
            app,
            started_at: None,
            usage,
            personalization,
            saved_items,
            capture_destination: None,
            last_external_target: None,
            tracking_external_target: Arc::new(AtomicBool::new(true)),
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
            settings.typing_wpm,
        ));
    }

    pub fn emit_usage_from_current_settings(&self) {
        let settings = self.settings.get();
        self.emit_usage_from_settings(&settings);
    }

    pub fn emit_personalization(&self) {
        let _ = self
            .app
            .emit("lyra://personalization", self.personalization.get());
    }

    pub fn duration_ms(&self) -> u64 {
        self.started_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(self.snapshot.duration_ms)
    }

    pub fn shutdown(&mut self) {
        self.tracking_external_target
            .store(false, Ordering::Release);
        self.activation.reset();
        self.scratchpad_activation.reset();
        self.stop_microphone_test(false);
        if let Some(capture) = self.active_capture.take() {
            capture.cancel();
        }
        self.snapshot.request_id = None;
        info!("Lyra shutdown requested");
    }

    pub(crate) fn stop_microphone_test(&mut self, notify_ui: bool) {
        if let Some(monitor) = self.microphone_test.take() {
            monitor.stop();
            let _ = self.app.emit("lyra://microphone-test-level", 0.0_f32);
            if notify_ui {
                let _ = self.app.emit("lyra://microphone-test-stopped", true);
            }
        }
    }
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = app.state::<Arc<Mutex<AppState>>>().inner().clone();
    let hotkey_status = try_register_hotkey(app, settings.clone());
    let scratchpad_hotkey_status = try_register_scratchpad_hotkey(app, settings.clone());
    if let Err(error) = register_recovery_shortcuts(app, settings) {
        warn!(error = %error, "one or more recovery shortcuts are unavailable");
    }
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
        state.scratchpad_hotkey_status = scratchpad_hotkey_status;
        let _ = state.app.emit(
            "lyra://scratchpad-hotkey-status",
            state.scratchpad_hotkey_status.clone(),
        );
        state.emit_runtime_status(state.runtime.status());
        let settings = state.settings.get();
        state
            .router
            .reconcile_model_policy(settings.model_warm_policy);
        state.emit_model_status(state.router.model_status());
        state.emit_usage(state.usage.snapshot(
            settings.usage_cost_estimates_enabled,
            settings.cloud_cost_per_minute_usd,
            settings.typing_wpm,
        ));
        state.emit_state();
    }
    track_external_foreground_target(app.clone());
    schedule_update_checks(app);
    Ok(())
}

fn track_external_foreground_target(app: AppHandle) {
    let tracker_running = app.try_state::<Arc<Mutex<AppState>>>().and_then(|state| {
        state
            .lock()
            .ok()
            .map(|state| state.tracking_external_target.clone())
    });
    let Some(tracker_running) = tracker_running else {
        return;
    };
    let tracker = thread::Builder::new()
        .name("lyra-external-target-tracker".to_owned())
        .spawn(move || {
            while tracker_running.load(Ordering::Acquire) {
                if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                    if let Some(target) = capture_external_foreground_target(std::process::id()) {
                        if let Ok(mut state) = state.lock() {
                            state.last_external_target = Some(target);
                        }
                    }
                } else {
                    break;
                }
                thread::sleep(Duration::from_millis(250));
            }
        });
    if let Err(error) = tracker {
        warn!(error = %error, "could not start external foreground target tracking");
    }
}

fn try_register_hotkey(app: &AppHandle, state: Arc<Mutex<AppState>>) -> HotkeyStatus {
    let settings = state.lock().expect("state lock poisoned").settings.get();
    let labels = dictation_shortcuts(&settings);
    let mut errors = Vec::new();
    for label in &labels {
        let result = parse_shortcut(label).and_then(|shortcut| {
            if app.global_shortcut().is_registered(shortcut) {
                return Ok(());
            }
            register_hotkey(app, state.clone(), shortcut)
        });
        if let Err(error) = result {
            warn!(shortcut = %label, error = %error, "dictation shortcut registration failed");
            errors.push(format!("{label}: {error}"));
        }
    }
    HotkeyStatus {
        hotkey: labels.join(" · "),
        registered: errors.is_empty(),
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    }
}

fn dictation_shortcuts(settings: &Settings) -> Vec<String> {
    std::iter::once(settings.hotkey.clone())
        .chain(settings.additional_hotkeys.clone())
        .collect()
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
                .map(|mut state| {
                    if intercept_shortcut_recording(&mut state, shortcut, event.state) {
                        return ActivationAction::Ignore;
                    }
                    let binding = canonical_shortcut(shortcut);
                    let settings = state.settings.get();
                    let mode = settings
                        .dictation_hotkey_modes
                        .get(&binding)
                        .copied()
                        .unwrap_or(settings.activation_mode);
                    state
                        .activation
                        .handle_binding_with_mode(&binding, mode, edge)
                })
                .unwrap_or(ActivationAction::Ignore);
            match action {
                ActivationAction::Start => {
                    match start_session(
                        handler_state.clone(),
                        ActivationSource::Hotkey,
                        SessionDestination::Dictation,
                    ) {
                        Ok(true) => {}
                        Ok(false) => reset_activation_controller(
                            &handler_state,
                            SessionDestination::Dictation,
                        ),
                        Err(error) => {
                            reset_activation_controller(
                                &handler_state,
                                SessionDestination::Dictation,
                            );
                            warn!(error = %error, "global shortcut could not start dictation");
                        }
                    }
                }
                ActivationAction::Stop => {
                    if let Err(error) = stop_session(
                        handler_state.clone(),
                        ActivationSource::Hotkey,
                        SessionDestination::Dictation,
                    ) {
                        warn!(error = %error, "global shortcut could not stop dictation");
                    }
                }
                ActivationAction::Ignore => {}
            }
            let _ = app.emit("lyra://shortcut", edge);
        })?;
    Ok(())
}

fn current_scratchpad_hotkey_label(state: &Arc<Mutex<AppState>>) -> String {
    state
        .lock()
        .map(|state| state.settings.get().scratchpad_hotkey)
        .unwrap_or_else(|_| "Ctrl+Alt+Shift+Space".to_owned())
}

fn try_register_scratchpad_hotkey(app: &AppHandle, state: Arc<Mutex<AppState>>) -> HotkeyStatus {
    let label = current_scratchpad_hotkey_label(&state);
    let status = parse_shortcut(&label).and_then(|shortcut| {
        register_scratchpad_hotkey(app, state, shortcut).map_err(|error| error.to_string().into())
    });
    match status {
        Ok(()) => HotkeyStatus {
            hotkey: canonical_shortcut(
                parse_shortcut(&label).expect("registered Scratchpad shortcut was parsed"),
            ),
            registered: true,
            error: None,
        },
        Err(error) => HotkeyStatus {
            hotkey: label,
            registered: false,
            error: Some(error.to_string()),
        },
    }
}

fn register_scratchpad_hotkey(
    app: &AppHandle,
    state: Arc<Mutex<AppState>>,
    shortcut: Shortcut,
) -> Result<(), Box<dyn std::error::Error>> {
    let handler_state = state.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_, _, event| {
            let edge = match event.state {
                ShortcutState::Pressed => ShortcutEdge::Pressed,
                ShortcutState::Released => ShortcutEdge::Released,
            };
            let action = handler_state
                .lock()
                .ok()
                .map(|mut state| {
                    if intercept_shortcut_recording(&mut state, shortcut, event.state) {
                        return ActivationAction::Ignore;
                    }
                    state
                        .scratchpad_activation
                        .handle_binding(&canonical_shortcut(shortcut), edge)
                })
                .unwrap_or(ActivationAction::Ignore);
            match action {
                ActivationAction::Start => {
                    match start_session(
                        handler_state.clone(),
                        ActivationSource::Hotkey,
                        SessionDestination::Scratchpad,
                    ) {
                        Ok(true) => {}
                        Ok(false) => reset_activation_controller(
                            &handler_state,
                            SessionDestination::Scratchpad,
                        ),
                        Err(error) => {
                            reset_activation_controller(
                                &handler_state,
                                SessionDestination::Scratchpad,
                            );
                            warn!(error = %error, "Scratchpad shortcut could not start capture");
                        }
                    }
                }
                ActivationAction::Stop => {
                    let _ = stop_session(
                        handler_state.clone(),
                        ActivationSource::Hotkey,
                        SessionDestination::Scratchpad,
                    );
                }
                ActivationAction::Ignore => {}
            }
        })?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptionPartialEvent {
    request_id: String,
    text: String,
    processed_duration_ms: Option<u64>,
    backend: String,
    model: String,
    finality: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryAction {
    PasteLast,
    CopyLast,
    Cancel,
    HandsFreeToggle,
    OpenSettings,
}

fn recovery_shortcuts(settings: &Settings) -> Vec<(RecoveryAction, String)> {
    [
        (
            RecoveryAction::PasteLast,
            settings.recovery_shortcuts.paste_last.clone(),
        ),
        (
            RecoveryAction::CopyLast,
            settings.recovery_shortcuts.copy_last.clone(),
        ),
        (
            RecoveryAction::Cancel,
            settings.recovery_shortcuts.cancel.clone(),
        ),
        (
            RecoveryAction::HandsFreeToggle,
            settings.recovery_shortcuts.hands_free_toggle.clone(),
        ),
        (
            RecoveryAction::OpenSettings,
            settings.recovery_shortcuts.open_settings.clone(),
        ),
    ]
    .into_iter()
    .filter_map(|(action, value)| {
        value
            .filter(|value| !value.trim().is_empty())
            .map(|value| (action, value))
    })
    .collect()
}

fn register_recovery_shortcuts(
    app: &AppHandle,
    shared: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = shared
        .lock()
        .map_err(|_| "state lock poisoned")?
        .settings
        .get();
    register_recovery_shortcuts_for(app, shared, &settings)
}

fn register_recovery_shortcuts_for(
    app: &AppHandle,
    shared: Arc<Mutex<AppState>>,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_shortcut_set(settings)?;
    for (action, label) in recovery_shortcuts(settings) {
        register_recovery_shortcut(app, shared.clone(), action, parse_shortcut(&label)?)?;
    }
    Ok(())
}

fn register_recovery_shortcut(
    app: &AppHandle,
    shared: Arc<Mutex<AppState>>,
    action: RecoveryAction,
    shortcut: Shortcut,
) -> Result<(), Box<dyn std::error::Error>> {
    let handler_state = shared.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if handler_state
                .lock()
                .map(|mut state| intercept_shortcut_recording(&mut state, shortcut, event.state))
                .unwrap_or(true)
            {
                return;
            }
            if event.state != ShortcutState::Pressed {
                return;
            }
            match action {
                RecoveryAction::CopyLast => {
                    if let Ok(state) = handler_state.lock() {
                        if let Some(text) = state.last_transcript.as_deref() {
                            let _ = state.injector.copy_text(text);
                        }
                    }
                }
                RecoveryAction::PasteLast => {
                    if let Ok(state) = handler_state.lock() {
                        if let (Some(text), Ok(target)) = (
                            state.last_transcript.as_deref(),
                            capture_foreground_target(),
                        ) {
                            let profile = state
                                .personalization
                                .get()
                                .profile_for(target_executable(target).as_deref());
                            let _ = state.injector.paste_with_profile(target, text, &profile);
                        }
                    }
                }
                RecoveryAction::Cancel => {
                    let _ = cancel_session(handler_state.clone());
                }
                RecoveryAction::HandsFreeToggle => {
                    let listening = handler_state
                        .lock()
                        .ok()
                        .is_some_and(|state| state.session.state() == DictationState::Listening);
                    if listening {
                        let _ = stop_session(
                            handler_state.clone(),
                            ActivationSource::Hotkey,
                            SessionDestination::Dictation,
                        );
                    } else {
                        let _ = start_session(
                            handler_state.clone(),
                            ActivationSource::Hotkey,
                            SessionDestination::Dictation,
                        );
                    }
                }
                RecoveryAction::OpenSettings => {
                    let _ = show_settings_window(app);
                }
            }
        })?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    Dictation,
    Scratchpad,
    Recovery(RecoveryAction),
}

// Keep OS registration and durable settings in one transaction. Callers supply
// the platform boundary so failure paths can be exercised without global hooks.
fn apply_shortcut_changes<B, T>(
    removed: &[B],
    added: &[B],
    mut register: impl FnMut(&B) -> Result<(), String>,
    mut unregister: impl FnMut(&B) -> Result<(), String>,
    persist: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let mut removed_count = 0;
    let mut added_count = 0;
    let result = (|| {
        for binding in removed {
            unregister(binding)?;
            removed_count += 1;
        }
        for binding in added {
            register(binding)?;
            added_count += 1;
        }
        persist()
    })();
    match result {
        Ok(saved) => Ok(saved),
        Err(error) => {
            let mut rollback_errors = Vec::new();
            for binding in added[..added_count].iter().rev() {
                if let Err(error) = unregister(binding) {
                    rollback_errors.push(error);
                }
            }
            for binding in &removed[..removed_count] {
                if let Err(error) = register(binding) {
                    rollback_errors.push(error);
                }
            }
            if rollback_errors.is_empty() {
                Err(error)
            } else {
                Err(format!(
                    "{error}; shortcut restoration failed: {}",
                    rollback_errors.join("; ")
                ))
            }
        }
    }
}

fn shortcut_bindings(settings: &Settings) -> Result<Vec<(Shortcut, ShortcutAction)>, String> {
    dictation_shortcuts(settings)
        .into_iter()
        .map(|label| (label, ShortcutAction::Dictation))
        .chain(std::iter::once((
            settings.scratchpad_hotkey.clone(),
            ShortcutAction::Scratchpad,
        )))
        .chain(
            recovery_shortcuts(settings)
                .into_iter()
                .map(|(action, label)| (label, ShortcutAction::Recovery(action))),
        )
        .map(|(label, action)| {
            parse_shortcut(&label)
                .map(|key| (key, action))
                .map_err(|e| e.to_string())
        })
        .collect()
}

fn register_binding(
    app: &AppHandle,
    shared: Arc<Mutex<AppState>>,
    binding: &(Shortcut, ShortcutAction),
) -> Result<(), Box<dyn std::error::Error>> {
    match binding.1 {
        ShortcutAction::Dictation => register_hotkey(app, shared, binding.0),
        ShortcutAction::Scratchpad => register_scratchpad_hotkey(app, shared, binding.0),
        ShortcutAction::Recovery(action) => {
            register_recovery_shortcut(app, shared, action, binding.0)
        }
    }
}

// Existing OS-registered chords may never reach the webview. Route them to the
// recorder instead of executing actions, and consume their matching release.
fn intercept_shortcut_recording(
    state: &mut AppState,
    shortcut: Shortcut,
    edge: ShortcutState,
) -> bool {
    let label = canonical_shortcut(shortcut);
    if !state
        .app
        .get_webview_window("settings")
        .is_some_and(|window| window.is_focused().unwrap_or(false))
    {
        state.shortcut_recording_until = None;
    }
    if state
        .shortcut_recording_until
        .is_some_and(|until| until > Instant::now())
    {
        if edge == ShortcutState::Pressed {
            state.recorded_held_shortcuts.insert(label.clone());
        } else if edge == ShortcutState::Released && state.recorded_held_shortcuts.remove(&label) {
            let _ = state
                .app
                .emit_to("settings", "lyra://shortcut-recorded", label);
        }
        return true;
    }
    if state.recorded_held_shortcuts.contains(&label) {
        if edge == ShortcutState::Released {
            state.recorded_held_shortcuts.remove(&label);
        }
        return true;
    }
    false
}

#[tauri::command]
pub fn set_shortcut_recording(
    recording: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned")?;
    if recording && (benchmark_is_blocked(state.session.state()) || state.benchmark_active) {
        return Err("Finish recording before editing shortcuts".to_owned());
    }
    state.shortcut_recording_until = recording.then(|| Instant::now() + Duration::from_secs(30));
    Ok(())
}

fn validate_shortcut_set(settings: &Settings) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    let recovery = recovery_shortcuts(settings);
    let labels = std::iter::once(settings.hotkey.as_str())
        .chain(settings.additional_hotkeys.iter().map(String::as_str))
        .chain(std::iter::once(settings.scratchpad_hotkey.as_str()))
        .chain(recovery.iter().map(|(_, shortcut)| shortcut.as_str()));
    for label in labels {
        let shortcut = parse_shortcut(label).map_err(|error| error.to_string())?;
        let canonical = canonical_shortcut(shortcut);
        if !seen.insert(canonical.clone()) {
            return Err(format!("shortcut {canonical} is assigned more than once"));
        }
    }
    Ok(())
}

fn parse_shortcut(value: &str) -> Result<Shortcut, Box<dyn std::error::Error>> {
    let shortcut = Shortcut::from_str(value.trim())?;
    if shortcut.mods.is_empty()
        && !matches!(
            shortcut.key.to_string().as_str(),
            "F13"
                | "F14"
                | "F15"
                | "F16"
                | "F17"
                | "F18"
                | "F19"
                | "F20"
                | "F21"
                | "F22"
                | "F23"
                | "F24"
        )
    {
        return Err("use a modifier combination or a single F13–F24 key".into());
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

fn create_tray(app: &AppHandle) -> Result<TrayController, Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let show_bar = app
        .state::<Arc<Mutex<AppState>>>()
        .lock()
        .map_err(|_| std::io::Error::other("state lock poisoned"))?
        .settings
        .get()
        .show_bar;
    let bar = MenuItem::with_id(
        app,
        "toggle-bar",
        if show_bar {
            "Hide dictation bar"
        } else {
            "Show dictation bar"
        },
        true,
        None::<&str>,
    )?;
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
    let menu = Menu::with_items(app, &[&settings, &bar, &copy, &separator, &update, &quit])?;
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
            "toggle-bar" => {
                if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                    if let Ok(mut state) = state.lock() {
                        let visible = !state.settings.get().show_bar;
                        if let Err(error) = set_bar_visibility(&mut state, visible) {
                            warn!(error = %error, "could not change dictation bar visibility");
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
    Ok(TrayController::new(update, bar))
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Settings, String> {
    state
        .lock()
        .map(|state| state.settings.get())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn get_theme_status(themes: State<'_, SeasonalThemeManager>) -> ThemeStatus {
    themes.status()
}

#[tauri::command]
pub fn get_theme_content(
    theme_id: Option<String>,
    themes: State<'_, SeasonalThemeManager>,
) -> Result<Option<ThemeContent>, String> {
    themes.content(theme_id.as_deref()).map_err(str::to_owned)
}

#[tauri::command]
pub async fn refresh_themes(
    themes: State<'_, SeasonalThemeManager>,
) -> Result<ThemeStatus, String> {
    let manager = themes.inner().clone();
    tauri::async_runtime::spawn_blocking(move || manager.refresh_now())
        .await
        .map_err(|_| "theme refresh unavailable".to_owned())
}

#[tauri::command]
pub fn get_hotkey_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<HotkeyStatus, String> {
    state
        .lock()
        .map(|state| state.hotkey_status.clone())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn get_scratchpad_hotkey_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<HotkeyStatus, String> {
    state
        .lock()
        .map(|state| state.scratchpad_hotkey_status.clone())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn retry_hotkey(state: State<'_, Arc<Mutex<AppState>>>) -> Result<HotkeyStatus, String> {
    let shared_state = state.inner().clone();
    let app = shared_state
        .lock()
        .map(|state| state.app.clone())
        .map_err(|_| "state lock poisoned".to_owned())?;
    let status = try_register_hotkey(&app, shared_state.clone());
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
    themes: State<'_, SeasonalThemeManager>,
) -> Result<Settings, String> {
    let shared_state = state.inner().clone();
    let mut state = shared_state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?;
    let previous = state.settings.get();
    settings.history_last_retention = if settings.transcript_history_retention.is_enabled() {
        settings.transcript_history_retention
    } else if previous.transcript_history_retention.is_enabled() {
        previous.transcript_history_retention
    } else {
        previous.history_last_retention
    };
    if let Err(message) = validate_shortcut_set(&settings) {
        state.scratchpad_hotkey_status = HotkeyStatus {
            hotkey: previous.scratchpad_hotkey.clone(),
            registered: parse_shortcut(&previous.scratchpad_hotkey)
                .ok()
                .is_some_and(|shortcut| state.app.global_shortcut().is_registered(shortcut)),
            error: Some(message.clone()),
        };
        let _ = state.app.emit(
            "lyra://scratchpad-hotkey-status",
            state.scratchpad_hotkey_status.clone(),
        );
        return Err(message);
    }
    if benchmark_is_blocked(state.session.state())
        && (dictation_shortcuts(&previous) != dictation_shortcuts(&settings)
            || previous.dictation_hotkey_modes != settings.dictation_hotkey_modes
            || previous.scratchpad_activation_mode != settings.scratchpad_activation_mode
            || previous.scratchpad_hotkey != settings.scratchpad_hotkey
            || previous.recovery_shortcuts != settings.recovery_shortcuts)
    {
        return Err("Finish recording before changing shortcuts".to_owned());
    }
    settings.dictation_hotkey_modes = settings
        .dictation_hotkey_modes
        .iter()
        .map(|(key, mode)| {
            parse_shortcut(key)
                .map(|shortcut| (canonical_shortcut(shortcut), *mode))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<_, _>>()?;
    settings.hotkey =
        canonical_shortcut(parse_shortcut(&settings.hotkey).map_err(|e| e.to_string())?);
    settings.additional_hotkeys = settings
        .additional_hotkeys
        .iter()
        .map(|label| {
            parse_shortcut(label)
                .map(canonical_shortcut)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<_, _>>()?;
    settings.scratchpad_hotkey =
        canonical_shortcut(parse_shortcut(&settings.scratchpad_hotkey).map_err(|e| e.to_string())?);
    let old_bindings = shortcut_bindings(&previous)?;
    let new_bindings = shortcut_bindings(&settings)?;
    // Re-register only changed assignments. Roll back every mutation if registration or persistence fails.
    let removed: Vec<_> = old_bindings
        .iter()
        .filter(|binding| !new_bindings.contains(binding))
        .cloned()
        .collect();
    let added: Vec<_> = new_bindings
        .iter()
        .filter(|binding| !old_bindings.contains(binding))
        .cloned()
        .collect();
    let app = state.app.clone();
    let result = apply_shortcut_changes(
        &removed,
        &added,
        |binding| {
            register_binding(&app, shared_state.clone(), binding)
                .map_err(|error| format!("{}: {error}", canonical_shortcut(binding.0)))
        },
        |binding| {
            app.global_shortcut()
                .unregister(binding.0)
                .map_err(|error| error.to_string())
        },
        || {
            state
                .settings
                .set(settings.clone())
                .map_err(|error| error.to_string())
        },
    );
    let saved = match result {
        Ok(saved) => saved,
        Err(error) => {
            let message = error;
            warn!(error = %message, "shortcut assignment change failed; previous settings preserved");
            state.hotkey_status.error = Some(message.clone());
            state.hotkey_status.registered = old_bindings
                .iter()
                .all(|(key, _)| state.app.global_shortcut().is_registered(*key));
            state.emit_hotkey_status();
            return Err(message);
        }
    };
    if !removed.is_empty() || !added.is_empty() {
        info!(
            dictation_bindings = saved.additional_hotkeys.len() + 1,
            "shortcut assignments updated"
        );
    }
    themes.update_preferences(saved.seasonal_themes_enabled, saved.seasonal_motion_enabled);
    if previous.activation_mode != saved.activation_mode {
        state.activation.set_mode(saved.activation_mode);
    }
    state.scratchpad_activation.set_mode(
        saved
            .scratchpad_activation_mode
            .unwrap_or(saved.activation_mode),
    );
    state.hotkey_status = HotkeyStatus {
        hotkey: dictation_shortcuts(&saved).join(" · "),
        registered: dictation_shortcuts(&saved).iter().all(|label| {
            parse_shortcut(label).is_ok_and(|key| state.app.global_shortcut().is_registered(key))
        }),
        error: None,
    };
    state.emit_hotkey_status();
    state.scratchpad_hotkey_status = HotkeyStatus {
        hotkey: saved.scratchpad_hotkey.clone(),
        registered: parse_shortcut(&saved.scratchpad_hotkey)
            .is_ok_and(|key| state.app.global_shortcut().is_registered(key)),
        error: None,
    };
    let _ = state.app.emit(
        "lyra://scratchpad-hotkey-status",
        state.scratchpad_hotkey_status.clone(),
    );
    if previous.max_duration_seconds != saved.max_duration_seconds {
        if let Some(control) = &state.active_capture {
            control.update_max_duration(saved.max_duration_seconds);
        }
    }
    state.router.reconcile_model_policy(saved.model_warm_policy);
    state.emit_model_status(state.router.model_status());
    state.emit_settings(&saved);
    state.emit_usage_from_settings(&saved);
    if previous.transcript_history_retention != saved.transcript_history_retention
        || previous.history_last_retention != saved.history_last_retention
    {
        if let Err(error) = state.saved_items.prune_history(
            saved.transcript_history_retention,
            saved.history_last_retention,
        ) {
            warn!(error = %error, "could not apply transcript history retention update");
        }
    }
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
        let _ = set_bar_menu_open(&bar, false);
        let _ = bar.hide();
    }
    if let Some(tray) = state.app.try_state::<TrayController>() {
        tray.set_bar_visibility(saved.show_bar);
    }
    Ok(saved)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<RuntimeStatus, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    // Runtime discovery is intentionally cached. Re-probing hardware here made
    // every renderer refresh launch nvidia-smi, and a session state change can
    // be observed by more than one Lyra window.
    let status = state.runtime.status();
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
        settings.typing_wpm,
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
        settings.typing_wpm,
    );
    state.emit_usage(stats.clone());
    Ok(stats)
}

#[tauri::command]
pub fn get_personalization(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Personalization, String> {
    state
        .lock()
        .map(|state| state.personalization.get())
        .map_err(|_| "state lock poisoned".to_owned())
}

#[tauri::command]
pub fn save_personalization(
    personalization: Personalization,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Personalization, String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    let saved = state
        .personalization
        .set(personalization)
        .map_err(|error| error.to_string())?;
    state.emit_personalization();
    Ok(saved)
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
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.benchmark_active {
            return Err("Finish the microphone benchmark before installing an update".to_owned());
        }
        state.stop_microphone_test(true);
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
pub fn start_microphone_test(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    if state.updater.installation_blocks_sessions() {
        return Err(
            "An update is being installed; microphone testing is temporarily blocked".to_owned(),
        );
    }
    if state.benchmark_active {
        return Err("Finish the microphone benchmark before starting a microphone test".to_owned());
    }
    if benchmark_is_blocked(state.session.state()) {
        return Err("Finish the active dictation before testing the microphone".to_owned());
    }
    if state.microphone_test.is_some() {
        return Ok(());
    }

    let settings = state.settings.get();
    let level_app = state.app.clone();
    let error_app = state.app.clone();
    let monitor = state
        .audio
        .start_input_level_monitor(
            settings.audio_device_id,
            Arc::new(move |level| {
                let _ = level_app.emit("lyra://microphone-test-level", level);
            }),
            Arc::new(move |error| {
                let _ = error_app.emit("lyra://microphone-test-error", error);
            }),
        )
        .map_err(|error| error.to_string())?;
    state.microphone_test = Some(monitor);
    Ok(())
}

#[tauri::command]
pub fn stop_microphone_test(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    state.stop_microphone_test(false);
    Ok(())
}

#[tauri::command]
pub fn get_benchmark_result(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<BenchmarkResult>, String> {
    let app = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .app
        .clone();
    Ok(benchmark::load(&app))
}

#[tauri::command]
pub fn run_benchmark(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let shared = state.inner().clone();
    let (capture, router, runtime, app, microphone) = {
        let mut state = shared
            .lock()
            .map_err(|_| "state lock poisoned".to_owned())?;
        if benchmark_is_blocked(state.session.state()) {
            return Err("Finish the active dictation before benchmarking".to_owned());
        }
        if state.benchmark_active {
            return Err("A microphone benchmark is already running".to_owned());
        }
        state.stop_microphone_test(true);
        let settings = state.settings.get();
        let app = state.app.clone();
        let level_app = app.clone();
        let capture = state
            .audio
            .start(
                CaptureOptions {
                    device_id: settings.audio_device_id.clone(),
                    max_duration_seconds: 12,
                    silence_after_ms: Some(1_500),
                    adaptive_audio: true,
                },
                Arc::new(move |level| {
                    let _ = level_app.emit("lyra://benchmark-level", level);
                }),
            )
            .map_err(|error| error.to_string())?;
        state.benchmark_active = true;
        (
            capture,
            state.router.clone(),
            state.runtime.status(),
            app,
            settings.audio_device_id,
        )
    };
    let _ = app.emit("lyra://benchmark-state", "recording");
    let benchmark_control = capture.control();
    let state_for_worker = shared.clone();
    let app_for_spawn_failure = app.clone();
    let worker = thread::Builder::new()
        .name("lyra-benchmark".to_owned())
        .spawn(move || {
            let _active_reset = BenchmarkActivityReset(state_for_worker);
            let Ok(captured) = capture.wait() else {
                let _ = app.emit("lyra://benchmark-state", "error");
                return;
            };
            let _ = app.emit("lyra://benchmark-state", "comparing");
            let duration = captured.duration_ms.max(1);
            let request = TranscriptionRequest {
                audio_samples: captured.samples.clone(),
                sample_rate: captured.sample_rate,
                language: crate::domain::LanguagePreference::En,
                request_id: Uuid::new_v4().to_string(),
            };
            let mut scores = Vec::new();
            match router.transcribe(&request, crate::domain::TranscriptionRoute::LocalOnly) {
                Ok(result) => scores.push(BenchmarkScore {
                    engine: "parakeet_tdt".to_owned(),
                    runtime: runtime.runtime_variant.clone(),
                    latency_ms: result.latency_ms,
                    real_time_factor: result.transcription_duration_ms as f64 / duration as f64,
                    transcript_distance: benchmark::transcript_distance(
                        BENCHMARK_PHRASE,
                        &result.text,
                    ),
                    partial_count: 0,
                    succeeded: true,
                    failure_category: None,
                }),
                Err(error) => scores.push(BenchmarkScore {
                    engine: "parakeet_tdt".to_owned(),
                    runtime: runtime.runtime_variant.clone(),
                    latency_ms: 0,
                    real_time_factor: 0.0,
                    transcript_distance: 1.0,
                    partial_count: 0,
                    succeeded: false,
                    failure_category: Some(
                        format!("{:?}", classify_error(&error)).to_ascii_lowercase(),
                    ),
                }),
            }
            if runtime.nemotron_model_verified {
                let mut partial_count = 0;
                match router.transcribe_nemotron_streaming(&request, Vec::new(), |result| {
                    if !result.is_final() {
                        partial_count += 1;
                    }
                }) {
                    Ok(result) => scores.push(BenchmarkScore {
                        engine: "nemotron_streaming_en".to_owned(),
                        runtime: runtime.runtime_variant.clone(),
                        latency_ms: result.latency_ms,
                        real_time_factor: result.transcription_duration_ms as f64 / duration as f64,
                        transcript_distance: benchmark::transcript_distance(
                            BENCHMARK_PHRASE,
                            &result.text,
                        ),
                        partial_count,
                        succeeded: true,
                        failure_category: None,
                    }),
                    Err(error) => scores.push(BenchmarkScore {
                        engine: "nemotron_streaming_en".to_owned(),
                        runtime: runtime.runtime_variant.clone(),
                        latency_ms: 0,
                        real_time_factor: 0.0,
                        transcript_distance: 1.0,
                        partial_count,
                        succeeded: false,
                        failure_category: Some(
                            format!("{:?}", classify_error(&error)).to_ascii_lowercase(),
                        ),
                    }),
                }
            }
            let result = BenchmarkResult {
                timestamp_ms: timestamp_ms(),
                phrase_word_count: BENCHMARK_PHRASE.split_whitespace().count(),
                microphone,
                recommendation: benchmark::recommend(&scores),
                scores,
            };
            let _ = benchmark::save(&app, &result);
            let _ = app.emit("lyra://benchmark-result", result);
            let _ = app.emit("lyra://benchmark-state", "complete");
        });
    if let Err(error) = worker {
        benchmark_control.cancel();
        if let Ok(mut state) = shared.lock() {
            state.benchmark_active = false;
        }
        let _ = app_for_spawn_failure.emit("lyra://benchmark-state", "error");
        return Err(error.to_string());
    }
    Ok(())
}

struct BenchmarkActivityReset(Arc<Mutex<AppState>>);

impl Drop for BenchmarkActivityReset {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.benchmark_active = false;
        }
    }
}

fn benchmark_is_blocked(state: DictationState) -> bool {
    matches!(
        state,
        DictationState::Listening | DictationState::Processing
    )
}

#[tauri::command]
pub fn start_dictation(
    source: Option<ActivationSource>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    start_session(
        state.inner().clone(),
        source.unwrap_or(ActivationSource::Bar),
        SessionDestination::Dictation,
    )
    .map(|_| ())
}

#[tauri::command]
pub fn stop_dictation(
    source: Option<ActivationSource>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    stop_session(
        state.inner().clone(),
        source.unwrap_or(ActivationSource::Bar),
        SessionDestination::Dictation,
    )
}

#[tauri::command]
pub fn start_scratchpad(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    start_session(
        state.inner().clone(),
        ActivationSource::Bar,
        SessionDestination::Scratchpad,
    )
    .map(|_| ())
}

#[tauri::command]
pub fn stop_scratchpad(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    stop_session(
        state.inner().clone(),
        ActivationSource::Bar,
        SessionDestination::Scratchpad,
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
pub async fn list_history(
    offset: usize,
    limit: usize,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemPage, String> {
    let (store, settings) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.settings.get())
    };
    tauri::async_runtime::spawn_blocking(move || {
        store.list_history_page(
            settings.transcript_history_retention,
            settings.history_last_retention,
            offset,
            limit,
        )
    })
    .await
    .map_err(|_| "Transcript History loading task could not complete".to_owned())?
    .map_err(|_| "Transcript History could not be read from local storage".to_owned())
}

#[tauri::command]
pub fn get_history_item(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemDetail, String> {
    let (store, settings) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.settings.get())
    };
    store
        .prune_history(
            settings.transcript_history_retention,
            settings.history_last_retention,
        )
        .map_err(|_| "Transcript History retention could not be applied".to_owned())?;
    let (summary, text) = store
        .history_text(&id)
        .map_err(|_| "This History item is unavailable or could not be decrypted".to_owned())?;
    Ok(SavedItemDetail { summary, text })
}

#[tauri::command]
pub fn delete_history_item(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let (store, app) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.app.clone())
    };
    store
        .delete_history(&id)
        .map_err(|_| "This History item could not be deleted".to_owned())?;
    let _ = app.emit("lyra://saved-items-updated", true);
    Ok(())
}

#[tauri::command]
pub fn clear_history(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let (store, app) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.app.clone())
    };
    store
        .clear_history()
        .map_err(|_| "Transcript History could not be cleared".to_owned())?;
    let _ = app.emit("lyra://saved-items-updated", true);
    Ok(())
}

#[tauri::command]
pub fn promote_history_item(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemSummary, String> {
    let (store, app, settings) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (
            state.saved_items.clone(),
            state.app.clone(),
            state.settings.get(),
        )
    };
    store
        .prune_history(
            settings.transcript_history_retention,
            settings.history_last_retention,
        )
        .map_err(|_| "Transcript History retention could not be applied".to_owned())?;
    let item = store
        .promote_history(&id)
        .map_err(|_| "This History item could not be sent to Scratchpad".to_owned())?;
    let _ = app.emit("lyra://saved-items-updated", true);
    Ok(item)
}

#[tauri::command]
pub fn list_scratchpad(
    offset: usize,
    limit: usize,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemPage, String> {
    let store = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .saved_items
        .clone();
    store
        .list_scratchpad_page(offset, limit)
        .map_err(|_| "Scratchpad could not be read from local storage".to_owned())
}

#[tauri::command]
pub fn get_scratchpad_item(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemDetail, String> {
    let store = state
        .lock()
        .map_err(|_| "state lock poisoned".to_owned())?
        .saved_items
        .clone();
    let (summary, text) = store
        .scratchpad_text(&id)
        .map_err(|_| "This Scratchpad item is unavailable or could not be decrypted".to_owned())?;
    Ok(SavedItemDetail { summary, text })
}

#[tauri::command]
pub fn delete_scratchpad_item(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let (store, app) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.app.clone())
    };
    store
        .delete_scratchpad(&id)
        .map_err(|_| "This Scratchpad item could not be deleted".to_owned())?;
    let _ = app.emit("lyra://saved-items-updated", true);
    Ok(())
}

#[tauri::command]
pub fn set_scratchpad_pinned(
    id: String,
    pinned: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SavedItemSummary, String> {
    let (store, app) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (state.saved_items.clone(), state.app.clone())
    };
    let item = store
        .set_pinned(&id, pinned)
        .map_err(|_| "This Scratchpad item could not be updated".to_owned())?;
    let _ = app.emit("lyra://saved-items-updated", true);
    Ok(item)
}

#[tauri::command]
pub fn copy_saved_item(
    collection: String,
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let collection = SavedItemCollection::parse(&collection)?;
    let (store, injector, settings) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (
            state.saved_items.clone(),
            state.injector.clone(),
            state.settings.get(),
        )
    };
    if matches!(collection, SavedItemCollection::History) {
        store
            .prune_history(
                settings.transcript_history_retention,
                settings.history_last_retention,
            )
            .map_err(|_| "Transcript History retention could not be applied".to_owned())?;
    }
    let (_, text) = match collection {
        SavedItemCollection::History => store.history_text(&id),
        SavedItemCollection::Scratchpad => store.scratchpad_text(&id),
    }
    .map_err(|_| "This saved item is unavailable or could not be decrypted".to_owned())?;
    injector
        .copy_text(&text)
        .map_err(|_| "The saved text could not be copied to the clipboard".to_owned())
}

#[tauri::command]
pub fn paste_saved_item(
    collection: String,
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let collection = SavedItemCollection::parse(&collection)?;
    let (store, injector, target, personalization, settings) = {
        let state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        (
            state.saved_items.clone(),
            state.injector.clone(),
            state.last_external_target,
            state.personalization.get(),
            state.settings.get(),
        )
    };
    if matches!(collection, SavedItemCollection::History) {
        store
            .prune_history(
                settings.transcript_history_retention,
                settings.history_last_retention,
            )
            .map_err(|_| "Transcript History retention could not be applied".to_owned())?;
    }
    let (_, text) = match collection {
        SavedItemCollection::History => store.history_text(&id),
        SavedItemCollection::Scratchpad => store.scratchpad_text(&id),
    }
    .map_err(|_| "This saved item is unavailable or could not be decrypted".to_owned())?;
    let Some(target) = target.filter(|target| target_is_alive(*target)) else {
        injector.copy_text(&text).map_err(|_| {
            "No safe target was available and the text could not be copied".to_owned()
        })?;
        return Err("Text copied. No recent external app was available for safe paste.".to_owned());
    };
    let executable = target_executable(target);
    let profile = personalization.profile_for(executable.as_deref());
    match injector.paste_with_profile(target, &text, &profile) {
        Ok(outcome) if outcome.pasted => Ok(()),
        Ok(_) => {
            injector.copy_text(&text).map_err(|_| {
                "This app profile uses clipboard-only mode, but the text could not be copied"
                    .to_owned()
            })?;
            Err("Text copied. This app profile is set to clipboard only.".to_owned())
        }
        Err(_) => {
            injector
                .copy_text(&text)
                .map_err(|_| "Paste failed and the text could not be copied".to_owned())?;
            Err("Paste failed. Text copied so you can paste it manually.".to_owned())
        }
    }
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
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
    set_bar_visibility(&mut state, false)
}

fn set_bar_visibility(state: &mut AppState, visible: bool) -> Result<(), String> {
    let bar = state
        .app
        .get_webview_window("bar")
        .ok_or_else(|| "dictation bar window is unavailable".to_owned())?;
    if !visible {
        set_bar_menu_open(&bar, false).map_err(|error| error.to_string())?;
    }
    let mut settings = state.settings.get();
    settings.show_bar = visible;
    let saved = state
        .settings
        .set(settings)
        .map_err(|error| error.to_string())?;
    state.emit_settings(&saved);
    if let Some(tray) = state.app.try_state::<TrayController>() {
        tray.set_bar_visibility(visible);
    }
    if visible { bar.show() } else { bar.hide() }.map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_bar_menu_visible(app: AppHandle, open: bool) -> Result<(), String> {
    let bar = app
        .get_webview_window("bar")
        .ok_or_else(|| "dictation bar window is unavailable".to_owned())?;
    set_bar_menu_open(&bar, open).map_err(|error| error.to_string())
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

fn start_session(
    state: Arc<Mutex<AppState>>,
    source: ActivationSource,
    destination: SessionDestination,
) -> Result<bool, String> {
    let state_for_error_reset = state.clone();
    let target = match destination {
        SessionDestination::Dictation => {
            Some(capture_foreground_target().map_err(|error| error.to_string())?)
        }
        SessionDestination::Scratchpad => None,
    };
    let (active_capture, app, request_id) = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.updater.installation_blocks_sessions() {
            return Err(
                "An update is being installed; new dictations are temporarily blocked".to_owned(),
            );
        }
        if state.benchmark_active {
            return Err(
                "A microphone benchmark is running; try dictating again when it finishes"
                    .to_owned(),
            );
        }
        if !matches!(
            state.session.state(),
            DictationState::Idle | DictationState::Success | DictationState::Error
        ) {
            return Ok(false);
        }
        if state.session.state() != DictationState::Idle {
            state
                .session
                .transition(SessionEvent::Reset)
                .map_err(|error| error.to_string())?;
        }
        state.stop_microphone_test(true);
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
            backend: None,
            fallback_reason: None,
            destination: Some(destination),
        };
        state.started_at = Some(Instant::now());
        let app = state.app.clone();
        let level_app = app.clone();
        let on_level = Arc::new(move |level: f32| {
            let _ = level_app.emit("lyra://input-level", level);
        });
        let settings = state.settings.get();
        // Endpointing follows the initiating gesture, independently of bar preferences.
        let capture_mode = match (source, destination) {
            (ActivationSource::Hotkey, SessionDestination::Dictation) => state
                .activation
                .active_mode()
                .unwrap_or(crate::domain::ActivationMode::Toggle),
            (ActivationSource::Hotkey, SessionDestination::Scratchpad) => {
                state.scratchpad_activation.active_mode().unwrap_or(
                    settings
                        .scratchpad_activation_mode
                        .unwrap_or(settings.activation_mode),
                )
            }
            _ => settings.activation_mode,
        };
        let capture = match state.audio.start(
            CaptureOptions {
                device_id: settings.audio_device_id.clone(),
                max_duration_seconds: settings.max_duration_seconds,
                silence_after_ms: (capture_mode == crate::domain::ActivationMode::Toggle)
                    .then_some(settings.auto_stop_after_silence_ms)
                    .flatten(),
                adaptive_audio: settings.adaptive_audio_enabled,
            },
            on_level,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                state.session.transition(SessionEvent::Failure).ok();
                state.activation.reset();
                state.scratchpad_activation.reset();
                state.snapshot.state = DictationState::Error;
                state.snapshot.error = Some(error.to_string());
                state.started_at = None;
                state.session_settings = None;
                state.capture_destination = None;
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
        state.target = target;
        state.capture_destination = Some(destination);
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
    Ok(true)
}

fn reset_activation_controller(state: &Arc<Mutex<AppState>>, destination: SessionDestination) {
    if let Ok(mut state) = state.lock() {
        match destination {
            SessionDestination::Dictation => state.activation.reset(),
            SessionDestination::Scratchpad => state.scratchpad_activation.reset(),
        }
    }
}

fn place_bar_for_session(state: &AppState, target: Option<TargetContext>) {
    let Some(bar) = state.app.get_webview_window("bar") else {
        return;
    };
    let settings = state.settings.get();
    match place_bar(
        &bar,
        settings.bar_placement,
        settings.bar_monitor_id.as_deref(),
        settings.bar_monitor_index,
        target,
    ) {
        Ok(status) => state.emit_placement_status(status),
        Err(error) => warn!(error = %error, "could not place dictation bar for session"),
    }
}

fn stop_session(
    state: Arc<Mutex<AppState>>,
    _source: ActivationSource,
    destination: SessionDestination,
) -> Result<(), String> {
    let control = {
        let mut state = state.lock().map_err(|_| "state lock poisoned".to_owned())?;
        if state.session.state() != DictationState::Listening {
            return Ok(());
        }
        if state.capture_destination != Some(destination) {
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
        if !matches!(
            state.session.state(),
            DictationState::Listening | DictationState::Processing
        ) {
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
        state.capture_destination = None;
        state.activation.reset();
        state.scratchpad_activation.reset();
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
        let destination = state
            .capture_destination
            .take()
            .unwrap_or(SessionDestination::Dictation);
        let target = state.target.take();
        let router = state.router.clone();
        let injector = state.injector.clone();
        let app = state.app.clone();
        state.emit_state();
        (settings, router, injector, target, app, destination)
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
    info!(
        peak_level = captured.peak_level,
        clipped_samples = captured.clipped_samples,
        noise_floor = captured.noise_floor,
        "capture audio diagnostics"
    );
    process_capture(
        state, request_id, context.0, context.1, context.2, context.3, context.4, context.5,
        captured,
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
    destination: SessionDestination,
    captured: CapturedAudio,
) {
    let captured_duration_ms = captured.duration_ms;
    let endpoint = format!("{:?}", captured.endpoint).to_ascii_lowercase();
    let target_executable_name = target.and_then(target_executable);
    let personalization = state
        .lock()
        .map(|state| state.personalization.get())
        .unwrap_or_default();
    let profile = personalization.profile_for(target_executable_name.as_deref());
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
    let mut result = if settings.local_engine == LocalEngine::NemotronStreamingEn
        && settings.language == crate::domain::LanguagePreference::En
    {
        let event_app = app.clone();
        let active_request = request_id.clone();
        let state_for_event = state.clone();
        let context_words = personalization
            .words
            .iter()
            .filter(|word| word.enabled)
            .map(|word| word.phrase.clone())
            .collect::<Vec<_>>();
        match router.transcribe_nemotron_streaming(&request, context_words, move |partial| {
            let is_current = state_for_event
                .lock()
                .ok()
                .and_then(|state| state.snapshot.request_id.clone())
                .as_deref()
                == Some(active_request.as_str());
            if !is_current {
                return;
            }
            let _ = event_app.emit(
                "lyra://transcription-partial",
                TranscriptionPartialEvent {
                    request_id: active_request.clone(),
                    text: partial.text().to_owned(),
                    processed_duration_ms: partial.audio_processed_ms(),
                    backend: "local_nemotron_streaming".to_owned(),
                    model: crate::runtime::NEMOTRON_MODEL_NAME.to_owned(),
                    finality: matches!(partial, StreamingResult::Final(_)),
                },
            );
        }) {
            Ok(result) => result,
            Err(error) => {
                let streaming_failure = error.to_string();
                match router.transcribe(&request, settings.route) {
                    Ok(mut result) => {
                        result.fallback_reason = Some(streaming_failure);
                        result
                    }
                    Err(error) => {
                        finish_error(&state, &request_id, error);
                        return;
                    }
                }
            }
        }
    } else {
        match router.transcribe(&request, settings.route) {
            Ok(result) => result,
            Err(error) => {
                finish_error(&state, &request_id, error);
                return;
            }
        }
    };
    result.audio_duration_ms = captured_duration_ms;
    // A canceled/replaced request should not inspect the user's current editor.
    // The later commit guard still covers cancellation during this bounded query.
    {
        let state_guard = match state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if !request_can_commit(
            &state_guard.snapshot,
            state_guard.session.state(),
            &request_id,
        ) {
            return;
        }
    }
    let insertion_context = if destination == SessionDestination::Dictation
        && settings.auto_paste
        && settings.formatting_commands_enabled
        && profile.formatting_enabled
        && profile.injection_strategy != crate::personalization::InjectionStrategy::ClipboardOnly
    {
        target.and_then(|target| injector.insertion_context(target))
    } else {
        None
    };
    let text_options = TextProcessingOptions {
        formatting_commands_enabled: settings.formatting_commands_enabled
            && profile.formatting_enabled,
        self_correction_enabled: personalization.self_correction_enabled,
        corrections: &personalization.corrections,
        snippets: &personalization.snippets,
    };
    info!(
        context_available = insertion_context.is_some(),
        continues_sentence = insertion_context.is_some_and(|context| context.continues_sentence),
        "insertion context resolved"
    );
    result.text = crate::domain::process_transcription_text_with_context(
        &result.text,
        insertion_context,
        &personalization.words,
        text_options,
    );
    if result.text.is_empty() {
        finish_error(&state, &request_id, BackendError::EmptyRecording);
        return;
    }
    // Cancellation and replacement must win before any result side effect. Keep
    // the state lock through insertion, persistence and completion so they form
    // one commit; checking the request and then unlocking would permit a late paste.
    let mut state_guard = match state.lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    if !request_can_commit(
        &state_guard.snapshot,
        state_guard.session.state(),
        &request_id,
    ) {
        return;
    }
    {
        state_guard.usage.record_result(&result);
        state_guard.emit_usage_from_settings(&settings);
    }
    let mut injection_error = None;
    let mut paste_attempted = false;
    let mut paste_succeeded = false;
    if destination == SessionDestination::Dictation && settings.auto_paste {
        paste_attempted = true;
        if let Some(target) = target {
            match injector.paste_with_profile(target, &result.text, &profile) {
                Ok(outcome) => paste_succeeded = outcome.pasted,
                Err(error) => injection_error = Some(error),
            }
        } else {
            let _ = injector.copy_text(&result.text);
            injection_error = Some(BackendError::DeviceFailure(
                "the original target window was unavailable; transcript copied for manual paste"
                    .to_owned(),
            ));
        }
    } else if destination == SessionDestination::Dictation {
        if let Err(error) = injector.copy_text(&result.text) {
            injection_error = Some(error);
        }
    }
    if paste_attempted {
        state_guard.usage.record_paste(paste_succeeded);
        state_guard.emit_usage_from_settings(&settings);
    }
    let timestamp = timestamp_ms();
    let metadata = SavedItemSummary {
        id: String::new(),
        timestamp_ms: timestamp,
        duration_ms: captured_duration_ms,
        word_count: result.text.split_whitespace().count() as u64,
        target_app: display_app_name(target_executable_name.as_deref()),
        target_executable: target_executable_name.clone(),
        engine: result.backend.clone(),
        backend: result.backend.clone(),
        model: result.model.clone(),
        latency_ms: result.latency_ms,
        paste_outcome: match destination {
            SessionDestination::Scratchpad => "not_applicable".to_owned(),
            SessionDestination::Dictation if paste_attempted && paste_succeeded => {
                "success".to_owned()
            }
            SessionDestination::Dictation if paste_attempted => "failed_or_clipboard".to_owned(),
            SessionDestination::Dictation if injection_error.is_some() => "copy_failed".to_owned(),
            SessionDestination::Dictation => "copied".to_owned(),
        },
        pinned: false,
    };
    let (saved_item, save_warning, scratchpad_save_error) = {
        match destination {
            SessionDestination::Dictation => {
                let current_settings = state_guard.settings.get();
                match state_guard.saved_items.save_history(
                    metadata,
                    &result.text,
                    current_settings.transcript_history_retention,
                ) {
                    Ok(item) => (item, None, None),
                    Err(_) => (
                        None,
                        Some("Lyra couldn't save this transcript to local History.".to_owned()),
                        None,
                    ),
                }
            }
            SessionDestination::Scratchpad => {
                match state_guard
                    .saved_items
                    .create_scratchpad(metadata, &result.text)
                {
                    Ok(item) => (Some(item), None, None),
                    Err(_) => (
                        None,
                        None,
                        Some("Lyra couldn't save this thought to Scratchpad.".to_owned()),
                    ),
                }
            }
        }
    };
    if save_warning.is_some() || scratchpad_save_error.is_some() {
        warn!(destination = ?destination, "recognized text could not be saved locally");
    }
    {
        state_guard.usage.record_session(SessionRecord {
            timestamp_ms: timestamp,
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            duration_ms: captured_duration_ms,
            word_count: result.text.split_whitespace().count() as u64,
            target_executable: target_executable_name,
            target_category: Some(match destination {
                SessionDestination::Dictation => {
                    format!("{:?}", profile.category).to_ascii_lowercase()
                }
                SessionDestination::Scratchpad => "scratchpad".to_owned(),
            }),
            endpoint,
            engine: format!("{:?}", settings.local_engine).to_ascii_lowercase(),
            runtime: state_guard
                .runtime
                .status()
                .runtime_variant
                .map(|variant| format!("{variant:?}").to_ascii_lowercase()),
            backend: Some(result.backend.clone()),
            fallback_chain: diagnostic_fallback_chain(
                result.fallback_reason.as_deref(),
                &result.backend,
            ),
            latency_ms: Some(result.latency_ms),
            real_time_factor: (captured_duration_ms > 0)
                .then_some(result.transcription_duration_ms as f64 / captured_duration_ms as f64),
            paste_outcome: if destination == SessionDestination::Scratchpad {
                "not_applicable"
            } else if paste_attempted {
                if paste_succeeded {
                    "success"
                } else {
                    "failed_or_clipboard"
                }
            } else {
                "copied"
            }
            .to_owned(),
            failure_category: injection_error
                .as_ref()
                .map(|_| FailureCategory::PasteInjection),
        });
    }
    let has_injection_error = injection_error.is_some();
    {
        let state = &mut *state_guard;
        state.last_transcript = Some(result.text.clone());
        state.last_result = Some(result.clone());
        state.activation.reset();
        state.scratchpad_activation.reset();
        if let Some(error) = injection_error {
            state.usage.record_failure();
            state.emit_usage_from_settings(&settings);
            state.session.transition(SessionEvent::Failure).ok();
            state.snapshot.state = DictationState::Error;
            state.snapshot.error = Some(format!(
                "{error}; The transcript remains available in Lyra for copying"
            ));
            state.snapshot.backend = Some(result.backend);
            state.snapshot.fallback_reason = result.fallback_reason;
        } else if let Some(error) = scratchpad_save_error.clone() {
            state.session.transition(SessionEvent::Failure).ok();
            state.snapshot.state = DictationState::Error;
            state.snapshot.error = Some(error);
            state.snapshot.backend = Some(result.backend);
            state.snapshot.fallback_reason = result.fallback_reason;
        } else {
            state.session.transition(SessionEvent::Success).ok();
            state.snapshot.state = DictationState::Success;
            state.snapshot.error = save_warning.clone();
            state.snapshot.backend = Some(result.backend);
            state.snapshot.fallback_reason = result.fallback_reason;
        }
        state.snapshot.duration_ms = state
            .started_at
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(result.audio_duration_ms);
        state.started_at = None;
        state.capture_destination = None;
        state.emit_state();
    }
    drop(state_guard);
    if has_injection_error {
        schedule_error_reset(&state, &request_id);
    }
    if scratchpad_save_error.is_some() {
        schedule_error_reset(&state, &request_id);
    }
    if let Some(message) = save_warning {
        let _ = app.emit("lyra://saved-item-notice", message);
    }
    if let Some(message) = scratchpad_save_error {
        let _ = app.emit("lyra://saved-item-notice", message);
    }
    if let Some(item) = saved_item {
        let _ = app.emit("lyra://saved-items-updated", true);
        if destination == SessionDestination::Scratchpad {
            let _ = show_settings_window(&app);
            let _ = app.emit("lyra://open-scratchpad", item.id);
        }
    }
}

fn request_can_commit(snapshot: &SessionSnapshot, state: DictationState, request_id: &str) -> bool {
    state == DictationState::Processing && snapshot.request_id.as_deref() == Some(request_id)
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
        state.capture_destination = None;
        state.activation.reset();
        state.scratchpad_activation.reset();
        state
            .usage
            .record_categorized_failure(classify_error(&error));
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

fn classify_error(error: &BackendError) -> FailureCategory {
    match error {
        BackendError::EmptyRecording => FailureCategory::NoSpeech,
        BackendError::MissingRuntimeModel(_) => FailureCategory::ModelRuntime,
        BackendError::LocalRecognitionFailure(_) => FailureCategory::Recognition,
        BackendError::StreamFailure(message) if message.to_ascii_lowercase().contains("queue") => {
            FailureCategory::QueueOverflow
        }
        BackendError::StreamFailure(_)
        | BackendError::DeviceFailure(_)
        | BackendError::PermissionDenied(_) => FailureCategory::Capture,
        BackendError::CloudFallbackUnavailable(_) => FailureCategory::Fallback,
        BackendError::Unsupported(message) if message.to_ascii_lowercase().contains("language") => {
            FailureCategory::UnsupportedLanguage
        }
        _ => FailureCategory::Unknown,
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
    use super::{
        apply_shortcut_changes, benchmark_is_blocked, canonical_shortcut, parse_shortcut,
        request_can_commit, validate_shortcut_set,
    };
    use crate::domain::DictationState;
    use crate::domain::{DictationStateMachine, SessionEvent, SessionSnapshot};
    use crate::settings::Settings;

    #[test]
    fn cancelled_and_replaced_transcriptions_cannot_commit_late_results() {
        let mut machine = DictationStateMachine::default();
        let mut snapshot = SessionSnapshot {
            request_id: Some("old".to_owned()),
            ..Default::default()
        };
        machine.transition(SessionEvent::Start).unwrap();
        machine.transition(SessionEvent::Stop).unwrap();
        assert!(request_can_commit(&snapshot, machine.state(), "old"));
        machine.transition(SessionEvent::Cancel).unwrap();
        snapshot = SessionSnapshot::default();
        assert!(!request_can_commit(&snapshot, machine.state(), "old"));
        machine.transition(SessionEvent::Start).unwrap();
        snapshot.request_id = Some("new".to_owned());
        machine.transition(SessionEvent::Stop).unwrap();
        assert!(!request_can_commit(&snapshot, machine.state(), "old"));
        assert!(request_can_commit(&snapshot, machine.state(), "new"));
        machine.transition(SessionEvent::Success).unwrap();
        assert!(!request_can_commit(&snapshot, machine.state(), "new"));
    }

    #[test]
    fn shortcuts_require_a_modifier_and_canonicalize_key_order() {
        let shortcut = parse_shortcut("alt + ctrl + Space").expect("valid shortcut");
        assert_eq!(canonical_shortcut(shortcut), "Ctrl+Alt+Space");
        assert!(parse_shortcut("Space").is_err());
        for key in 13..=24 {
            assert!(parse_shortcut(&format!("F{key}")).is_ok());
        }
        assert!(parse_shortcut("F12").is_err());
        assert!(parse_shortcut("A").is_err());
        assert!(parse_shortcut("Ctrl+Alt").is_err());
        assert!(parse_shortcut("Ctrl+Space+A").is_err());
    }

    #[test]
    fn shortcut_transactions_restore_old_assignments_on_registration_or_save_failure() {
        use std::cell::RefCell;
        for fail_registration in [true, false] {
            let registered = RefCell::new(vec![1, 2]);
            let result = apply_shortcut_changes(
                &[1],
                &[3, 4],
                |key| {
                    if fail_registration && *key == 4 {
                        return Err("occupied shortcut".to_owned());
                    }
                    registered.borrow_mut().push(*key);
                    Ok(())
                },
                |key| {
                    registered.borrow_mut().retain(|old| old != key);
                    Ok(())
                },
                || Err::<(), _>("disk write failed".to_owned()),
            );
            assert!(result.is_err());
            registered.borrow_mut().sort();
            assert_eq!(*registered.borrow(), vec![1, 2]);
        }
    }

    #[test]
    fn shortcut_transaction_handles_removal_failure_and_reports_restore_failure() {
        use std::cell::RefCell;
        let registered = RefCell::new(vec![1, 2]);
        let result = apply_shortcut_changes(
            &[1, 2],
            &[3],
            |key| {
                registered.borrow_mut().push(*key);
                Ok(())
            },
            |key| {
                if *key == 2 {
                    return Err("remove failed".to_owned());
                }
                registered.borrow_mut().retain(|old| old != key);
                Ok(())
            },
            || Ok(()),
        );
        assert_eq!(result, Err("remove failed".to_owned()));
        registered.borrow_mut().sort();
        assert_eq!(*registered.borrow(), vec![1, 2]);
        let result = apply_shortcut_changes(
            &[1],
            &[3],
            |_| Err("register failed".to_owned()),
            |_| Ok(()),
            || Ok(()),
        );
        assert!(result.unwrap_err().contains("restoration failed"));
    }

    #[test]
    fn shortcut_transaction_commits_successful_registration() {
        use std::cell::RefCell;
        let registered = RefCell::new(vec![1, 2]);
        let result = apply_shortcut_changes(
            &[1],
            &[3],
            |key| {
                registered.borrow_mut().push(*key);
                Ok(())
            },
            |key| {
                registered.borrow_mut().retain(|old| old != key);
                Ok(())
            },
            || Ok(42),
        );
        assert_eq!(result, Ok(42));
        assert_eq!(*registered.borrow(), vec![2, 3]);
    }

    #[test]
    fn benchmark_only_blocks_an_active_capture_or_processing_request() {
        assert!(benchmark_is_blocked(DictationState::Listening));
        assert!(benchmark_is_blocked(DictationState::Processing));
        assert!(!benchmark_is_blocked(DictationState::Idle));
        assert!(!benchmark_is_blocked(DictationState::Success));
        assert!(!benchmark_is_blocked(DictationState::Error));
    }

    #[test]
    fn shortcut_validation_detects_primary_and_recovery_conflicts() {
        let mut settings = Settings::default();
        assert!(validate_shortcut_set(&settings).is_ok());
        settings.additional_hotkeys = vec!["F20".to_owned()];
        assert!(validate_shortcut_set(&settings).is_ok());
        settings
            .additional_hotkeys
            .push("alt+ctrl+Space".to_owned());
        assert!(validate_shortcut_set(&settings).is_err());
        settings.additional_hotkeys = vec![settings.scratchpad_hotkey.clone()];
        assert!(validate_shortcut_set(&settings).is_err());
        settings.additional_hotkeys =
            vec![settings.recovery_shortcuts.open_settings.clone().unwrap()];
        assert!(validate_shortcut_set(&settings).is_err());
        settings.additional_hotkeys.clear();

        settings.scratchpad_hotkey = settings.hotkey.clone();
        assert!(validate_shortcut_set(&settings).is_err());

        settings.scratchpad_hotkey = settings
            .recovery_shortcuts
            .open_settings
            .clone()
            .expect("default settings shortcut");
        assert!(validate_shortcut_set(&settings).is_err());
    }
}
