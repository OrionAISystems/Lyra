# Lyra v0.4 acceptance

The v0.4.0 release evidence below remains the baseline for the v0.4.x series. v0.4.4 is published; publication does not close the installed-hardware cases that remain pending in this matrix.

## Automated evidence

- Frontend typecheck, unit tests, and production build: required before tag.
- Rust formatting, check, and tests: required before tag.
- Version, changelog, release configuration, updater fixture, and whitespace validators: required before tag.
- Migration, retention, privacy serialization, processing order, streaming queue, fallback, and benchmark scoring are covered by focused Rust tests.
- Security regression tests cover open-session partial drains, bounded clipboard reads, sequence-checked clipboard recovery, foreground-target checks, oversized downloads, and allowlisted diagnostic reports.

## Rendered product evidence

- Reference: `docs/design/v0.4-reference.png`.
- Screen-by-screen findings and repairs: `design-qa.md`.
- Required states: seven screens, dark/light/system, compact density, minimum window, focus/hover/open controls, custom title bar, and one content scroll owner.

## Installed-hardware matrix

| Case                                                  | Result                  | Notes                                                                                                            |
| ----------------------------------------------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Existing v0.3 settings/usage migration                | passed in native dev    | Preserved the current 78-start/73-result baseline and v2 user preferences without inventing detailed history.    |
| CUDA / Vulkan / CPU Parakeet                          | pending installed build | Exercise each verified local runtime.                                                                            |
| Nemotron native streaming ABI                         | passed on installed GPU | Loaded the pinned 699,872,960-byte GGUF on CUDA, started a stream, accepted PCM, finalized, and drained cleanly. |
| Nemotron spoken partial / final paste                 | pending installed build | Verify English-only routing, active-request partials, and one final paste with live microphone speech.           |
| Nemotron failure → Parakeet replay                    | pending installed build | Reuse the same in-memory PCM.                                                                                    |
| Notepad, browser, ChatGPT, Codex, VS Code, PowerShell | pending installed build | Include long multiline chunking and recovery.                                                                    |
| v0.3.2 → latest v0.4.x updater                       | pending installed build | v0.4.4 is published; still verify check, download, install, restart, and the migrated settings on an installed client. |

## Native dev-bundle smoke — 2026-09-22

- Launched `src-tauri/target/debug/lyra.exe` through `tauri dev` against the real local data directory.
- Confirmed CUDA discovery on the installed RTX 2060 and successful warm load of the verified 713,975,456-byte Parakeet model.
- Used Windows Computer Use against the native Tauri window; its accessibility tree exposed all seven navigation targets, custom window actions, v0.4.0 label, Home controls, and local readiness.
- Opened Settings using the registered `Ctrl+Alt+Shift+L` recovery shortcut, closed it with the native close gesture, confirmed Lyra remained alive with only the tray/bar window, and reopened Settings with the shortcut.
- The Windows.Graphics.Capture screenshot path returned `SetIsBorderRequired failed: No such interface supported`; the native accessibility path remained usable. Browser-based visual comparison is recorded separately in `design-qa.md`.

## Installed Nemotron ABI smoke — 2026-09-22

- Verified the separately installed `nemotron-speech-streaming-en-0.6b.q8_0.gguf` artifact at 699,872,960 bytes against the pinned SHA-256 before loading it.
- Loaded the NeMo-Speech.cpp 0.1.0 CUDA runtime on the installed NVIDIA GeForce RTX 2060, opened the streaming API, supplied one second of 16 kHz PCM, finalized, drained, and closed the stream successfully.
- The runtime reported that this GGUF has no embedded SentencePiece tokenizer, so it could not tokenize the supplied contextual phrases and disabled word boosting internally. Lyra still passes enabled Words through the owned native speech-context boundary; the experimental capability is surfaced without affecting Parakeet or fallback routing.

No row may be marked passed without the observed installed behavior. Audio and transcript text must not be written during acceptance.
