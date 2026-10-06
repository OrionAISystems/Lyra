# Lyra v0.5.7 release acceptance

Release date: 2026-10-06 (America/New_York). This maintenance release ships cursor-context dictation casing from [PR #10](https://github.com/OrionAISystems/Lyra/pull/10).

## Included changes

- Read a bounded text prefix immediately before the focused insertion caret and lowercase only an opening dictated word when the existing text continues an unfinished sentence.
- Preserve current formatting and personalization behavior. Unsupported controls or focus changes fall back to the recognized casing.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| PR #10 Windows verification | Pass | [Run 37417515738](https://github.com/OrionAISystems/Lyra/actions/runs/37417515738); merged as `88c0c77` |
| PR #11 release preparation Windows verification | Pass | [Run 37419718951](https://github.com/OrionAISystems/Lyra/actions/runs/37419718951); merged as `abf5a17` |
| Version, changelog, release configuration and theme separation | Pass | Local release preparation checks and exact-main CI [run 37421055823](https://github.com/OrionAISystems/Lyra/actions/runs/37421055823) |
| Frontend checks, release-mode Rust tests and formatting | Pass | PR #11, exact-main [run 37421055823](https://github.com/OrionAISystems/Lyra/actions/runs/37421055823), and tagged [run 37423223391](https://github.com/OrionAISystems/Lyra/actions/runs/37423223391) |
| Exact v0.5.7 main verification and release-cache warm | Pass | [Run 37421055823](https://github.com/OrionAISystems/Lyra/actions/runs/37421055823) at `abf5a17cf1cef6d517f9f1f1a3a7596cb2f87962` |
| Tagged NSIS/updater publication | Pass | [Run 37423223391](https://github.com/OrionAISystems/Lyra/actions/runs/37423223391); annotated `v0.5.7` points to `abf5a17cf1cef6d517f9f1f1a3a7596cb2f87962`; [stable release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.7) published 2026-10-06 at 06:31:43 UTC |
| Windows Authenticode signing | Not configured | The workflow reported no Windows certificate and produced an installer without Authenticode signing; the separate Tauri updater signature was verified below |
| Public changelog, installer/signature, `latest.json` and existing theme assets | Pass | `npm run verify:public-release -- --version 0.5.7`: stable latest v0.5.7, `Lyra_0.5.7_x64-setup.exe` (3,031,040 bytes), published Tauri signature, and signed theme revision 1 with one theme and two PNGs (322,765 bytes) |
| Exact public changelog mirror | Pass | Public `CHANGELOG.md` Git blob equals tagged canonical source: `a21a66524fdf5bbc682f6a00d5acda5230a5a431` |

Installer SHA-256: `afc21dc7e560fc77a51605db6400622cb4cbc670b03cf5fa063e974f76b7deda`. The release contains only `latest.json`, the Windows NSIS installer and its `.sig` companion. No private source, benchmark reports/fixtures, runtime or model artifacts were published.

## Development acceptance

| Check | Result | Evidence |
| --- | --- | --- |
| Cursor-context casing in the development bundle | User-reported pass | On 2026-10-06, the user said the behavior seems to work. Individual scenarios were not specified. |

Microphone lifecycle, focus changes, and custom editor compatibility beyond the synthetic UI Automation fixture remain separate acceptance areas. Do not infer installed-app or updater behavior from the development-bundle test.
