# Cursor-context dictation acceptance

This development change adjusts an opening word when automatic dictation paste continues text at the actual caret. Its policy and fallback behavior are defined in [TEXT-FORMATTING.md](TEXT-FORMATTING.md). The user reports that it seems to work in the development bundle; the individual scenario checks below were not specified and remain pending. Release authorization remains a separate step.

## Automated evidence

| Check | Result | Evidence | Date |
| --- | --- | --- | --- |
| Continuation, sentence/paragraph boundaries, opening quotes, later sentences, I/contractions, acronyms, mixed casing, Words, Corrections, Snippets and formatting opt-out | Pass | Three domain policy tests in the release suite | 2026-10-06 |
| Default-feature release check, format and regression-bench release suite | Pass | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`; `cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench` (142 passed, 3 explicitly ignored); `cargo check --release --locked --manifest-path src-tauri/Cargo.toml`; rerun after the final cancellation guard | 2026-10-06 |
| Native TextPattern range and clipboard preservation | Pass | `scripts/cursor-context-fixture.ps1`: all seven cases passed (continuation, caret in the middle, bounded lookback, sentence boundary, selection, password and unsupported legacy field); each also rejects a mismatched process and preserves the clipboard sequence | 2026-10-06 |
| Development executable with embedded frontend | Pass | `npm run build`; `npm run tauri:build -- --debug --no-bundle` rebuilt `src-tauri/target/debug/lyra.exe` after the final cancellation guard | 2026-10-06 |
| Overall hands-on development-bundle use | User reports working | User said the behavior seems to work in the running development bundle; individual scenarios were not specified | 2026-10-06 |

The v0.5.8 fixture additionally covers a completely empty RichEdit field and passes lowercase input with boundary whitespace through cleanup. All eight cases passed locally on 2026-10-08; the first pass had one transient unavailable sentence-boundary query, followed by an isolated pass and a full passing rerun. See [v0.5.8 acceptance](ACCEPTANCE-V0.5.8.md).

To repeat the native synthetic fixture after compiling the regression-bench test executable:

```powershell
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --features regression-bench --no-run
$cursorTest = Get-ChildItem src-tauri/target/release/deps/lyra_lib-*.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1
powershell -NoProfile -File scripts/cursor-context-fixture.ps1 -TestBinary $cursorTest.FullName
```

The fixture temporarily shows/focuses its own Windows forms, clicks only a point verified to belong to the synthetic form and restores the pointer, records only window/process metadata in TEMP, checks the clipboard sequence without reading clipboard text, and closes/cleans its resources afterward. It does not invoke recognition or inspect user documents. Positive cases use Windows RichEdit fields; the legacy Windows Forms plain TextBox is rejected by the reader in this environment and verifies the fallback rather than being counted as supported.

The work is on `codex/cursor-context-dictation`, based on remote main `284d89d` (v0.5.6). The previous feature branch and its local documentation commit remain intact. Branch cleanup is deferred until after testing and merge.

For the hands-on test, quit installed Lyra through its tray menu, then open `src-tauri/target/debug/lyra.exe`. This debug executable embeds the built frontend and needs no Vite server; it uses the existing local settings/runtime/model, so the installed single instance must be closed first. Return to the installed application by quitting the debug app and launching installed Lyra normally. Application versioning/publication remains a later authorized release step.

## Hands-on development checks

| Behavior | Result | Evidence |
| --- | --- | --- |
| Type an unfinished sentence, then dictate an ordinary continuation in the actual browser/Codex/editor used daily | Pending | User dev-bundle test |
| Stop a dictation without terminating punctuation and resume at the same caret | Pending | User dev-bundle test |
| Move the caret into existing typed text, then dictate | Pending | User dev-bundle test |
| Dictate after `.`, `!`, `?`, and on an empty field/new paragraph | Pending | User dev-bundle test |
| Names in Words, I/contractions, acronyms and explicit correction/snippet casing remain intact | Pending | User dev-bundle test |
| Formatting off, clipboard-only output and Scratchpad retain their behavior | Pending | User dev-bundle test |
| Changed focus/unsupported editor falls back safely; target/focus guarded paste remains intact | Pending | User dev-bundle test |

Synthetic fields establish the accessibility mechanism and deterministic policy, not microphone recognition, installed-app behavior or universal custom-editor support. No release is authorized by these checks.
