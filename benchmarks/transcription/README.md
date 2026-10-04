# Internal transcription regression benchmark

This development tool lives with the private Lyra source so it calls the real `TranscriptionRouter`, local Parakeet backend, Nemotron streaming replay, and `process_transcription_text`. It adds no application UI. The `regression-bench` Cargo feature is off by default; its example executable is not a Tauri bundle binary. The release workflow does not enable this feature or upload these sources, fixtures, reports, or baselines to Lyra-Releases. CI tests the harness without native models; recognition runs are an explicit developer task.

The existing in-app one-phrase microphone benchmark remains separate. It stores numeric scores but never audio; no reusable recordings were found in this checkout. This harness does not capture microphones, write app settings, download models, use credentials, call cloud recognition, or paste text. It reads installed verified artifacts from `%LOCALAPPDATA%\Lyra` (or `--root`). Native Parakeet runtime fallback is retained and recorded per trial.

## Run

The current corpus includes the punctuation/quantity cleanup regression cases; see [CLEANUP-RESULTS.md](CLEANUP-RESULTS.md) for current results and [the formatting contract](../../docs/TEXT-FORMATTING.md) for canonical number rules. Historical results below remain snapshots of the original run.

From the repository root on Windows, generate the starter corpus once:

```powershell
powershell.exe -NoProfile -File benchmarks/transcription/generate-synthetic.ps1
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/parakeet-1.json --machine Ryzen2600X-RTX2060-AC
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/nemotron-1.json --machine Ryzen2600X-RTX2060-AC --engine nemotron
```

Use an installed Windows desktop voice with `-Voice 'Microsoft David Desktop'`. The generator uses Windows PowerShell 5.1/.NET Framework `System.Speech`; it fails rather than overwriting fixtures. It emits PCM16 mono 16 kHz WAVs plus `audio/provenance.json`. Voice/runtime changes can alter generated audio; retain the same bytes for comparisons. All starter speech is **synthetic**, not microphone audio. Silence is two seconds of digitally generated zeros. The interrupted case contains a 1.5-second pause and continuation; it does not simulate cancellation or capture endpointing.

Approve a **passing** report after reviewing its transcripts, then copy it to an explicit local baseline path. The tool never overwrites output files or updates baselines automatically:

```powershell
Copy-Item benchmarks/transcription/local/parakeet-1.json benchmarks/transcription/local/parakeet-baseline.json
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/parakeet-2.json --machine Ryzen2600X-RTX2060-AC --baseline benchmarks/transcription/local/parakeet-baseline.json
```

The process exits 1 on any case failure, warmup failure, incompatible baseline, or regression; JSON is still written for recognition/gate failures. Missing fixtures or invalid CLI/manifest/WAV inputs fail before recognition. Run `npm run bench:transcription -- --help` for options. Required `--machine` should identify CPU, GPU, power mode and relevant driver configuration; compare on the same idle machine with Lyra closed. Default repetitions: 3; range: 1–20. Use release builds for meaningful timings.

When extending an existing generated corpus, use `-CaseId` to generate only new fixtures without overwriting retained audio. The provenance sidecar keeps an append-only `generations` history (schema 2), retaining each batch's voice, timestamp and completed case IDs. Legacy metadata is preserved verbatim when extending an older corpus. Selected existing WAVs fail before generation; provenance is saved after each completed case so partial runs retain their evidence. For example:

```powershell
powershell.exe -NoProfile -Command "& './benchmarks/transcription/generate-synthetic.ps1' -CaseId @('decimal','currency','percentage','standalone-numbers','number-prose')"
```

## Fixtures and policy

`suite.json` pairs each relative audio path with an expected **final processed** transcript, ID, category, provenance (`synthetic` or `microphone`), maximum WER, and optional exact-text requirement. Empty-reference cases may explicitly set `accept_empty_recording: true` to accept Lyra's typed no-speech outcome, while keeping the native error visible. Schema 1 rejects unknown fields and duplicate IDs. WAV input must be complete RIFF PCM16 mono at 16 kHz, no more than five minutes; optional RIFF metadata and padded chunks are accepted. Convert/downmix recordings deliberately before running; the runner does not change their sample rate. References and audio bytes are hashed along with acceptance policy, so changing a fixture invalidates its baseline.

To add real recordings, create a separate manifest under ignored `local/`, use `provenance: "microphone"`, and store consented audio alongside it. Keep source mic/device, room/noise, speaker/language, input gain, conversion method and manual reference verification in a sidecar record. Do not relabel generated speech as microphone speech. Reports contain actual/reference transcripts and free-form native errors; keep private recordings and their reports out of Git. `audio/`, `local/`, and WAV files are ignored. The checked-in results contain only the public, invented synthetic prompts.

The thirteen cases cover short speech, silence hallucinations, spoken punctuation/newline commands, canonical numbers, signed decimals, currency minor units, percentages, standalone multiword numbers, preserved single-number prose, proper names, a roughly 100-word passage, pause/resume, and explicit self-correction. Personalization is fixed: formatting and self-correction enabled, no corrections/snippets, no preceding character, English. This isolates recognition and built-in cleanup from a user's preferences. Raw text and raw scores remain available to distinguish recognizer errors from cleanup errors.

