# Punctuation and canonical quantity cleanup — 2026-10-04

Both installed engines now pass **all 13 fixtures**, three trials per case. Final saved-baseline comparisons also pass with the original zero-WER-increase policy and default latency tolerances. The historical seven-case Parakeet baseline still passes. No application UI, dependency, settings migration, model download, installer or public release change was required.

## What changed

The shared domain cleanup now consumes recognizer punctuation attached to spoken comma/period commands instead of adding a second mark. A spoken period restores the following sentence's initial uppercase letter, including across a newline. Self-correction preserves the original clause's initial sentence casing. Literal ellipses, versions, decimals and URLs remain intact; mixed-case words and later user corrections retain their intentional spelling.

The new `quantities.rs` parser applies the selected number policy while spoken formatting is enabled: clear quantities and standalone multiword English number phrases become digits, and explicit currencies use symbols. It handles signs, decimals, percentages and major/minor currency units with integer/string arithmetic. Single ambiguous words, unsupported number sequences and identifier-like text are preserved. Weight/currency `pounds` is resolved conservatively. The full grammar, opt-out and ambiguity limits are documented in [TEXT-FORMATTING.md](../../docs/TEXT-FORMATTING.md).

| Case | Previously observed | Current final output, both engines |
| --- | --- | --- |
| Spoken punctuation | `report..`; Parakeet also emitted lowercase `please` after the newline | `Hello, Orion.\nPlease send the report.` |
| Number quantities | Nemotron retained `twelve` and `forty five dollars` | `The order contains 12 boxes and costs $45.` |
| Self-correction | `send the report on Friday..` | `Send the report on Friday.` |
| Signed decimal | New fixture | `The temperature is -12.5 degrees.` |
| Currency minor units | New fixture | `The total is $45.50.` |
| Percentage | New fixture | `The discount is 12.5%.` |
| Standalone multiword numbers | New fixture | `We counted 25 and then 105.` |
| Ambiguous prose | New fixture | `I need one more thing before we finish.` |

The original eight audio files were retained. Five new synthetic Microsoft David Desktop fixtures were added; [their generation provenance](results/cleanup-provenance.json) records the selected case IDs. The numeric reference deliberately changed from `45 dollars` to `$45` to match the chosen currency policy, and its maximum WER was strengthened from 15% to **0%**. Negative-decimal, currency and percentage fixtures require exact text, so normalized WER cannot hide lost signs or wrong symbols. Historical pre-fix reports remain unchanged in [RESULTS.md](RESULTS.md).

## Final native replay evidence

Same Windows x86_64 / Ryzen 5 2600X / RTX 2060 6 GiB / NVIDIA 616.64 environment and verified CUDA 0.1.0 artifacts as the initial run. Lyra remained running. These are synthetic fixture replay results, not microphone recordings or calibrated engine speed comparisons. Parakeet timing is warm after its first replay; Nemotron timing includes a new model load per isolated worker and excludes worker startup. Text cleanup/scoring is outside the recognition timer.

| Case | Parakeet median ms | Nemotron median ms | Final-output gate |
| --- | ---: | ---: | --- |
| Silence | 19.6 | 646.3 | Both pass explicit no-speech policy |
| Short dictation | 27.7 | 685.8 | Both pass |
| Spoken punctuation | 38.3 | 793.7 | Both pass exact text |
| Number quantities | 35.2 | 849.2 | Both pass 0% WER |
| Proper names | 34.7 | 800.2 | Both pass |
| Long dictation | 310.0 | 3,188.1 | Both pass; Nemotron retains its existing one-word error |
| Pause/resume | 38.8 | 897.7 | Both pass |
| Self-correction | 41.4 | 891.9 | Both pass; final text also matches exactly |
| Signed decimal | 31.6 | 792.6 | Both pass exact text |
| Currency minor units | 30.7 | 775.5 | Both pass exact text |
| Percentage | 26.3 | 717.0 | Both pass exact text |
| Standalone multiword numbers | 28.6 | 779.9 | Both pass 0% WER |
| Ambiguous number prose | 26.3 | 720.4 | Both pass 0% WER |

