# Lyra build stage

The current published version is v0.5.2 (2026-09-28 local time). It fixes the dictation bar menu placement and paste after releasing the push-to-talk Space key. The release workflow and public artifacts succeeded; pending installed-app checks are tracked in [docs/ACCEPTANCE-V0.5.2.md](docs/ACCEPTANCE-V0.5.2.md). Earlier v0.5 acceptance remains in [docs/ACCEPTANCE-V0.5.1.md](docs/ACCEPTANCE-V0.5.1.md) and [docs/ACCEPTANCE-V0.5.md](docs/ACCEPTANCE-V0.5.md).

## Stage 0 — foundation

- [x] Private `OrionAISystems/Lyra` GitHub repository created.
- [x] Windows-first Tauri/React/Rust project shape.
- [x] Standalone documentation and artifact exclusion rules.
- [x] Settings/bar UI direction: charcoal, restrained typography, cool accent, low-motion active signal.
- [x] Application versioning and release-tag drift validation established (the initial synchronized version was `0.3.1`).
- [x] Per-user NSIS bundling configuration with WebView2 download bootstrapper behavior.
- [x] Single-instance protection and one dynamic update item in the tray.
- [x] Rust-owned updater interface with stable-only checks, scheduling, install guards, and public signed-feed endpoint.
- [x] Resumable, checksum-verified, staged runtime/model bootstrapper without bundled native artifacts.
- [x] Public release-only `OrionAISystems/Lyra-Releases` repository created.

## Stage 1 — first vertical slice

- [x] Domain types and seams for activation, audio, transcription, post-processing, and injection.
- [x] Settings persistence and migration seam.
- [x] Test fakes for the core session state machine.
- [ ] Hardware-backed WASAPI capture smoke test; evidence is tracked in the v0.3 matrix.
- [ ] App-local NeMo-Speech.cpp runtime verification; evidence is tracked in the v0.3 matrix.
- [ ] Pinned Parakeet model installation and warm recognizer smoke test; evidence is tracked in the v0.3 matrix.
- [ ] Manual Notepad/browser/ChatGPT/Codex/VS Code paste acceptance; evidence is tracked in the v0.3 matrix.

## Stage 2 — v0.2.0 distribution

- [x] General Settings software-update card and startup prompt.
- [x] Update progress and error events bridged to the settings UI.
- [x] Release workflow for type checks, frontend tests, release-mode Rust checks/tests, cached release compilation, NSIS, updater artifacts, and cross-repository publication.
- [x] Configure the real Tauri updater public key in `src-tauri/tauri.conf.json`.
- [x] Add the updater signing key and GitHub App credential to repository/organization secrets; Authenticode signing remains optional until a certificate is available.
- [x] Run a tagged CI release and inspect the public `latest.json`, installer, signature, and sanitized notes.
- [x] Rust updater gate and local signed fixture cover valid, offline, corrupt-signature, and failed-install paths.
- [x] Installed v0.2.0 → public v0.3.0 check/download/passive-install/restart acceptance.

## Stage 3 — v0.3.0 release gate

- [x] Settings schema v2, v0.2 migration, normalization, and durable `lyra://settings` convergence event.
- [x] Mode-aware bar: click-to-toggle and pointer-captured push-to-talk cancellation paths.
- [x] Toggle-only RMS silence endpointing with speech arming and shared endpoint processing.
- [x] Dynamic maximum duration, including active `120 → 300 → 60` updates.
- [x] Deterministic formatting commands and local correction dictionary across all result routes.
- [x] Active-window, primary, and identity/index-resolved fixed monitor placement.
- [x] Recorded hotkey parsing, canonicalization, immediate registration, and rollback behavior.
- [x] Asynchronous model warm policies and compact recognizer lifecycle status.
- [x] Local aggregate usage store with opt-in cost estimates and confirmation-required reset.
- [x] Automated frontend, Rust, version, release-config, updater-fixture, and whitespace gates.
- [ ] Manual hardware, cross-application, lifecycle, and installed-updater evidence in [docs/ACCEPTANCE-V0.3.md](docs/ACCEPTANCE-V0.3.md).

## Stage 4 — v0.3.1 maintenance release

- [x] Settings shell keeps the sidebar pinned to the viewport while only the main content pane scrolls.
- [x] Rendered Playwright validation at 1100x680 and 390x844 confirms only `.content-scroll` scrolls while the sidebar remains pinned.

## Explicit setup gates

Runtime and model are separate approvals. The runtime setup reports source, size, destination, variant, checksum, and hardware selection before installation. The model setup reports source, size, destination, license, revision, checksum, and measured available storage before installation.

## Stage 5 — v0.4.0