## Metrics and baseline contract

- WER is token edit distance divided by reference token count; insertions can make it exceed 1. Unicode lowercasing and splitting at every non-alphanumeric character normalize case, punctuation and whitespace. Decimal separators and apostrophes split tokens. Number words are **not** equated with digits; name aliases are **not** forgiven.
- Both raw and final processed outputs report edit count, reference words, WER, normalized match and exact match. Exact match preserves case, punctuation and line breaks. Spoken punctuation has an exact gate, so normalization cannot hide a formatting regression.
- For silence, WER is `null` (undefined denominator), and any recognized word fails. Its exact-empty gate also catches punctuation-only hallucinations. The explicitly accepted `EmptyRecording` outcome keeps `score: null`, the error, and `expected_empty_recording: true`; it is not presented as a successful empty transcript. Other errors always fail.
- Latency uses an external monotonic timer around the router call after request construction. It includes native model initialization/retry and streaming setup/drain/close, but excludes file reading, scoring, text cleanup and simulated recording time. A separate first replay measures warmup/load cost; Parakeet remains warm thereafter. Nemotron's current pipeline loads a recognizer for each replay, so that cost remains in every trial. Nemotron replays run in individual child processes because the pinned library aborts on a second model load in one process in this environment. Successful trial timing excludes process startup; aborted workers become failed trials with exit status and diagnostic tail. Temporary worker JSON files are deleted after reading. This benchmark containment does not fix or validate the app's repeated Nemotron session lifecycle.
- Each case reports nearest-rank median, p95, and median recognition/audio real-time factor. Three trials make p95 the maximum; use more repetitions for a meaningful tail estimate. Nemotron feeds existing 1,600-sample chunks as fast as possible; partial count is diagnostic, not a real-time responsiveness measurement.
- Baselines must pass and match schema, normalization/processing policy, engine, machine label, OS/architecture, GPU, pinned model/runtime hashes, active runtime and repetitions. Cases must have the same fingerprints and trial backend/runtime paths. Source revision may differ: that is the change being tested.
- Defaults allow no worst-trial WER increase or loss of normalized/exact matches. Comparison and WER deltas recalculate ratios from integer edit/reference-word counts, so JSON floating-point round-trip differences cannot invent a regression. Both first-replay warmup latency and per-case median latency fail above `old * 1.25 + 50 ms`; `--wer-delta`, `--latency-ratio`, and `--latency-ms` adjust tolerances explicitly. The complete ordered fixture corpus must match for warmup comparison. Warmup selects the first nonempty expected transcript, independent of recording gain; a silence-only suite uses its first case and accepts only explicitly allowed typed no-speech results. Warmup is a single startup measurement, distinct from the repeated warm case medians; compare under the same idle hardware/power conditions. Absolute case gates always apply. Reports include per-case baseline latency/WER deltas and failure reasons. Model/fixture changes require an intentionally reviewed new baseline.

Nemotron workers receive temporary WAV snapshots of the same bytes used for fingerprints and audio duration, so source-file edits during a run cannot alter later trials. Snapshots use unique, create-new paths and are removed after each worker finishes, including failed workers; file staging is outside successful recognition timing. An abrupt process termination can leave a temporary snapshot requiring manual cleanup.

Report context records `warmup_policy: expected-transcript-v1`. Historical reports without this policy are readable but incompatible with current baseline comparison: their amplitude-selected warmup may have replayed a different fixture. Establish a separately reviewed passing baseline under the current policy; retain historical reports rather than editing their context to claim new semantics.

Runtime/model hashes describe the app's pinned, verified installation markers, not a fresh full-file integrity audit. The benchmark uses Lyra's readiness checks and does not modify installation state.

## Verification and hardware checks

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench
cargo check --release --locked --manifest-path src-tauri/Cargo.toml
powershell.exe -NoProfile -File benchmarks/transcription/test-generator.ps1
```

Unit tests cover WER insertions/deletions, Unicode/case/punctuation, undefined silence WER, numbers, WAV corruption/rates/padding, baseline compatibility, backend fallback changes, accuracy, text matches and latency gates. Model-free CI runs these along with the existing application tests. The generator regression script uses an installed Windows desktop voice in an isolated temporary corpus to check retained audio/provenance, legacy and repeated extensions, and duplicate preflight; it needs Windows PowerShell 5.1 but no recognition model. Recognition needs installed native models and compatible GPU/CPU drivers; failures remain visible in reports.

Microphone quality, noise/accents, actual stop/restart/cancellation, silence auto-stop, maximum-duration capture, live Nemotron partial timing and focus-safe injection require hardware-backed app checks. Fixture replay bypasses `cpal`, capture state, endpoint timing and injection. In particular, a pause inside a WAV is not evidence that Lyra's silence endpoint would retain the continuation. Evaluate those separately using the existing acceptance documents, with approved recordings and fixed settings.

See [RESULTS.md](RESULTS.md) for this environment's measured evidence and limitations.
