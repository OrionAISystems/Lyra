# v0.5.12 acceptance

Scope: multiple recorded dictation bindings, independent Hold/Toggle modes, initiating-binding ownership, schema v7 migration and compact dropdown bounds.

## Local evidence

- Frontend typecheck, 38 tests, production build and whitespace checks passed.
- Rust formatting, default-feature release checking and regression-bench release tests passed locally: 151 passed, 5 intentionally ignored. The local tree includes pre-existing uncommitted runtime changes excluded from this release; Windows CI verifies the committed source independently.
- The user confirmed MaxxStick F20 Toggle start/stop and Ctrl+Alt+Space Hold dictation/paste in the local release build.
- Playwright in local Chrome verified recorded bindings, independent selectors, mode-preserving edits/promotion, Home guidance, option selection and Escape dismissal. Screenshots at 1020x760 and 760x760 confirm compact menus stay inside the window. No page errors were observed; the Browser plugin was unavailable.
- Local default-feature release executable launched and persisted Ctrl+Alt+Space = Hold, F20 = Toggle and Scratchpad = Hold. Detailed historical evidence is in [ACCEPTANCE-SHORTCUTS.md](ACCEPTANCE-SHORTCUTS.md).

## Publication gates

- [Windows main CI](https://github.com/OrionAISystems/Lyra/actions/runs/38665059225) passed on release commit `c0bb5fabe13bdfaddc691d3803812f819e54b22b`: frontend checks and 38 tests, Rust regression-bench tests (150 passed, 4 ignored), signed theme fixtures and the default application build.
- Annotated tag `v0.5.12` points to that commit. The [tagged release workflow](https://github.com/OrionAISystems/Lyra/actions/runs/38667171032) passed, including default-feature Rust tests (140 passed, 4 ignored), signed theme fixtures and NSIS publication.
- `npm run verify:public-release -- --version 0.5.12` passed on October 14, 2026 (America/New_York): public changelog matches, GitHub latest and `latest.json` identify v0.5.12, downloaded installer digest and published Tauri signature verify, and the separate signed seasonal catalog verifies.
- [Published stable release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.12): `Lyra_0.5.12_x64-setup.exe`, 3,049,564 bytes, SHA-256 `6c4095bd77de963eb23de451b64ff8f7d669482096d8dab6cd8db5495c60c8a1`, companion signature and updater manifest. No Authenticode certificate was configured; the Windows installer is unsigned by Authenticode, independently of its verified Tauri updater signature.
- Tag-protection configuration could not be independently inspected with the available GitHub account (rulesets HTTP 403; legacy protection endpoint HTTP 404). No protection settings were changed.

Installed v0.5.11-to-v0.5.12 updater acceptance is not claimed. Runtime download diagnostics are an untagged follow-up for the next release and are absent from v0.5.12.
