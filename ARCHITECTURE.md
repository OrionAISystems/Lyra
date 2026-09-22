# Lyra architecture

## Product shape

Lyra is a single-user desktop process with three user-facing surfaces:

1. A system tray entry that owns lifecycle, settings access, onboarding, and quit.
2. A small frameless, transparent dictation bar that stays above the active workspace without taking focus.
3. A frameless settings shell with Home, Dictation, Personalization, Audio & Engine, Insights, Appearance, and Diagnostics modules.

The settings window is intentionally compact rather than dashboard-like. It explains readiness, privacy, routing, and recovery in the places where those choices matter.

The settings shell owns the viewport height. Its sidebar remains pinned as a single viewport column, while the `.content-scroll` pane owns vertical scrolling for long sections such as General and Usage. On narrow windows the same two-row shell keeps navigation visible while the content row scrolls.

## Runtime data flow

```text
global shortcut / bar gesture
          |
          v
ActivationSource -> TargetCapture -> DictationSession state machine
                                      |
                                      v
AudioCapture -> in-memory PCM -> TranscriptionRouter
                                  |               |
                       local Parakeet       optional Nemotron stream
                                  |               | request-scoped partials
                                  +-------+-------+
                                          |
                              final result / local retry
                                          |
                                  optional cloud fallback
                                          |
                                          v
                   normalize -> backtracking -> formatting -> corrections -> snippets
                                          |
                                          v
                   ClipboardSnapshot + sequence guard -> target-aware injection
                                          |
                                          v
                                  result / recovery events

release tag -> private CI -> changelog sync -> NSIS + Tauri-signed updater artifact -> public Lyra-Releases
                                                                                     ^
                                                                         signed latest.json polling
```

The Rust side owns state transitions and side effects. The frontend renders events and issues intent-level commands; it does not own the recording, injection, update, or recognizer lifecycle. A capture coordinator consumes every endpoint through the same processing path, so manual stop, maximum duration, and silence cannot drift in behavior or metrics.

## Rust boundaries

- `domain.rs`: serializable product types, state transition rules, deterministic formatting/correction processing, and routing decisions.
- `settings.rs`: versioned schema v3, legacy migration, normalization, and persistence. Secrets are not settings.
- `audio.rs`: device enumeration, format conversion, mono downmix, in-memory capture, RMS speech arming, silence/max/manual endpoints, dynamic duration control, level events, and ephemeral WAV encoding.
- `transcription.rs`: `TranscriptionBackend` contract, local/cloud implementations, route/fallback policy, and recognizer lifecycle status.
- `native/nemo.rs`: app-local dynamic loading boundary for the pinned NeMo-Speech.cpp C ABI. No machine-wide SDK or PATH dependency.
- `runtime.rs`: pinned runtime/model metadata, resumable downloads, checksum verification, safe staged extraction, and explicit setup progress.
- `updater.rs`: `UpdateProvider` seam, stable-version filtering, signed Tauri updater integration, update state machine, scheduling, passive restart behavior, and the installation reservation gate.
- `usage.rs`: local aggregate counters, daily history, metadata-only recent sessions, and derived metrics. It never receives transcript text or raw samples.
- `tray.rs`: the single tray update menu item, kept in sync with updater status.
- `clipboard.rs`: text snapshot, sequence-number guard, and Windows clipboard operations.
- `windows.rs`: foreground target capture, display enumeration, active/primary/fixed monitor placement, fallback status, non-activating bar behavior, and `SendInput` integration.
- `commands.rs`: Tauri commands and event emission; this is presentation wiring, not domain policy.

## v0.4 settings and lifecycle contracts

Schema v3 separates growing personalization data into an atomic local store. Processing order is normalize → explicit-cue backtracking → spoken formatting → corrections → snippets. The usage ledger retains 90 daily aggregate buckets and 200 privacy-safe sessions; it never receives raw samples or recognized text. Model artifacts have isolated paths and markers, and the experimental Nemotron path emits request-scoped partial events while only a final result can enter text processing or paste.

