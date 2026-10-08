# Lyra v0.5.9 composer hotfix acceptance

Release date: 2026-10-08 (America/New_York).

## Scope and diagnosis

The user reports a leading space and lowercase opening in an empty Codex/ChatGPT composer with installed v0.5.8; other editors work correctly. A bounded, read-only UI Automation probe against the running Codex `ProseMirror` field reproduced a 128-unit preceding range ending in an alphanumeric character. Its document and caret enclosing elements differed from the field; `RangeFromChild(field)` returned the same document range. A metadata-only ancestry check identified the document owner as a `placeholder` Group descendant of the composer. This establishes ambiguous ownership, not that the returned text came from the conversation. Only lengths, character categories, classes and endpoint/ancestry relationships were inspected; no surrounding text was logged or persisted.

The reader now rejects a TextPattern whose document enclosing element differs from the focused field, before retrieving text. Accepted caret ranges must lie within that document, and lookback is clamped to its start. Rejected providers use the existing fresh-dictation fallback, with boundary trimming and personalization intact. This deliberately disables automatic continuation casing in editors exposing placeholder/container-owned ranges, including the observed Codex provider, while retaining continuation behavior in field-owned editors.

## Verification

Main will carry `[skip ci]` on a documentation-only tip after passing local checks; the annotated `v0.5.9` tag will target the release source commit without a skip directive, allowing the existing signed tag publisher to run.

| Gate | Result |
| --- | --- |
| Clean dependency installation | Pass; existing `whatwg-encoding` deprecation warning |
| Version, changelog, release configuration, theme separation | Pass |
| Frontend typecheck, production build, tests | Pass; 26 tests in 9 files |
| Signed updater fixture | Pass; valid/corrupt signatures, offline check, failed-install preservation |
| Release-mode regression-bench Rust tests | Pass; 144 tests, 4 explicitly ignored native/fixture checks |
| Live Codex delegated TextPattern regression | Pass; running Codex provider exposes a placeholder-owned document; production guard rejects it and fallback produces `And this is dictated` without boundary whitespace |
| Native cursor-context fixture | Pass; all eight synthetic cases, including empty field, continuation, middle caret, bounded lookback, sentence boundary, selection, password and unsupported field; clipboard sequence preserved |
| Default-feature check/tests, Rust format | Pass; release check, 134 default tests with 4 explicitly ignored, and format check |
| Signed seasonal theme fixture | Pass; valid/corrupt signatures, invalid assets and offline cache recovery |
| Unbundled Tauri release build | Pass; `npm run tauri build -- --no-bundle`, executable file version `0.5.9` |
| Tagged publication and public artifacts | Pending |

The read-only live provider regression is opt-in and never changes focus, selection, text or the clipboard:

```powershell
$composer = Get-Process ChatGPT | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$env:LYRA_COMPOSER_PROBE_HWND = $composer.MainWindowHandle.ToString()
$env:LYRA_COMPOSER_PROBE_PID = $composer.Id.ToString()
# Use the regression-bench test binary emitted by cargo test --no-run.
& $cursorTest.FullName --ignored --exact cursor_context::platform::tests::delegated_composer_range_is_rejected_without_reading_text
Remove-Item Env:LYRA_COMPOSER_PROBE_HWND, Env:LYRA_COMPOSER_PROBE_PID
```

Installed microphone dictation, the separate ChatGPT desktop provider and the installed v0.5.8-to-v0.5.9 updater flow remain untested. The live Codex provider check establishes rejection of its misleading range and deterministic fallback, not end-to-end audio capture/paste. No model/runtime artifacts are installed or replaced during release verification.
