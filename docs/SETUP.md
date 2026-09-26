# Lyra setup

## Installed application

The v0.5.0 Windows x64 release is a per-user NSIS installer. It creates the Lyra Start Menu entry and offers the desktop shortcut during setup. It does not require administrator elevation and does not bundle the native speech runtime or the 714 MB (about 681 MiB) Parakeet model. Existing debug builds remain separate under `src-tauri\target\debug`.

The installer uses Tauri's WebView2 download bootstrapper. On a machine without a suitable WebView2 runtime, the installer obtains the standard Microsoft runtime; Lyra itself never changes `PATH`, installs the CUDA Toolkit, or installs CMake/Ninja.

## Prerequisites

Development requires Node.js, Rust with the MSVC Windows target, and the Tauri prerequisites. The packaged app does not require a machine-wide CUDA Toolkit, CMake, Ninja, FFmpeg, or NeMo-Speech.cpp installation.

## Settings and dictation policy

Settings are persisted as schema v4 under the app configuration directory. Existing settings migrate on first load; legacy Corrections move into the separate local personalization store, which also holds Words, Snippets, app profiles, and self-correction preferences. Transcript History migrates as Off, and existing shortcuts and other preferences are preserved.

Push-to-talk on the bar starts on primary-pointer down and stops on pointer up, cancellation, or lost pointer capture. Toggle mode starts and stops on clicks. Silence endpointing is off by default, is available at 1.0, 1.5, 2.0, or 3.0 seconds, and applies only after a short voiced interval has crossed Lyra's fixed conservative RMS threshold. Maximum duration changes are applied to the persistent bar and active capture without restarting Lyra.

## Runtime and model policy

Lyra keeps the native runtime and model in app-managed directories:

