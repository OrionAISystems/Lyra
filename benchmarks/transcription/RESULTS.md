# Initial benchmark evidence — 2026-10-04

This is the historical pre-fix snapshot. See [CLEANUP-RESULTS.md](CLEANUP-RESULTS.md) for the subsequent punctuation and canonical-quantity fixes and expanded corpus; the original reports below are retained unchanged.

The internal harness is implemented and exercised against the installed native engines. **The complete starter corpus does not pass:** Parakeet passes 7/8 cases and Nemotron passes 6/8. These are benchmark findings, not failures of the harness tests, and the thresholds were not relaxed to hide them. No application recognition or cleanup behavior was changed.

## Environment and scope

Windows x86_64; AMD Ryzen 5 2600X; NVIDIA GeForce RTX 2060, 6 GiB VRAM; NVIDIA driver 616.64; AC desktop. Lyra's verified CUDA runtime 0.1.0 and pinned Q8 Parakeet/Nemotron models were already installed. Nothing was downloaded. Git revision and pinned hashes are in each report. Lyra remained running; about 2,256 MiB of GPU memory was occupied before replay. Builds/tests also ran during initial measurements. Treat these timings as functional evidence under contention, not calibrated performance targets or a fair warm-model engine speed ranking.

No reusable WAV recordings were present in the repository. Every case here uses **synthetic** Microsoft David Desktop speech (`System.Speech`, rate 0), except digitally generated silence. [Generator provenance](results/synthetic-provenance.json) is saved with the evidence. No real microphone quality, room noise, speaker diversity or capture behavior was measured. The long fixture is 39.79 seconds and 103 normalized reference words; pause/resume inserts 1.5 seconds of silence.

Each full suite contains eight cases, three measured trials per case, and a separate first replay. Parakeet stays loaded; Nemotron's production replay reloads its model per request. Successful isolated Nemotron timing includes that load and streaming drain, but excludes child-process startup. Parakeet's first replay took 1,012.8 ms. See the raw reports for all trials, raw/final scores, p95, real-time factors and Nemotron partial counts.

## Full-corpus outcomes

WER below is the worst measured final-output WER. Silence has an undefined denominator and is checked by its explicit no-speech policy.

| Case | Parakeet WER | Median ms | Nemotron WER | Median ms | Gate outcome |
| --- | ---: | ---: | ---: | ---: | --- |
| Silence | undefined | 40.2 | undefined | 954.1 | Both pass; Parakeet reports accepted `EmptyRecording`, Nemotron returns empty final text |
| Short dictation | 0% | 54.6 | 0% | 954.2 | Both pass |
| Spoken punctuation | 0% | 73.4 | 0% | 1,045.7 | Both fail exact-text formatting |
| Numbers | 11.11% | 63.1 | 33.33% | 997.7 | Parakeet passes; Nemotron fails the 15% limit |
| Proper names | 0% | 72.9 | 0% | 868.1 | Both pass normalized gate; NVIDIA casing differs |
| Long dictation | 0% | 578.6 | 0.97% | 3,434.6 | Both pass |
| Pause/resume | 0% | 78.8 | 0% | 950.3 | Both pass normalized gate |
| Explicit self-correction | 0% | 121.8 | 0% | 903.5 | Both pass normalized gate; formatting caveat remains |

[Parakeet report](results/parakeet.json) and [Nemotron report](results/nemotron.json) each have `passed: false`; both CLI runs exited 1 as intended.

The punctuation reference ends with `Please send the report.`. Both engines recognize a spoken final `period` followed by automatic punctuation; existing cleanup emits `report..`. Parakeet also leaves the sentence after the newline lowercase. Normalized WER cannot detect these differences, while the exact gate does. Self-correction similarly removes the earlier clause correctly but leaves doubled punctuation and lowercase text; that case deliberately gates correction word content, with formatting covered by the separate exact case.

The numeric reference expects `12` and `45 dollars`. Parakeet produces `12` and `$45` (one missing word under this normalization); Nemotron leaves `twelve` and `forty five dollars`. Number words and digits are intentionally distinct. Nemotron also substitutes `cleared` for `clear` in the long passage. These are visible product/model behavior differences, not evidence that speech was misheard in every numeric case.

