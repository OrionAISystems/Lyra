# Initial benchmark evidence — 2026-10-04

This pre-fix snapshot established the punctuation and number defects. Current behavior, candidates and verification are in [CLEANUP-RESULTS.md](CLEANUP-RESULTS.md).

Parakeet passed **7/8** starter cases; Nemotron passed **6/8**. Both CLI runs exited 1 as intended. The unchanged [Parakeet](results/parakeet.json) and [Nemotron](results/nemotron.json) reports contain raw/final transcripts, scores, timings, pinned hashes and source revisions.

| Case | Parakeet final WER | Nemotron final WER | Outcome |
| --- | ---: | ---: | --- |
| Silence | undefined | undefined | Both passed explicit no-speech policy |
| Short dictation | 0% | 0% | Both passed |
| Spoken punctuation | 0% | 0% | Both failed exact text |
| Numbers | 11.11% | 33.33% | Parakeet passed; Nemotron failed 15% limit |
| Proper names | 0% | 0% | Both passed normalized comparison |
| Long dictation | 0% | 0.97% | Both passed |
| Pause/resume | 0% | 0% | Both passed |
| Self-correction | 0% | 0% | Word content passed; formatting issue remained |

Both engines supplied automatic punctuation after spoken `period`; cleanup then produced `report..`. Parakeet also left the sentence after a newline lowercase. WER normalization hid these differences, while the exact-text gate caught them.

The original numeric reference expected `12` and `45 dollars`. Parakeet supplied `12` and `$45`; Nemotron retained `twelve` and `forty five dollars`. The subsequent agreed policy uses digits and currency symbols. Nemotron also substituted `cleared` for `clear` in the long passage.

## Method and limitations

Windows x86_64, Ryzen 2600X, RTX 2060 6 GiB, NVIDIA 616.64, AC power, verified CUDA runtime 0.1.0 and pinned Q8 models. Nothing was downloaded. Each engine replayed eight cases three times after a separate first replay. Lyra and builds/tests were active during these initial measurements, so their latency is functional evidence under contention, not a calibrated performance target.

Every speech fixture was **synthetic Microsoft David Desktop speech**, with digitally generated silence. [Provenance](results/synthetic-provenance.json) records generation. No reusable real microphone recordings were found.

Repeated Nemotron model reloads initially aborted the process with `0xC0000409`. Isolated workers completed all 24 measured trials; this benchmark workaround does not repair the application lifecycle. Original diagnostic logs and obsolete intermediate/smoke reports remain in ignored `local/` and `local/archive/`.

Historical reports use an older warmup policy and are evidence only, not compatible current baselines. Microphone quality, endpointing, cancellation and focus-safe insertion remain hardware-dependent checks.
