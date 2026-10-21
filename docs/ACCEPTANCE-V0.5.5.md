# Lyra v0.5.5 release acceptance

v0.5.5 adds the October seasonal theme, persistent seasonality and motion preferences, and the signed seasonal theme catalog path. The app release and any theme catalog publication have separate evidence below.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, app release configuration, and theme/app feed separation | Pass | `npm run verify:version`, `npm run verify:changelog` (15 entries), `npm run verify:release-config`, and `npm run verify:themes` | 2026-09-30 | |
| Frontend tests, type check, and production build | Pass | `npm test` (24 tests across 8 files); `npm run typecheck`; `npm run build` | 2026-09-30 | Browser tests cover the renderer and state fixtures; they do not exercise a native Tauri window. |
| Updater fixture | Pass | `npm run test:updater-fixture` | 2026-09-30 | Confirms signature verification, corrupt-signature rejection, failed-install preservation, and offline update checks. |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` (90 passed, 2 ignored) | 2026-10-01 | The ignored opt-in tests are `installed_nemotron_stream_smoke` (requires an installed NeMo runtime/model) and `live_published_theme_channel`; the live theme test passed separately below. |
| Signed theme fixture through the Rust production verifier | Pass | `npm run test:themes-fixture`; also passed in PR #5, main CI, and the tagged release workflow | 2026-10-01 | Temporary Tauri signing, valid catalog acceptance, bad signature and artwork rejection, cache recovery, and same-revision image repair all passed through the Rust production verifier. |
| Windows app release workflow | Pass | [Run 36813156515](https://github.com/OrionAISystems/Lyra/actions/runs/36813156515); tag `v0.5.5` points to `618b5a0aaaffb40643fe68378095cae33ab05d2a` | 2026-10-01 UTC | The NSIS/updater release workflow completed successfully. |
| Public changelog, installer, updater signature, and `latest.json` | Pass | `npm run verify:public-release -- --version 0.5.5` | 2026-10-01 | Verified public changelog and stable latest `v0.5.5`, `Lyra_0.5.5_x64-setup.exe` (3,010,572 bytes), and its published Tauri signature. |
| Theme prerelease and PNG assets | Pass | `npm run verify:public-release -- --version 0.5.5` | 2026-10-01 | The verifier checked the `themes-october-v1` prerelease metadata, both referenced PNG assets, their byte lengths, PNG structure, and SHA-256 values. |
| Public raw signed catalog and theme prereleases | Pass | [Publisher run 36812298661](https://github.com/OrionAISystems/Lyra/actions/runs/36812298661); `npm run verify:public-themes -- --latest-version 0.5.4` | 2026-10-01 | Verified catalog revision 1, its signature/schema and two unique PNG assets before the app tag; stable app latest remained `v0.5.4`. |
| Backend live public theme-channel smoke | Pass | `cargo test --locked --manifest-path src-tauri/Cargo.toml live_published_theme_channel -- --ignored --nocapture` (1 passed) | 2026-10-01 | The production fetch, pinned-key verification, artwork checks, temporary cache stage, and offline reload passed. This is backend delivery evidence only. |
| Stable latest release before and after theme publication | Pass | `gh api repos/OrionAISystems/Lyra-Releases/releases/latest --jq .tag_name`; both public verification commands above | 2026-10-01 | Stable latest remained `v0.5.4` after theme publication and became `v0.5.5` only after the app release. |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Appearance controls and October artwork in browser-rendered desktop and compact layouts | Pass | Screenshots: `local-only QA screenshots (not included in the repository)` | 2026-09-30 | Home dark/light; Appearance dark/light at 1020 × 760 and 760 × 580; History and Scratchpad. Keyboard focus and Space activation worked for preview and motion controls, system light/dark changes updated appearance, no console errors occurred, and the 92 × 92 mic target stayed unchanged. These are browser renders, not native-window captures. |
| Appearance shows the scheduled theme entry and preview artwork for a future pack | Pass | Same browser screenshot folder; asynchronous November content fixture | 2026-09-30 | Preview switched from October to the future entry; live schedule information showed November 1–30 and identified the remote source. |
| Seasonal artwork for Home and bar state fixtures | Pass | Same browser screenshot folder; all five Home and five bar fixtures | 2026-09-30 | Listening, processing, success, error, empty, and ambience presentation were checked in browser-rendered fixtures. This does not verify the native floating bar window or hardware states. |
| November 1 local midnight removes seasonal layer; next October 1 restores it | Pass | Simulated local date/focus transitions in browser | 2026-09-30 | Saved Blue `#4f7cff` returned after the October interval; next-year October restored seasonal art; year-round preview remained available. |
| Turning seasonality or motion off restores the saved base palette or stops motion | Pass | Browser interaction and state checks | 2026-09-30 | Seasonality-off removed overrides without changing saved preferences; calendar expiry restored Blue `#4f7cff`; motion-off kept static listening glow and stopped decorative motion. |
| Reduced-motion system preference suppresses decorative motion | Pass | Browser reduced-motion emulation | 2026-09-30 | System preference suppressed decorative motion. |
| Automated v4-to-v5 settings migration fixture preserves preferences and applies new defaults | Pass | `settings::tests::v4_settings_migrate_to_v5_and_preserve_existing_preferences` in the release Rust suite | 2026-10-01 | The fixture verifies schema v5, seasonality/motion defaults, and prior settings preservation. |
| Installed v0.5.4 client migrates its settings to schema v5 | Pending | | | The automated migration fixture passes; migration on an installed v0.5.4 Windows client has not been observed. |
| Installed v0.5.4 updates to v0.5.5 and reports the new version | Pending | | | The release is public; this still requires an installed Windows client. |
| Installed seasonal catalog downloads and uses a verified remote pack | Pending | | | Requires a published theme catalog and an installed client with network access. |

Automated and browser checks do not establish installed-app behavior. Native floating-window geometry/rendering, physical microphone and recognition hardware, cross-application injection, settings migration on an installed v0.5.4 client, and native updater behavior remain pending until directly observed on the target Windows installation. Browser bar fixtures must not be recorded as native bar or hardware evidence.
