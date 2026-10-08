# Lyra v0.5.8 hotfix acceptance

Release date: 2026-10-08 (America/New_York).

## Scope

- Capitalize fresh dictations using the existing formatting switch and cursor-context policy. Preserve unfinished-sentence continuations, mixed-case names, structured tokens and personalized casing.
- Strip leading/trailing whitespace, BOMs and zero-width space boundary markers from final text before paste, copy or save. Internal text remains intact.
- Pass Words entries into cleanup even when insertion context is unavailable.

## Verification

Local gates and publication evidence are recorded here as they finish. Main-branch verification is skipped with `[skip ci]` at the user's request after local CI-equivalent checks; the version-tag workflow still builds and publishes signed updater artifacts. The release tag points to the hotfix commit without a skip directive; a documentation commit carrying the skip directive is the main-branch push tip.

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
| Public release, updater manifest/signature, changelog mirror | Pending |

Installed-app microphone dictation and the installed v0.5.7-to-v0.5.8 updater flow have not been exercised. Unit tests and synthetic fixtures do not establish those checks. No models or runtime artifacts are installed or replaced during this release verification.

The first native fixture pass returned unavailable context once for the sentence-boundary case. That case passed on an isolated retry and all eight cases passed on a full rerun. This remains a provider-readiness limitation; the production reader retains its bounded budget and fresh-utterance fallback.
