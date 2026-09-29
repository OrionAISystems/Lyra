# Lyra v0.5.3 release acceptance

v0.5.3 groups Insights by the computer's local calendar day. Retained session metadata from older UTC daily buckets is relocated on upgrade; lifetime totals are preserved. Aggregates without retained timestamps remain on their original dates.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-28 | Updater-signed publication configuration validated; Authenticode remains optional. |
| Frontend build and tests | Pass | `npm run build` succeeded; `npm test -- --run` passed 7 tests in 4 files after `npm ci --no-audit --no-fund`. | 2026-09-28 | Build includes TypeScript checking. |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed valid signature, corrupt signature, failed install, offline, and temporary configuration cases. | 2026-09-28 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded; 76 passed, 1 ignored. | 2026-09-28 | Ignored test requires separately installed NeMo runtime and Nemotron model. |
| Local unbundled Tauri release build | Pass | `npm run tauri build -- --no-bundle` succeeded and produced `src-tauri/target/release/lyra.exe`. | 2026-09-28 | The tagged workflow creates the installer and updater signature. |
| Main GitHub Windows workflow | Not run | | | Required before tagging. |
| Tagged GitHub release workflow | Not run | | | |
| Public release assets and updater manifest | Not run | | | |
| Public changelog mirror | Not run | | | |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Existing usage migrates to the correct local day without losing lifetime totals. | Not run | | | Check a copy before publication and the installed app after update. |
| A new dictation increments today's local bucket. | Not run | | | Requires a real installed-app dictation. |
| Installed v0.5.2 updates to v0.5.3 and reports the new version. | Not run | | | |

Automated checks do not establish installed-app or hardware behavior. Record hands-on results only after directly observing them.
