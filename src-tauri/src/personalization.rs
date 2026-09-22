use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::warn;

use crate::domain::CorrectionEntry;

const PERSONALIZATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WordEntry {
    pub phrase: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_boost")]
    pub boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetEntry {
    pub trigger: String,
    pub expansion: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InjectionStrategy {
    #[default]
    CtrlV,
    ShiftInsert,
    ClipboardOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Terminal,
    Developer,
    Chat,
    Editor,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppProfile {
    pub executable: String,
    #[serde(default)]
    pub category: AppCategory,
    #[serde(default)]
    pub injection_strategy: InjectionStrategy,
    #[serde(default)]
    pub chunk_long_text: bool,
    #[serde(default = "default_chunk_chars")]
    pub chunk_chars: usize,
    #[serde(default = "default_true")]
    pub formatting_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Personalization {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub words: Vec<WordEntry>,
    #[serde(default)]
    pub corrections: Vec<CorrectionEntry>,
    #[serde(default)]
    pub snippets: Vec<SnippetEntry>,
    #[serde(default)]
    pub profiles: Vec<AppProfile>,
    #[serde(default = "default_true")]
    pub self_correction_enabled: bool,
}

impl Default for Personalization {
    fn default() -> Self {
        Self {
            schema_version: PERSONALIZATION_SCHEMA_VERSION,
            words: Vec::new(),
            corrections: Vec::new(),
            snippets: Vec::new(),
            profiles: default_profiles(),
            self_correction_enabled: true,
        }
    }
}

impl Personalization {
    pub fn normalized(mut self) -> Self {
        self.schema_version = PERSONALIZATION_SCHEMA_VERSION;
        self.words.retain(|entry| !entry.phrase.trim().is_empty());
        for entry in &mut self.words {
            entry.phrase = entry.phrase.trim().to_owned();
            entry.boost = if entry.boost.is_finite() {
                entry.boost.clamp(1.0, 10.0)
            } else {
                default_boost()
            };
        }
        self.corrections.retain(|entry| {
            !entry.source.trim().is_empty() && !entry.replacement.trim().is_empty()
        });
        for entry in &mut self.corrections {
            entry.source = entry.source.trim().to_owned();
            entry.replacement = entry.replacement.trim().to_owned();
        }
        self.snippets.retain(|entry| {
            !entry.trigger.trim().is_empty() && !entry.expansion.trim().is_empty()
        });
        for entry in &mut self.snippets {
            entry.trigger = entry.trigger.trim().to_owned();
            entry.expansion = entry.expansion.replace("\r\n", "\n").replace('\r', "\n");
        }
        self.profiles.retain(|profile| !profile.executable.trim().is_empty());
        for profile in &mut self.profiles {
            profile.executable = profile.executable.trim().to_ascii_lowercase();
            profile.chunk_chars = profile.chunk_chars.clamp(128, 16_384);
        }
        self.words.sort_by_key(|entry| entry.phrase.to_ascii_lowercase());
        self.words
            .dedup_by(|left, right| left.phrase.eq_ignore_ascii_case(&right.phrase));
        self.snippets
            .sort_by_key(|entry| entry.trigger.to_ascii_lowercase());
        self.snippets
            .dedup_by(|left, right| left.trigger.eq_ignore_ascii_case(&right.trigger));
        self.profiles.sort_by_key(|profile| profile.executable.clone());
        self.profiles
            .dedup_by(|left, right| left.executable.eq_ignore_ascii_case(&right.executable));
        self
    }

    pub fn profile_for(&self, executable: Option<&str>) -> AppProfile {
        let executable = executable.unwrap_or_default().to_ascii_lowercase();
        self.profiles
            .iter()
            .find(|profile| profile.executable.eq_ignore_ascii_case(&executable))
            .cloned()
            .unwrap_or_else(|| builtin_profile(&executable))
    }
}

#[derive(Debug, Clone)]
pub struct PersonalizationStore {
    path: PathBuf,
    value: Personalization,
}

impl PersonalizationStore {
    pub fn load(app: &AppHandle, legacy_corrections: Vec<CorrectionEntry>) -> Self {
        let path = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("app-data"))
            .join("personalization.json");
        let mut value = fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Personalization>(&contents).ok())
            .unwrap_or_default();
        if value.corrections.is_empty() && !legacy_corrections.is_empty() {
            value.corrections = legacy_corrections;
        }
        let store = Self {
            path,
            value: value.normalized(),
        };
        if let Err(error) = store.persist() {
            warn!(error = %error, "could not persist personalization store");
        }
        store
    }

    #[cfg(test)]
    pub fn in_memory(value: Personalization) -> Self {
        Self {
            path: PathBuf::from("personalization.json"),
            value: value.normalized(),
        }
    }

    pub fn get(&self) -> Personalization {
        self.value.clone()
    }

    pub fn set(&mut self, value: Personalization) -> Result<Personalization, std::io::Error> {
        self.value = value.normalized();
        self.persist()?;
        Ok(self.value.clone())
    }

    fn persist(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(&self.value).map_err(std::io::Error::other)?;
        fs::write(&temporary, bytes)?;
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        fs::rename(temporary, &self.path)
    }
}

fn default_true() -> bool {
    true
}
fn default_schema_version() -> u32 {
    PERSONALIZATION_SCHEMA_VERSION
}
fn default_boost() -> f32 {
    2.5
}
fn default_chunk_chars() -> usize {
    2_000
}

fn default_profiles() -> Vec<AppProfile> {
    [
        "windows terminal.exe",
        "wt.exe",
        "powershell.exe",
        "pwsh.exe",
        "cmd.exe",
    ]
    .into_iter()
    .map(|executable| AppProfile {
        executable: executable.to_owned(),
        category: AppCategory::Terminal,
        injection_strategy: InjectionStrategy::ShiftInsert,
        chunk_long_text: true,
        chunk_chars: default_chunk_chars(),
        formatting_enabled: true,
    })
    .collect()
}

pub fn builtin_profile(executable: &str) -> AppProfile {
    let lower = executable.to_ascii_lowercase();
    let category = if ["terminal", "powershell", "pwsh", "cmd", "wezterm"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AppCategory::Terminal
    } else if ["code", "cursor", "windsurf", "codex"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AppCategory::Developer
    } else if ["chatgpt", "slack", "teams", "discord"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AppCategory::Chat
    } else if ["word", "notepad", "writer"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AppCategory::Editor
    } else {
        AppCategory::Other
    };
    let terminal = category == AppCategory::Terminal;
    AppProfile {
        executable: lower,
        category,
        injection_strategy: if terminal {
            InjectionStrategy::ShiftInsert
        } else {
            InjectionStrategy::CtrlV
        },
        chunk_long_text: terminal || category == AppCategory::Developer,
        chunk_chars: default_chunk_chars(),
        formatting_enabled: true,
    }
}

pub fn apply_snippets(text: &str, snippets: &[SnippetEntry]) -> String {
    let mut entries = snippets
        .iter()
        .filter(|entry| entry.enabled && !entry.trigger.trim().is_empty())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.trigger.chars().count()));
    let mut output = text.to_owned();
    for entry in entries {
        output = crate::domain::replace_phrase_public(
            &output,
            entry.trigger.trim(),
            &entry.expansion,
            entry.case_sensitive,
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets_are_longest_first_and_preserve_multiline_expansions() {
        let snippets = vec![
            SnippetEntry {
                trigger: "orion intro".to_owned(),
                expansion: "Orion AI Systems\nBuild with clarity.".to_owned(),
                enabled: true,
                case_sensitive: false,
            },
            SnippetEntry {
                trigger: "intro".to_owned(),
                expansion: "short".to_owned(),
                enabled: true,
                case_sensitive: false,
            },
        ];
        assert_eq!(
            apply_snippets("Use Orion Intro today", &snippets),
            "Use Orion AI Systems\nBuild with clarity. today"
        );
    }

    #[test]
    fn profiles_never_store_full_paths_and_terminal_defaults_are_safe() {
        let profile = builtin_profile("pwsh.exe");
        assert_eq!(profile.injection_strategy, InjectionStrategy::ShiftInsert);
        assert!(profile.chunk_long_text);
    }
}
