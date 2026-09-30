# Lyra v0.5.4 release acceptance

v0.5.4 adds visible progress for updater downloads and installation, plus dismissible Parakeet and Nemotron download progress that stays available in Settings while the downloads continue.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-30 | Updater-signed publication configuration validated; Authenticode is optional. |
| Frontend build and tests | Pass | `npm run build` succeeded; `npm test -- --run` passed 8 tests in 5 files. | 2026-09-30 | Build includes TypeScript checking and the canonical Parakeet-kind regression test. |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed valid signature, corrupt signature, failed install, offline, and temporary configuration cases. | 2026-09-30 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded; 76 passed, 1 ignored. | 2026-09-30 | Ignored test requires the separately installed NeMo runtime and Nemotron model. |
| Local unbundled Tauri release build | Pass | `npm run tauri build -- --no-bundle` produced `src-tauri/target/release/lyra.exe`. | 2026-09-30 | The tagged workflow creates the installer and updater signature. |
| Main GitHub Windows workflow | Not run | | | Required before tagging. |
| Tagged GitHub release workflow | Not run | | | |
| Public release assets and updater manifest | Not run | | | |
| Public changelog mirror | Not run | | | |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Progress dialog and compact cards remain readable in light theme at desktop and compact sizes. | Pass | Playwright screenshots at 1280×800 and 760×580; computed foreground colors are fixed to the dark-surface palette. | 2026-09-30 | Progress was simulated in the Vite browser; actual native downloads remain pending. |
| The updater progress card reports download progress before restart. | Not run | | | Requires an installed release and an available update. |
| Parakeet and Nemotron downloads report progress, continue after the dialog closes, and can be reopened from the Settings card. | Not run | | | Requires model downloads in the installed application. |
| Installed v0.5.3 updates to v0.5.4 and reports the new version. | Not run | | | |

Automated checks do not establish installed-app or hardware behavior. Record hands-on results only after directly observing them.