## Native lifecycle finding

Before isolation, the first Nemotron replay succeeded, but the next replay aborted the process with exit `-1073740791` (`0xC0000409`) and `GGML_ASSERT(prev != ggml_uncaught_exception) failed`. Reproduced with a speech-only short fixture, so silence was not the cause. The harness now starts a separate native worker for every Nemotron replay, still calling the existing streaming pipeline. All 24 measured Nemotron workers completed after containment. This is a benchmark workaround; repeated Nemotron model reloads in the desktop process remain an application/native-library follow-up. Native logs from the original reproduction remain in ignored `local/nemotron-initial.log` and `local/nemotron-short.log` on the test machine.

## Saved baseline comparison

A seven-case [smoke manifest](results/smoke-suite.json) excludes the **known failing punctuation case**, preserving every other original policy and audio fingerprint. Its [saved passing demonstration baseline](results/parakeet-smoke-baseline.json) was compared to a fresh [seven-case run](results/parakeet-smoke-comparison.json). Both exited 0; every WER delta was zero, there was no loss of text matches, and all median latency deltas stayed within the default tolerance. This proves saved-baseline replay and comparison work. It is a partial demonstration baseline, not approval of the full corpus or a release performance gate.

With the generated audio still present, reproduce that comparison from the repository root:

```powershell
npm run bench:transcription -- --manifest benchmarks/transcription/results/smoke-suite.json --output benchmarks/transcription/local/smoke-new.json --machine Ryzen2600X-RTX2060-AC --baseline benchmarks/transcription/results/parakeet-smoke-baseline.json
```

Different fixture hashes, environment labels, models, runtime, repetition counts or policies are rejected; intentionally establish a new reviewed baseline for a different machine/corpus. Baseline compatibility, increased WER, lost exact/normalized matches, latency regressions and changed fallback runtime are also covered by model-free unit tests.

The existing-output guard was exercised: exit 1 and SHA-256 unchanged. A missing-runtime suite exited 1, wrote a report with all eight failures, and did not misclassify missing models as successful silence. Native recognition failures are retained rather than replaced with fabricated transcripts.

## Harness and build verification

Completed in this environment:

- `cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench`: 95 passed, 0 failed, 2 ignored. All four new benchmark tests passed. The installed Nemotron smoke test was then explicitly run with its required local artifact paths and `--ignored`: 1 passed. The remaining ignored live theme-channel test is unrelated to transcription.
- `npm test -- --run`: 9 files, 26 tests passed.
- `cargo check --release --locked --manifest-path src-tauri/Cargo.toml`: passed with default features, confirming the ordinary application still compiles without the harness.
- Cargo formatting, `git diff --check`, and `npm run verify:release-config`: passed. Existing unused streaming-API warnings remain; no dependencies were added.
- Cargo metadata confirms `transcription-bench` is an example requiring `regression-bench`; normal Tauri configuration has no benchmark bundle resources, and the public release workflow does not enable its feature.

Native smoke reproduction (PowerShell):

```powershell
$env:LYRA_RUNTIME_PATH = Join-Path $env:LOCALAPPDATA 'Lyra\runtime\0.1.0\windows-x86_64-cuda'
$env:LYRA_NEMOTRON_MODEL_PATH = Join-Path $env:LOCALAPPDATA 'Lyra\models\nemotron-speech-streaming-en-0.6b.q8_0.gguf'
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench installed_nemotron_stream_smoke -- --ignored
```

## Remaining hardware checks

Before adopting release gates, record consented real microphone fixtures with hand-checked references, then repeat on an idle machine with fixed driver/power settings and more trials. Keep synthetic and microphone results separate. Validate live silence endpointing, manual stop/restart/cancellation, maximum duration, actual streaming partial responsiveness and injection through the existing app acceptance workflow. Neither synthetic fixture replay nor a pause inside a WAV verifies those hardware and session transitions.

The benchmark feature is disabled in the normal application build. The existing release workflow publishes installers, updater files and changelog content, and does not include the benchmark feature, example, fixtures or reports. No public repository or release assets were changed.
