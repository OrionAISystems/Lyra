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
| Version, changelog, release configuration and feed separation | Pass | Local `verify:version` (0.5.6), `verify:changelog` (16 entries), `verify:release-config`, `verify:themes`; main CI |
| Frontend type check, tests and production build | Pass | Local `typecheck`, `test` (26 tests across 9 files), `build`; main CI |
| Release-mode Rust tests with regression harness, formatting and default-feature check | Pass | Local `cargo test --release --locked --features regression-bench` (139 passed, 2 explicitly ignored), `cargo fmt --check`, and default-feature `cargo check --release --locked`; manifest path `src-tauri/Cargo.toml` |
| Updater signature/offline/failed-install fixture | Pass | Local `npm run test:updater-fixture`: valid signature, corruption rejection, preserved installed bytes on simulated failure, offline checks and ephemeral configuration |
| Signed seasonal fixture through production Rust verifier | Pass | Local `npm run test:themes-fixture` and main CI: valid catalog, signature/artwork rejection and offline cache repair |
| Existing public theme channel before application tag | Pass | Local `npm run verify:public-themes -- --latest-version 0.5.5`: catalog revision 1, one theme, two PNGs (322,765 bytes); stable latest stayed v0.5.5 |
| Exact release commit Windows verification and unbundled Tauri build | Pass | [Run 37236294270](https://github.com/OrionAISystems/Lyra/actions/runs/37236294270) at `bb2da5272eabb5c2b88c51ab9b3cb2a822fa8c4f` |
| Tagged NSIS/updater publication | Pass | [Run 37238037304](https://github.com/OrionAISystems/Lyra/actions/runs/37238037304); annotated `v0.5.6` points to `bb2da5272eabb5c2b88c51ab9b3cb2a822fa8c4f`; [stable release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.6) published 2026-10-04 at 18:03:58 EDT |
| Default-feature release Rust suite in tagged workflow | Pass | 129 passed, 2 explicitly ignored; the internal regression harness is not enabled in the shipping build |
| Windows Authenticode signing | Not configured | The workflow reported no Windows certificate and produced an installer without Authenticode signing; its separate Tauri updater signature was verified below |
| Public changelog, installer/signature, latest.json and existing theme assets | Pass | `npm run verify:public-release -- --version 0.5.6`: stable latest v0.5.6, `Lyra_0.5.6_x64-setup.exe` (3,024,849 bytes), pinned-key Tauri signature, matching manifest/platform URLs, signed theme revision 1 and two PNGs |
| Exact public changelog mirror | Pass | Public `CHANGELOG.md` Git blob equals tagged canonical source: `fb1eec53f335beb8f164858a39caebea83e70f9f` |

Installer SHA-256: `79f041688e7501ab6c38a460d4991510c451e5f95171a76e8d5dea04f6c3d309`. The release contains only `latest.json`, the Windows NSIS installer and its `.sig` companion. No private source, benchmark reports/fixtures, runtime or model artifacts were published.

The ignored Rust tests require separately installed Nemotron runtime/model artifacts and a live published theme channel, respectively. The separate public theme verifier passed; it is not an installed-app test. Existing native/streaming unused-code warnings remain. No dependency upgrades or recognition fixture baseline replacement occurred in release preparation.

## Installed Windows and hardware checks

Physical microphone rapid start/stop, device disconnect/recovery, cancellation during native/cloud recognition, repeated hotkeys, cross-application paste and clipboard contention, and the installed v0.5.5-to-v0.5.6 updater remain **pending**. No running user installation, settings or model artifacts are changed by these gates. Cancellation suppresses results; it does not interrupt a running native call or cloud request. The full checklist and remaining resource/concurrency limits are recorded in [RELIABILITY-PASS.md](RELIABILITY-PASS.md).
