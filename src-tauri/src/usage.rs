use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::warn;

use crate::domain::TranscriptionResult;

const STATS_SCHEMA_VERSION: u32 = 3;
const DEFAULT_RATE_USD_PER_MINUTE: f64 = 0.003;
const SESSION_RETENTION_MS: u64 = 365 * 86_400_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Capture,
    NoSpeech,
    Clipping,
    ModelRuntime,
    Recognition,
    UnsupportedLanguage,
    QueueOverflow,
    Fallback,
    TargetRecovery,
    Clipboard,
    PasteInjection,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub timestamp_ms: u64,
    pub app_version: String,
    pub duration_ms: u64,
    pub word_count: u64,
    pub target_executable: Option<String>,
    pub target_category: Option<String>,
    pub endpoint: String,
    pub engine: String,
    pub runtime: Option<String>,
    pub backend: Option<String>,
    pub fallback_chain: Vec<String>,
    pub latency_ms: Option<u64>,
    pub real_time_factor: Option<f64>,
    pub paste_outcome: String,
    pub failure_category: Option<FailureCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub day: String,
    pub sessions: u64,
    pub completed: u64,
    pub failed: u64,
    pub words: u64,
    #[serde(default)]
    pub parakeet_words: u64,
    #[serde(default)]
    pub nemotron_words: u64,
    pub duration_ms: u64,
    pub paste_attempts: u64,
    pub paste_successes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct UsageCounters {
    #[serde(default)]
    schema_version: u32,
    #[serde(default)]
    today_key: String,
    #[serde(default)]
    started_dictations_today: u64,
    #[serde(default)]
    total_started_dictations: u64,
    #[serde(default)]
    captured_audio_duration_ms: u64,
    #[serde(default)]
    completed_results: u64,
    #[serde(default)]
    total_latency_ms: u64,
    #[serde(default)]
    total_transcription_duration_ms: u64,
    #[serde(default)]
    local_result_count: u64,
    #[serde(default)]
    cloud_result_count: u64,
    #[serde(default)]
    failed_sessions: u64,
    #[serde(default)]
    paste_attempts: u64,
    #[serde(default)]
    paste_successes: u64,
    #[serde(default)]
    cloud_fallback_count: u64,
    #[serde(default)]
    local_audio_duration_ms: u64,
    #[serde(default)]
    cloud_audio_duration_ms: u64,
    #[serde(default)]
    total_words: u64,
    #[serde(default)]
    daily: Vec<DailyUsage>,
    #[serde(default)]
    sessions: Vec<SessionRecord>,
    #[serde(default)]
    failure_counts: BTreeMap<String, u64>,
    #[serde(default)]
    detailed_since: Option<String>,
}

impl Default for UsageCounters {
    fn default() -> Self {
        let day = today_key();
        Self {
            schema_version: STATS_SCHEMA_VERSION,
            today_key: day.clone(),
            started_dictations_today: 0,
            total_started_dictations: 0,
            captured_audio_duration_ms: 0,
            completed_results: 0,
            total_latency_ms: 0,
            total_transcription_duration_ms: 0,
            local_result_count: 0,
            cloud_result_count: 0,
            failed_sessions: 0,
            paste_attempts: 0,
            paste_successes: 0,
            cloud_fallback_count: 0,
            local_audio_duration_ms: 0,
            cloud_audio_duration_ms: 0,
            total_words: 0,
            daily: Vec::new(),
            sessions: Vec::new(),
            failure_counts: BTreeMap::new(),
            detailed_since: Some(day),
        }
    }
}

impl UsageCounters {
    fn normalized(self) -> Self {
        self.normalized_at(timestamp_ms())
    }

    fn normalized_at(mut self, now_ms: u64) -> Self {
        let current_day = today_key();
        if self.schema_version < STATS_SCHEMA_VERSION {
            if self.today_key.is_empty() {
                self.today_key = current_day.clone();
            }
            self.daily.push(DailyUsage {
                day: current_day.clone(),
                sessions: self.started_dictations_today,
                completed: self.completed_results,
                failed: self.failed_sessions,
                words: 0,
                parakeet_words: 0,
                nemotron_words: 0,
                duration_ms: self.captured_audio_duration_ms,
                paste_attempts: self.paste_attempts,
                paste_successes: self.paste_successes,
            });
            self.detailed_since = Some(current_day.clone());
        }
        self.schema_version = STATS_SCHEMA_VERSION;
        for entry in &mut self.daily {
            if !is_iso_day(&entry.day) {
                entry.day = current_day.clone();
            }
            if entry.parakeet_words == 0 && entry.nemotron_words == 0 && entry.words > 0 {
                entry.parakeet_words = entry.words;
            }
        }
        if self.today_key != current_day {
            self.today_key = current_day;
            self.started_dictations_today = 0;
        }
        self.daily.sort_by(|left, right| left.day.cmp(&right.day));
        self.daily.dedup_by(|left, right| {
            if left.day != right.day {
                return false;
            }
            left.sessions += right.sessions;
            left.completed += right.completed;
            left.failed += right.failed;
            left.words += right.words;
            left.parakeet_words += right.parakeet_words;
            left.nemotron_words += right.nemotron_words;
            left.duration_ms += right.duration_ms;
            left.paste_attempts += right.paste_attempts;
            left.paste_successes += right.paste_successes;
            true
        });
        let session_cutoff = now_ms.saturating_sub(SESSION_RETENTION_MS);
        self.sessions
            .retain(|session| session.timestamp_ms >= session_cutoff);
        self
    }

