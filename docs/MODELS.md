# Runtime and model manifest

## Native runtime

- Project: NVIDIA NeMo-Speech.cpp
- Version: `v0.1.0`
- Candidate: `nemo-speech-0.1.0-windows-x86_64-cuda.zip`
- Approximate archive size: 106 MB (106,044,768 bytes at the current release)
- Alternatives: Windows x64 Vulkan and CPU archives, separately approved
- Source: <https://github.com/NVIDIA/NeMo-Speech.cpp/releases/tag/v0.1.0>
- ABI reference: <https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/main/include/nemo_speech/asr.h>

Lyra verifies the adjacent official SHA-256 file before extracting an archive. Only an app-local, verified runtime is loadable.

## Local model

- Model: `nvidia/parakeet-tdt-0.6b-v3.q8_0.gguf`
- Current pinned revision: `541d1f99c6b0c3cd0b11a95167540bb8edefd82b`
- File size: 713,975,456 bytes (approximately 714 MB)
- Model license: CC BY 4.0
- Source: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>

The model supports English and Greek and performs automatic language detection. The selected NeMo-Speech.cpp customization matrix does not expose forced-language behavior or language metadata for this model, so Lyra presents English/Greek/Auto as preferences and labels local recognition truthfully as auto-detecting.

## Storage and secrets

Neither artifact is checked into Git. Download archives, extracted DLLs, model files, temporary WAV files, logs, credentials, and local settings are excluded by `.gitignore` or stored through OS-backed secret storage.
