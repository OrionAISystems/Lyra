# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. It keeps the first recognition path local: a global push-to-talk shortcut captures microphone audio, a warm Parakeet recognizer transcribes it, conservative text cleanup runs, and the result is safely pasted into the application that was active before recording began.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.3 is the first daily-use hardening release: the dictation bar honors push-to-talk versus toggle mode, toggle sessions can endpoint after detected speech falls silent, and deterministic formatting plus a local vocabulary make results feel personal without adding an LLM or transcript history. It also adds active-window or fixed-monitor placement, recorded hotkeys, explicit model warm policies, local aggregate usage statistics, and a release-gated updater.

The v0.2 → v0.3 settings migration is automatic. Legacy `keepModelWarm` values become `always_warm` or `never_keep_warm`; all other v2 fields use documented defaults. Runtime/model installation remains a separate, explicit setup step because the NeMo-Speech.cpp runtime and Parakeet model are large external artifacts.

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

The v0.3 local checks are:

```powershell
npm run typecheck
npm test
npm run test:updater-fixture
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

The durable manual evidence sheet is [docs/ACCEPTANCE-V0.3.md](docs/ACCEPTANCE-V0.3.md). Hardware and installed-updater cases stay explicitly separate from automated tests.

The first public release is published from the private source repository to the release-only [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients use its public signed `latest.json` feed and do not need GitHub credentials.

To build the local installer after configuring updater signing, run `npm run tauri:build`. Authenticode signing is optional, so local and early release installers may be unsigned by Windows. The debug output under `src-tauri/target/debug` is left intact for development; release output is written under `src-tauri/target/release`.

## Architecture and roadmap

- [ARCHITECTURE.md](ARCHITECTURE.md) describes boundaries and data flow.
- [BUILD-STAGE.md](BUILD-STAGE.md) records what is complete, what is hardware-gated, and the next vertical slices.
- [docs/MODELS.md](docs/MODELS.md) records pinned runtime/model sources, checksums, licenses, and storage policy.
- [docs/RELEASES.md](docs/RELEASES.md) records the tag, signing, cross-repository publication, and update-feed contract.

The v1 scope is Windows x64, local-first utterance recognition, optional OpenAI transcription fallback, secure clipboard injection, aggregate-only local statistics, and no saved audio/transcript history. Streaming recognition, mobile/macOS/Linux, accounts, and LAN services remain future phases.
