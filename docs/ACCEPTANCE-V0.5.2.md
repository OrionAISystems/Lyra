# Lyra v0.5.2 release acceptance

Lyra v0.5.2 was published on 2026-09-29 UTC (2026-09-28 local time). The [public release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.2) contains the NSIS installer, updater signature, and manifest. The source tag `v0.5.2` points to commit `d33704efd1ebaaff939513db0ae697a62aaaa72b`.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-28 | Updater-signed publication configuration validated; optional Authenticode signing remains unconfigured. |
| Frontend build and tests | Pass | `npm run typecheck`, `npm test -- --run`, and `npm run build` succeeded; 7 tests passed in 4 files. | 2026-09-28 | Dependencies were installed with `npm ci --no-audit --no-fund`. |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed signed, corrupt-signature, failed-install, offline, and temporary-configuration cases. | 2026-09-28 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded. | 2026-09-28 | 73 passed, 0 failed, 1 ignored. The ignored test requires the separately installed NeMo runtime and Nemotron model. |
| Local unbundled Tauri release build | Pass | `npm run tauri build -- --no-bundle` succeeded and produced `src-tauri/target/release/lyra.exe`. | 2026-09-28 | The installer bundle and updater signature are created by the tagged workflow, not this build. |
| Main GitHub Windows workflow | Pass | [Run 36508214156](https://github.com/OrionAISystems/Lyra/actions/runs/36508214156) succeeded in 12m54s on release preparation commit `086a415`; [run 36509269133](https://github.com/OrionAISystems/Lyra/actions/runs/36509269133) succeeded in 9m36s on the tagged commit `d33704e`. Both included metadata validation, frontend build/tests, Rust release tests, and unbundled Tauri build. | 2026-09-29 UTC | GitHub warned that actions/cache@v4, actions/checkout@v4, and actions/setup-node@v4 target Node 20 and were forced to run on Node 24; both workflows passed. |
| Tagged GitHub release workflow | Pass | [Run 36510105068](https://github.com/OrionAISystems/Lyra/actions/runs/36510105068) passed signing-key validation, release-mode checks, publication credentials, public changelog sync, and NSIS/updater publication. | 2026-09-29 UTC | |
| Public release assets and updater manifest | Pass | `Lyra_0.5.2_x64-setup.exe` (2,587,978 bytes), `.sig` (412 bytes), and `latest.json` (1,664 bytes) are present. The downloaded installer SHA-256 matches GitHub's `351408f1aa9402e8050f735c57a461312d3335c782032b204dd5adc16afd4f9e`. Both Windows manifest entries point to the installer and embed the exact detached signature. The live `releases/latest/download/latest.json` feed matches the release asset and reports version `0.5.2`. | 2026-09-29 UTC | Authenticode reports `NotSigned` because the optional certificate is not configured; the Tauri updater signature is present. |
| Public changelog mirror | Pass | Public `CHANGELOG.md` Git blob SHA `a7636a8e4d557c38e8ad6f9b4c71f367874f3e9c` matches the source. | 2026-09-29 UTC | |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Dictation bar menu is visible above the bar. | Pass | Native dev bundle window expanded upward while open and returned to compact size after a second right-click. | 2026-09-28 | Escape and outside-click dismissal, and installed v0.5.2, remain hands-on checks. |
| Releasing Space while holding Ctrl and Alt pastes, then Space starts another dictation. | Pass | User tested the v0.5.2 dev bundle and confirmed paste and immediate restart. | 2026-09-28 | |
| Releasing Ctrl or Alt while Space remains held continues recording. | Pass | User tested the narrowed dev bundle and confirmed recording continued without stopping. | 2026-09-28 | The global shortcut library reports release of the trigger key, Space. |
| Installed v0.5.1 updates to v0.5.2 and reports the new version. | Pass | User reported the update completed and that v0.5.2 is running. The active installed executable reports product and file version `0.5.2`. | 2026-09-28 | |
| Installed v0.5.2 menu and push-to-talk paste behavior. | Pending | | | Dev-bundle behavior passed; confirm the installed release in normal use. |

Release automation and dev-bundle checks do not establish installed-app behavior. The updater result above includes the user's direct report and the installed process version; menu and shortcut behavior in that installed release remain unverified.
