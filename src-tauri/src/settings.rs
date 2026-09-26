use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::domain::{ActivationMode, CorrectionEntry, LanguagePreference, TranscriptionRoute};

pub const CURRENT_SCHEMA_VERSION: u32 = 4;

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
        if let Err(error) = store.persist() {
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
        self.settings = settings.normalized();
        self.persist()?;
        info!(
            schema_version = self.settings.schema_version,
            "settings updated"
        );
        Ok(self.settings.clone())
    }

    fn persist(&self) -> Result<(), SettingsError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(&self.settings)?;
        let temp_path = self.path.with_extension("json.tmp");
        fs::write(&temp_path, json)?;
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(temp_path, &self.path)?;
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
    use super::*;

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
