# Bounded reliability pass — 2026-10-04

Scope: one investigation–fix–verification pass around dictation reliability. No recognition-quality changes, features, dependency upgrades, publishing, or broad refactors.

## Review boundary

Worktree: `../Lyra-reliability`, branch `hardening/reliability-pass`. The investigation initially used snapshot `ec530bc`, equivalent to the separately committed benchmark/formatting work at `8e27bb8`. For integration, the reliability commit was rebased onto current `main`, removing that snapshot from this branch. Review the PR against `main`: it contains only the four Rust reliability files and this handoff, while benchmark work remains in its separate PR. The pass reused installed node modules through a worktree junction and the existing Cargo target cache; it did not install/upgrade dependencies or edit primary source files.

## Lifecycle traced

- Primary and Scratchpad global shortcuts feed their activation controllers. Push-to-talk release requests stop; toggle mode stops on the next press. Bar commands use the same start/stop entry points.
- `start_session` admits only Idle/Success/Error, blocks benchmark/update conflicts, snapshots settings/destination/target, starts CPAL capture, and creates one capture coordinator.
- `stop_session` transitions Listening to Processing and signals capture once. Subsequent stops are no-ops. Silence and duration endpoints use the same coordinator.
- `complete_capture` checks request identity and state, takes capture context, then invokes the router. Recognition, retry and optional cloud fallback run outside the app mutex.
- Final processing now acquires the app mutex and validates current request identity plus Processing state before insertion, clipboard writes, History/Scratchpad persistence, result counters or completion. These effects share one commit boundary.
- Cancel clears the request and session context for Listening **and Processing**. A canceled/replaced worker cannot commit a late result. Shutdown also invalidates the request. Stale worker failures and delayed error reset already check request identity.
- Capture drops its stream before returning audio/errors. Stream errors now endpoint promptly rather than waiting for manual/silence/duration stop. Raw PCM remains in memory and drops when its worker returns.

## Fixes and evidence

1. **Cancellation during transcription:** the command previously returned without doing anything in Processing. It now invalidates the request. Result side effects previously preceded their request check; that check now precedes all result commits and remains locked through completion. Regression covers cancel, replacement, successful completion and rejection of duplicate completion.
2. **Recording after microphone failure:** callback errors were checked only after the capture loop ended. The loop now exits on a recorded stream error. Regressions cover device-error, stop and cancel endpoints without microphone hardware.
3. **Settings failure consistency:** `set` previously updated memory before attempting persistence. Memory changes only after a successful save. Settings now reuse the existing Windows atomic replacement used by saved items, instead of deleting the original settings file before rename. Regressions cover temporary-write failure, an exclusively locked Windows destination, original-byte preservation, temporary cleanup, and successful retry.
4. **Settings error-path deadlocks/rollback:** while holding the app mutex, registration failure and persistence rollback called `configured_shortcut`, which locks that mutex again. Both now parse the already-captured previous setting. Failed recovery-shortcut saves now unregister the candidate set before restoring the old set. Source-inspected and compiled; real shortcut-registration failure remains a manual check.

## Suspected issues and limits left outside this pass

- Cancellation discards output; it does not interrupt a synchronous native recognition or an in-flight cloud request, and the existing router may still attempt its configured cloud fallback after a canceled local request fails. Repeated cancel/restart can leave old workers waiting on the recognizer mutex, retaining their audio until they return. A hung native call has no proven recovery here. No claim of bounded canceled-worker resource use.
- Once the result commit acquires the app mutex, insertion completes before Cancel can execute. This prevents a check/unlock/late-paste race, but long chunked pastes can delay other commands. Hardware timing needs measurement before introducing another concurrency mechanism.
- Input-level events are not request-scoped. A callback during stream teardown may deliver a stale level after cancellation; no rendered symptom was established.
- Loading unreadable/invalid settings currently falls back to defaults and attempts to persist them. Recovery/backup policy deserves its own scoped change; startup corruption recovery was not changed here.
- A clipboard restoration error can follow a successful SendInput paste and surface as a paste failure. Actual clipboard contention and cross-app behavior were not exercised.
- Rapid release/start ordering across command dispatch and native hotkey delivery needs live Windows testing. Existing activation/state-machine tests exercise logical edges, not OS callback ordering.

## Windows/hardware checklist

- Rapid taps and repeated press/release in push-to-talk and toggle modes; switch between Dictation and Scratchpad. Confirm one result per capture and no stuck Listening/Processing.
- Stop, press Escape during slow local/cloud recognition, then immediately dictate again. Confirm old text never pastes, copies or enters either saved collection; check memory settles after old workers return.
- Disconnect/change the microphone while recording; verify prompt Error and a usable next capture. Test max-duration and silence endpoints.
- Fail recognition/fallback intentionally; confirm error recovery, preserved last good transcript, and successful next session.
- Paste into ordinary editors, terminals, elevated windows and slow/chunked targets; switch focus and modify clipboard during paste. Check modifier release, target protection, duplicates and clipboard recovery.
- Force hotkey-registration conflict and settings-write failure after changing primary/Scratchpad/recovery shortcuts. Confirm no frozen commands and the old shortcut set still works.
- With History enabled, cancel during transcription and restart; check no canceled History/Scratchpad entry. Verify retention Off during processing prevents History persistence.

## Verification

The local results below were recorded during the investigation against the snapshot baseline. GitHub PR checks validate the final reliability-only branch against `main`; the historical test count includes the separate benchmark/formatting tests.

- `cargo test --locked --manifest-path src-tauri/Cargo.toml`: 103 passed, 2 explicitly ignored (installed Nemotron smoke and live published theme channel). Five new regression tests passed, including real Windows exclusive-file-lock replacement failure.
- `cargo check --locked --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: passed.
- `npm test`: 26 passed across 9 files.
- `npm run build`: TypeScript check and Vite production build passed.
- `git diff --check`: passed.
- Existing native/streaming unused-code warnings remain. The request-gate test verifies cancellation/replacement/completion policy; it is not an end-to-end paste test. Shortcut error-path corrections are source-inspected and compiled, not reproduced against a running Tauri app.

 No live app, microphone, recognizer artifacts, clipboard automation, installed updater or installer publication was used.
