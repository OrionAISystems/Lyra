# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. It keeps the first recognition path local: a global push-to-talk shortcut captures microphone audio, a warm Parakeet recognizer transcribes it, conservative text cleanup runs, and the result is safely pasted into the application that was active before recording began.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.2 adds the Windows-first distributable path: a per-user NSIS installer, explicit resumable runtime/model bootstrapper, signed public update feed integration, and a testable Rust update manager. Native runtime/model installation remains a separate, explicit setup step because the NeMo-Speech.cpp runtime and Parakeet model are large external artifacts.

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

The first public release is published from the private source repository to the release-only [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients use its public signed `latest.json` feed and do not need GitHub credentials.

To build the local installer after configuring updater signing, run `npm run tauri:build`. Authenticode signing is optional, so local and early release installers may be unsigned by Windows. The debug output under `src-tauri/target/debug` is left intact for development; release output is written under `src-tauri/target/release`.

## Architecture and roadmap

- [ARCHITECTURE.md](ARCHITECTURE.md) describes boundaries and data flow.
- [BUILD-STAGE.md](BUILD-STAGE.md) records what is complete, what is hardware-gated, and the next vertical slices.
- [docs/MODELS.md](docs/MODELS.md) records pinned runtime/model sources, checksums, licenses, and storage policy.
- [docs/RELEASES.md](docs/RELEASES.md) records the tag, signing, cross-repository publication, and update-feed contract.

The v1 scope is Windows x64, local-first utterance recognition, optional OpenAI transcription fallback, secure clipboard injection, and no saved audio/transcript history. Streaming recognition, silence endpointing, mobile/macOS/Linux, accounts, and LAN services remain future phases.