    fn today_mut(&mut self) -> &mut DailyUsage {
        let today = today_key();
        if self.daily.last().is_none_or(|entry| entry.day != today) {
            self.daily.push(DailyUsage {
                day: today,
                ..DailyUsage::default()
            });
        }
        self.daily.last_mut().expect("daily entry exists")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub dictations_today: u64,
    pub dictations_all_time: u64,
    pub minutes_dictated: f64,
    pub total_words: u64,
    pub completed_results: u64,
    pub average_transcription_latency_ms: f64,
    pub average_real_time_factor: f64,
    pub local_result_count: u64,
    pub cloud_result_count: u64,
    pub local_percent: f64,
    pub cloud_percent: f64,
    pub failed_transcriptions: u64,
    pub paste_attempts: u64,
    pub paste_successes: u64,
    pub paste_success_rate: f64,
    pub cloud_fallbacks: u64,
    pub cost_estimates_enabled: bool,
    pub cloud_cost_per_minute_usd: f64,
    pub estimated_cloud_cost_usd: Option<f64>,
    pub estimated_local_cost_avoided_usd: Option<f64>,
    pub estimated_typing_hours_saved: f64,
    pub detailed_since: Option<String>,
    pub daily: Vec<DailyUsage>,
    pub recent_sessions: Vec<SessionRecord>,
    pub failure_counts: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct UsageStatsStore {
    path: Arc<PathBuf>,
    counters: Arc<Mutex<UsageCounters>>,
}

impl UsageStatsStore {
    pub fn new(app: &AppHandle) -> Self {
        let path = local_root(app).join("usage.json");
        let store = Self {
            path: Arc::new(path.clone()),
            counters: Arc::new(Mutex::new(load_counters(&path))),
        };
        if let Err(error) = store.persist() {
            warn!(error = %error, "could not persist initial usage statistics");
        }
        store
    }
    #[cfg(test)]
    pub fn for_tests(path: PathBuf) -> Self {
        Self {
            counters: Arc::new(Mutex::new(load_counters(&path))),
            path: Arc::new(path),
        }
    }
    pub fn record_started(&self) {
        self.update(|c| {
            c.started_dictations_today += 1;
            c.total_started_dictations += 1;
            c.today_mut().sessions += 1;
        });
    }
    pub fn record_capture(&self, duration_ms: u64) {
        self.update(|c| {
            c.captured_audio_duration_ms += duration_ms;
            c.today_mut().duration_ms += duration_ms;
        });
    }
    pub fn record_result(&self, result: &TranscriptionResult) {
        let words = result.text.split_whitespace().count() as u64;
        self.update(|c| {
            c.completed_results += 1;
            c.total_words += words;
            c.total_latency_ms += result.latency_ms;
            c.total_transcription_duration_ms += result.transcription_duration_ms;
            let today = c.today_mut();
            today.completed += 1;
            today.words += words;
            if result.backend.to_ascii_lowercase().contains("nemotron") {
                today.nemotron_words += words;
            } else {
                today.parakeet_words += words;
            }
            if is_cloud_backend(&result.backend) {
                c.cloud_result_count += 1;
                c.cloud_audio_duration_ms += result.audio_duration_ms;
            } else {
                c.local_result_count += 1;
                c.local_audio_duration_ms += result.audio_duration_ms;
            }
            if result.fallback_reason.is_some() {
                c.cloud_fallback_count += 1;
            }
        });
    }
    pub fn record_failure(&self) {
        self.record_categorized_failure(FailureCategory::Unknown);
    }
    pub fn record_categorized_failure(&self, category: FailureCategory) {
        self.update(|c| {
            c.failed_sessions += 1;
            c.today_mut().failed += 1;
            *c.failure_counts
                .entry(format!("{category:?}").to_ascii_lowercase())
                .or_default() += 1;
        });
    }
    pub fn record_paste(&self, success: bool) {
        self.update(|c| {
            c.paste_attempts += 1;
            c.today_mut().paste_attempts += 1;
            if success {
                c.paste_successes += 1;
                c.today_mut().paste_successes += 1;
            }
        });
    }
    pub fn record_session(&self, record: SessionRecord) {
        self.update(|c| c.sessions.push(record));
    }
    pub fn snapshot(&self, cost_estimates_enabled: bool, rate: f64, typing_wpm: u32) -> UsageStats {
        let c = self
            .counters
            .lock()
            .map(|v| v.clone().normalized())
            .unwrap_or_default();
        let total_results = c.local_result_count + c.cloud_result_count;
        let minutes = c.captured_audio_duration_ms as f64 / 60_000.0;
        let rate = if rate.is_finite() && rate >= 0.0 {
            rate
        } else {
            DEFAULT_RATE_USD_PER_MINUTE
        };
        let mut sessions = c.sessions.clone();
        sessions.reverse();
        UsageStats {
            dictations_today: c.started_dictations_today,
            dictations_all_time: c.total_started_dictations,
            minutes_dictated: minutes,
            total_words: c.total_words,
            completed_results: c.completed_results,
            average_transcription_latency_ms: ratio(c.total_latency_ms as f64, c.completed_results),
            average_real_time_factor: ratio(
                c.total_transcription_duration_ms as f64,
                c.captured_audio_duration_ms,
            ),
            local_result_count: c.local_result_count,
            cloud_result_count: c.cloud_result_count,
            local_percent: percentage(c.local_result_count, total_results),
            cloud_percent: percentage(c.cloud_result_count, total_results),
            failed_transcriptions: c.failed_sessions,
            paste_attempts: c.paste_attempts,
            paste_successes: c.paste_successes,
            paste_success_rate: percentage(c.paste_successes, c.paste_attempts),
            cloud_fallbacks: c.cloud_fallback_count,
            cost_estimates_enabled,
            cloud_cost_per_minute_usd: rate,
            estimated_cloud_cost_usd: cost_estimates_enabled
                .then_some(c.cloud_audio_duration_ms as f64 / 60_000.0 * rate),
            estimated_local_cost_avoided_usd: cost_estimates_enabled
                .then_some(c.local_audio_duration_ms as f64 / 60_000.0 * rate),
            estimated_typing_hours_saved: c.total_words as f64
                / typing_wpm.clamp(10, 200) as f64
                / 60.0,
            detailed_since: c.detailed_since,
            daily: c.daily,
            recent_sessions: sessions,
            failure_counts: c.failure_counts,
        }
    }
    pub fn reset(&self) -> UsageStats {
        if let Ok(mut c) = self.counters.lock() {
            *c = UsageCounters::default();
        }
        if let Err(error) = self.persist() {
            warn!(error = %error, "could not persist reset usage statistics");
        }
        self.snapshot(false, DEFAULT_RATE_USD_PER_MINUTE, 40)
    }
    fn update(&self, operation: impl FnOnce(&mut UsageCounters)) {
        if let Ok(mut c) = self.counters.lock() {
            *c = c.clone().normalized();
            operation(&mut c);
            *c = c.clone().normalized();
        }
        if let Err(error) = self.persist() {
            warn!(error = %error, "could not persist usage statistics update");
        }
    }
    fn persist(&self) -> Result<(), std::io::Error> {
        let c = self
            .counters
            .lock()
            .map_err(|_| std::io::Error::other("usage statistics lock poisoned"))?
            .clone()
            .normalized();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(&c).map_err(std::io::Error::other)?,
        )?;
        if self.path.exists() {
            fs::remove_file(self.path.as_ref())?;
        }
        fs::rename(temporary, self.path.as_ref())
    }
}

fn load_counters(path: &Path) -> UsageCounters {
    let Ok(contents) = fs::read_to_string(path) else {
        return UsageCounters::default();
    };
    match serde_json::from_str::<UsageCounters>(&contents) {
        Ok(c) => c.normalized(),
        Err(error) => {
            warn!(error = %error, "usage statistics file invalid; using empty counters");
            UsageCounters::default()
        }
    }
}
fn is_cloud_backend(backend: &str) -> bool {
    backend.to_ascii_lowercase().contains("cloud")
}
pub fn diagnostic_fallback_chain(fallback_reason: Option<&str>, backend: &str) -> Vec<String> {
    if fallback_reason.is_none() {
        return Vec::new();
    }
    vec![if is_cloud_backend(backend) {
        "local_to_cloud"
    } else {
        "local_backend_retry"
    }
    .to_owned()]
}
fn ratio(numerator: f64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f64
    }
}
fn percentage(value: u64, total: u64) -> f64 {
    ratio(value as f64 * 100.0, total)
}
pub fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
fn today_key() -> String {
    let days = timestamp_ms() / 86_400_000;
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
fn is_iso_day(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .chars()
            .enumerate()
            .all(|(index, value)| matches!(index, 4 | 7) || value.is_ascii_digit())
}
fn local_root(app: &AppHandle) -> PathBuf {
    #[cfg(windows)]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local_app_data).join("Lyra");
    }
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("app-data"))
        .join("Lyra")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn result(backend: &str) -> TranscriptionResult {
        TranscriptionResult {
            text: "hello from lyra".to_owned(),
            backend: backend.to_owned(),
            model: "test".to_owned(),
            language: None,
            audio_duration_ms: 1_000,
            transcription_duration_ms: 200,
            latency_ms: 300,
            fallback_reason: None,
        }
    }
    #[test]
    fn migrates_v1_and_never_persists_transcript_text() {
        let path = std::env::temp_dir().join(format!("lyra-usage-{}.json", uuid::Uuid::new_v4()));
        fs::write(&path, r#"{"schemaVersion":1,"todayKey":"2026-09-22","startedDictationsToday":24,"totalStartedDictations":76,"completedResults":71,"failedSessions":5}"#).unwrap();
        let store = UsageStatsStore::for_tests(path.clone());
        store.record_result(&result("local_nemo"));
        let stats = store.snapshot(false, 0.003, 40);
        assert_eq!(stats.dictations_all_time, 76);
        assert_eq!(stats.total_words, 3);
        assert!(!fs::read_to_string(&path)
            .unwrap()
            .contains("hello from lyra"));
        fs::remove_file(path).ok();
    }
    #[test]
    fn daily_aggregates_are_indefinite_and_sessions_expire_by_age() {
        let mut c = UsageCounters::default();
        c.daily = (0..1_000)
            .map(|i| DailyUsage {
                day: format!("2026-{:02}-{:02}", i / 28 + 1, i % 28 + 1),
                ..DailyUsage::default()
            })
            .collect();
        let now = timestamp_ms();
        let cutoff = now.saturating_sub(SESSION_RETENTION_MS);
        c.sessions = (0..3)
            .map(|i| SessionRecord {
                timestamp_ms: match i {
                    0 => cutoff.saturating_sub(1),
                    1 => cutoff,
                    _ => now,
                },
                app_version: "0.4.0".to_owned(),
                duration_ms: 1,
                word_count: 0,
                target_executable: None,
                target_category: None,
                endpoint: "manual".to_owned(),
                engine: "parakeet_tdt".to_owned(),
                runtime: None,
                backend: None,
                fallback_chain: Vec::new(),
                latency_ms: None,
                real_time_factor: None,
                paste_outcome: "not_attempted".to_owned(),
                failure_category: None,
            })
            .collect();
        let c = c.normalized_at(now);
        assert_eq!(c.daily.len(), 1_000);
        assert_eq!(c.sessions.len(), 2);
        assert_eq!(c.sessions[0].timestamp_ms, cutoff);
        assert_eq!(c.sessions[1].timestamp_ms, now);
    }
    #[test]
    fn paste_rate_is_safe_when_empty() {
        let path =
            std::env::temp_dir().join(format!("lyra-usage-rate-{}.json", uuid::Uuid::new_v4()));
        let store = UsageStatsStore::for_tests(path.clone());
        assert_eq!(store.snapshot(false, 0.003, 40).paste_success_rate, 0.0);
        fs::remove_file(path).ok();
    }

    #[test]
    fn fallback_details_do_not_persist_paths() {
        let path =
            std::env::temp_dir().join(format!("lyra-fallback-{}.json", uuid::Uuid::new_v4()));
        let store = UsageStatsStore::for_tests(path.clone());
        let sensitive_detail = r"C:\Users\Met\AppData\Local\Lyra\runtime\nemo_speech_asr_c.dll";
        let record = SessionRecord {
            timestamp_ms: timestamp_ms(),
            app_version: "0.4.4".to_owned(),
            duration_ms: 1,
            word_count: 1,
            target_executable: None,
            target_category: None,
            endpoint: "manual".to_owned(),
            engine: "nemotron".to_owned(),
            runtime: None,
            backend: Some("local_parakeet".to_owned()),
            fallback_chain: diagnostic_fallback_chain(Some(sensitive_detail), "local_parakeet"),
            latency_ms: Some(1),
            real_time_factor: Some(1.0),
            paste_outcome: "copied".to_owned(),
            failure_category: None,
        };
        store.record_session(record);

        let persisted = fs::read_to_string(&path).expect("read persisted usage record");
        assert!(!persisted.contains("C:\\Users\\Met"));
        assert!(persisted.contains("local_backend_retry"));
        fs::remove_file(path).ok();
    }
}
