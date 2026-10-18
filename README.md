# Lyra

Lyra is a Windows-first, tray-first dictation utility for the Orion ecosystem. A global shortcut or bar gesture captures microphone audio, local Parakeet or optional English Nemotron recognition processes it, and the final text is cleaned up and safely injected into the application that was active before recording began. With the default local-first cloud-fallback route, Lyra sends audio to OpenAI automatically if local recognition fails and an OpenAI credential is stored; it does not ask for confirmation per capture. Credentials stay in Windows Credential Manager.

The repository is intentionally standalone. Lyra does not depend on Orion's gateway or shared UI package, and runtime/model artifacts are managed outside Git under `%LOCALAPPDATA%\Lyra\`.

## Current phase

v0.5.13 (2026-10-18 local time) moves History listing off the UI thread and reduces repeated filesystem scans. It also clarifies runtime download failures and preserves interrupted downloads for retry. Validation and publication status are tracked in [docs/ACCEPTANCE-V0.5.13.md](docs/ACCEPTANCE-V0.5.13.md); signed installers are published through [Lyra-Releases](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.13).

Settings migrations through schema v7 are automatic and idempotent. Existing History, Scratchpad, appearance, audio, personalization, and shortcut preferences are preserved; seasonal themes and motion can each be disabled. Runtime and model installation remains separate and explicit; the optional Nemotron model is independently versioned and verified.

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

The core local checks are:

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
cargo check --release --locked --manifest-path src-tauri/Cargo.toml
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench
```

Verify the existing signed theme channel while confirming the stable app remains v0.5.13:

```powershell
npm run verify:public-themes -- --latest-version 0.5.13
```

Verify the public app and artwork feeds without credentials:

```powershell
npm run verify:public-release -- --version 0.5.6
```

The durable manual evidence sheet is [docs/ACCEPTANCE-V0.4.md](docs/ACCEPTANCE-V0.4.md). Hardware and installed-updater cases stay explicitly separate from automated tests.

The v0.5.0 scope and review gate are in [PLAN.md](PLAN.md); local build and hands-on results belong in [docs/ACCEPTANCE-V0.5.md](docs/ACCEPTANCE-V0.5.md).

The first public release is published from the private source repository to the release-only [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients use its public signed `latest.json` feed and do not need GitHub credentials.

To build the local installer after configuring updater signing, run `npm run tauri:build`. Authenticode signing is optional, so local and early release installers may be unsigned by Windows. The debug output under `src-tauri/target/debug` is left intact for development; release output is written under `src-tauri/target/release`.

## Architecture and roadmap

The internal [transcription regression benchmark](benchmarks/transcription/README.md) replays labeled audio fixtures through the existing local pipeline, measures accuracy/timing, and compares approved baselines. It is an opt-in developer tool excluded from normal app builds and public release assets.

- [ARCHITECTURE.md](ARCHITECTURE.md) describes boundaries and data flow.
- [BUILD-STAGE.md](BUILD-STAGE.md) records what is complete, what is hardware-gated, and the next vertical slices.
- [docs/MODELS.md](docs/MODELS.md) records pinned runtime/model sources, checksums, licenses, and storage policy.
- [docs/RELEASES.md](docs/RELEASES.md) records the tag, signing, cross-repository publication, and update-feed contract.
- [docs/THEMES.md](docs/THEMES.md) records the signed seasonal catalog, authoring format, publication workflow, and validation limits.
- [docs/engineering/commits.md](docs/engineering/commits.md) defines commit subjects, types, message bodies, and commit boundaries.
- [docs/engineering/pull-requests.md](docs/engineering/pull-requests.md) guides branch-level PR titles, descriptions, validation, and risks.
- [CHANGELOG.md](CHANGELOG.md) records the canonical per-version changes and is mirrored to the public [Lyra-Releases changelog](https://github.com/OrionAISystems/Lyra-Releases/blob/main/CHANGELOG.md).

The v1 scope is Windows x64, local-first dictation, optional English streaming recognition, optional OpenAI fallback, secure target-aware injection, independent metadata-only Insights, optional local transcript History, and intentional Scratchpad entries. Lyra never saves audio. Transcript History starts Off, while Scratchpad text is saved only after a deliberate Scratchpad capture. Mobile/macOS/Linux, accounts, teams, rewriting, editor extensions, and persistent developer context remain future phases.
