# Lyra v0.5.5 release acceptance

v0.5.5 adds the October seasonal theme, persistent seasonality and motion preferences, and the signed seasonal theme catalog path. The app release and any theme catalog publication have separate evidence below.

## Automated and release checks

| Check | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Version, changelog, app release configuration, and theme/app feed separation | Pass | `npm run verify:version`, `npm run verify:changelog` (15 entries), `npm run verify:release-config`, and `npm run verify:themes` | 2026-09-30 | |
| Frontend tests, type check, and production build | Pass | `npm test` (24 tests across 8 files); `npm run typecheck`; `npm run build` | 2026-09-30 | Browser tests cover the renderer and state fixtures; they do not exercise a native Tauri window. |
| Updater fixture | Pass | `npm run test:updater-fixture` | 2026-09-30 | Confirms signature verification, corrupt-signature rejection, failed-install preservation, and offline update checks. |
| Rust format and release tests | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; `cargo test --release --locked --manifest-path src-tauri/Cargo.toml` (90 passed, 2 ignored) | 2026-09-30 | Ignored tests include the live published channel smoke and one fixture test that is exercised by the dedicated signed fixture runner. |
| Signed theme fixture through the Rust production verifier | Pass | `npm run test:themes-fixture` | 2026-09-30 | Temporary Tauri signing, valid catalog acceptance, bad signature and artwork rejection, cache recovery, and same-revision image repair all passed through the Rust production verifier. |
| Windows app release workflow | Pending | | | |
| Public changelog, installer, updater signature, and `latest.json` | Pending | | | Run `npm run verify:public-release -- --version 0.5.5` after both publications. It checks the stable tag, latest manifest, installer signature, signed theme catalog, and PNG hashes without credentials. |
| Theme prerelease and PNG assets | Pending | | | Verify tag `themes-october-v1` is a prerelease; verify each asset name, byte length, PNG validity, and SHA-256. |
| Public raw signed catalog and theme prereleases | Pending | | | After theme publication and before app tagging, run `npm run verify:public-themes -- --latest-version 0.5.4`; it checks the envelope signature/schema/revision, prerelease metadata, each referenced release asset, and PNG size, SHA-256, and structure. |
| Backend live public theme-channel smoke | Pending | | | After catalog publication, run `cargo test --locked --manifest-path src-tauri/Cargo.toml live_published_theme_channel -- --ignored --nocapture`; it exercises production HTTP verification and a temporary offline cache. This is backend delivery evidence only. |
| Stable latest release before and after theme publication | Pending | | | Record `gh api repos/OrionAISystems/Lyra-Releases/releases/latest --jq .tag_name` before and after; pre-tag verification must keep it at v0.5.4, and the post-release full verifier must find v0.5.5. |

## Hands-on checks

| Behavior | Result | Evidence | Date | Notes |
| --- | --- | --- | --- | --- |
| Appearance controls and October artwork in browser-rendered desktop and compact layouts | Pass | Screenshots: `C:\Users\Met\.codex\visualizations\2026\10\01\01a0f506-6fdc-7c32-b744-13c7799969f8` | 2026-09-30 | Home dark/light; Appearance dark/light at 1020 × 760 and 760 × 580; History and Scratchpad. Keyboard focus and Space activation worked for preview and motion controls, system light/dark changes updated appearance, no console errors occurred, and the 92 × 92 mic target stayed unchanged. These are browser renders, not native-window captures. |
| Appearance shows the scheduled theme entry and preview artwork for a future pack | Pass | Same browser screenshot folder; asynchronous November content fixture | 2026-09-30 | Preview switched from October to the future entry; live schedule information showed November 1–30 and identified the remote source. |
| Seasonal artwork for Home and bar state fixtures | Pass | Same browser screenshot folder; all five Home and five bar fixtures | 2026-09-30 | Listening, processing, success, error, empty, and ambience presentation were checked in browser-rendered fixtures. This does not verify the native floating bar window or hardware states. |
| November 1 local midnight removes seasonal layer; next October 1 restores it | Pass | Simulated local date/focus transitions in browser | 2026-09-30 | Saved Blue `#4f7cff` returned after the October interval; next-year October restored seasonal art; year-round preview remained available. |
| Turning seasonality or motion off restores the saved base palette or stops motion | Pass | Browser interaction and state checks | 2026-09-30 | Seasonality-off removed overrides without changing saved preferences; calendar expiry restored Blue `#4f7cff`; motion-off kept static listening glow and stopped decorative motion. |
| Reduced-motion system preference suppresses decorative motion | Pass | Browser reduced-motion emulation | 2026-09-30 | System preference suppressed decorative motion. |
| Existing v0.5.4 settings migrate to schema v5 with prior settings preserved | Pending | | | Verify both new preferences use their documented defaults. |
| Installed v0.5.4 updates to v0.5.5 and reports the new version | Pending | | | Requires the published app release and an installed Windows client. |
| Installed seasonal catalog downloads and uses a verified remote pack | Pending | | | Requires a published theme catalog and an installed client with network access. |

Automated and browser checks do not establish installed-app behavior. Native floating-window geometry/rendering, physical microphone and recognition hardware, cross-application injection, settings migration on an installed v0.5.4 client, and native updater behavior remain pending until directly observed on the target Windows installation. Browser bar fixtures must not be recorded as native bar or hardware evidence.
