# Lyra v0.5.3 release acceptance

v0.5.3 was published on 2026-09-29 UTC (2026-09-28 local time). The [public release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.3) contains the NSIS installer, updater signature, and manifest. The source tag `v0.5.3` points to commit `969c7a9af3ed2112ec4580962e0f463a99b4c79b`. Retained session metadata from older UTC daily buckets is relocated on upgrade; lifetime totals are preserved. Aggregates without retained timestamps remain on their original dates.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, and release configuration | Pass | `npm run verify:version`, `npm run verify:changelog`, and `npm run verify:release-config` exited 0. | 2026-09-28 | Updater-signed publication configuration validated; Authenticode remains optional. |
| Frontend build and tests | Pass | `npm run build` succeeded; `npm test -- --run` passed 7 tests in 4 files after `npm ci --no-audit --no-fund`. | 2026-09-28 | Build includes TypeScript checking. |
| Updater fixtures | Pass | `npm run test:updater-fixture` passed valid signature, corrupt signature, failed install, offline, and temporary configuration cases. | 2026-09-28 | |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` succeeded; 76 passed, 1 ignored. | 2026-09-28 | Ignored test requires separately installed NeMo runtime and Nemotron model. |
| Local unbundled Tauri release build | Pass | `npm run tauri build -- --no-bundle` succeeded and produced `src-tauri/target/release/lyra.exe`. | 2026-09-28 | The tagged workflow creates the installer and updater signature. |
| Main GitHub Windows workflow | Pass | [Run 36517193676](https://github.com/OrionAISystems/Lyra/actions/runs/36517193676) succeeded on release preparation commit `969c7a9` in 12 minutes, including metadata, frontend, Rust, and unbundled Tauri build checks. | 2026-09-29 UTC | Passed before tagging. |
| Tagged GitHub release workflow | Pass | [Run 36518125554](https://github.com/OrionAISystems/Lyra/actions/runs/36518125554) succeeded in 9m45s, including signing-key validation and NSIS/updater publication. | 2026-09-29 UTC | |
| Public release assets and updater manifest | Pass | Installer (2,593,286 bytes), detached `.sig` (412 bytes), and `latest.json` (1,669 bytes) are present. Downloaded installer SHA-256 `17307e44626ab467ece08c3415fd462c1ea573e8df4c6a528f979b46f678653c` matches GitHub's digest. Both Windows manifest entries point to the installer and embed the detached signature; the live latest feed reports `0.5.3`. | 2026-09-29 UTC | Authenticode is `NotSigned` because the optional certificate is not configured; the Tauri updater signature is present. |
| Public changelog mirror | Pass | Public `CHANGELOG.md` Git blob SHA `30caeaa2c94233c8a277238f65bc7ad1b60c6a00` matches the source. | 2026-09-29 UTC | |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Existing usage migrates to the correct local day without losing lifetime totals. | Pass | Before update, UTC September 28 held 3,814 words and UTC September 29 held 838. After update, September 28 local held 4,652, schema advanced from 3 to 4, and lifetime words remained 6,314. | 2026-09-28 | Compared the installed store with a local pre-update copy; no transcript text or audio was used. |
| A new dictation increments today's local bucket. | Pass | User confirmed a short dictation appeared in Insights. The persisted September 28 local bucket rose from 4,652 to 4,665 words, while lifetime words rose from 6,314 to 6,327. | 2026-09-28 | 13-word increase in both totals; session count rose by one. |
| Installed v0.5.2 updates to v0.5.3 and reports the new version. | Pass | User confirmed the in-app update. The running installed executable at `%LOCALAPPDATA%\Lyra\lyra.exe` reports file and product version `0.5.3`. | 2026-09-28 | |

Other hardware, cross-application, and longer-running behavior remains tracked in the earlier acceptance files.
