# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. It keeps the first recognition path local: a global push-to-talk shortcut captures microphone audio, a warm Parakeet recognizer transcribes it, conservative text cleanup runs, and the result is safely pasted into the application that was active before recording began.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.1 establishes the Tauri 2 + React/Vite + Rust foundation, tray/bar/settings surfaces, domain interfaces, settings schema, and test seams. Native runtime/model installation is deliberately a separate, explicit setup step because the NeMo-Speech.cpp runtime and Parakeet model are large external artifacts.

## Stack

- Tauri 2 desktop shell and tray
- Rust session, audio, routing, native Windows interaction, secure credential, and runtime layers
- React 19 + Vite + TypeScript settings/bar UI
- Native NeMo-Speech.cpp C ABI loaded from an app-local pinned runtime
- `cpal` for Windows-compatible microphone capture

## Development

```powershell
npm install
npm run typecheck
npm test
npm run build
npm run tauri:dev
```

Read [docs/SETUP.md](docs/SETUP.md) before installing the native runtime or model. Those downloads require separate user approval and are never part of `npm install` or the Git repository.

## Architecture and roadmap

- [ARCHITECTURE.md](ARCHITECTURE.md) describes boundaries and data flow.
- [BUILD-STAGE.md](BUILD-STAGE.md) records what is complete, what is hardware-gated, and the next vertical slices.
- [docs/MODELS.md](docs/MODELS.md) records pinned runtime/model sources, checksums, licenses, and storage policy.

The v1 scope is Windows x64, local-first utterance recognition, optional OpenAI transcription fallback, secure clipboard injection, and no saved audio/transcript history. Streaming recognition, silence endpointing, mobile/macOS/Linux, accounts, and LAN services remain future phases.
