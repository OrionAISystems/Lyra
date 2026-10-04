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

Historical baselines above retain the earlier amplitude-based warmup context. Current comparison rejects them rather than comparing potentially different startup inputs; see the warmup policy contract in [benchmark usage](README.md). Establish a new reviewed report under `expected-transcript-v1` before current comparisons.

New semantic-warmup candidates are [Parakeet](results/parakeet-warmup-v1-candidate.json) and [Nemotron](results/nemotron-warmup-v1-candidate.json), recorded on `a49987b`. These are explicit review candidates, not automatic replacement of historical baselines. All 39 processed transcripts per engine match the historical reports. Sequential idle comparison runs passed **13/13 cases and 39 trials each**, zero WER deltas, no comparison failures, and unchanged latency tolerances. Warmup measured 512.0 ms against the 523.8 ms Parakeet candidate and 694.6 ms against the 710.6 ms Nemotron candidate. Ignored local comparison reports are `parakeet-review-8-idle-comparison.json` and `nemotron-review-8-idle-comparison.json`. An earlier overlapped run failed the Parakeet warmup gate; those GPU-contended measurements are retained in `local/` and excluded from the idle candidates. No tolerance was loosened.

Replay the current corpus against either new candidate baseline (review these candidates before adopting them):

```powershell
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/parakeet-review.json --machine Ryzen2600X-RTX2060-NVIDIA616.64-AC --baseline benchmarks/transcription/results/parakeet-warmup-v1-candidate.json
npm run bench:transcription -- --manifest benchmarks/transcription/suite.json --output benchmarks/transcription/local/nemotron-review.json --machine Ryzen2600X-RTX2060-NVIDIA616.64-AC --engine nemotron --baseline benchmarks/transcription/results/nemotron-warmup-v1-candidate.json
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

The ninth review identified attached structured suffixes, interleaved punctuation marks, unsupported complete minor clauses, repeated cardinal connectors, and incomplete environment labels. Focused tests reproduced the three quantity failures before fixing complete-span preservation and query/fragment suffix guards. Spoken cues now consume alternating horizontal whitespace/marks on both sides, preserving literal punctuation without cues. The full feature-enabled release suite passed **131 tests, 0 failed, 2 ignored**, plus default-feature release check and formatting. `nvidia-smi` verified RTX 2060 / driver **616.64** in the same environment; the two not-yet-adopted candidate labels were amended to include that driver, without changing recorded timings, transcripts or acceptance policy. Historical reports remain untouched, and current replay commands include driver identity.

The eighth review identified missing versioning for semantic warmup selection and URL queries exposed by punctuation adjacency. Reports now carry `expected-transcript-v1` in their context, rejecting historical or different warmup policies while retaining their evidence. Attached structured URL/domain/path/identifier prefixes preserve query and path punctuation. The full feature-enabled release suite passed **127 tests, 0 failed, 2 ignored**, plus default-feature release check and formatting. Tests cover legacy/current policy compatibility and URL/query/path counterexamples.

The seventh review identified repeated minor annotations, missing spaces after clause punctuation, amplitude-dependent warmup selection, and mutable fixture rereads in Nemotron workers. Cleanup now preserves repeated minor denominations and permits quantities after unambiguous clause punctuation while protecting dot/colon identifier prefixes. Warmup uses the first expected speech case, with explicit typed no-speech acceptance for silence-only suites. Workers stage snapshots of the exact fingerprinted bytes and remove them after success/failure. Tests cover source mutation and create-new protection as well as semantic warmup selection. The full feature-enabled release suite passed **125 tests, 0 failed, 2 ignored**, plus default-feature release check and formatting. A fresh native replay on preceding revision `35b26ff` passed all 13 synthetic cases / 39 trials per engine, zero WER deltas, including the new warmup latency gate; those ignored local reports supplement the historical committed reports without replacing baselines.

The sixth review identified non-idempotent repeated currency annotations, sentence recasing of hyphenated mixed-case names, and an ungated first-replay warmup measurement. Repeated annotations now remain wholly literal; sentence casing scans word-internal hyphens; baseline comparison gates warmup with the existing latency tolerance and requires an identical ordered corpus before comparing startup timing. Regression tests cover matching symbol/name pairs, lowercase hyphenated words, self-corrections, threshold boundaries, explicit tolerance changes and reordered corpora. The full feature-enabled release suite passed **121 tests, 0 failed, 2 ignored**, plus default-feature release check and formatting. Native replay measurements remain historical evidence; the added startup gate is verified deterministically.

The fifth review supplied four additional reproducible inputs: conflicting trailing currency symbols, minor units after all-zero fractional major amounts, attached Windows path separators, and singular `cent`/`penny` clauses. All four failed focused tests before the fixes and passed afterward. Currency annotation checks now recognize symbols in both major and minor conflicts; whole-valued decimal currency amounts combine minor units; identifier tails protect backslashes; singular minor units retain the existing currency compatibility checks. The full feature-enabled release suite passed **118 tests, 0 failed, 2 ignored**, plus default-feature release check and formatting. Native replay evidence remains unchanged.

The first connector review identified five concrete edge cases. The follow-up requires attached hyphens before combining number words (including currency minor units), stops monetary cues at line breaks, preserves attached dotted identifiers, replaces conflicting marks before spoken punctuation cues, and retains fixture generation provenance across extensions. Focused regression inputs cover subtraction, valid hyphenated amounts, CRLF/paragraph boundaries, domain/version components and terminal sentence periods.

The updated full feature-enabled release Rust suite passed **104 tests, 0 failed, 2 ignored**. An isolated Windows PowerShell 5.1 / Microsoft David Desktop generator check passed initial generation, legacy metadata migration, repeated extensions, unchanged retained WAV bytes, and duplicate-fixture preflight without modifying the original corpus. The native replay reports above remain the earlier immutable evidence; these additional edge cases were verified with deterministic cleanup tests, rather than presented as new microphone or recognizer measurements. Contributor and evidence-based review guidance now lives in the root [AGENTS.md](../../AGENTS.md).

The second connector review used that branch's `AGENTS.md` and identified three further formatter gaps. All three were reproduced as failing tests before fixing symbol-prefixed major/minor currency amounts, capitalized unit-like titles, and adjacent conflicting named currencies. The resulting full release suite passed **107 tests, 0 failed, 2 ignored**, including negative-symbol amounts, wrong minor-unit currencies, line breaks, lowercase unit quantities, and punctuation-separated denominations as counterexamples. Current native replay evidence remains unchanged; this follow-up adds deterministic formatting evidence.

The third review identified incomplete conflict handling inside minor-currency clauses, uppercase unit abbreviations mistaken for titles, and self-correction style detection before opening delimiters. Minor clauses are now parsed before committing the major rewrite; incompatible denominations preserve their full source span. Recognized uppercase short units such as GB/MB remain quantities, and self-correction shares the existing sentence-content boundary helper. The full release suite passed **110 tests, 0 failed, 2 ignored**; the default-feature release check passed. Counterexamples cover multiword conflicting minor amounts, punctuation/newline-separated denominations, preserved title words, Unicode/opening quotes, lowercase clauses and mixed-case replacements. Native measurements above remain the earlier snapshots.

The fourth review's useful findings were fixed with a bounded monetary-cue grammar through short copulas/modifiers/payment pronouns, mixed-case name preservation across typographic apostrophes, and quantity conversion inside smart single quotes. The date objection was resolved as a policy clarification: ordinary multiword cardinal conversion beside a month preserves date layout and follows the selected number rule; written numeric dates remain literal and calendar interpretation is not added. Tests preserve that accepted behavior. Final full release validation passed **114 tests, 0 failed, 2 ignored** plus the default-feature check. No native corpus, acceptance tolerance or saved baseline was replaced during these follow-ups.
