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

If any injection step fails, Lyra leaves the transcript on the clipboard and says `Copied — paste manually.` It never presses Enter.

## Cloud credential

The optional OpenAI credential is entered through the settings flow and stored in Windows Credential Manager. It is not written to the JSON settings file, logs, frontend bundle, or Git.
