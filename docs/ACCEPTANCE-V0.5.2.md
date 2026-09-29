# Lyra v0.5.2 release acceptance

v0.5.2 is prepared for release. The current published release remains v0.5.1 until the protected source tag runs the publication workflow and the public assets are verified.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-28 | Updater-signed publication configuration validated; optional Authenticode signing remains unconfigured. |
| Frontend build and tests | Pass | `npm run typecheck`, `npm test -- --run`, and `npm run build` succeeded; 7 tests passed in 4 files. | 2026-09-28 | Dependencies were installed with `npm ci --no-audit --no-fund`. |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed signed, corrupt-signature, failed-install, offline, and temporary-configuration cases. | 2026-09-28 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded. | 2026-09-28 | 73 passed, 0 failed, 1 ignored. The ignored test requires the separately installed NeMo runtime and Nemotron model. |
| Local unbundled Tauri release build | Pass | `npm run tauri build -- --no-bundle` succeeded and produced `src-tauri/target/release/lyra.exe`. | 2026-09-28 | The installer bundle and updater signature are created by the tagged workflow, not this build. |
| Main GitHub Windows workflow | Pass | [Run 36508214156](https://github.com/OrionAISystems/Lyra/actions/runs/36508214156) succeeded in 12m54s on release preparation commit `086a415`, including metadata validation, frontend build/tests, Rust release tests, and unbundled Tauri build. | 2026-09-28 | GitHub warned that actions/cache@v4, actions/checkout@v4, and actions/setup-node@v4 target Node 20 and were forced to run on Node 24; the workflow passed. |
| Tagged GitHub release workflow | Pending | | | Tagging publishes to Lyra-Releases. |
| Public release assets and updater manifest | Pending | | | Verify after publication. |
| Public changelog mirror | Pending | | | Verify after publication. |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Dictation bar menu is visible above the bar. | Pass | Native dev bundle window expanded upward while open and returned to compact size after a second right-click. | 2026-09-28 | Escape and outside-click dismissal, and installed v0.5.2, remain hands-on checks. |
| Releasing Space while holding Ctrl and Alt pastes, then Space starts another dictation. | Pass | User tested the v0.5.2 dev bundle and confirmed paste and immediate restart. | 2026-09-28 | |
| Releasing Ctrl or Alt while Space remains held continues recording. | Pass | User tested the narrowed dev bundle and confirmed recording continued without stopping. | 2026-09-28 | The global shortcut library reports release of the trigger key, Space. |
| Installed v0.5.1 updates to v0.5.2 and reports the new version. | Pending | | | Verify after publication. |

Release automation and dev-bundle checks do not establish installed-app updater behavior. Record that result only after direct observation.
