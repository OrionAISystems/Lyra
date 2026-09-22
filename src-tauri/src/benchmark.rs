use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const BENCHMARK_PHRASE: &str = "Lyra turns a clear thought into useful text.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkScore {
    pub engine: String,
    pub runtime: Option<String>,
    pub latency_ms: u64,
    pub real_time_factor: f64,
    pub transcript_distance: f64,
    pub partial_count: u32,
    pub succeeded: bool,
    pub failure_category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub timestamp_ms: u64,
    pub phrase_word_count: usize,
    pub microphone: Option<String>,
    pub scores: Vec<BenchmarkScore>,
    pub recommendation: Option<String>,
}

pub fn transcript_distance(reference: &str, candidate: &str) -> f64 {
    let left = words(reference);
    let right = words(candidate);
    if left.is_empty() {
        return if right.is_empty() { 0.0 } else { 1.0 };
    }
    let mut row = (0..=right.len()).collect::<Vec<_>>();
    for (i, a) in left.iter().enumerate() {
        let mut next = vec![i + 1];
        for (j, b) in right.iter().enumerate() {
            next.push(
                (row[j + 1] + 1)
                    .min(next[j] + 1)
                    .min(row[j] + usize::from(a != b)),
            );
        }
        row = next;
    }
    row[right.len()] as f64 / left.len().max(right.len()) as f64
}

pub fn recommend(scores: &[BenchmarkScore]) -> Option<String> {
    scores
        .iter()
        .filter(|score| {
            score.succeeded && score.transcript_distance <= 0.25 && score.real_time_factor <= 1.0
        })
        .min_by(|a, b| {
            (a.transcript_distance + a.real_time_factor * 0.15)
                .total_cmp(&(b.transcript_distance + b.real_time_factor * 0.15))
        })
        .map(|score| score.engine.clone())
}

pub fn save(app: &AppHandle, result: &BenchmarkResult) -> Result<(), String> {
    let path = path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    fs::rename(temporary, path).map_err(|e| e.to_string())
}

pub fn load(app: &AppHandle) -> Option<BenchmarkResult> {
    serde_json::from_slice(&fs::read(path(app)).ok()?).ok()
}

fn path(app: &AppHandle) -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(root).join("Lyra").join("benchmark.json");
    }
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| PathBuf::from("app-data"))
        .join("Lyra")
        .join("benchmark.json")
}
fn words(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distance_and_recommendation_require_accuracy_and_reliability() {
        assert_eq!(transcript_distance(BENCHMARK_PHRASE, BENCHMARK_PHRASE), 0.0);
        let scores = vec![
            BenchmarkScore {
                engine: "fast".into(),
                runtime: None,
                latency_ms: 50,
                real_time_factor: 0.1,
                transcript_distance: 0.5,
                partial_count: 2,
                succeeded: true,
                failure_category: None,
            },
            BenchmarkScore {
                engine: "accurate".into(),
                runtime: None,
                latency_ms: 100,
                real_time_factor: 0.2,
                transcript_distance: 0.0,
                partial_count: 1,
                succeeded: true,
                failure_category: None,
            },
        ];
        assert_eq!(recommend(&scores).as_deref(), Some("accurate"));
    }
}
