# v0.5.12 acceptance

Scope: multiple recorded dictation bindings, independent Hold/Toggle modes, initiating-binding ownership, schema v7 migration and compact dropdown bounds.

## Local evidence

- Frontend typecheck, 38 tests, production build and whitespace checks passed.
- Rust formatting, default-feature release checking and regression-bench release tests passed locally: 151 passed, 5 intentionally ignored. The local tree includes pre-existing uncommitted runtime changes excluded from this release; Windows CI verifies the committed source independently.
- The user confirmed MaxxStick F20 Toggle start/stop and Ctrl+Alt+Space Hold dictation/paste in the local release build.
- Playwright in local Chrome verified recorded bindings, independent selectors, mode-preserving edits/promotion, Home guidance, option selection and Escape dismissal. Screenshots at 1020x760 and 760x760 confirm compact menus stay inside the window. No page errors were observed; the Browser plugin was unavailable.
- Local default-feature release executable launched and persisted Ctrl+Alt+Space = Hold, F20 = Toggle and Scratchpad = Hold. Detailed historical evidence is in [ACCEPTANCE-SHORTCUTS.md](ACCEPTANCE-SHORTCUTS.md).

## Publication gates

Windows main CI, the tagged release workflow and public release verification are pending. Installed v0.5.11-to-v0.5.12 updater acceptance is not claimed.
