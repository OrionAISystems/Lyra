# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. A global shortcut or bar gesture captures microphone audio, local Parakeet or optional English Nemotron recognition processes it, and the final text is cleaned up and safely injected into the application that was active before recording began. Optional cloud fallback is explicit and credentials stay in Windows Credential Manager.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.4.4 is the current published release. The local v0.5.0 development build adds optional, Windows-account-encrypted Transcript History (Off by default) and an intentional Scratchpad for thoughts spoken directly into Lyra. Daily Insights aggregates are retained indefinitely; detailed session metadata expires after one year. This v0.5.0 work is at the local review checkpoint and has not been tagged or published.

The v0.3 → v0.4 settings and personalization migrations are automatic and idempotent. Existing microphone, ten-minute duration, startup, placement, correction, and cost preferences are preserved. Runtime and model installation remains separate and explicit; the optional Nemotron model is independently versioned and verified.

The current published baseline is v0.4.4. The next local product pass is v0.5.0, focused on keeping opt-in dictated text separate from Insights metadata and on a small, deliberate Scratchpad workflow. The review build must be accepted before any v0.5.0 release action.

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

The v0.4 local checks are:

```powershell
npm run typecheck
npm test
npm run build
npm run test:updater-fixture
npm run verify:version
npm run verify:changelog
npm run verify:release-config
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

The durable manual evidence sheet is [docs/ACCEPTANCE-V0.4.md](docs/ACCEPTANCE-V0.4.md). Hardware and installed-updater cases stay explicitly separate from automated tests.

The v0.5.0 scope and review gate are in [PLAN.md](PLAN.md); local build and hands-on results belong in [docs/ACCEPTANCE-V0.5.md](docs/ACCEPTANCE-V0.5.md).

The first public release is published from the private source repository to the release-only [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients use its public signed `latest.json` feed and do not need GitHub credentials.

To build the local installer after configuring updater signing, run `npm run tauri:build`. Authenticode signing is optional, so local and early release installers may be unsigned by Windows. The debug output under `src-tauri/target/debug` is left intact for development; release output is written under `src-tauri/target/release`.

## Architecture and roadmap

- [ARCHITECTURE.md](ARCHITECTURE.md) describes boundaries and data flow.
- [BUILD-STAGE.md](BUILD-STAGE.md) records what is complete, what is hardware-gated, and the next vertical slices.
- [docs/MODELS.md](docs/MODELS.md) records pinned runtime/model sources, checksums, licenses, and storage policy.
- [docs/RELEASES.md](docs/RELEASES.md) records the tag, signing, cross-repository publication, and update-feed contract.
- [CHANGELOG.md](CHANGELOG.md) records the canonical per-version changes and is mirrored to the public [Lyra-Releases changelog](https://github.com/OrionAISystems/Lyra-Releases/blob/main/CHANGELOG.md).

The v1 scope is Windows x64, local-first dictation, optional English streaming recognition, optional OpenAI fallback, secure target-aware injection, independent metadata-only Insights, optional local transcript History, and intentional Scratchpad entries. Lyra never saves audio. Transcript History starts Off, while Scratchpad text is saved only after a deliberate Scratchpad capture. Mobile/macOS/Linux, accounts, teams, rewriting, editor extensions, and persistent developer context remain future phases.
