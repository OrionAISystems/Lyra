use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::domain::{ActivationMode, CorrectionEntry, LanguagePreference, TranscriptionRoute};

pub const CURRENT_SCHEMA_VERSION: u32 = 7;

const DEFAULT_CLOUD_COST_PER_MINUTE_USD: f64 = 0.003;
const DEFAULT_TYPING_WPM: u32 = 40;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptHistoryRetention {
    Off,
    SevenDays,
    ThirtyDays,
    NinetyDays,
    OneYear,
    Forever,
}

impl Default for TranscriptHistoryRetention {
    fn default() -> Self {
        Self::Off
    }
}

impl TranscriptHistoryRetention {
    pub fn days(self) -> Option<u64> {
        match self {
            Self::Off => None,
            Self::SevenDays => Some(7),
            Self::ThirtyDays => Some(30),
            Self::NinetyDays => Some(90),
            Self::OneYear => Some(365),
            Self::Forever => Some(u64::MAX),
        }
    }

    pub fn is_enabled(self) -> bool {
        self != Self::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BarPlacement {
    FollowActiveWindow,
    PrimaryDisplay,
    FixedMonitor,
}

impl Default for BarPlacement {
    fn default() -> Self {
        Self::FollowActiveWindow
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelWarmPolicy {
    AlwaysWarm,
    #[serde(rename = "unload_after_5_minutes")]
    UnloadAfter5Minutes,
    #[serde(rename = "unload_after_15_minutes")]
    UnloadAfter15Minutes,
    #[serde(rename = "unload_after_30_minutes")]
    UnloadAfter30Minutes,
    NeverKeepWarm,
}

impl Default for ModelWarmPolicy {
    fn default() -> Self {
        Self::AlwaysWarm
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LocalEngine {
    #[default]
    ParakeetTdt,
    NemotronStreamingEn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryShortcuts {
    #[serde(default)]
    pub paste_last: Option<String>,
    #[serde(default)]
    pub copy_last: Option<String>,
    #[serde(default)]
    pub cancel: Option<String>,
    #[serde(default)]
    pub hands_free_toggle: Option<String>,
    #[serde(default = "default_open_settings_hotkey")]
    pub open_settings: Option<String>,
}

impl Default for RecoveryShortcuts {
    fn default() -> Self {
        Self {
            paste_last: None,
            copy_last: None,
            cancel: None,
            hands_free_toggle: None,
            open_settings: default_open_settings_hotkey(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub activation_mode: ActivationMode,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// Additional dictation bindings; the existing primary binding remains compatible.
    #[serde(default)]
    pub additional_hotkeys: Vec<String>,
    #[serde(default)]
    pub dictation_hotkey_modes: std::collections::BTreeMap<String, ActivationMode>,
    #[serde(default)]
    pub scratchpad_activation_mode: Option<ActivationMode>,
    #[serde(default = "default_true")]
    pub show_bar: bool,
    #[serde(default)]
    pub language: LanguagePreference,
    #[serde(default)]
    pub route: TranscriptionRoute,
    #[serde(default)]
    pub audio_device_id: Option<String>,
    #[serde(default = "default_max_duration")]
    pub max_duration_seconds: u32,
    #[serde(default = "default_true")]
    pub auto_paste: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default)]
    pub auto_stop_after_silence_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub formatting_commands_enabled: bool,
    #[serde(default = "default_corrections", skip_serializing)]
    pub corrections: Vec<CorrectionEntry>,
    #[serde(default)]
    pub recovery_shortcuts: RecoveryShortcuts,
    #[serde(default)]
    pub bar_placement: BarPlacement,
    #[serde(default)]
    pub bar_monitor_id: Option<String>,
    #[serde(default)]
    pub bar_monitor_index: Option<u32>,
    #[serde(default)]
    pub model_warm_policy: ModelWarmPolicy,
    #[serde(default)]
    pub usage_cost_estimates_enabled: bool,
    #[serde(default = "default_cloud_cost_per_minute_usd")]
    pub cloud_cost_per_minute_usd: f64,
    #[serde(default = "default_typing_wpm")]
    pub typing_wpm: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_true")]
    pub seasonal_themes_enabled: bool,
    #[serde(default = "default_true")]
    pub seasonal_motion_enabled: bool,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default = "default_true")]
    pub show_tray_icon: bool,
    #[serde(default)]
    pub local_engine: LocalEngine,
    #[serde(default = "default_true")]
    pub adaptive_audio_enabled: bool,
    #[serde(default)]
    pub onboarding_complete: bool,
    #[serde(default)]
    pub transcript_history_retention: TranscriptHistoryRetention,
    #[serde(default = "default_history_last_retention")]
    pub history_last_retention: TranscriptHistoryRetention,
    #[serde(default = "default_scratchpad_hotkey")]
    pub scratchpad_hotkey: String,
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}
fn default_hotkey() -> String {
    "Ctrl+Alt+Space".to_owned()
}
fn default_true() -> bool {
    true
}
fn default_max_duration() -> u32 {
    120
}
fn default_theme() -> String {
    "dark".to_owned()
}
fn default_accent() -> String {
    "purple".to_owned()
}
fn default_open_settings_hotkey() -> Option<String> {
    Some("Ctrl+Alt+Shift+L".to_owned())
}

fn default_cloud_cost_per_minute_usd() -> f64 {
    DEFAULT_CLOUD_COST_PER_MINUTE_USD
}
fn default_typing_wpm() -> u32 {
    DEFAULT_TYPING_WPM
}

fn default_history_last_retention() -> TranscriptHistoryRetention {
    TranscriptHistoryRetention::ThirtyDays
}

fn default_scratchpad_hotkey() -> String {
    "Ctrl+Alt+Shift+Space".to_owned()
}

fn default_corrections() -> Vec<CorrectionEntry> {
    vec![
        CorrectionEntry::new("orion ai systems", "Orion AI Systems"),
        CorrectionEntry::new("geo core", "GeoCore"),
        CorrectionEntry::new("war map", "WarMap"),
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            activation_mode: ActivationMode::PushToTalk,
            hotkey: default_hotkey(),
            additional_hotkeys: Vec::new(),
            dictation_hotkey_modes: std::collections::BTreeMap::from([(
                default_hotkey(),
                ActivationMode::PushToTalk,
            )]),
            scratchpad_activation_mode: Some(ActivationMode::PushToTalk),
            show_bar: true,
            language: LanguagePreference::Auto,
            route: TranscriptionRoute::LocalFirstCloudFallback,
            audio_device_id: None,
            max_duration_seconds: default_max_duration(),
            auto_paste: true,
            launch_at_login: false,
            auto_stop_after_silence_ms: None,
            formatting_commands_enabled: true,
            corrections: default_corrections(),
            recovery_shortcuts: RecoveryShortcuts::default(),
            bar_placement: BarPlacement::default(),
            bar_monitor_id: None,
            bar_monitor_index: None,
            model_warm_policy: ModelWarmPolicy::default(),
            usage_cost_estimates_enabled: false,
            cloud_cost_per_minute_usd: default_cloud_cost_per_minute_usd(),
            typing_wpm: default_typing_wpm(),
            theme: default_theme(),
            seasonal_themes_enabled: true,
            seasonal_motion_enabled: true,
            accent: default_accent(),
            compact_mode: false,
            show_tray_icon: true,
            local_engine: LocalEngine::default(),
            adaptive_audio_enabled: true,
            onboarding_complete: false,
            transcript_history_retention: TranscriptHistoryRetention::Off,
            history_last_retention: default_history_last_retention(),
            scratchpad_hotkey: default_scratchpad_hotkey(),
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.schema_version = CURRENT_SCHEMA_VERSION;
        if !self.history_last_retention.is_enabled() {
            self.history_last_retention = TranscriptHistoryRetention::ThirtyDays;
        }
        if self.scratchpad_hotkey.trim().is_empty() {
            self.scratchpad_hotkey = default_scratchpad_hotkey();
        }
        self.max_duration_seconds = self.max_duration_seconds.clamp(1, 600);
        if self.hotkey.trim().is_empty() {
            self.hotkey = default_hotkey();
        }
        let bindings: Vec<_> = std::iter::once(self.hotkey.clone())
            .chain(self.additional_hotkeys.clone())
            .collect();
        self.dictation_hotkey_modes
            .retain(|key, _| bindings.contains(key));
        for binding in bindings {
            self.dictation_hotkey_modes
                .entry(binding)
                .or_insert(self.activation_mode);
        }
        self.scratchpad_activation_mode
            .get_or_insert(self.activation_mode);
        if self.theme != "system" && self.theme != "dark" && self.theme != "light" {
            self.theme = default_theme();
        }
        if !matches!(self.accent.as_str(), "purple" | "blue" | "teal" | "rose") {
            self.accent = default_accent();
        }
        if !self.show_tray_icon
            && self
                .recovery_shortcuts
                .open_settings
                .as_deref()
                .is_none_or(str::is_empty)
        {
            self.show_tray_icon = true;
        }
        self.auto_stop_after_silence_ms = match self.auto_stop_after_silence_ms {
            Some(value) if matches!(value, 1_000 | 1_500 | 2_000 | 3_000) => Some(value),
            _ => None,
        };
        self.corrections.retain(|entry| {
            !entry.source.trim().is_empty() && !entry.replacement.trim().is_empty()
        });
        for entry in &mut self.corrections {
            entry.source = entry.source.trim().to_owned();
            entry.replacement = entry.replacement.trim().to_owned();
        }
        if !self.cloud_cost_per_minute_usd.is_finite() || self.cloud_cost_per_minute_usd < 0.0 {
            self.cloud_cost_per_minute_usd = default_cloud_cost_per_minute_usd();
        }
        self.cloud_cost_per_minute_usd = self.cloud_cost_per_minute_usd.min(1_000.0);
        self.typing_wpm = self.typing_wpm.clamp(10, 200);
        self
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
    settings: Settings,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> Self {
        let path = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("app-data"))
            .join("settings.json");

        let settings = match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<serde_json::Value>(&contents)
                .and_then(migrate_settings_value)
            {
                Ok(settings) => settings.normalized(),
                Err(error) => {
                    warn!(error = %error, "settings file invalid; using defaults");
                    Settings::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                warn!(error = %error, "settings file unavailable; using defaults");
                Settings::default()
            }
        };

        let store = Self { path, settings };
        if let Err(error) = store.persist(&store.settings) {
            warn!(error = %error, "could not persist initial settings");
        }
        store
    }

    #[cfg(test)]
    pub fn in_memory(settings: Settings) -> Self {
        Self {
            path: PathBuf::from("settings.json"),
            settings: settings.normalized(),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.clone()
    }

    pub fn set(&mut self, settings: Settings) -> Result<Settings, SettingsError> {
        let settings = settings.normalized();
        self.persist(&settings)?;
        self.settings = settings;
        info!(
            schema_version = self.settings.schema_version,
            "settings updated"
        );
        Ok(self.settings.clone())
    }

    fn persist(&self, settings: &Settings) -> Result<(), SettingsError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(settings)?;
        crate::saved_items::atomic_write(&self.path, &json)?;
        Ok(())
    }
}

fn migrate_settings_value(mut value: serde_json::Value) -> Result<Settings, serde_json::Error> {
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    if schema_version < 2 {
        let legacy_keep_warm = value
            .get("keepModelWarm")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "modelWarmPolicy".to_owned(),
                serde_json::Value::String(if legacy_keep_warm {
                    "always_warm".to_owned()
                } else {
                    "never_keep_warm".to_owned()
                }),
            );
            object.insert("schemaVersion".to_owned(), serde_json::Value::from(2));
            object.remove("keepModelWarm");
        }
    }
    if schema_version < CURRENT_SCHEMA_VERSION as u64 {
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "schemaVersion".to_owned(),
                serde_json::Value::from(CURRENT_SCHEMA_VERSION),
            );
        }
    }
    serde_json::from_value(value)
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    #[test]
    fn shortcut_modes_migrate_once_and_remain_independent_of_bar_mode() {
        let mut fresh = Settings::default();
        fresh.activation_mode = ActivationMode::Toggle;
        assert_eq!(
            fresh.normalized().dictation_hotkey_modes["Ctrl+Alt+Space"],
            ActivationMode::PushToTalk
        );
        let mut legacy = serde_json::to_value(Settings::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("dictationHotkeyModes");
        legacy
            .as_object_mut()
            .unwrap()
            .remove("scratchpadActivationMode");
        legacy["schemaVersion"] = serde_json::json!(6);
        legacy["activationMode"] = serde_json::json!("toggle");
        legacy["additionalHotkeys"] = serde_json::json!(["F20"]);
        let mut settings = migrate_settings_value(legacy).unwrap().normalized();
        assert_eq!(
            settings.dictation_hotkey_modes["F20"],
            ActivationMode::Toggle
        );
        assert_eq!(
            settings.dictation_hotkey_modes["Ctrl+Alt+Space"],
            ActivationMode::Toggle
        );
        assert_eq!(
            settings.scratchpad_activation_mode,
            Some(ActivationMode::Toggle)
        );
        settings.activation_mode = ActivationMode::PushToTalk;
        settings
            .dictation_hotkey_modes
            .insert("Ctrl+Alt+Space".to_owned(), ActivationMode::PushToTalk);
        let settings = settings.normalized();
        assert_eq!(
            settings.dictation_hotkey_modes["F20"],
            ActivationMode::Toggle
        );
        assert_eq!(
            settings.scratchpad_activation_mode,
            Some(ActivationMode::Toggle)
        );
        assert_eq!(
            serde_json::from_value::<Settings>(serde_json::to_value(&settings).unwrap()).unwrap(),
            settings
        );
    }
    #[test]
    fn legacy_shortcut_is_preserved_with_empty_additional_bindings() {
        let mut legacy = serde_json::to_value(Settings::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("additionalHotkeys");
        legacy["schemaVersion"] = serde_json::json!(5);
        legacy["hotkey"] = serde_json::json!("Ctrl+Alt+K");
        let settings = migrate_settings_value(legacy).unwrap();
        assert_eq!(settings.hotkey, "Ctrl+Alt+K");
        assert!(settings.additional_hotkeys.is_empty());
        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        let mut multiple = settings;
        multiple.additional_hotkeys = vec!["F20".to_owned(), "Ctrl+Shift+K".to_owned()];
        assert_eq!(
            serde_json::from_value::<Settings>(serde_json::to_value(&multiple).unwrap()).unwrap(),
            multiple
        );
    }
    use super::*;

    #[test]
    fn failed_settings_save_preserves_memory_and_existing_file() {
        let root = std::env::temp_dir().join(format!("lyra-settings-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let mut store = SettingsStore {
            path: root.join("settings.json"),
            settings: Settings::default(),
        };
        store.persist(&store.settings).unwrap();
        let before = fs::read(&store.path).unwrap();
        // Deterministic write failure: the temporary path cannot be opened as a file.
        fs::create_dir(store.path.with_extension("json.tmp")).unwrap();
        let mut candidate = store.get();
        candidate.auto_paste = false;
        assert!(store.set(candidate).is_err());
        assert!(store.get().auto_paste);
        assert_eq!(fs::read(&store.path).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn failed_settings_replacement_preserves_the_previous_file() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = std::env::temp_dir().join(format!("lyra-settings-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let mut store = SettingsStore {
            path: root.join("settings.json"),
            settings: Settings::default(),
        };
        store.persist(&store.settings).unwrap();
        let before = fs::read(&store.path).unwrap();
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&store.path)
            .unwrap();
        let mut candidate = store.get();
        candidate.auto_paste = false;
        assert!(store.set(candidate).is_err());
        assert!(store.get().auto_paste);
        drop(locked);
        assert_eq!(fs::read(&store.path).unwrap(), before);
        assert!(!store.path.with_extension("json.tmp").exists());
        store
            .set(Settings {
                auto_paste: false,
                ..Default::default()
            })
            .unwrap();
        let persisted: Settings = serde_json::from_slice(&fs::read(&store.path).unwrap()).unwrap();
        assert!(!persisted.auto_paste);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn defaults_match_v2_product_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.hotkey, "Ctrl+Alt+Space");
        assert_eq!(settings.activation_mode, ActivationMode::PushToTalk);
        assert_eq!(settings.route, TranscriptionRoute::LocalFirstCloudFallback);
        assert!(settings.show_bar && settings.auto_paste);
        assert_eq!(settings.model_warm_policy, ModelWarmPolicy::AlwaysWarm);
        assert_eq!(settings.corrections.len(), 3);
        assert!(settings.formatting_commands_enabled);
        assert!(settings.seasonal_themes_enabled);
        assert!(settings.seasonal_motion_enabled);
    }

    #[test]
    fn settings_normalize_bounds_and_invalid_values() {
        let mut settings = Settings::default();
        settings.max_duration_seconds = 9999;
        settings.hotkey = " ".to_owned();
        settings.theme = "neon".to_owned();
        settings.auto_stop_after_silence_ms = Some(900);
        settings.cloud_cost_per_minute_usd = f64::NAN;
        let normalized = settings.normalized();
        assert_eq!(normalized.max_duration_seconds, 600);
        assert_eq!(normalized.hotkey, "Ctrl+Alt+Space");
        assert_eq!(normalized.theme, "dark");
        assert_eq!(normalized.auto_stop_after_silence_ms, None);
        assert_eq!(
            normalized.cloud_cost_per_minute_usd,
            DEFAULT_CLOUD_COST_PER_MINUTE_USD
        );
    }

    #[test]
    fn v1_keep_model_warm_migrates_to_explicit_policy() {
        let legacy = serde_json::json!({
            "schemaVersion": 1,
            "activationMode": "toggle",
            "hotkey": "Ctrl+Alt+Space",
            "keepModelWarm": false,
            "maxDurationSeconds": 120
        });
        let migrated = migrate_settings_value(legacy).unwrap().normalized();
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.activation_mode, ActivationMode::Toggle);
        assert_eq!(migrated.model_warm_policy, ModelWarmPolicy::NeverKeepWarm);
        assert_eq!(migrated.corrections.len(), 3);
    }

    #[test]
    fn v2_settings_keep_user_corrections_and_policy() {
        let current = serde_json::json!({
            "schemaVersion": 2,
            "modelWarmPolicy": "unload_after_15_minutes",
            "corrections": [{"source":"Tauri","replacement":"Tauri","enabled":false,"caseSensitive":true}]
        });
        let migrated = migrate_settings_value(current).unwrap().normalized();
        assert_eq!(
            migrated.model_warm_policy,
            ModelWarmPolicy::UnloadAfter15Minutes
        );
        assert_eq!(migrated.corrections.len(), 1);
        assert!(!migrated.corrections[0].enabled);
    }

    #[test]
    fn v3_settings_migrate_with_history_off_and_preserve_shortcuts() {
        let legacy = serde_json::json!({
            "schemaVersion": 3,
            "hotkey": "Ctrl+Alt+K",
            "recoveryShortcuts": {
                "pasteLast": "Ctrl+Shift+V",
                "openSettings": "Ctrl+Alt+Shift+L"
            },
            "activationMode": "toggle",
            "autoPaste": false
        });

        let migrated = migrate_settings_value(legacy).unwrap().normalized();

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(
            migrated.transcript_history_retention,
            TranscriptHistoryRetention::Off
        );
        assert_eq!(
            migrated.history_last_retention,
            TranscriptHistoryRetention::ThirtyDays
        );
        assert_eq!(migrated.scratchpad_hotkey, default_scratchpad_hotkey());
        assert_eq!(migrated.hotkey, "Ctrl+Alt+K");
        assert_eq!(
            migrated.recovery_shortcuts.paste_last.as_deref(),
            Some("Ctrl+Shift+V")
        );
        assert_eq!(migrated.activation_mode, ActivationMode::Toggle);
        assert!(!migrated.auto_paste);
    }

    #[test]
    fn v4_settings_migrate_and_preserve_existing_preferences() {
        let legacy = serde_json::json!({
            "schemaVersion": 4,
            "hotkey": "Ctrl+Alt+K",
            "theme": "light",
            "accent": "teal",
            "autoPaste": false,
            "maxDurationSeconds": 90,
            "transcriptHistoryRetention": "thirty_days"
        });

        let migrated = migrate_settings_value(legacy).unwrap().normalized();

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.seasonal_themes_enabled);
        assert!(migrated.seasonal_motion_enabled);
        assert_eq!(migrated.hotkey, "Ctrl+Alt+K");
        assert_eq!(migrated.theme, "light");
        assert_eq!(migrated.accent, "teal");
        assert!(!migrated.auto_paste);
        assert_eq!(migrated.max_duration_seconds, 90);
        assert_eq!(
            migrated.transcript_history_retention,
            TranscriptHistoryRetention::ThirtyDays
        );
    }

    #[test]
    fn saved_seasonal_theme_preferences_survive_migration() {
        let current = serde_json::json!({
            "schemaVersion": 5,
            "seasonalThemesEnabled": false,
            "seasonalMotionEnabled": false
        });

        let migrated = migrate_settings_value(current).unwrap().normalized();

        assert!(!migrated.seasonal_themes_enabled);
        assert!(!migrated.seasonal_motion_enabled);
    }

    #[test]
    fn history_retention_off_keeps_the_last_non_off_choice() {
        let settings = Settings {
            transcript_history_retention: TranscriptHistoryRetention::Off,
            history_last_retention: TranscriptHistoryRetention::OneYear,
            ..Settings::default()
        }
        .normalized();

        assert_eq!(
            settings.transcript_history_retention,
            TranscriptHistoryRetention::Off
        );
        assert_eq!(
            settings.history_last_retention,
            TranscriptHistoryRetention::OneYear
        );
    }

    #[test]
    fn store_writes_and_reads_a_versioned_shape() {
        let store = SettingsStore::in_memory(Settings::default());
        let value = serde_json::to_value(store.get()).unwrap();
        assert_eq!(value["schemaVersion"], CURRENT_SCHEMA_VERSION);
        assert!(value.get("openaiApiKey").is_none());
    }
}
