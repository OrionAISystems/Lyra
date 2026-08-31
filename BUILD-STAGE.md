# Lyra build stage

## Stage 0 — foundation

- [x] Private `OrionAISystems/Lyra` GitHub repository created.
- [x] Windows-first Tauri/React/Rust project shape.
- [x] Standalone documentation and artifact exclusion rules.
- [x] Settings/bar UI direction: charcoal, restrained typography, cool accent, low-motion active signal.

## Stage 1 — first vertical slice

- [x] Domain types and seams for activation, audio, transcription, post-processing, and injection.
- [x] Settings persistence and migration seam.
- [x] Test fakes for the core session state machine.
- [ ] Hardware-backed WASAPI capture smoke test.
- [ ] App-local NeMo-Speech.cpp runtime verification.
- [ ] Pinned Parakeet model installation and warm recognizer smoke test.
- [ ] Manual Notepad/browser/ChatGPT/Codex/VS Code paste acceptance.

## Explicit setup gates

Runtime and model are separate approvals. The runtime setup reports source, size, destination, variant, checksum, and hardware selection before installation. The model setup reports source, size, destination, license, revision, checksum, and measured available storage before installation.

## Future stages

- streaming recognizer and incremental bar text
- richer diagnostics and benchmark history without transcript retention
- optional silence endpointing when the selected backend exposes safe controls
- signed distribution and update channel