Final reports: [Parakeet comparison](results/parakeet-cleanup-comparison.json), [Nemotron comparison](results/nemotron-cleanup-comparison.json). Each contains 39 measured trials, `passed: true`, and an empty comparison-failure list. No accuracy/latency tolerance was relaxed. All final WER deltas are zero. Nemotron still recognizes `cleared` instead of `clear` in the 103-word long passage (1/103 WER, 0.97%); that existing recognition error is below the unchanged 10% long-passage gate, and this pass does not claim perfect transcription.

Saved candidate baselines are [Parakeet](results/parakeet-cleanup.json) and [Nemotron](results/nemotron-cleanup.json). The [historical seven-case comparison](results/cleanup-historical-comparison.json) also passed with current code. These are development baselines for this hardware/corpus, not approval of a public release or real microphone quality.

Replay the current corpus against either saved baseline:

```powershell
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/parakeet-review.json --machine Ryzen2600X-RTX2060-AC --baseline benchmarks/transcription/results/parakeet-cleanup.json
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/nemotron-review.json --machine Ryzen2600X-RTX2060-AC --engine nemotron --baseline benchmarks/transcription/results/nemotron-cleanup.json
```

## Saved-baseline precision fix

The repeat run exposed a harness defect: JSON parsing changed `1/103` from `0.00970873786407766906` to `0.00970873786407766733`. Comparing the newly computed float against the parsed float falsely reported a WER increase even though all edit counts and transcripts were identical. The comparison and delta code now recompute WER from the saved integer edit/reference-word counts, which are the source of truth. A round-trip regression test locks this down; genuine accuracy and text-match regressions still fail. The original false-positive report is retained in ignored `local/nemotron-cleanup-comparison.json` on this machine.

## Verification and remaining limits

- Full feature-enabled release Rust suite: **103 passed, 0 failed, 2 ignored**. This includes punctuation, sentence/correction casing, mixed spoken/written numbers, exact amounts, preservation boundaries, formatting/profile opt-out, personalization order, idempotence, and baseline round-trip/regression tests.
- Default-feature release `cargo check`: passed, confirming the normal application compiles with the new domain formatter and without the internal harness.
- Rust formatting, `git diff --check` and changelog validation: passed. Existing unused streaming-API warnings remain.
- Both final 13-case native suites and their saved-baseline comparisons: passed; the historical Parakeet comparison also passed.

Live microphone capture, accents/noise, silence endpointing, actual stop/restart/cancellation and cross-application paste still need the existing hardware-backed app acceptance checks. The existing Nemotron repeated-model-load abort remains contained by benchmark workers; this pass does not fix the desktop native lifecycle. The frontend was unchanged. The formatting switches/profile preferences provide the existing opt-out for literal dictation, and standalone multiword conversions retain the documented risk for lowercase number-like names/idioms.

## PR review follow-up

The first connector review identified five concrete edge cases. The follow-up requires attached hyphens before combining number words (including currency minor units), stops monetary cues at line breaks, preserves attached dotted identifiers, replaces conflicting marks before spoken punctuation cues, and retains fixture generation provenance across extensions. Focused regression inputs cover subtraction, valid hyphenated amounts, CRLF/paragraph boundaries, domain/version components and terminal sentence periods.

The updated full feature-enabled release Rust suite passed **104 tests, 0 failed, 2 ignored**. An isolated Windows PowerShell 5.1 / Microsoft David Desktop generator check passed initial generation, legacy metadata migration, repeated extensions, unchanged retained WAV bytes, and duplicate-fixture preflight without modifying the original corpus. The native replay reports above remain the earlier immutable evidence; these additional edge cases were verified with deterministic cleanup tests, rather than presented as new microphone or recognizer measurements. Contributor and evidence-based review guidance now lives in the root [AGENTS.md](../../AGENTS.md).

The second connector review used that branch's `AGENTS.md` and identified three further formatter gaps. All three were reproduced as failing tests before fixing symbol-prefixed major/minor currency amounts, capitalized unit-like titles, and adjacent conflicting named currencies. The resulting full release suite passed **107 tests, 0 failed, 2 ignored**, including negative-symbol amounts, wrong minor-unit currencies, line breaks, lowercase unit quantities, and punctuation-separated denominations as counterexamples. Current native replay evidence remains unchanged; this follow-up adds deterministic formatting evidence.
