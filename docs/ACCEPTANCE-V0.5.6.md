# Lyra v0.5.6 release acceptance

Release date: 2026-10-04 (America/New_York). This maintenance release ships the changes merged in [PR #7](https://github.com/OrionAISystems/Lyra/pull/7), [PR #9](https://github.com/OrionAISystems/Lyra/pull/9), and [PR #8](https://github.com/OrionAISystems/Lyra/pull/8).

## Included changes

- PR #7: seasonal cache recovery, redundant artwork refresh and validation hardening.
- PR #9: cancellation/result commit guards, microphone-error endpointing, atomic settings persistence and shortcut rollback fixes. See [the reliability handoff](RELIABILITY-PASS.md).
- PR #8: shared quantity, punctuation and casing cleanup plus an internal regression harness. The harness stays behind `regression-bench` and is excluded from the default app and public assets. Synthetic evidence is in [the benchmark results](../benchmarks/transcription/CLEANUP-RESULTS.md); it does not prove microphone or paste behavior.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| PR #9 Windows verification | Pass | [Run 37234976937](https://github.com/OrionAISystems/Lyra/actions/runs/37234976937); merged as `1fce7f8` |
| PR #8 Windows verification after incorporating #9 | Pass | [Run 37235498447](https://github.com/OrionAISystems/Lyra/actions/runs/37235498447); merged as `ba507c8` |
| Version, changelog, release configuration and feed separation | Pending | Local v0.5.6 gates |
| Frontend type check, tests and production build | Pending | Local v0.5.6 gates |
| Release-mode Rust tests with regression harness, formatting and default-feature check | Pending | Local v0.5.6 gates |
| Updater signature/offline/failed-install fixture | Pending | Local v0.5.6 gates |
| Signed seasonal fixture through production Rust verifier | Pending | Local v0.5.6 gates |
| Existing public theme channel before application tag | Pending | Stable application latest must remain v0.5.5 |
| Tagged NSIS/updater publication and public artifacts | Pending | Tag `v0.5.6`, release workflow and public verifier |

## Installed Windows and hardware checks

Physical microphone rapid start/stop, device disconnect/recovery, cancellation during native/cloud recognition, repeated hotkeys, cross-application paste and clipboard contention, and the installed v0.5.5-to-v0.5.6 updater remain **pending**. No running user installation, settings or model artifacts are changed by these gates. Cancellation suppresses results; it does not interrupt a running native call or cloud request. The full checklist and remaining resource/concurrency limits are recorded in [RELIABILITY-PASS.md](RELIABILITY-PASS.md).
