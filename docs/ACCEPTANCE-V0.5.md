# Lyra v0.5.0 local acceptance

The current published release remains v0.4.4. This sheet is for the local v0.5.0 development build only. No v0.5.0 tag, installer, updater feed, or public release has been produced. Do not mark a behavior Pass without observing it in the running local app or its persisted data.

## Automated and build checks

| Check | Status | Evidence | Date | Notes |
|---|---|---|---|---|
| Frontend typecheck | Pass | `npm run typecheck` exited 0 | 2026-09-26 | |
| Frontend unit tests | Pass | `npm test`: 4 files, 7 tests passed | 2026-09-26 | |
| Frontend production build | Pass | `npm run build` completed; Vite emitted the production bundle | 2026-09-26 | |
| Rust formatting check | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` exited 0 | 2026-09-26 | |
| Rust compile check | Pass | `cargo check --manifest-path src-tauri/Cargo.toml` exited 0 | 2026-09-26 | Existing dead-code warnings in streaming/NeMo paths |
| Rust unit tests | Pass | 66 passed, 0 failed, 1 ignored | 2026-09-26 | Ignored test requires separately installed NeMo runtime/model |
| Version, changelog, and release-config validation | Pass | All three `verify:*` scripts exited 0 | 2026-09-26 | |
| Local Tauri development build launches as v0.5.0 | Pass | `npm run tauri:dev` compiled and started `target/debug/lyra.exe`; the Lyra process responds with window title `Lyra`. The local Vite preview footer reads `v0.5.0`. | 2026-09-26 | Local dev process left open for review; screen review used the matching Vite UI preview, not native saved-item actions. |

## Product behavior

| Scenario | Status | Observed evidence | Date | Notes |
|---|---|---|---|---|
| Existing v0.4 settings migrate to schema v4 with History Off and prior preferences intact | Pending | | | Automated migration test plus local settings inspection |
| New install starts with History Off | Pending | | | |
| Dictation while History is Off creates no History text | Pending | | | Inspect local storage after a controlled test; do not infer from UI alone |
| Opt-in History shows final processed text and its timestamp, target app, duration, engine, latency, and paste outcome | Pending | | | |
| Failed automatic paste leaves the transcript recoverable in History when enabled | Pending | | | |
| Retention choices Off, 7, 30, 90, 365 days, and Forever display and behave correctly | Pending | | | Automated retention tests plus local selector inspection |
| Switching History Off stops new saves while prior entries follow the last selected retention | Pending | | | |
| Shortening retention removes older History entries immediately | Pending | | | |
| Clear History removes History entries but leaves Scratchpad and Insights intact | Pending | | | |
| Saved text is encrypted at rest for the current Windows user; corrupted ciphertext fails closed | Pending | | | Inspect saved files and run storage tests |
| Scratchpad UI and global shortcut capture a deliberate thought using the selected activation mode | Pending | | | Hardware microphone test may remain pending if no safe input device is available |
| Scratchpad does not auto-paste or change the clipboard during capture | Pending | | | Compare clipboard and active target before/after a controlled capture |
| Scratchpad entry persists after Lyra restarts and is independent of History retention | Pending | | | |
| Scratchpad copy, paste into current app, pin/unpin, delete, and History promotion work | Pending | | | |
| A Scratchpad or History paste targets the verified most recent external app, or copies text with a clear recovery message | Pending | | | |
| Target loss or injection refusal never pastes into an unverified window | Pending | | | |
| Daily aggregates older than 90 days remain; metadata sessions older than a year expire without changing lifetime totals | Pending | | | Automated usage tests plus local store inspection |
| History and Scratchpad lists page in groups of 50 without losing ordering or selection | Pending | | | |
| Desktop and minimum-width layouts keep navigation and content usable; keyboard focus and actions are visible | Pending | | | |
| History, Scratchpad, and extended Insights screens render with the intended empty states and controls | Pass (visual) | Local 1280×720 render; History Off state and six retention choices; Scratchpad empty state and shortcut; Insights All time range | 2026-09-26 | Vite preview only; does not exercise native storage, capture, or saved-item commands. Minimum-width/keyboard review remains pending. |
| User review of the v0.5.0 local build | Pass (reported) | User reports that the basic initial tests worked and approves proceeding to release | 2026-09-26 | Specific scenarios were not enumerated; detailed behavior rows above retain their observed status. |

## Hardware and installed-release cases

Installed updater acceptance and public artifact inspection are intentionally outside this local checkpoint. Audio device, local runtime/model, cross-application injection, and real spoken partial/final behavior require direct observation; record them as Pending when the local environment cannot safely exercise them. No microphone recording should be performed as part of acceptance without explicit user direction.
