# v0.5.13 acceptance

Scope: History listing on a blocking worker with a shared retention/metadata scan, and sanitized runtime download failure guidance with partial-download preservation. Includes all committed changes since v0.5.12.

## Verification and publication

- Local version, changelog, release configuration and theme feed separation checks passed.
- Local frontend typecheck, all 38 tests and production build passed.
- Rust formatting and regression-bench release tests passed locally: 154 passed, 4 intentionally ignored. Coverage includes a 650-record History collection and loopback download failure/partial-file preservation.
- Local updater fixture passed valid/corrupt signatures, offline handling and failed-install preservation checks.
- Local default-feature release check and signed seasonal theme fixture passed, including corrupt catalog/artwork rejection and offline cache recovery.
- [Windows main CI](https://github.com/OrionAISystems/Lyra/actions/runs/39178014391) passed on release commit `500ab880ee838d89207d622bbee452d6254505c3`, including frontend checks, Rust regression-bench tests (154 passed, 4 ignored), signed theme fixtures and the default application build.
- Annotated tag `v0.5.13` points to that commit. The [tagged release workflow](https://github.com/OrionAISystems/Lyra/actions/runs/39180628080) passed, including default-feature Rust tests (144 passed, 4 ignored), signed theme fixtures and NSIS publication.
- `npm run verify:public-release -- --version 0.5.13` passed on October 18, 2026 (America/New_York): public changelog matches, GitHub latest and `latest.json` identify v0.5.13, downloaded installer digest and published Tauri signature verify, and the separate signed seasonal catalog verifies.
- [Published stable release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.13): `Lyra_0.5.13_x64-setup.exe`, 3,052,695 bytes, SHA-256 `6756ff239df86b5ab1b2da8ddcdac3a190480ac6ed3d442010988324635a9ae9`, companion signature and updater manifest. No Authenticode certificate was configured; the Windows installer is unsigned by Authenticode, independently of its verified Tauri updater signature.
- GitHub ruleset inspection returned HTTP 403 with a plan/public-repository requirement. Tag-protection configuration could not be independently verified; no protection settings were changed.

## Acceptance limits

Installed History responsiveness with a real collection, installed v0.5.12-to-v0.5.13 updating, microphone/capture/insertion behavior and real runtime network recovery have not been verified for this release. Automated saved-item and loopback download tests do not establish those hardware or installed-app outcomes. No model or runtime downloads are required for release verification.
