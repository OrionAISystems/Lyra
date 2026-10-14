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
  failed-install preservation. Signed seasonal-theme fixture is pending.
- Windows main CI, protected v0.5.11 tag publication and public artifact
  verification are pending.
