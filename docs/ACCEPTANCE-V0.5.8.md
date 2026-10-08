# Lyra v0.5.8 hotfix acceptance

Release date: 2026-10-08 (America/New_York).

## Scope

- Capitalize fresh dictations using the existing formatting switch and cursor-context policy. Preserve unfinished-sentence continuations, mixed-case names, structured tokens and personalized casing.
- Strip leading/trailing whitespace, BOMs and zero-width space boundary markers from final text before paste, copy or save. Internal text remains intact.
- Pass Words entries into cleanup even when insertion context is unavailable.

## Verification

Main-branch verification was skipped with `[skip ci]` at the user's request after local CI-equivalent checks; the version-tag workflow built and published signed updater artifacts. Annotated `v0.5.8` points to `065d4ba31e4a6c63456cf2ee9e9472363abed3d4`, without a skip directive; the main-branch push tip carries the skip directive on a documentation commit.

| Gate | Result |
| --- | --- |
| Clean dependency installation | Pass; existing `whatwg-encoding` deprecation warning |
| Version, changelog, release configuration, theme separation | Pass |
| Frontend typecheck, production build, tests | Pass; 26 tests in 9 files |
| Signed updater fixture | Pass; valid/corrupt signatures, offline check, failed-install preservation |
| Release-mode Rust tests and default-feature check | Pass; 144 tests, 3 explicitly ignored; default-feature release check and Rust format check |
| Signed seasonal theme fixture | Pass; valid/corrupt catalog signatures, invalid assets and offline cache recovery |
| Native cursor-context fixture | Pass; all eight cases, including a completely empty RichEdit field and lowercase input with boundary whitespace; clipboard sequence preserved |
| Unbundled Tauri release build | Pass; `npm run tauri build -- --no-bundle` produced the default-feature release executable |
| Tagged NSIS/updater publication | Pass; [release run 37812479995](https://github.com/OrionAISystems/Lyra/actions/runs/37812479995); [stable v0.5.8 release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.8) published at 2026-10-08 17:09:48 UTC |
| Public release, updater manifest/signature, changelog mirror | Pass; `npm run verify:public-release -- --version 0.5.8` verified stable latest, the 3,032,914-byte installer, its Tauri signature, and signed seasonal catalog revision 1 |
| Exact public changelog mirror | Pass; public and tagged source Git blobs both equal `de8c46d635922f066c1e0631dd8446e0198facfe` |

Installer SHA-256: `c2d529f61f39139273361135f85c1101f568871fb76d0b785d087d3b7c859901`. Public assets are only `latest.json`, `Lyra_0.5.8_x64-setup.exe` and its `.sig` companion. No source, benchmark inputs, models or runtime archives were published.

Installed-app microphone dictation and the installed v0.5.7-to-v0.5.8 updater flow have not been exercised. Unit tests and synthetic fixtures do not establish those checks. No models or runtime artifacts are installed or replaced during this release verification.

The first native fixture pass returned unavailable context once for the sentence-boundary case. That case passed on an isolated retry and all eight cases passed on a full rerun. Native context availability remains transient; the production reader retains its bounded budget and fresh-utterance fallback.
