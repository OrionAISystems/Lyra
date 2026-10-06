# Lyra v0.5.7 release acceptance

Release date: 2026-10-06 (America/New_York). This maintenance release ships cursor-context dictation casing from [PR #10](https://github.com/OrionAISystems/Lyra/pull/10).

## Included changes

- Read a bounded text prefix immediately before the focused insertion caret and lowercase only an opening dictated word when the existing text continues an unfinished sentence.
- Preserve current formatting and personalization behavior. Unsupported controls or focus changes fall back to the recognized casing.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| PR #10 Windows verification | Pass | [Run 37417515738](https://github.com/OrionAISystems/Lyra/actions/runs/37417515738); merged as `88c0c77` |
| Version, changelog and release configuration | Pending | v0.5.7 release preparation checks |
| Main branch verification and release-cache warm | Pending | Runs on the exact v0.5.7 release commit |
| Protected `v0.5.7` tag and tagged build/publication | Pending | Tag-triggered workflow |
| Authenticode signing | Pending | Release workflow reports whether the optional certificate is configured |
| Public changelog, installer, updater signature and `latest.json` | Pending | `npm run verify:public-release -- --version 0.5.7` |
| Public changelog mirror matches tagged source | Pending | Compare public `CHANGELOG.md` Git blob with tagged canonical source |

## Development acceptance

| Check | Result | Evidence |
| --- | --- | --- |
| Cursor-context casing in the development bundle | User-reported pass | On 2026-10-06, the user said the behavior seems to work. Individual scenarios were not specified. |

Microphone lifecycle, focus changes, and custom editor compatibility beyond the synthetic UI Automation fixture remain separate acceptance areas. Do not infer installed-app or updater behavior from the development-bundle test.
