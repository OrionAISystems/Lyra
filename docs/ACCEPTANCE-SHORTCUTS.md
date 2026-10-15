# Multiple dictation shortcuts and shortcut recording

The Dictation screen supports a primary shortcut plus additional dictation bindings. Existing installations retain their primary shortcut (Ctrl+Alt+Space by default). Click a shortcut pill to replace it, or Add shortcut to record another. Press a modifier combination or a single F13–F24 key, then release to save. Escape or changing window focus cancels. Remove controls appear when more than one dictation shortcut exists; at least one remains configured. Each dictation pill has a Hold/Toggle selector. Bar activation is independent, and Scratchpad has its own mode selector. The initiating shortcut owns each recording: Hold ends on its release; Toggle ends on its next press. Other shortcut presses/releases are ignored until completion. Existing shortcuts inherit the previous shared mode during migration. Scratchpad and the displayed recovery shortcuts use the same recorder.

Automated coverage includes settings migration/serialization, canonical duplicate validation, bare-key restrictions, initiating-binding ownership, independent Hold/Toggle modes, repeat suppression, key recording and cancellation, and native event forwarding in renderer tests.

## Verification on 2026-10-14

- `npm run typecheck`, `npm test` (36 tests), and `npm run build` passed.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` passed.
- `cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench` passed: 151 passed, 5 intentionally ignored.
- Default-feature `cargo check --release --locked --manifest-path src-tauri/Cargo.toml` passed, with existing unused-code warnings.
- Playwright against `http://127.0.0.1:1425/` in local Chrome passed at 1100×820 and 600×760: add a synthetic F20 event, preserve the primary binding, replace a chord, reject bare typing keys, cancel, remove an extra binding, preserve the last binding, and show all bindings and mode-aware instructions on Home. No page errors, failed resource requests or horizontal overflow remained. F20 required synthetic DOM events because Playwright's keyboard API does not support that key; this does not verify physical MaxxStick input or native global registration.
- `git diff --check` passed. Temporary browser QA files were kept outside the repository.
- `npm run tauri:build -- --no-bundle` produced the local release executable. At the user's request, the installed instance was closed and `src-tauri/target/release/lyra.exe` was launched. Its process path and rendered settings window were verified; it reported Ready locally. This is local launch evidence, not installer or physical-shortcut acceptance.

## Per-binding mode iteration (local testing only)

Schema v7 adds per-binding modes with one-time inheritance of the old shared setting. Unit coverage checks mixed Hold/Toggle ownership, repeat and release handling, cancellation/reset, mode-preserving migration and serialization, and independence from later bar changes. Renderer tests check independent selectors, key edits and promotion of an additional binding to primary.

Verification on 2026-10-14: `npm run typecheck`, `npm test` (38 tests), `npm run build`, Rust formatting, default-feature release checking, and the complete regression-bench release test command passed (151 passed, 5 intentionally ignored). Playwright in local Chrome at 1100×820 and 760×760 verified independent selectors, mode preservation after key editing and promotion, mode-aware Home instructions and absence of page errors, failed requests or horizontal overflow. The Browser plugin was unavailable; screenshots and scripts remained outside the repository. Synthetic F20 browser input does not establish physical-device acceptance.

The user confirmed live MaxxStick and Ctrl+Alt+Space dictation/paste in the previous local build. Mixed-mode physical acceptance remains separate. No publication, version tag or installer release is part of this iteration.

The user subsequently confirmed F20 Toggle start/stop and Ctrl+Alt+Space Hold dictation/paste in the mixed-mode local build. The compact Hold/Toggle menus were then corrected to match their 100px controls instead of inheriting the shared 175px menu minimum. Browser screenshots at 1020×760 and 760×760 confirmed the menus remain inside the viewport for both dictation and Scratchpad; option selection and Escape dismissal passed without page errors. Typechecking, all 38 frontend tests, and the frontend build passed. Per the user's instruction, this CSS fix was not packaged into another local executable.

`npm run tauri:build -- --no-bundle` produced a fresh default-feature local release executable. The previous local instance was closed and the new executable launched from `src-tauri/target/release/lyra.exe`. The saved test configuration uses Ctrl+Alt+Space = Hold, F20 = Toggle, and preserves Scratchpad = Hold. Both the persisted configuration and the native Home screen's per-binding instructions were verified after launch; the app reported Ready locally. A settings backup was retained in the user's temporary directory before configuring these test modes. This proves launch and configuration, not physical mixed-mode capture/insertion.

Installed Windows acceptance still requires:

- Add F20 by clicking the MaxxStick while preserving Ctrl+Alt+Space; verify both after restarting Lyra.
- Set Ctrl+Alt+Space to Hold and F20 to Toggle. Start with F20; press and release Ctrl+Alt+Space without stopping the recording, then press F20 to stop.
- Start with Ctrl+Alt+Space, press/release F20 without stopping it, and release Ctrl+Alt+Space to stop. Verify silence endpointing applies to Toggle and not Hold, independently of Bar activation. Verify edits, restart and promotion preserve each binding's mode.
- Re-record an existing dictation, Scratchpad and recovery binding; no capture, paste or recovery action should run while recording a shortcut.
- Assign a chord owned by another application; the old bindings remain usable after registration fails. Verify persistence failure restores the old assignments as well.
- Verify microphone capture, endpointing, cancellation and focus-safe insertion with both keyboard and MaxxStick. Browser and unit checks do not establish this hardware acceptance.

No hardware acceptance is claimed by this document.
