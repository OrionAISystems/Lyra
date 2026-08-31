# Lyra setup

## Installed application

The v0.2.0 Windows x64 release is a per-user NSIS installer. It creates the Lyra Start Menu entry and offers the desktop shortcut during setup. It does not require administrator elevation and does not bundle the native speech runtime or the 681 MB model. Existing debug builds remain separate under `src-tauri\target\debug`.

The installer uses Tauri's WebView2 download bootstrapper. On a machine without a suitable WebView2 runtime, the installer obtains the standard Microsoft runtime; Lyra itself never changes `PATH`, installs the CUDA Toolkit, or installs CMake/Ninja.

## Prerequisites

Development requires Node.js, Rust with the MSVC Windows target, and the Tauri prerequisites. The packaged app does not require a machine-wide CUDA Toolkit, CMake, Ninja, FFmpeg, or NeMo-Speech.cpp installation.

## Runtime and model policy

Lyra keeps the native runtime and model in app-managed directories:

- Runtime: `%LOCALAPPDATA%\Lyra\runtime\0.1.0\windows-x86_64-<variant>`
- Model: `%LOCALAPPDATA%\Lyra\models\parakeet-tdt-0.6b-v3.q8_0.gguf`
- Partial downloads and staged extraction: `%LOCALAPPDATA%\Lyra\staging\`

The setup flow asks for two independent approvals. It shows the official source, exact download size, target path, variant, license/checksum status, and whether CUDA hardware is available. A download is never hidden inside first launch, dependency installation, or a build script. A runtime download resumes from its `.partial` file when the server supports HTTP ranges; a model download follows the same path. Every completed artifact is checked against its pinned SHA-256 before staged installation. Failed, cancelled, or interrupted setup can be retried without treating an unverified artifact as ready.

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

## Updates

Lyra checks the public signed feed at startup, once per hour while running, and when `Check for updates` is pressed in General Settings. It accepts stable versions only and silently treats offline checks as an unavailable feed. Installing is disabled while a dictation session is listening or processing; Windows launches the passive signed NSIS updater and restarts Lyra after installation.

The release workflow publishes only the NSIS installer, Tauri updater artifact/signature, `latest.json`, and sanitized release notes to [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases). Before tagging a production release, configure the Tauri updater public key in `src-tauri/tauri.conf.json`, the GitHub App credentials below, and the following repository secrets/variables:

- `TAURI_SIGNING_PRIVATE_KEY`
- `LYRA_RELEASES_APP_CLIENT_ID` (repository variable)
- `LYRA_RELEASES_APP_PRIVATE_KEY` (repository secret)

`WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` are optional until Authenticode signing is available. When omitted, Lyra publishes an unsigned Windows installer but retains Tauri updater signatures. The v0.2.0 updater key is unencrypted, so no signing-password secret is required; the private key remains protected by GitHub Secret storage. The workflow creates the short-lived GitHub App installation token automatically; no personal access token is stored or manually renewed.
