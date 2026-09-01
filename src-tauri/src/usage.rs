use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::warn;

use crate::domain::TranscriptionResult;

const STATS_SCHEMA_VERSION: u32 = 1;
const DEFAULT_RATE_USD_PER_MINUTE: f64 = 0.003;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
}

impl Default for UsageCounters {
    fn default() -> Self {
        Self {
            schema_version: STATS_SCHEMA_VERSION,
            today_key: today_key(),
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
        }
    }
}

impl UsageCounters {
    fn normalized(mut self) -> Self {
        self.schema_version = STATS_SCHEMA_VERSION;
        if self.today_key != today_key() {
            self.today_key = today_key();
            self.started_dictations_today = 0;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub dictations_today: u64,
    pub dictations_all_time: u64,
    pub minutes_dictated: f64,
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
}

#[derive(Clone)]
pub struct UsageStatsStore {
    path: Arc<PathBuf>,
    counters: Arc<Mutex<UsageCounters>>,
}

impl UsageStatsStore {
    pub fn new(app: &AppHandle) -> Self {
        let path = local_root(app).join("usage.json");
        let counters = load_counters(&path);
        let store = Self {
            path: Arc::new(path),
            counters: Arc::new(Mutex::new(counters)),
        };
        if let Err(error) = store.persist() {
            warn!(error = %error, "could not persist initial usage statistics");
        }
        store
    }

    #[cfg(test)]
    pub fn for_tests(path: PathBuf) -> Self {
        let counters = load_counters(&path);
        Self {
            path: Arc::new(path),
            counters: Arc::new(Mutex::new(counters)),
        }
    }

    pub fn record_started(&self) {
        self.update(|counters| {
            counters.started_dictations_today += 1;
            counters.total_started_dictations += 1;
        });
    }

    pub fn record_capture(&self, duration_ms: u64) {
        self.update(|counters| counters.captured_audio_duration_ms += duration_ms);
    }

    pub fn record_result(&self, result: &TranscriptionResult) {
        self.update(|counters| {
            counters.completed_results += 1;
            counters.total_latency_ms += result.latency_ms;
            counters.total_transcription_duration_ms += result.transcription_duration_ms;
            if is_cloud_backend(&result.backend) {
                counters.cloud_result_count += 1;
                counters.cloud_audio_duration_ms += result.audio_duration_ms;
            } else {
                counters.local_result_count += 1;
                counters.local_audio_duration_ms += result.audio_duration_ms;
            }
            if result.fallback_reason.is_some() {
                counters.cloud_fallback_count += 1;
            }
        });
    }

    pub fn record_failure(&self) {
        self.update(|counters| counters.failed_sessions += 1);
    }

    pub fn record_paste(&self, success: bool) {
        self.update(|counters| {
            counters.paste_attempts += 1;
            if success {
                counters.paste_successes += 1;
            }
        });
    }

    pub fn snapshot(&self, cost_estimates_enabled: bool, rate: f64) -> UsageStats {
        let counters = self
            .counters
            .lock()
            .map(|counters| counters.clone().normalized())
            .unwrap_or_default();
        let total_results = counters.local_result_count + counters.cloud_result_count;
        let minutes = counters.captured_audio_duration_ms as f64 / 60_000.0;
        let average_latency = ratio(counters.total_latency_ms as f64, counters.completed_results);
        let average_rtf = ratio(
            counters.total_transcription_duration_ms as f64,
            counters.captured_audio_duration_ms,
        );
        let paste_success_rate = percentage(counters.paste_successes, counters.paste_attempts);
        let rate = if rate.is_finite() && rate >= 0.0 {
            rate
        } else {
            DEFAULT_RATE_USD_PER_MINUTE
        };
        UsageStats {
            dictations_today: counters.started_dictations_today,
            dictations_all_time: counters.total_started_dictations,
            minutes_dictated: minutes,
            completed_results: counters.completed_results,
            average_transcription_latency_ms: average_latency,
            average_real_time_factor: average_rtf,
            local_result_count: counters.local_result_count,
            cloud_result_count: counters.cloud_result_count,
            local_percent: percentage(counters.local_result_count, total_results),
            cloud_percent: percentage(counters.cloud_result_count, total_results),
            failed_transcriptions: counters.failed_sessions,
            paste_attempts: counters.paste_attempts,
            paste_successes: counters.paste_successes,
            paste_success_rate,
            cloud_fallbacks: counters.cloud_fallback_count,
            cost_estimates_enabled,
            cloud_cost_per_minute_usd: rate,
            estimated_cloud_cost_usd: cost_estimates_enabled
                .then_some(counters.cloud_audio_duration_ms as f64 / 60_000.0 * rate),
            estimated_local_cost_avoided_usd: cost_estimates_enabled
                .then_some(counters.local_audio_duration_ms as f64 / 60_000.0 * rate),
        }
    }

    pub fn reset(&self) -> UsageStats {
        if let Ok(mut counters) = self.counters.lock() {
            *counters = UsageCounters::default();
        }
        if let Err(error) = self.persist() {
            warn!(error = %error, "could not persist reset usage statistics");
        }
        self.snapshot(false, DEFAULT_RATE_USD_PER_MINUTE)
    }

    fn update(&self, operation: impl FnOnce(&mut UsageCounters)) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters = counters.clone().normalized();
            operation(&mut counters);
        }
        if let Err(error) = self.persist() {
            warn!(error = %error, "could not persist usage statistics update");
        }
    }

