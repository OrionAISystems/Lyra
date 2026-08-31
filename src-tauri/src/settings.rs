use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::{info, warn};

use crate::domain::{ActivationMode, LanguagePreference, TranscriptionRoute};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

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
    #[serde(default = "default_true")]
    pub keep_model_warm: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub onboarding_complete: bool,
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
            keep_model_warm: true,
            theme: default_theme(),
            onboarding_complete: false,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.schema_version = CURRENT_SCHEMA_VERSION;
        self.max_duration_seconds = self.max_duration_seconds.clamp(1, 600);
        if self.hotkey.trim().is_empty() {
            self.hotkey = default_hotkey();
        }
        if self.theme != "system" && self.theme != "dark" {
            self.theme = default_theme();
        }
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
            Ok(contents) => match serde_json::from_str::<Settings>(&contents) {
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
    fn defaults_match_v1_product_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.hotkey, "Ctrl+Alt+Space");
        assert_eq!(settings.activation_mode, ActivationMode::PushToTalk);
        assert_eq!(settings.route, TranscriptionRoute::LocalFirstCloudFallback);
        assert!(settings.show_bar && settings.auto_paste && settings.keep_model_warm);
    }

    #[test]
    fn settings_normalize_bounds_and_invalid_values() {
        let mut settings = Settings::default();
        settings.max_duration_seconds = 9999;
        settings.hotkey = " ".to_owned();
        settings.theme = "neon".to_owned();
        let normalized = settings.normalized();
        assert_eq!(normalized.max_duration_seconds, 600);
        assert_eq!(normalized.hotkey, "Ctrl+Alt+Space");
        assert_eq!(normalized.theme, "dark");
    }

    #[test]
    fn store_writes_and_reads_a_versioned_shape() {
        let store = SettingsStore::in_memory(Settings::default());
        let value = serde_json::to_value(store.get()).unwrap();
        assert_eq!(value["schemaVersion"], CURRENT_SCHEMA_VERSION);
        assert!(value.get("openaiApiKey").is_none());
    }
}
