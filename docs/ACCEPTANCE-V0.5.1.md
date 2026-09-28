# Lyra v0.5.1 release acceptance

Lyra v0.5.1 was published on 2026-09-28. The [public release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.1) contains the NSIS installer, updater signature, and manifest. The source tag `v0.5.1` points to commit `4525a413fa509b5b09e643ad34e0113d7534d9cd`.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-28 | Updater-signed publication configuration validated. |
| Frontend build and tests | Pass | `npm run build` succeeded; `npm test -- --run` passed 7 tests in 4 files. | 2026-09-28 | |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed valid and corrupt signature, failed-install preservation, offline update, and temporary configuration cases. | 2026-09-28 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded. | 2026-09-28 | 69 passed, 0 failed, 1 ignored. The ignored test requires the separately installed NeMo runtime and Nemotron model. |
| Main GitHub Windows workflow | Pass | [Run 36384893178](https://github.com/OrionAISystems/Lyra/actions/runs/36384893178) succeeded in 12m58s, including frontend checks, Rust release tests, and the no-bundle Tauri build. | 2026-09-28 | |
| Tagged GitHub release workflow | Pass | [Run 36386016761](https://github.com/OrionAISystems/Lyra/actions/runs/36386016761) succeeded in 8m43s and published the release. | 2026-09-28 | GitHub noted that actions/cache@v4, actions/checkout@v4, and actions/setup-node@v4 target Node 20 and were forced to run on Node 24; this did not fail the workflow. |
| Public release assets and updater manifest | Pass | `Lyra_0.5.1_x64-setup.exe` (2,586,164 bytes), `.sig` (412 bytes), and `latest.json` (1,628 bytes) are present. Manifest version is `0.5.1`; its Windows entries point to the installer, and the embedded updater signature exactly matches the detached `.sig` asset. | 2026-09-28 | GitHub reports installer SHA-256 `b2bfc3e9b2038799688761112352934c64eddc687c27ec1a219042bd17578d51`. Authenticode is `NotSigned` because the optional certificate is not configured; the Tauri updater signature is present. |
| Public changelog mirror | Pass | The public `CHANGELOG.md` Git blob SHA matches the source changelog. | 2026-09-28 | |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| “New line” inserts a line break without leaving recognizer-added punctuation at the start of the next line. | Pending | | | Verify in the installed v0.5.1 app. |
| “New paragraph” starts a paragraph without leaving recognizer-added punctuation at the start of the new paragraph. | Pending | | | Verify in the installed v0.5.1 app. |
| Live microphone input-level test responds to the selected device. | Pending | | | The test is designed not to record or save audio. |
| Installed update from v0.5.0 to v0.5.1 completes and the app reports the new version. | Pending | | | |

Release automation and public artifact checks do not establish these installed-app or hardware behaviors. Record each result only after directly observing it.
