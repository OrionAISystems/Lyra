# Lyra v0.3.0 acceptance matrix

Every row is a release gate. Automated rows should reference a command and output; manual rows require an operator, date, and concrete evidence such as a recording, screenshot, log excerpt, or observed result. Do not mark a row passed from code inspection alone.

Status values: `Not run`, `Pass`, `Fail`, or `Blocked`. Keep failed evidence in the repository issue/release record before retrying.

## Automated gates

| Gate | Status | Command / evidence | Date | Operator | Notes |
| --- | --- | --- | --- | --- | --- |
| Frontend typecheck | Pass | `npm run typecheck` — `tsc --noEmit` exit 0 | 2026-09-01 | Codex |  |
| Frontend production build | Pass | `npm run build` — 1,578 modules transformed, Vite build exit 0 | 2026-09-01 | Codex |  |
| Vitest suite | Pass | `npm test -- --run` — 2 files, 3 tests passed | 2026-09-01 | Codex |  |
| Rust format | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` exit 0 | 2026-09-01 | Codex |  |
| Rust check | Pass | `cargo check --manifest-path src-tauri/Cargo.toml` — finished dev profile | 2026-09-01 | Codex |  |
| Rust tests | Pass | `cargo test --manifest-path src-tauri/Cargo.toml` — 39 passed, 0 failed | 2026-09-01 | Codex |  |
| Unbundled Tauri release build | Pass | `npm run tauri -- build --no-bundle` — release executable built at `src-tauri/target/release/lyra.exe` | 2026-09-01 | Codex |  |
| Settings v1 → v2 migration/normalization | Pass | Rust settings tests — defaults, bounds, v1 migration, v2 preservation, persistence | 2026-09-01 | Codex |  |
| Activation and pointer helper contracts | Pass | Rust domain tests plus `barActivation.test.ts` — 3 frontend tests passed | 2026-09-01 | Codex |  |
| Silence arming/endpoint contracts | Pass | Rust audio tests — no endpoint before speech, voiced arming, supported values | 2026-09-01 | Codex |  |
| Dynamic duration `120 → 300 → 60` | Pass | Rust capture-control test — atomic max-duration updates to 300s then 60s | 2026-09-01 | Codex | Manual bar/session rows remain separate below. |
| Formatting/correction ordering and boundaries | Pass | Rust domain tests — deterministic formatting, whole phrases, longest-first, case behavior | 2026-09-01 | Codex |  |
| Hotkey parsing/canonicalization | Pass | Rust commands test — modifier requirement, canonical order, invalid chord rejection | 2026-09-01 | Codex |  |
| Monitor identity/index resolution | Pass | Rust Windows placement test — identity match wins, ordinal fallback resolves | 2026-09-01 | Codex |  |
| Warm-policy state contract | Pass | Rust transcription/settings tests — all five inactivity policies and migration | 2026-09-01 | Codex |  |
| Usage aggregate/reset/corruption | Pass | Rust usage tests — aggregation, cost avoided, reset, corrupt-file recovery, safe percentages | 2026-09-01 | Codex |  |
| Updater state/gate/offline/install failure | Pass | Rust updater tests — active-session gate, state flow, offline and failed-install recovery | 2026-09-01 | Codex |  |
| Local signed updater fixture | Pass | `npm run test:updater-fixture` — valid signature, corrupt signature, failed install preservation, offline, ephemeral config | 2026-09-01 | Codex |  |
| Version synchronization | Pass | `npm run verify:version` — Lyra release version verified: 0.3.0 | 2026-09-01 | Codex |  |
| Production release configuration | Pass | `npm run verify:release-config` — updater-signed publication ready; Authenticode optional | 2026-09-01 | Codex | Publication secrets are still checked by CI. |
| Whitespace validation | Pass | `git diff --check` exit 0 | 2026-09-01 | Codex | Git reported only expected LF/CRLF normalization warnings. |

## Hardware and lifecycle

| Acceptance case | Status | Evidence | Date | Operator | Notes |
| --- | --- | --- | --- | --- | --- |
| Samson G-Track Pro capture | Not run |  |  |  | Device enumerates, captures, and stops cleanly. |
| RTX 2060 CUDA recognition | Not run |  |  |  | Record backend/runtime status and latency. |
| Vulkan fallback recognition | Not run |  |  |  | Switch verified runtime, record successful result. |
| Microphone unplug/replug | Not run |  |  |  | Confirm clear recovery without restart. |
| Windows sleep/wake | Not run |  |  |  | Confirm bar, hotkey, device, and capture recover. |
| Model preload and Ready status | Pass | Computer Use accessibility tree on rebuilt worktree v0.3.0 showed verified CUDA/runtime/model artifacts and `Model runtime — Ready` with Always warm selected. | 2026-09-01 | Codex | Rendered accessibility evidence; no screenshot was available from the Tauri window capture API. |
| Timed model unload | Not run |  |  |  | Verify selected timeout and no unload during capture/transcription. |
| Never keep warm | Pass | Computer Use settings flow selected Never keep warm; the rebuilt app reported `Unloaded`, then returned to `Ready` after restoring Always warm. | 2026-09-01 | Codex | This validates the visible lifecycle state, not a speech recognition request. |
| Bar follows active window at startup | Not run |  |  |  | Include three-monitor workstation. |
| Bar follows active window at session start | Not run |  |  |  | Move target before new session; no continuous browsing movement expected. |
| Primary display placement | Not run |  |  |  |  |
| Fixed monitor 1/2/3 placement | Not run | Computer Use fixed-monitor picker enumerated `\\.\DISPLAY1`, `\\.\DISPLAY2`, and `\\.\DISPLAY3`; physical placement on each display was not exercised. | 2026-09-01 | Codex | Full three-monitor placement remains an operator check. |
| Saved monitor unavailable fallback | Not run |  |  |  | Primary placement plus non-blocking status. |

## Dictation and injection

| Acceptance case | Status | Evidence | Date | Operator | Notes |
| --- | --- | --- | --- | --- | --- |
| PTT press starts | Not run |  |  |  |  |
| PTT drag off bar and release stops | Not run |  |  |  | Pointer capture path. |
| PTT pointer cancellation stops | Not run |  |  |  |  |
| PTT lost pointer capture stops | Not run |  |  |  |  |
| PTT rapid press/release while start is pending | Not run |  |  |  | No stuck listening state or duplicate processing. |
| Toggle click-to-start/click-to-stop | Pass | Computer Use bar accessibility showed `0s:300s` while listening after one click; the second click returned the bar to idle. | 2026-09-01 | Codex | No speech was injected, so this validates bar/session interaction only. |
| Toggle silence Off | Not run |  |  |  | Does not endpoint from silence before manual stop/max. |
| Toggle silence 1.0s | Not run |  |  |  | Speech first, then continuous silence. |
| Toggle silence 1.5s | Not run |  |  |  |  |
| Toggle silence 2.0s | Not run |  |  |  |  |
| Toggle silence 3.0s | Not run |  |  |  |  |
| PTT never silence-stops | Not run |  |  |  |  |
| Formatting commands | Not run |  |  |  | New line, new paragraph, comma, period, open/close quote. |
| Seed vocabulary corrections | Not run |  |  |  | Orion AI Systems, GeoCore, WarMap. |
| Added/disabled/case-sensitive correction | Not run |  |  |  |  |
| Notepad paste | Not run |  |  |  |  |
| Chrome textarea paste | Not run |  |  |  |  |
| Edge textarea paste | Not run |  |  |  |  |
| ChatGPT paste | Not run |  |  |  |  |
| Codex paste | Not run |  |  |  |  |
| VS Code paste | Not run |  |  |  |  |
| Clipboard changed during paste | Not run |  |  |  | Sequence guard preserves other application's clipboard. |
| Target window closes before completion | Not run |  |  |  | Transcript remains available and failure is visible. |
| Maximum duration idle `120 → 300 → 60` | Not run |  |  |  | Bar updates immediately; no restart. |
| Maximum duration while recording | Not run |  |  |  | Lowering below elapsed ends at the new limit. |

## Updates and release

| Acceptance case | Status | Evidence | Date | Operator | Notes |
| --- | --- | --- | --- | --- | --- |
| Update attempted while actively dictating | Not run |  |  |  | Install rejected; session remains safe. |
| Local valid signed manifest/artifact | Pass | `npm run test:updater-fixture` — valid signed manifest/artifact passed | 2026-09-01 | Codex |  |
| Local offline check | Pass | `npm run test:updater-fixture` — offline update check passed | 2026-09-01 | Codex |  |
| Local invalid/corrupt signature | Pass | `npm run test:updater-fixture` — invalid/corrupt signature passed | 2026-09-01 | Codex |  |
| Local failed install preserves app | Pass | `npm run test:updater-fixture` — failed install preserves existing application passed | 2026-09-01 | Codex |  |
| Installed v0.2.0 → public v0.3.0 check | Not run |  |  |  | Real production feed. |
| Installed v0.2.0 → v0.3.0 download | Not run |  |  |  |  |
| Passive install and restart | Not run |  |  |  |  |
| Post-upgrade version and migrated settings | Not run |  |  |  | Confirm v0.3.0, warm policy migration, seeded dictionary, stats store. |
