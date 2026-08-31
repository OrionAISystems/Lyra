# Lyra build stage

The current implementation target is v0.2.0, the first Windows x64 installer release.

## Stage 0 — foundation

- [x] Private `OrionAISystems/Lyra` GitHub repository created.
- [x] Windows-first Tauri/React/Rust project shape.
- [x] Standalone documentation and artifact exclusion rules.
- [x] Settings/bar UI direction: charcoal, restrained typography, cool accent, low-motion active signal.
- [x] App version synchronized to `0.2.0` with release-tag drift validation.
- [x] Per-user NSIS bundling configuration with WebView2 download bootstrapper behavior.
- [x] Single-instance protection and one dynamic update item in the tray.
- [x] Rust-owned updater interface with stable-only checks, scheduling, install guards, and public signed-feed endpoint.
- [x] Resumable, checksum-verified, staged runtime/model bootstrapper without bundled native artifacts.
- [x] Public release-only `OrionAISystems/Lyra-Releases` repository created.

## Stage 1 — first vertical slice

- [x] Domain types and seams for activation, audio, transcription, post-processing, and injection.
- [x] Settings persistence and migration seam.
- [x] Test fakes for the core session state machine.
- [ ] Hardware-backed WASAPI capture smoke test.
- [ ] App-local NeMo-Speech.cpp runtime verification.
- [ ] Pinned Parakeet model installation and warm recognizer smoke test.
- [ ] Manual Notepad/browser/ChatGPT/Codex/VS Code paste acceptance.

## Stage 2 — v0.2.0 distribution

- [x] General Settings software-update card and startup prompt.
- [x] Update progress and error events bridged to the settings UI.
- [x] Release workflow for type checks, frontend tests, Rust checks/tests, unbundled validation, NSIS, updater artifacts, and cross-repository publication.
- [x] Configure the real Tauri updater public key in `src-tauri/tauri.conf.json`.
- [x] Add the updater signing key and GitHub App credential to repository/organization secrets; Authenticode signing remains optional until a certificate is available.
- [ ] Run a tagged CI release and inspect the public `latest.json`, installer, signature, and sanitized notes.
- [ ] Fresh-install, upgrade, offline, invalid-signature, active-dictation, and restart acceptance tests.

## Explicit setup gates

Runtime and model are separate approvals. The runtime setup reports source, size, destination, variant, checksum, and hardware selection before installation. The model setup reports source, size, destination, license, revision, checksum, and measured available storage before installation.

## Future stages

- optional silence endpointing when the selected backend exposes safe controls
- streaming recognizer and incremental bar text
- richer diagnostics and benchmark history without transcript retention
