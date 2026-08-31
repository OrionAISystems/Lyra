# Lyra setup

## Prerequisites

Development requires Node.js, Rust with the MSVC Windows target, and the Tauri prerequisites. The packaged app does not require a machine-wide CUDA Toolkit, CMake, Ninja, FFmpeg, or NeMo-Speech.cpp installation.

## Runtime and model policy

Lyra keeps the native runtime and model in app-managed directories:

- Runtime: `%LOCALAPPDATA%\Lyra\runtime\0.1.0\windows-x86_64-<variant>`
- Model: `%LOCALAPPDATA%\Lyra\models\parakeet-tdt-0.6b-v3.q8_0.gguf`

The setup flow must ask for two independent approvals. It must show the official source, exact download size, target path, variant, license/checksum status, and whether CUDA hardware is available. A download is never hidden inside first launch, dependency installation, or a build script.

## Runtime selection

The first candidate is the official NeMo-Speech.cpp v0.1.0 Windows x64 CUDA archive. If CUDA initialization fails, the user can separately approve the Vulkan archive and then the CPU archive. Lyra does not install a CUDA Toolkit or modify PATH as a recovery action.

## First launch checklist

1. Confirm microphone enumeration and choose the Windows default or a specific device.
2. Detect the GPU and display the measured VRAM.
3. Approve and verify the native runtime.
4. Approve and verify the Parakeet model.
5. Confirm `Ctrl+Alt+Space` and push-to-talk behavior.
6. Record a short sample and verify the target application paste.

The dictation bar starts at the bottom center of the Windows primary monitor and remains there while dictating, even when the target application is on another monitor.

If any injection step fails, Lyra leaves the transcript on the clipboard and says `Copied — paste manually.` It never presses Enter.

## Troubleshooting

Lyra registers the hotkey and its event handler together during startup. If the hotkey still shows `Pending`, another application owns the global registration; fully restart that application, then use `Retry` in Lyra's readiness checklist. Lyra keeps the tray and dictation bar available while the shortcut is unavailable.

The native runtime is loaded with its app-local `bin` directory as the dependency search location. No machine-wide CUDA Toolkit or PATH entry is required. If a local attempt reports `no audio was captured`, the runtime and model have already loaded and the recording itself was empty.

## Cloud credential

The optional OpenAI credential is entered through the settings flow and stored in Windows Credential Manager. It is not written to the JSON settings file, logs, frontend bundle, or Git.
