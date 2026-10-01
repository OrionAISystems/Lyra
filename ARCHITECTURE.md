# Lyra architecture

## Product shape

Lyra is a single-user desktop process with three user-facing surfaces:

1. A system tray entry that owns lifecycle, settings access, onboarding, and quit.
2. A small frameless, transparent dictation bar that stays above the active workspace without taking focus.
3. A frameless settings shell with Home, Dictation, History, Scratchpad, Personalization, Audio & Engine, Insights, Appearance, and Diagnostics modules.

The settings window is intentionally compact rather than dashboard-like. It explains readiness, privacy, routing, and recovery in the places where those choices matter.

The settings shell owns the viewport height. Its sidebar remains pinned as a single viewport column, while the `.content-scroll` pane owns vertical scrolling for long sections such as General and Usage. On narrow windows the same two-row shell keeps navigation visible while the content row scrolls.

Right-clicking the dictation bar opens Settings, Hide bar, and Quit Lyra. The transparent bar window grows upward while this menu is open and keeps its lower edge fixed, so the menu remains visible above the bar without moving the dictation control. A second right-click or a click in the empty window area closes it.

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
                                request destination
                                   /          \
                                  /            \
                         Dictation          Scratchpad
                         /       \              |
             optional encrypted  safe paste    encrypted local
               History copy      or copy       Scratchpad entry
                         \          |             /
                           result / recovery events

release tag -> private CI -> changelog sync -> NSIS + Tauri-signed updater artifact -> public Lyra-Releases
                                                                                     ^
                                                                         signed latest.json polling

seasonal catalog URL -> Tauri signature verification -> bounded catalog/PNG validation -> verified theme content -> renderer
        ^                                      |
        |                                      +-> content-addressed artwork cache under local app data
