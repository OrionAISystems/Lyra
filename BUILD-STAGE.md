# Lyra build stage

The current implementation target is v0.3.1, a focused maintenance release after the v0.3.0 daily-use hardening milestone.

## Stage 0 — foundation

- [x] Private `OrionAISystems/Lyra` GitHub repository created.
- [x] Windows-first Tauri/React/Rust project shape.
- [x] Standalone documentation and artifact exclusion rules.
- [x] Settings/bar UI direction: charcoal, restrained typography, cool accent, low-motion active signal.
- [x] App version synchronized to `0.3.1` with release-tag drift validation.
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
- [x] Release workflow for type checks, frontend tests, Rust checks/tests, unbundled validation, NSIS, updater artifacts, and cross-repository publication.
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

## Future stages

- streaming recognizer and incremental bar text
- richer diagnostics and benchmark history without transcript retention
