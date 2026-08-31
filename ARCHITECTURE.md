# Lyra architecture

## Product shape

Lyra is a single-user desktop process with two user-facing surfaces:

1. A system tray entry that owns lifecycle, settings access, onboarding, and quit.
2. A small frameless, transparent dictation bar that stays above the active workspace without taking focus.

The settings window is intentionally compact rather than dashboard-like. It explains readiness, privacy, routing, and recovery in the places where those choices matter.

## Runtime data flow

```text
global shortcut / bar gesture
          |
          v
ActivationSource -> TargetCapture -> DictationSession state machine
                                      |
                                      v
AudioCapture -> in-memory PCM -> TranscriptionRouter
                                  |                |
                       local NeMo backend     optional cloud backend
                                  \                /
                                   v              v
                                  PostProcessor
                                       |
                                       v
                 ClipboardSnapshot + sequence guard -> SendInput Ctrl+V
                                       |
                                       v
                               result / recovery events
```

The Rust side owns state transitions and side effects. The frontend renders events and issues intent-level commands; it does not own the recording or injection lifecycle.

## Rust boundaries

- `domain.rs`: serializable product types, state transition rules, conservative text normalization, and routing decisions.
- `settings.rs`: versioned settings schema with migration and atomic persistence. Secrets are not settings.
- `audio.rs`: device enumeration, format conversion, mono downmix, in-memory capture, level events, and ephemeral WAV encoding.
- `transcription.rs`: `TranscriptionBackend` contract, local/cloud implementations, and route/fallback policy.
- `native/nemo.rs`: app-local dynamic loading boundary for the pinned NeMo-Speech.cpp C ABI. No machine-wide SDK or PATH dependency.
- `clipboard.rs`: text snapshot, sequence-number guard, and Windows clipboard operations.
- `windows.rs`: foreground target capture, primary-monitor bar placement, non-activating bar behavior, and `SendInput` integration.
- `commands.rs`: Tauri commands and event emission; this is presentation wiring, not domain policy.

## Privacy boundaries

- Raw samples exist only in memory for the lifetime of a session.
- A PCM WAV is created only for a cloud upload and is dropped after the request.
- Only the last transcript is retained in memory for recovery.
- Logs contain state, backend, timing, and redacted failures; never raw audio, transcript text, API keys, or clipboard content.
- Cloud credentials use Windows Credential Manager through the Rust backend.

## Extension points

The v1 session is utterance-oriented, but `AudioCapture` and `TranscriptionBackend` carry request/session boundaries that can support streaming later. Backend results include model, language metadata when available, duration, latency, and fallback reason so the UI and diagnostics do not need backend-specific knowledge.