public Lyra-Releases theme metadata + versioned PNG prerelease assets
```

The Rust side owns state transitions and side effects. The frontend renders events and issues intent-level commands; it does not own the recording, injection, update, or recognizer lifecycle. A capture coordinator consumes every endpoint through the same processing path, so manual stop, maximum duration, and silence cannot drift in behavior or metrics.

## Rust boundaries

- `domain.rs`: serializable product types, state transition rules, deterministic formatting/correction processing, and routing decisions.
- `settings.rs`: versioned schema v5, legacy migration, normalization, and persistence. Secrets are not settings.
- `seasonal_themes.rs`: signed catalog validation, bounded PNG downloads and SHA-256 checks, annual date selection, verified artwork cache, and renderer content. Seasonal release assets never use the app updater manifest.
- `audio.rs`: device enumeration, format conversion, mono downmix, in-memory dictation capture, RMS speech arming, silence/max/manual endpoints, dynamic duration control, level events, an explicit sample-free input-level monitor, and ephemeral WAV encoding.
- `transcription.rs`: `TranscriptionBackend` contract, local/cloud implementations, route/fallback policy, streaming partial drains that preserve the active native session, and recognizer lifecycle status.
- `native/nemo.rs`: app-local dynamic loading boundary for the pinned NeMo-Speech.cpp C ABI. No machine-wide SDK or PATH dependency.
- `runtime.rs`: pinned runtime/model metadata, resumable size-bounded downloads, checksum verification, safe staged extraction, and explicit setup progress.
- `updater.rs`: `UpdateProvider` seam, stable-version filtering, signed Tauri updater integration, update state machine, scheduling, passive restart behavior, and the installation reservation gate.
- `usage.rs`: local aggregate counters, indefinite daily aggregates keyed to the Windows time zone, one-year metadata-only sessions, and derived metrics. It never receives transcript text or raw samples. A one-time migration moves retained completed-session metadata from legacy UTC buckets to local calendar days; older aggregates without timestamps remain on their original dates.
- `saved_items.rs`: independent History and Scratchpad file stores, user-bound DPAPI protection for text, retention, bounded paginated metadata reads, and collection-specific operations.
- `tray.rs`: the single tray update menu item, kept in sync with updater status.
- `clipboard.rs`: bounded Windows text snapshots, sequence-number-guarded clipboard recovery, and clipboard operations.
- `windows.rs`: foreground target capture, display enumeration, active/primary/fixed monitor placement, fallback status, non-activating bar behavior, and `SendInput` integration with an HWND/process/focus recheck before each injection.
- `commands.rs`: Tauri commands and event emission; this is presentation wiring, not domain policy.

## v0.5 settings and lifecycle contracts

Schema v5 adds saved seasonality and seasonal-motion preferences while preserving existing user settings. Processing order is normalize → explicit-cue backtracking → spoken formatting → corrections → snippets. Each capture has an immutable request destination: Dictation follows its original target and optional History save; Scratchpad stores a deliberate thought and never auto-pastes or copies it. Both use the same microphone, recognition, fallback, and text-processing pipeline. Audio & Engine also provides an explicit microphone test: it opens the selected device and reports throttled RMS levels without creating a capture buffer or invoking recognition. History and Scratchpad use separate local stores; only their text payloads are DPAPI-protected, and only explicitly requested item details cross the Rust/frontend bridge. The usage ledger retains daily aggregates indefinitely plus metadata-only sessions up to one year; it never receives raw samples or recognized text. Model artifacts have isolated paths and markers, and the experimental Nemotron path emits request-scoped partial events while only a final result can enter text processing or a saved-item store. Partial-result drains do not finish the native session; the router explicitly finishes and drains after the final audio chunk.

Recovery shortcuts are registered as a set with duplicate validation and rollback. Tray hiding requires a valid Open Settings shortcut. Target identity is reduced to an executable basename and built-in category before profile matching; full paths and window titles never cross into persistence.

The benchmark records one phrase, retains its PCM only in memory, replays it through installed eligible engines, and persists numeric scores plus environment metadata. A recommendation is informational until the user explicitly applies it.

## v0.3 settings and lifecycle contracts

`Settings` is persisted as schema v2. `autoStopAfterSilenceMs` is either null or one of 1000, 1500, 2000, or 3000; it is passed only to toggle captures. The bar and capture worker both use `maxDurationSeconds`, and an active `CaptureControl` receives an atomic update when that value changes. A lowered limit is observed by the capture worker within its polling interval.

The primary bar gesture is derived from the durable activation mode: toggle uses click-to-toggle, while push-to-talk uses pointer capture and stops on pointer-up, cancel, or lost capture. The hotkey uses the same activation controller. The frontend never invents a second activation setting.

On Windows, the global shortcut library reports release of its trigger key. Releasing Space ends the default push-to-talk chord; releasing only Ctrl or Alt while Space remains held does not. Paste input checks held modifier state and temporarily lifts unrelated modifiers in one `SendInput` batch so Ctrl+Alt held after Space release does not turn Ctrl+V into Ctrl+Alt+V.

Text processing is deterministic and ordered: normalize line endings, replace whole formatting phrases, normalize punctuation/quote spacing, apply enabled whole-word corrections longest-source-first, then run conservative insertion cleanup. Corrections are case-insensitive unless an entry opts into case sensitivity.

The bar is placed at startup and at session start. Follow-active-window resolves the target window's monitor at those points; fixed placement resolves the saved monitor identity first and ordinal second. Missing fixed monitors fall back to primary and emit a non-blocking placement status.

The model artifact state (`RuntimeStatus`) is separate from recognizer state (`ModelRuntimeStatus`). `always_warm` preloads asynchronously when verified artifacts exist. Timed policies unload only after recognizer inactivity, and `never_keep_warm` releases the recognizer after each request; the recognizer lock prevents unload during recognition. A failed CUDA or Vulkan recognition retries against an already-installed verified fallback runtime before the error reaches cloud routing.

The updater reserves installation before the async download begins. The reservation is made while the app state lock is held by the command, so a new capture cannot slip between the session check and installation. Errors release the reservation; a successful passive install keeps it until restart.

## Privacy boundaries

- Raw samples exist only in memory for the lifetime of a session.
- The microphone test computes RMS from each callback buffer and retains no sample array. It emits only throttled numeric level readings and stops when the test is stopped, the user leaves Audio & Engine, a dictation or benchmark begins, the settings window is closed, or Lyra exits.
- A PCM WAV is created only for a cloud upload and is dropped after the request.
- The latest transcript remains in memory for normal recovery. Optional Transcript History stores final, processed dictation text only when its retention is enabled; History starts Off. Scratchpad stores final text only after an intentional Scratchpad capture or History promotion. Both stores are under `%LOCALAPPDATA%\Lyra\saved-items\` and protect text using current-user DPAPI; metadata is kept beside the encrypted text.
- `%LOCALAPPDATA%\Lyra\usage.json` contains aggregate counters for dictations, audio duration, result timing, backend counts, failures, paste attempts, and fallbacks. Daily aggregates remain indefinitely; metadata-only sessions expire after a year. Derived cost estimates are opt-in and use the local configured rate.
- Logs contain state, backend, timing, and redacted failures; never raw audio, transcript text, API keys, or clipboard content. Persisted fallback details use fixed labels rather than free-form error strings.
- Copied diagnostic reports are built from an allowlist and omit local filesystem paths, endpoints, executable paths, and free-form errors.
- Cloud credentials use Windows Credential Manager through the Rust backend.
- Release artifacts are published separately from source in the public `OrionAISystems/Lyra-Releases` repository. The client only trusts the configured Tauri updater public key and HTTPS manifest endpoint.

## Extension points

Capture and endpointing remain utterance-oriented. The optional Nemotron backend already streams each captured request through the native recognizer and emits request-scoped partials; only its final result enters text processing or paste. Native multi-final VAD/endpointing and simultaneous warm models remain future work. Backend results include model, language metadata when available, duration, latency, and fallback reason so the UI and diagnostics do not need backend-specific knowledge.

The installer deliberately contains only the Lyra application. Runtime and model installation is an explicit bootstrapper flow under `%LOCALAPPDATA%\Lyra`, so application upgrades do not overwrite user-managed recognition artifacts. Seasonal artwork uses a separate bounded cache. The local updater fixture lives outside production configuration and uses an ephemeral signing key plus an HTTP endpoint only for tests; the theme fixture generates an ephemeral signed catalog and passes it through the Rust production verification path. Production continues to use the committed public key and HTTPS release-only feeds.

`CHANGELOG.md` is the canonical source-repository release history. The tagged release workflow validates that its newest section matches the application version, mirrors it to the public release-only repository, and uses that section as the release body shown alongside the updater artifacts.

The manual seasonal publishing workflow creates a versioned prerelease for PNG assets with `make_latest=false`, verifies their bytes, and then commits only the signed `themes/catalog.json` envelope to `Lyra-Releases/main`. The app updater remains pinned to `releases/latest/download/latest.json`; theme tags use the `themes-<id>-v<revision>` form and cannot trigger the app release workflow. See [docs/THEMES.md](docs/THEMES.md) for the signed catalog contract and publication gates.