- [x] Settings schema v3 plus atomic personalization and metadata-only usage migrations.
- [x] Deterministic backtracking, Words, Corrections, multiline Snippets, and app profiles.
- [x] Recovery shortcut rollback, tray-access guard, executable categories, and injection strategies.
- [x] Adaptive noise floor, silence trimming, clipping metadata, and one-recording benchmark.
- [x] Isolated optional Nemotron streaming model, contextual words, partial events, final-only processing, and Parakeet replay.
- [x] Seven-screen frameless product redesign with functional theme, density, startup, tray, placement, and updater controls.
- [x] Automated frontend and Rust gates plus screen-by-screen in-app browser inspection.
- [ ] Installed-hardware and public updater acceptance recorded in `docs/ACCEPTANCE-V0.4.md` after publication.

## Stage 6 — v0.4.x stabilization

- [x] v0.4.1: stabilize shortcut event handling and runtime status reads; improve chart labels and native dropdown contrast.
- [x] v0.4.2: polish theme and dropdown behavior; add engine-split Insights, advanced usage metrics, configurable typing speed, and silence-only success-rate handling.
- [x] v0.4.3: repair native window controls and compact bar behavior; polish dropdown and diagnostics states; allow benchmarking from completed or failed idle sessions.
- [x] v0.4.4: harden streaming lifecycle, bounded clipboard and artifact handling, foreground-target injection, diagnostic privacy, and TLS dependencies.
- [ ] Complete the remaining installed-hardware, cross-application injection, Nemotron spoken partial/final, fallback replay, and updater acceptance in [docs/ACCEPTANCE-V0.4.md](docs/ACCEPTANCE-V0.4.md).
- [x] Product scope set for v0.5.0: independent optional transcript History and intentional Scratchpad.

## Stage 7 — v0.5.0 release

- [x] Settings schema v4 migrates existing users with Transcript History Off and preserves prior preferences.
- [x] Independent encrypted History and Scratchpad stores, History retention controls, and explicit clear/delete/promotion actions.
- [x] Intent-level Scratchpad capture shares recognition and processing, does not auto-paste, and supports its own shortcut and visible control.
- [x] Daily aggregate Insights retained indefinitely; metadata-only sessions retained by age for one year.
- [ ] Detailed History and Scratchpad actions, persistence, paste recovery, hardware capture, and installed-updater scenarios remain to be verified as listed in `docs/ACCEPTANCE-V0.5.md`.
- [x] Automated gates and local build recorded in `docs/ACCEPTANCE-V0.5.md`.
- [x] User reports successful basic initial tests and approves the v0.5.0 release.
- [x] Tagged release workflow publishes the NSIS installer, updater signature, `latest.json`, and public changelog.
- [x] Public installer digest, feed version and URL, signature asset, and changelog mirror inspected; Authenticode is unsigned because the optional certificate is not configured.

## Stage 8 — v0.5.1 release

- [x] Added a live microphone input-level test without recording or saving audio.
- [x] Fixed recognizer-added punctuation at the start of lines inserted by “new line” and “new paragraph” commands.
- [x] Version, changelog, release configuration, frontend, updater fixture, Rust, and Windows Tauri build checks passed.
- [x] Tagged GitHub Actions release workflow published the NSIS installer, updater signature, `latest.json`, and public changelog.
- [x] Verified the public manifest version and installer URL, its signature against the detached signature asset, the public changelog mirror, and the GitHub installer digest. Authenticode is unsigned because the optional certificate is not configured.
- [ ] Hands-on checks of the voice formatting commands, microphone level test, and installed update remain pending in `docs/ACCEPTANCE-V0.5.1.md`.

## Stage 9 — v0.5.2 release

- [x] Made the dictation bar context menu visible above the bar, with working dismissal and menu actions.
- [x] Fixed paste when Space ends push-to-talk while Ctrl and Alt remain held; releasing Ctrl or Alt alone continues recording until Space is released.
- [x] Complete and record v0.5.2 local release gates in `docs/ACCEPTANCE-V0.5.2.md`.
- [x] Complete the main Windows workflow gate before tagging.
- [x] Tagged v0.5.2; the GitHub Actions workflow published the NSIS installer, updater signature, `latest.json`, and public changelog.
- [x] Verified installer digest, manifest version and URLs, signature match, live latest feed, and public changelog mirror.
- [x] Confirm the installed v0.5.1 to v0.5.2 update and running file version.
- [ ] Verify installed v0.5.2 menu and push-to-talk paste behavior in `docs/ACCEPTANCE-V0.5.2.md`.

## Stage 10 — v0.5.3 release preparation

- [x] Merge local-calendar-day Insights and the one-time UTC aggregate migration.
- [x] Complete and record v0.5.3 local release gates in `docs/ACCEPTANCE-V0.5.3.md`.
- [ ] Tag, publish, and inspect the v0.5.3 updater and installer after the main Windows gate passes.
- [ ] Verify the installed update and local-day behavior.

## Future stages

- native multi-final VAD endpointing and simultaneous warm models
- Nemotron multilingual releases when a validated 3.5-compatible local artifact is available