Recovery shortcuts are registered as a set with duplicate validation and rollback. Tray hiding requires a valid Open Settings shortcut. Target identity is reduced to an executable basename and built-in category before profile matching; full paths and window titles never cross into persistence.

The benchmark records one phrase, retains its PCM only in memory, replays it through installed eligible engines, and persists numeric scores plus environment metadata. A recommendation is informational until the user explicitly applies it.

## v0.3 settings and lifecycle contracts

`Settings` is persisted as schema v2. `autoStopAfterSilenceMs` is either null or one of 1000, 1500, 2000, or 3000; it is passed only to toggle captures. The bar and capture worker both use `maxDurationSeconds`, and an active `CaptureControl` receives an atomic update when that value changes. A lowered limit is observed by the capture worker within its polling interval.

The primary bar gesture is derived from the durable activation mode: toggle uses click-to-toggle, while push-to-talk uses pointer capture and stops on pointer-up, cancel, or lost capture. The hotkey uses the same activation controller. The frontend never invents a second activation setting.

Text processing is deterministic and ordered: normalize line endings, replace whole formatting phrases, normalize punctuation/quote spacing, apply enabled whole-word corrections longest-source-first, then run conservative insertion cleanup. Corrections are case-insensitive unless an entry opts into case sensitivity.

The bar is placed at startup and at session start. Follow-active-window resolves the target window's monitor at those points; fixed placement resolves the saved monitor identity first and ordinal second. Missing fixed monitors fall back to primary and emit a non-blocking placement status.

The model artifact state (`RuntimeStatus`) is separate from recognizer state (`ModelRuntimeStatus`). `always_warm` preloads asynchronously when verified artifacts exist. Timed policies unload only after recognizer inactivity, and `never_keep_warm` releases the recognizer after each request; the recognizer lock prevents unload during recognition. A failed CUDA or Vulkan recognition retries against an already-installed verified fallback runtime before the error reaches cloud routing.

The updater reserves installation before the async download begins. The reservation is made while the app state lock is held by the command, so a new capture cannot slip between the session check and installation. Errors release the reservation; a successful passive install keeps it until restart.

## Privacy boundaries

- Raw samples exist only in memory for the lifetime of a session.
- A PCM WAV is created only for a cloud upload and is dropped after the request.
- Only the last transcript is retained in memory for recovery.
- `%LOCALAPPDATA%\Lyra\usage.json` contains aggregate counters for dictations, audio duration, result timing, backend counts, failures, paste attempts, and fallbacks. Derived cost estimates are opt-in and use the local configured rate.
- Logs contain state, backend, timing, and redacted failures; never raw audio, transcript text, API keys, or clipboard content.
- Cloud credentials use Windows Credential Manager through the Rust backend.
- Release artifacts are published separately from source in the public `OrionAISystems/Lyra-Releases` repository. The client only trusts the configured Tauri updater public key and HTTPS manifest endpoint.

## Extension points

Capture and endpointing remain utterance-oriented. The optional Nemotron backend already streams each captured request through the native recognizer and emits request-scoped partials; only its final result enters text processing or paste. Native multi-final VAD/endpointing and simultaneous warm models remain future work. Backend results include model, language metadata when available, duration, latency, and fallback reason so the UI and diagnostics do not need backend-specific knowledge.

The installer deliberately contains only the Lyra application. Runtime and model installation is an explicit bootstrapper flow under `%LOCALAPPDATA%\Lyra`, so application upgrades do not overwrite user-managed recognition artifacts. The local updater fixture lives outside production configuration and uses an ephemeral signing key plus an HTTP endpoint only for tests; production continues to use the committed public key and HTTPS release-only feed.

`CHANGELOG.md` is the canonical source-repository release history. The tagged release workflow validates that its newest section matches the application version, mirrors it to the public release-only repository, and uses that section as the release body shown alongside the updater artifacts.
