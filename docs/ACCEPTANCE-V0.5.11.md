# v0.5.11 acceptance

Scope: expose persistent dictation bar visibility in Appearance and the tray
menu, and make the bar's Hide bar action use the same saved preference.

## Automated and rendered evidence

- Frontend typecheck, 28 tests and production build passed before release preparation.
- Appearance regression covers hiding/showing the bar and reflecting settings
  changes from other surfaces.
- Rust formatting and default-feature release check passed. Release tests with
  `regression-bench`: 145 passed, 5 intentionally ignored in the local working
  tree, including pre-existing uncommitted runtime changes outside this release.
  CI verifies the committed release source independently.
- Playwright CLI inspected Appearance at 1000x740 and 760x600, verified mouse
  toggling and keyboard Space activation, and captured the rendered control.
  Browser plugin was unavailable. Browser preview cannot invoke native tray or
  window operations. The preview's favicon request returned 404; no application
  errors or framework overlay were observed.

## Remaining desktop acceptance

- Native tray show/hide, convergence with the open settings window, persisted
  visibility after restart, and dictation while hidden remain unverified by a
  hands-on desktop run.
- Installed v0.5.10-to-v0.5.11 updater acceptance remains unverified.

## Release gates and publication

- Version, changelog, release configuration and theme feed separation checks passed.
- Signed updater fixture passed: valid/corrupt signatures, offline handling and
  failed-install preservation. Signed seasonal-theme fixture passed locally.
- [Windows main CI](https://github.com/OrionAISystems/Lyra/actions/runs/38613812495)
  passed for release source commit `d88e818`, including 28 frontend tests,
  144 Rust tests with `regression-bench` (4 intentionally ignored), the signed
  seasonal fixture and unbundled Tauri release build.
- Annotated tag `v0.5.11` points to that tested commit and was accepted by the
  remote. GitHub tag protection inspection was unavailable: rulesets returned
  a plan-related 403 and the legacy tag protection endpoint returned 404.
- [The tagged release workflow](https://github.com/OrionAISystems/Lyra/actions/runs/38617544482)
  passed, including 28 frontend tests, 134 default-feature Rust tests (4
  intentionally ignored), signed seasonal fixture and NSIS/updater publication.
- Public release is [v0.5.11](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.11),
  neither draft nor prerelease. GitHub latest and `latest.json` resolve to v0.5.11.
- `npm run verify:public-release -- --version 0.5.11` passed: canonical public
  changelog, stable latest release/feed, installer bytes and published Tauri
  signature, plus the separate signed seasonal catalog/artwork.
- Installer: `Lyra_0.5.11_x64-setup.exe`, 3,036,528 bytes;
  SHA-256 `0c930fd71fffb77293a1472e4ab201f301b9ab8d7e76b1c1f3d118b126efc77b`.
  Installer, detached `.sig` and `latest.json` are published. This verifies
  Tauri updater signing; it does not establish installed-updater acceptance.
  No Windows Authenticode certificate was configured; the installer is
  Authenticode-unsigned under the existing optional-signing release policy.