- Runtime: `%LOCALAPPDATA%\Lyra\runtime\0.1.0\windows-x86_64-<variant>`
- Model: `%LOCALAPPDATA%\Lyra\models\parakeet-tdt-0.6b-v3.q8_0.gguf`
- Partial downloads and staged extraction: `%LOCALAPPDATA%\Lyra\staging\`

The setup flow asks for two independent approvals. It shows the official source, exact download size, target path, variant, license/checksum status, and whether CUDA hardware is available. A download is never hidden inside first launch, dependency installation, or a build script. A runtime download resumes from its `.partial` file when the server supports HTTP ranges; a model download follows the same path. Downloads cannot exceed their recorded expected size, and oversized partial artifacts are removed. Every completed artifact is checked against its pinned SHA-256 before staged installation. Failed, cancelled, or interrupted setup can be retried without treating an unverified artifact as ready.

## Runtime selection

The first candidate is the official NeMo-Speech.cpp v0.1.0 Windows x64 CUDA archive. If a verified CUDA or Vulkan runtime fails during recognition, Lyra retries the request with an already-installed verified fallback (Vulkan first, then CPU). The user can separately approve the Vulkan archive and then the CPU archive; Lyra does not download a fallback, install a CUDA Toolkit, or modify PATH as a recovery action.

## First launch checklist

1. Confirm microphone enumeration and choose the Windows default or a specific device.
2. Detect the GPU and display the measured VRAM.
3. Approve and verify the native runtime.
4. Approve and verify the Parakeet model. Separately approve the Nemotron model only if you want to try the experimental English streaming engine.
5. Use Record shortcut to capture a modifier plus one main key and confirm the registration status.
6. Record a short sample and verify the target application paste.

The default bar placement is Follow active window. Lyra places the bar at startup and when each session starts; it does not continuously move while browsing. Primary display and one fixed option per detected display are also available. Fixed monitors are resolved by identity first and ordinal second. If a saved monitor is unavailable, Lyra uses the primary display and shows a non-blocking status in General Settings.

The model resource policy is independent from artifact installation: Always warm, unload after 5/15/30 minutes, or Never keep warm. General Settings shows Loading, Ready, Unloaded, or an error detail.

If any injection step fails, Lyra keeps the transcript available in the app for copying and says `Copied — paste manually.` Before restoring earlier clipboard contents, Lyra checks that the clipboard still matches its last write so it does not overwrite newer contents placed there by another app. Lyra never presses Enter.

## Troubleshooting

Lyra registers the hotkey and its event handler together during startup. If the hotkey still shows `Pending`, another application owns the global registration; fully restart that application, then use `Retry` in Lyra's readiness checklist. Lyra keeps the tray and dictation bar available while the shortcut is unavailable.

The native runtime is loaded with its app-local `bin` directory as the dependency search location. No machine-wide CUDA Toolkit or PATH entry is required. If a local attempt reports `no audio was captured`, the runtime and model have already loaded and the recording itself was empty. If recognition still fails after an installed-runtime retry, open Diagnostics and use the native status detail; it includes the sample count and input rate without recording transcript or audio content. Copied diagnostic reports use an allowlist and omit local paths, service endpoints, executable paths, and free-form error details.

## Cloud credential

The optional OpenAI credential is entered through the settings flow and stored in Windows Credential Manager. It is not written to the JSON settings file, logs, frontend bundle, or Git.

## Usage statistics

The Usage sidebar stores daily aggregate Insights indefinitely and metadata-only session records from the latest year in `%LOCALAPPDATA%\Lyra\usage.json`. Older session records expire by age; lifetime totals and daily aggregates remain. These records include counts and timing, engine/runtime/fallback information, paste outcome, failure category, and a target executable basename/category. They do not include transcript text, raw audio, clipboard content, window titles, full paths, or credentials. Cost estimates are hidden until enabled and use the editable local rate, defaulting to `$0.003/minute`. Reset requires confirmation and restores empty counters.

Transcript History and Scratchpad text use separate stores under `%LOCALAPPDATA%\Lyra\saved-items\history` and `%LOCALAPPDATA%\Lyra\saved-items\scratchpad`. The transcript text is protected with Windows DPAPI for the current Windows user; saved-item metadata remains local alongside the encrypted text. History is Off by default and offers 7 days, 30 days, 90 days, 1 year, or Forever. Turning History Off stops new History saves; existing entries continue under the last selected retention until they expire. Clear transcript history affects neither Scratchpad nor Insights. Scratchpad entries are created only when deliberately recorded or promoted from History, and remain until deleted. Neither store contains audio. Encrypted text cannot be opened by another Windows account or moved to another machine without a future export feature.

Use Scratchpad when you want to speak a thought without sending it to the current app. Its shortcut and the visible Record control use the same activation mode and speech pipeline as dictation, but a Scratchpad capture does not automatically paste or change the clipboard. Copy and Paste into current app are explicit actions on a saved entry.

## Updates

Lyra checks the public signed feed at startup, once per hour while running, and when `Check for updates` is pressed in General Settings. It accepts stable versions only and silently treats offline checks as an unavailable feed. The install reservation rejects listening/processing sessions and blocks new sessions as soon as installation begins. Windows launches the passive signed NSIS updater and restarts Lyra after installation.

For local updater testing, run `npm run test:updater-fixture`. The harness generates an ephemeral Tauri signing key, serves a local signed manifest/artifact, exercises offline and corrupt-signature cases, and verifies a failed staged install cannot replace the existing application. It uses [src-tauri/tauri.fixture.conf.json](../src-tauri/tauri.fixture.conf.json); production configuration is not changed.

The release workflow publishes only the NSIS installer, Tauri updater artifact/signature, `latest.json`, and sanitized release notes to [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases). Before tagging a production release, configure the Tauri updater public key in `src-tauri/tauri.conf.json`, the GitHub App credentials below, and the following repository secrets/variables:

- `TAURI_SIGNING_PRIVATE_KEY`
- `LYRA_RELEASES_APP_CLIENT_ID` (repository variable)
- `LYRA_RELEASES_APP_PRIVATE_KEY` (repository secret)

`WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` are optional until Authenticode signing is available. When omitted, Lyra publishes an unsigned Windows installer but retains Tauri updater signatures. The v0.2.0 updater key is unencrypted, so no signing-password secret is required; the private key remains protected by GitHub Secret storage. The workflow creates the short-lived GitHub App installation token automatically; no personal access token is stored or manually renewed.
