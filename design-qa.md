# v0.4 design QA

Reference: `docs/design/v0.4-reference.png`

Viewport inspected in the Codex in-app browser on 2026-09-22. The browser surface was narrower than the 1020 × 760 Tauri default, so it also exercised the minimum-width compact navigation state.

| Screen          | Comparison and repair                                                                                                           | Result |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Home            | Matched centered hero, shortcut keys, purple listening orb, three quick actions, readiness context, and restrained quote.       | passed |
| Dictation       | Matched stacked control rows and inline toggles; recovery shortcuts remain visible below the primary controls.                  | passed |
| Personalization | Preserved four explicit tabs, high-contrast add action, empty state, and capability notice.                                     | passed |
| Audio & Engine  | Matched microphone meter, engine card, readiness badges, optional model install, privacy notice, and enabled benchmark action.  | passed |
| Insights        | Matched metric cards, accessible daily bars, range selector, narrative, and detailed-data boundary copy.                        | passed |
| Appearance      | Matched theme previews and functional accent, density, tray, startup, and placement rows.                                       | passed |
| Diagnostics     | Retained the support screen with health cards, artifact status, recovery, sanitized report, and an explicit in-app updater row. | passed |

Keyboard-visible native controls, labelled icon buttons, 32–40px minimum interaction targets, a single content scroll owner, and responsive compact navigation were retained. No P0–P2 visual mismatches remain.

final result: passed
