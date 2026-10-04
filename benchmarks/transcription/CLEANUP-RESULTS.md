# Transcription cleanup and benchmark results

The shared cleanup fixes doubled spoken punctuation, sentence/self-correction casing, and the agreed canonical quantity formatting. For example, `report period.` becomes `report.`, `twelve boxes` becomes `12 boxes`, and `forty five dollars` becomes `$45`. See [formatting policy](../../docs/TEXT-FORMATTING.md) and [benchmark usage](README.md).

The internal benchmark calls the existing local transcription pipeline. Its opt-in feature stays off in application builds; no application UI, dependency, model download or public-release artifact was added.

## Verification

- Feature-enabled release Rust suite: **134 passed, 0 failed, 2 ignored**.
- Default-feature release check, Rust formatting and whitespace checks: passed.
- Isolated Windows/System.Speech generator checks: passed retained WAV/provenance, repeated extensions, legacy migration and duplicate preflight.
- Installed Parakeet and Nemotron: **13/13 synthetic accuracy cases, 39 measured trials each**, with zero WER deltas against the candidate baselines.
- Native replay evidence is from `0599070`; subsequent bounded grammar and generic silence acceptance changes were verified by deterministic tests. Frontend unchanged.

The corpus covers silence, punctuation, quantities, names, long dictation, pause/resume, explicit self-correction, signed decimals, currency, percentages and ambiguous prose. All speech is **synthetic Microsoft David Desktop audio**, not microphone recordings. Original WAVs were retained; [initial provenance](results/synthetic-provenance.json) and [extension provenance](results/cleanup-provenance.json) identify generation.

## Baselines and latency

Current review candidates: [Parakeet](results/parakeet-warmup-v1-candidate.json) and [Nemotron](results/nemotron-warmup-v1-candidate.json). They use `expected-transcript-v1`, Windows x86_64, Ryzen 2600X, RTX 2060, NVIDIA 616.64, AC power and verified CUDA 0.1.0 artifacts. Review candidates before adopting them; the tool never replaces a baseline automatically.

Sequential idle comparisons on `a49987b` passed every gate. On `0599070`, Nemotron passed again. Parakeet's first startup failed at **722.9 ms** against a 523.8 ms baseline and unchanged 704.7 ms threshold; one confirmation passed at **532.4 ms**. Both outcomes remain in ignored `local/parakeet-review-9.json` and `parakeet-review-9-confirmation.json`. Startup evidence is **not uniformly green**; no tolerance or baseline was adjusted to hide that failure. Warmup is one startup sample, distinct from repeated warm-case medians.

```powershell
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/parakeet-review.json --machine Ryzen2600X-RTX2060-NVIDIA616.64-AC --baseline benchmarks/transcription/results/parakeet-warmup-v1-candidate.json
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/nemotron-review.json --machine Ryzen2600X-RTX2060-NVIDIA616.64-AC --engine nemotron --baseline benchmarks/transcription/results/nemotron-warmup-v1-candidate.json
```

Run engines sequentially on idle hardware. Historical reports lacking the warmup policy are incompatible with current comparisons. The [initial failure evidence](RESULTS.md) remains in Git; redundant intermediate and obsolete smoke snapshots were archived under ignored `local/archive/`.

WER comparisons derive ratios from integer edit/reference-word counts to avoid false JSON float-roundtrip regressions. Zero-word references require exact text, so punctuation-only hallucinations cannot pass silence. Nemotron still has one existing long-passage error (`cleared` for `clear`, 1/103 words), below its unchanged 10% gate.

## Hardware limits

Microphone quality, accents/noise, live silence endpointing, cancellation/restart, maximum-duration capture and cross-application paste need the existing hardware-backed acceptance checks. A pause inside a WAV does not validate those transitions.

Nemotron workers replay immutable WAV snapshots because the installed native library aborts on repeated model loads in one process. This contains the benchmark failure; it does not fix the desktop recognizer lifecycle. Successful worker timing excludes process startup and file staging. Text cleanup/scoring is outside recognition timing.
