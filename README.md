# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. A global shortcut or bar gesture captures microphone audio, local Parakeet or optional English Nemotron recognition processes it, and the final text is cleaned up and safely injected into the application that was active before recording began. Optional cloud fallback is explicit and credentials stay in Windows Credential Manager.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.5.4 (2026-09-30 local time) is the current published release. It adds visible progress for application updates and Parakeet/Nemotron model downloads. The release and updater feed are published in [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.4).

v0.5.5 is being prepared with an October seasonal theme, saved theme and motion preferences, and a signed catalog path for independently versioned seasonal artwork. Its release evidence is tracked in [docs/ACCEPTANCE-V0.5.5.md](docs/ACCEPTANCE-V0.5.5.md).

Settings migrations through schema v5 are automatic and idempotent. Existing History, Scratchpad, appearance, audio, and personalization preferences are preserved; seasonal themes and motion can each be disabled. Runtime and model installation remains separate and explicit; the optional Nemotron model is independently versioned and verified.

Remaining installed-updater, hardware-backed recognition, and cross-application injection checks are tracked in [docs/ACCEPTANCE-V0.5.md](docs/ACCEPTANCE-V0.5.md). v0.5.4 release evidence is in [docs/ACCEPTANCE-V0.5.4.md](docs/ACCEPTANCE-V0.5.4.md); v0.5.5 automated and hands-on results are in [docs/ACCEPTANCE-V0.5.5.md](docs/ACCEPTANCE-V0.5.5.md).

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

The core v0.5.5 local checks are:

```powershell
npm run typecheck
npm test
npm run build
npm run test:updater-fixture
npm run test:themes-fixture
npm run verify:version
npm run verify:changelog
npm run verify:release-config
npm run verify:themes
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

After the October catalog is published and before tagging v0.5.5, verify its public signature and PNG assets while the stable app release remains v0.5.4:

```powershell
npm run verify:public-themes -- --latest-version 0.5.4
```

After v0.5.5 and its seasonal catalog are published, verify the public app and artwork feeds without credentials:

```powershell
npm run verify:public-release -- --version 0.5.5
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
- [docs/THEMES.md](docs/THEMES.md) records the signed seasonal catalog, authoring format, publication workflow, and validation limits.
- [CHANGELOG.md](CHANGELOG.md) records the canonical per-version changes and is mirrored to the public [Lyra-Releases changelog](https://github.com/OrionAISystems/Lyra-Releases/blob/main/CHANGELOG.md).

The v1 scope is Windows x64, local-first dictation, optional English streaming recognition, optional OpenAI fallback, secure target-aware injection, independent metadata-only Insights, optional local transcript History, and intentional Scratchpad entries. Lyra never saves audio. Transcript History starts Off, while Scratchpad text is saved only after a deliberate Scratchpad capture. Mobile/macOS/Linux, accounts, teams, rewriting, editor extensions, and persistent developer context remain future phases.
