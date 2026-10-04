//! Internal, opt-in fixture replay. Never called by the desktop application.
use std::{collections::HashSet, fs, path::Path, time::Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        process_transcription_text, BackendError, LanguagePreference, TextProcessingOptions,
        TranscriptionRequest, TranscriptionResult, TranscriptionRoute,
    },
    runtime::RuntimeManager,
    transcription::TranscriptionRouter,
};

const SCHEMA: u32 = 1;
const NORMALIZATION: &str = "unicode-lowercase-alphanumeric-tokens-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    schema: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    category: String,
    provenance: String,
    audio: String,
    expected: String,
    max_wer: f64,
    #[serde(default)]
    require_exact: bool,
    #[serde(default)]
    accept_empty_recording: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Context {
    engine: String,
    machine: String,
    os: String,
    arch: String,
    runtime_version: Option<String>,
    runtime_variant: Option<String>,
    runtime_sha256: Option<String>,
    model_sha256: String,
    gpu: Option<String>,
    repetitions: usize,
    normalization: String,
    processing: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Score {
    edits: usize,
    reference_words: usize,
    // WER is undefined for a zero-word reference. Insertions remain explicit.
    wer: Option<f64>,
    normalized_match: bool,
    exact_match: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Trial {
    latency_ms: f64,
    raw: Option<String>,
    processed: Option<String>,
    backend: Option<String>,
    runtime_variant: Option<String>,
    raw_score: Option<Score>,
    score: Option<Score>,
    partial_count: usize,
    error: Option<String>,
    #[serde(default)]
    expected_empty_recording: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CaseResult {
    id: String,
    category: String,
    provenance: String,
    // Hash covers audio, reference, provenance and acceptance policy.
    fingerprint: String,
    expected: String,
    audio_ms: u64,
    trials: Vec<Trial>,
    median_latency_ms: f64,
    p95_latency_ms: f64,
    real_time_factor: f64,
    passed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Report {
    schema: u32,
    source_revision: String,
    timestamp_ms: u128,
    context: Context,
    warmup_latency_ms: f64,
    warmup_error: Option<String>,
    cases: Vec<CaseResult>,
    comparison: Vec<String>,
    baseline_deltas: Vec<BaselineDelta>,
    #[serde(default)]
    comparison_policy: Option<ComparisonPolicy>,
    passed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonPolicy {
    wer_delta: f64,
    latency_ratio: f64,
    latency_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineDelta {
    id: String,
    median_latency_ms: f64,
    worst_wer: Option<f64>,
}

fn normalized(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn score(reference: &str, candidate: &str) -> Score {
    let left = normalized(reference);
    let right = normalized(candidate);
    let mut row: Vec<usize> = (0..=right.len()).collect();
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
    let edits = row[right.len()];
    Score {
        edits,
        reference_words: left.len(),
        wer: (!left.is_empty()).then(|| edits as f64 / left.len() as f64),
        normalized_match: left == right,
        exact_match: reference == candidate,
    }
}

fn expected_empty_recording(case: &Case, error: &BackendError) -> bool {
    case.accept_empty_recording
        && case.expected.is_empty()
        && matches!(error, BackendError::EmptyRecording)
}

type Recognition = (Result<TranscriptionResult, BackendError>, f64, usize);

fn recognize(router: &TranscriptionRouter, engine: &str, samples: &[f32], id: &str) -> Recognition {
    let request = TranscriptionRequest {
        audio_samples: samples.to_vec(),
        sample_rate: 16_000,
        language: LanguagePreference::En,
        request_id: id.into(),
    };
    let mut partial_count = 0;
    let started = Instant::now();
    let result = if engine == "nemotron" {
        router.transcribe_nemotron_streaming(&request, vec![], |result| {
            if matches!(result, crate::streaming::StreamingResult::Partial(_)) {
                partial_count += 1;
            }
        })
    } else {
        router.transcribe(&request, TranscriptionRoute::LocalOnly)
    };
    (
        result,
        started.elapsed().as_secs_f64() * 1000.0,
        partial_count,
    )
}

// The pinned native Nemotron library aborts on repeated model reloads in one
// process. Its production path loads per request, so isolate each replay without
// replacing that path. An abort becomes a failed trial, not a lost suite report.
fn isolated_nemotron(audio: &Path, root: &Path) -> Recognition {
    let started = Instant::now();
    let result_path =
        std::env::temp_dir().join(format!("lyra-bench-{}.json", uuid::Uuid::new_v4()));
    let result = (|| -> Result<Recognition, String> {
        let output =
            std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .arg("--native-worker")
                .arg(audio)
                .arg(root)
                .arg(&result_path)
                .output()
                .map_err(|e| e.to_string())?;
        if !output.status.success() {
            let diagnostics = String::from_utf8_lossy(&output.stderr);
            let tail: String = diagnostics
                .chars()
                .rev()
                .take(1500)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            return Err(format!("native worker exited {}: {tail}", output.status));
        }
        serde_json::from_slice(&fs::read(&result_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    })();
    if result_path.exists() {
        let _ = fs::remove_file(&result_path);
    }
    result.unwrap_or_else(|error| {
        (
            Err(BackendError::StreamFailure(error)),
            started.elapsed().as_secs_f64() * 1000.0,
            0,
        )
    })
}

/// Strict RIFF PCM16 mono reader. Chunk padding and optional metadata are supported.
/// No implicit downmix/resample: fixtures must match Lyra's native streaming input.
fn wav(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    if bytes.len() < 12
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WAVE"
        || u32_at(4) + 8 != bytes.len()
    {
        return Err("expected a complete RIFF WAV".into());
    }
    let mut cursor = 12;
    let mut format = false;
    let mut data = None;
    while cursor + 8 <= bytes.len() {
        let size = u32_at(cursor + 4);
        let end = cursor
            .checked_add(8)
            .and_then(|n| n.checked_add(size))
            .ok_or("WAV chunk overflow")?;
        if end > bytes.len() {
            return Err("truncated WAV chunk".into());
        }
        let chunk = &bytes[cursor + 8..end];
        match &bytes[cursor..cursor + 4] {
            b"fmt " => {
                if format
                    || chunk.len() < 16
                    || chunk[..16] != [1, 0, 1, 0, 128, 62, 0, 0, 0, 125, 0, 0, 2, 0, 16, 0]
                {
                    return Err("fixture must be PCM16 mono at 16000 Hz".into());
                }
                format = true;
            }
            b"data" => {
                if data.is_some() || size == 0 || size % 2 != 0 {
                    return Err("invalid WAV data chunk".into());
                }
                data = Some(chunk);
            }
            _ => {}
        }
        cursor = end + size % 2;
    }
    if !format || cursor != bytes.len() {
        return Err("invalid WAV layout".into());
    }
    let data = data.ok_or("missing WAV audio")?;
    if data.len() > 16_000 * 2 * 300 {
        return Err("fixture exceeds five minutes".into());
    }
    Ok(data
        .chunks_exact(2)
        .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0)
        .collect())
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    ordered[((ordered.len() as f64 * fraction).ceil() as usize).saturating_sub(1)]
}

// Integer counts are the source of truth. JSON float round-tripping can change
// a WER by one ULP, which must not turn an identical result into a regression.
fn worst_wer(case: &CaseResult) -> Option<f64> {
    case.trials
        .iter()
        .filter_map(|trial| trial.score.as_ref())
        .filter(|score| score.reference_words > 0)
        .map(|score| score.edits as f64 / score.reference_words as f64)
        .reduce(f64::max)
}

fn compare(
    current: &Report,
    baseline: &Report,
    wer_delta: f64,
    latency_ratio: f64,
    latency_ms: f64,
) -> Vec<String> {
    let mut failures = vec![];
    if baseline.schema != SCHEMA || current.context != baseline.context || !baseline.passed {
        failures.push(
            "baseline schema, context or acceptance incompatible; establish an approved baseline"
                .into(),
        );
        return failures;
    }
    if current.cases.len() != baseline.cases.len() {
        failures.push("baseline case set changed".into());
    }
    for case in &current.cases {
        let Some(old) = baseline.cases.iter().find(|old| old.id == case.id) else {
            failures.push(format!("{}: missing from baseline", case.id));
            continue;
        };
        if case.fingerprint != old.fingerprint || case.trials.len() != old.trials.len() {
            failures.push(format!("{}: fixture or policy changed", case.id));
            continue;
        }
        if !case.passed
            || worst_wer(case).unwrap_or(0.0) > worst_wer(old).unwrap_or(0.0) + wer_delta
        {
            failures.push(format!("{}: accuracy regressed", case.id));
        }
        let matches = |value: &CaseResult| {
            value
                .trials
                .iter()
                .filter(|t| t.score.as_ref().is_some_and(|s| s.normalized_match))
                .count()
        };
        let exact = |value: &CaseResult| {
            value
                .trials
                .iter()
                .filter(|t| t.score.as_ref().is_some_and(|s| s.exact_match))
                .count()
        };
        if matches(case) < matches(old) || exact(case) < exact(old) {
            failures.push(format!("{}: text matches regressed", case.id));
        }
        if case
            .trials
            .iter()
            .map(|t| &t.backend)
            .ne(old.trials.iter().map(|t| &t.backend))
            || case
                .trials
                .iter()
                .map(|t| &t.runtime_variant)
                .ne(old.trials.iter().map(|t| &t.runtime_variant))
        {
            failures.push(format!("{}: backend changed", case.id));
        }
        if case.median_latency_ms > old.median_latency_ms * (1.0 + latency_ratio) + latency_ms {
            failures.push(format!(
                "{}: median latency regressed ({:.1} -> {:.1} ms)",
                case.id, old.median_latency_ms, case.median_latency_ms
            ));
        }
    }
    failures
}

/// CLI entry point. Audio stays local; no cloud routes, capture, injection or settings writes.
pub fn run(args: Vec<String>) -> Result<(), String> {
    if args.first().is_some_and(|a| a == "--native-worker") {
        if args.len() != 4 {
            return Err("invalid native worker invocation".into());
        }
        let samples = wav(&fs::read(&args[1]).map_err(|e| e.to_string())?)?;
        let router = TranscriptionRouter::new(RuntimeManager::for_tests((&args[2]).into()));
        let result = recognize(&router, "nemotron", &samples, "isolated-replay");
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[3])
            .map_err(|e| e.to_string())?;
        return serde_json::to_writer(file, &result).map_err(|e| e.to_string());
    }
    if args.iter().any(|a| a == "--help") {
        println!("--manifest FILE --output FILE --machine LABEL [--engine parakeet|nemotron] [--root DIR] [--baseline FILE] [--repetitions N] [--wer-delta 0] [--latency-ratio 0.25] [--latency-ms 50]");
        return Ok(());
    }
    let mut options = std::collections::HashMap::new();
    if args.len() % 2 != 0 {
        return Err("arguments must be --option value pairs; use --help".into());
    }
    for pair in args.chunks_exact(2) {
        if ![
            "--manifest",
            "--output",
            "--machine",
            "--engine",
            "--root",
            "--baseline",
            "--repetitions",
            "--wer-delta",
            "--latency-ratio",
            "--latency-ms",
        ]
        .contains(&pair[0].as_str())
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or duplicate option {}", pair[0]));
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("missing {key}"))
    };
    let manifest = required("--manifest")?;
    let output = required("--output")?;
    let machine = required("--machine")?;
    if machine.trim().is_empty() {
        return Err("machine label must identify hardware and power mode".into());
    }
    if Path::new(output).exists() {
        return Err("output exists; choose a new path (baselines are never overwritten)".into());
    }
    let engine = options.get("--engine").copied().unwrap_or("parakeet");
    if !["parakeet", "nemotron"].contains(&engine) {
        return Err("engine must be parakeet or nemotron".into());
    }
    let repetitions: usize = options
        .get("--repetitions")
        .unwrap_or(&"3")
        .parse()
        .map_err(|_| "invalid repetitions")?;
    if !(1..=20).contains(&repetitions) {
        return Err("repetitions must be 1..20".into());
    }
    let tolerance = |key, default: &str| -> Result<f64, String> {
        let value: f64 = options
            .get(key)
            .copied()
            .unwrap_or(default)
            .parse()
            .map_err(|_| format!("invalid {key}"))?;
        if !value.is_finite() || value < 0.0 {
            return Err(format!("invalid {key}"));
        }
        Ok(value)
    };
    let wer_delta = tolerance("--wer-delta", "0")?;
    let latency_ratio = tolerance("--latency-ratio", "0.25")?;
    let latency_ms = tolerance("--latency-ms", "50")?;
    let suite: Suite = serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if suite.schema != SCHEMA || suite.cases.is_empty() {
        return Err("unsupported or empty suite".into());
    }
    let mut ids = HashSet::new();
    let mut fixtures = vec![];
    for case in &suite.cases {
        if case.id.is_empty()
            || !ids.insert(&case.id)
            || !["synthetic", "microphone"].contains(&case.provenance.as_str())
            || !case.max_wer.is_finite()
            || case.max_wer < 0.0
            || (case.accept_empty_recording && !case.expected.is_empty())
        {
            return Err(format!("invalid case {}", case.id));
        }
        let path = Path::new(manifest)
            .parent()
            .unwrap_or(Path::new("."))
            .join(&case.audio);
        if fs::metadata(&path)
            .map_err(|e| format!("{}: {e}", case.id))?
            .len()
            > 10_000_000
        {
            return Err("WAV file exceeds size limit".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let samples = wav(&bytes).map_err(|e| format!("{}: {e}", case.id))?;
        let mut digest = Sha256::new();
        digest.update(&bytes);
        digest.update(
            serde_json::to_vec(&(
                &case.id,
                &case.category,
                &case.provenance,
                &case.expected,
                case.max_wer,
                case.require_exact,
                case.accept_empty_recording,
            ))
            .unwrap(),
        );
        fixtures.push((
            samples,
            digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        ));
    }
    let baseline: Option<Report> = options
        .get("--baseline")
        .map(|path| {
            fs::read(path)
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
        })
        .transpose()?;
    let root = options
        .get("--root")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(|p| Path::new(&p).join("Lyra")))
        .ok_or("supply --root on this platform")?;
    let runtime = RuntimeManager::for_tests(root.clone());
    let status = runtime.status();
    let context = Context {
        engine: engine.into(),
        machine: machine.into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        runtime_version: status.runtime_version,
        runtime_variant: status.runtime_variant,
        runtime_sha256: status.runtime_sha256,
        model_sha256: if engine == "parakeet" {
            crate::runtime::MODEL_SHA256
        } else {
            crate::runtime::NEMOTRON_MODEL_SHA256
        }
        .into(),
        gpu: status.gpu_name,
        repetitions,
        normalization: NORMALIZATION.into(),
        processing: "formatting-on-self-correction-on-no-personalization-v1".into(),
    };
    let router = TranscriptionRouter::new(runtime);
    let transcribe = |samples: &[f32], id: &str, audio: &Path| {
        if engine == "nemotron" {
            isolated_nemotron(audio, &root)
        } else {
            recognize(&router, engine, samples, id)
        }
    };
    let warmup = fixtures
        .iter()
        .position(|(s, _)| s.iter().any(|v| v.abs() > 0.01))
        .unwrap_or(0);
    let audio_path = |case: &Case| {
        Path::new(manifest)
            .parent()
            .unwrap_or(Path::new("."))
            .join(&case.audio)
    };
    let (warmup_result, warmup_latency_ms, _) = transcribe(
        &fixtures[warmup].0,
        "warmup",
        &audio_path(&suite.cases[warmup]),
    );
    let mut report = Report {
        schema: SCHEMA,
        source_revision: std::process::Command::new("git")
            .args(["describe", "--always", "--dirty"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_else(|| "unknown".into()),
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
        context,
        warmup_latency_ms,
        warmup_error: warmup_result.err().map(|e| e.to_string()),
        cases: vec![],
        comparison: vec![],
        baseline_deltas: vec![],
        comparison_policy: Some(ComparisonPolicy {
            wer_delta,
            latency_ratio,
            latency_ms,
        }),
        passed: true,
    };
    for (case, (samples, fingerprint)) in suite.cases.iter().zip(fixtures) {
        println!("{} ({}, {})", case.id, case.category, case.provenance);
        let mut trials = vec![];
        for repetition in 0..repetitions {
            let (result, latency_ms, partial_count) = transcribe(
                &samples,
                &format!("{}-{repetition}", case.id),
                &audio_path(case),
            );
            let mut trial = Trial {
                latency_ms,
                raw: None,
                processed: None,
                backend: None,
                runtime_variant: None,
                raw_score: None,
                score: None,
                partial_count,
                error: None,
                expected_empty_recording: false,
            };
            match result {
                Ok(result) => {
                    let processed = process_transcription_text(
                        &result.text,
                        None,
                        TextProcessingOptions {
                            formatting_commands_enabled: true,
                            self_correction_enabled: true,
                            corrections: &[],
                            snippets: &[],
                        },
                    );
                    trial.score = Some(score(&case.expected, &processed));
                    trial.raw_score = Some(score(&case.expected, &result.text));
                    trial.runtime_variant = if engine == "parakeet" {
                        router.benchmark_runtime_variant()
                    } else {
                        report.context.runtime_variant.clone()
                    };
                    trial.processed = Some(processed);
                    trial.raw = Some(result.text);
                    trial.backend = Some(result.backend);
                }
                Err(error) => {
                    trial.expected_empty_recording = expected_empty_recording(case, &error);
                    trial.error = Some(error.to_string());
                }
            }
            trials.push(trial);
        }
        let timings: Vec<_> = trials.iter().map(|t| t.latency_ms).collect();
        let median_latency_ms = percentile(&timings, 0.5);
        let audio_ms = samples.len() as u64 * 1000 / 16000;
        let passed = trials.iter().all(|t| {
            t.expected_empty_recording
                || t.score.as_ref().is_some_and(|s| {
                    s.wer.map_or(s.edits == 0, |wer| wer <= case.max_wer)
                        && (!case.require_exact || s.exact_match)
                })
        });
        println!(
            "  {} median {:.1} ms",
            if passed { "PASS" } else { "FAIL" },
            median_latency_ms
        );
        report.cases.push(CaseResult {
            id: case.id.clone(),
            category: case.category.clone(),
            provenance: case.provenance.clone(),
            fingerprint,
            expected: case.expected.clone(),
            audio_ms,
            trials,
            median_latency_ms,
            p95_latency_ms: percentile(&timings, 0.95),
            real_time_factor: median_latency_ms / (samples.len() as f64 / 16.0),
            passed,
        });
    }
    report.passed = report.warmup_error.is_none() && report.cases.iter().all(|c| c.passed);
    if let Some(baseline) = baseline {
        report.comparison = compare(&report, &baseline, wer_delta, latency_ratio, latency_ms);
        for case in &report.cases {
            if let Some(old) = baseline.cases.iter().find(|old| old.id == case.id) {
                report.baseline_deltas.push(BaselineDelta {
                    id: case.id.clone(),
                    median_latency_ms: case.median_latency_ms - old.median_latency_ms,
                    worst_wer: worst_wer(case)
                        .zip(worst_wer(old))
                        .map(|(now, old)| now - old),
                });
            }
        }
    }
    report.passed &= report.comparison.is_empty();
    if let Some(parent) = Path::new(output)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &report).map_err(|e| e.to_string())?;
    println!("Report: {output}");
    for failure in &report.comparison {
        eprintln!("{failure}");
    }
    if report.passed {
        Ok(())
    } else {
        Err("accuracy, runtime or baseline gate failed; inspect report".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> Report {
        serde_json::from_value(serde_json::json!({
            "schema":1,"source_revision":"test","timestamp_ms":0,
            "context":{"engine":"parakeet","machine":"test","os":"windows","arch":"x86_64","runtime_version":"0.1.0","runtime_variant":"cuda","runtime_sha256":"runtime","model_sha256":"model","gpu":"test","repetitions":1,"normalization":NORMALIZATION,"processing":"test"},
            "warmup_latency_ms":200,"warmup_error":null,
            "cases":[{"id":"one","category":"test","provenance":"synthetic","fingerprint":"abc","expected":"hello","audio_ms":1000,
                "trials":[{"latency_ms":100,"raw":"hello","processed":"hello","backend":"local_nemo","runtime_variant":"cuda","raw_score":null,"score":{"edits":0,"reference_words":1,"wer":0,"normalized_match":true,"exact_match":true},"partial_count":0,"error":null}],
                "median_latency_ms":100,"p95_latency_ms":100,"real_time_factor":0.1,"passed":true}],
            "comparison":[],"baseline_deltas":[],"passed":true
        })).unwrap()
    }

    #[test]
    fn baseline_gates_accuracy_timing_and_compatibility() {
        let old = baseline();
        let mut now = baseline();
        assert!(compare(&now, &old, 0.0, 0.25, 50.0).is_empty());
        now.cases[0].median_latency_ms = 176.0;
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("latency"));
        now = baseline();
        let score = now.cases[0].trials[0].score.as_mut().unwrap();
        score.edits = 1;
        score.reference_words = 10;
        score.wer = Some(0.1);
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("accuracy"));
        now = baseline();
        now.cases[0].trials[0].score.as_mut().unwrap().exact_match = false;
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("matches"));
        now = baseline();
        now.cases[0].fingerprint = "changed".into();
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("fixture"));
        now = baseline();
        now.context.machine = "different".into();
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("incompatible"));
        now = baseline();
        now.cases[0].trials[0].runtime_variant = Some("cpu".into());
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("backend"));
        now = baseline();
        now.cases.clear();
        assert!(compare(&now, &old, 0.0, 0.25, 50.0)[0].contains("case set"));
        now = baseline();
        now.passed = false;
        assert!(compare(&old, &now, 0.0, 0.25, 50.0)[0].contains("incompatible"));
    }

    #[test]
    fn standard_wer_can_exceed_one_and_silence_is_explicit() {
        assert_eq!(score("one", "two three four").wer, Some(3.0));
        assert_eq!(score("one two three", "one three").wer, Some(1.0 / 3.0));
        assert!(score("ÉLAN, Orion!", "élan orion").normalized_match);
        assert!(!score("Hello!", "hello").exact_match);
        assert_eq!(score("", "hallucination").wer, None);
        assert_eq!(score("", "hallucination").edits, 1);
        assert!(score("", "").normalized_match);
        assert!(!score("12", "twelve").normalized_match);
    }

    #[test]
    fn baseline_round_trip_does_not_invent_fractional_wer_regressions() {
        let mut now = baseline();
        let score = now.cases[0].trials[0].score.as_mut().unwrap();
        score.edits = 1;
        score.reference_words = 103;
        score.wer = Some(1.0 / 103.0);
        let old: Report = serde_json::from_slice(&serde_json::to_vec(&now).unwrap()).unwrap();
        assert!(compare(&now, &old, 0.0, 0.25, 50.0).is_empty());
        assert_eq!(worst_wer(&now.cases[0]), worst_wer(&old.cases[0]));
    }

    #[test]
    fn silence_accepts_only_the_explicit_no_speech_outcome() {
        let mut case: Case = serde_json::from_value(serde_json::json!({
            "id":"silence","category":"silence","provenance":"synthetic","audio":"silence.wav","expected":"","max_wer":0,"accept_empty_recording":true
        })).unwrap();
        assert!(expected_empty_recording(
            &case,
            &BackendError::EmptyRecording
        ));
        assert!(!expected_empty_recording(
            &case,
            &BackendError::MissingRuntimeModel("missing".into())
        ));
        assert!(!expected_empty_recording(
            &case,
            &BackendError::LocalRecognitionFailure("failed".into())
        ));
        case.accept_empty_recording = false;
        assert!(!expected_empty_recording(
            &case,
            &BackendError::EmptyRecording
        ));
        case.accept_empty_recording = true;
        case.expected = "speech".into();
        assert!(!expected_empty_recording(
            &case,
            &BackendError::EmptyRecording
        ));
    }

    #[test]
    fn wav_validation_and_chunk_padding() {
        let original = crate::audio::encode_pcm_wav(&[0.0, 0.5, -0.5], 16000);
        assert_eq!(wav(&original).unwrap().len(), 3);
        assert!(wav(&original[..original.len() - 1]).is_err());
        assert!(wav(&crate::audio::encode_pcm_wav(&[0.0], 48000)).is_err());
        let mut padded = original.clone();
        padded.splice(12..12, [b'J', b'U', b'N', b'K', 1, 0, 0, 0, 5, 0]);
        let size = (padded.len() as u32 - 8).to_le_bytes();
        padded[4..8].copy_from_slice(&size);
        assert_eq!(wav(&padded).unwrap().len(), 3);
    }
}