    fn persist(&self) -> Result<(), std::io::Error> {
        let counters = self
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
            serde_json::to_vec_pretty(&counters).map_err(std::io::Error::other)?,
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
        Ok(counters) => counters.normalized(),
        Err(error) => {
            warn!(error = %error, "usage statistics file invalid; using empty counters");
            UsageCounters::default()
        }
    }
}

fn is_cloud_backend(backend: &str) -> bool {
    backend.to_ascii_lowercase().contains("cloud")
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

fn today_key() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_secs() / 86_400).to_string())
        .unwrap_or_else(|_| "0".to_owned())
}

fn local_root(app: &AppHandle) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("Lyra");
        }
    }
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("app-data"))
        .join("Lyra")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TranscriptionResult;
    use std::fs;

    fn result(backend: &str, audio_ms: u64, latency_ms: u64) -> TranscriptionResult {
        TranscriptionResult {
            text: "hello".to_owned(),
            backend: backend.to_owned(),
            model: "test".to_owned(),
            language: None,
            audio_duration_ms: audio_ms,
            transcription_duration_ms: latency_ms / 2,
            latency_ms,
            fallback_reason: None,
        }
    }

    #[test]
    fn aggregates_local_cloud_latency_and_cost_avoided() {
        let path = std::env::temp_dir().join(format!("lyra-usage-{}.json", uuid::Uuid::new_v4()));
        let store = UsageStatsStore::for_tests(path.clone());
        store.record_started();
        store.record_capture(120_000);
        store.record_result(&result("local_nemo", 60_000, 300));
        let mut cloud = result("openai_cloud", 60_000, 900);
        cloud.fallback_reason = Some("GPU unavailable".to_owned());
        store.record_result(&cloud);
        store.record_paste(true);
        let stats = store.snapshot(true, 0.003);
        assert_eq!(stats.dictations_all_time, 1);
        assert_eq!(stats.completed_results, 2);
        assert_eq!(stats.local_percent, 50.0);
        assert_eq!(stats.cloud_percent, 50.0);
        assert_eq!(stats.cloud_fallbacks, 1);
        assert_eq!(stats.paste_success_rate, 100.0);
        assert!((stats.estimated_local_cost_avoided_usd.unwrap() - 0.003).abs() < 1e-9);
        assert!(path.exists());
        fs::remove_file(path).expect("remove usage fixture");
    }

    #[test]
    fn corrupt_statistics_recover_to_empty_and_reset() {
        let path =
            std::env::temp_dir().join(format!("lyra-usage-corrupt-{}.json", uuid::Uuid::new_v4()));
        fs::write(&path, b"not-json").expect("write corrupt fixture");
        let store = UsageStatsStore::for_tests(path.clone());
        let empty = store.snapshot(true, 0.003);
        assert_eq!(empty.dictations_all_time, 0);
        store.record_started();
        assert_eq!(store.snapshot(false, 0.003).dictations_all_time, 1);
        assert_eq!(store.reset().dictations_all_time, 0);
        fs::remove_file(path).expect("remove usage fixture");
    }

    #[test]
    fn paste_success_rate_is_safe_when_no_attempt_exists() {
        let path =
            std::env::temp_dir().join(format!("lyra-usage-rate-{}.json", uuid::Uuid::new_v4()));
        let store = UsageStatsStore::for_tests(path.clone());
        assert_eq!(store.snapshot(false, 0.003).paste_success_rate, 0.0);
        fs::remove_file(path).ok();
    }
}
