# Runtime and model manifest

These external artifacts retain their upstream licenses; they are not relicensed
under Lyra's MIT license. See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)
for attribution, source references, and downstream redistribution boundaries.

## Native runtime

- Project: NVIDIA NeMo-Speech.cpp
- Version: `v0.1.0`
- Candidate: `nemo-speech-0.1.0-windows-x86_64-cuda.zip`
- Approximate archive size: 106 MB (106,044,768 bytes at the current release)
- Alternatives: Windows x64 Vulkan and CPU archives, separately approved
- Source: <https://github.com/NVIDIA/NeMo-Speech.cpp/releases/tag/v0.1.0>
- ABI reference: <https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/main/include/nemo_speech/asr.h>
- Pinned archive SHA-256: `ba024204e76ca2fa4eefa8787506c3c49e418147f627f60cf9206a582b60089c`

Lyra also exposes the official Windows x64 Vulkan fallback (21,967,184 bytes, SHA-256 `b5e7b04a637da4eb25a60253e2db65774998e8dfb48c08b4db763009b82ac7ac`) and CPU fallback (4,730,421 bytes, SHA-256 `5e4ea81046012edcd77fd8848de8eefb5a4ba38cc26f52eb544ab184695a75d6`). Each variant is separately approved, downloaded, verified, and stored under its own versioned runtime directory.

Lyra verifies downloaded archive bytes against the compile-time pinned SHA-256 above before extracting them. The adjacent official checksum file is listed as a source reference; the runtime does not fetch it. Only an app-local, verified runtime is loadable.

## Local model

- Model: `nvidia/parakeet-tdt-0.6b-v3.q8_0.gguf`
- Current pinned revision: `541d1f99c6b0c3cd0b11a95167540bb8edefd82b`
- File size: 713,975,456 bytes (approximately 714 MB)
- Model license: CC BY 4.0
- Source: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>
- Pinned file SHA-256/ETag: `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`

The model supports English and Greek and performs automatic language detection. The selected NeMo-Speech.cpp customization matrix does not expose forced-language behavior or language metadata for this model, so Lyra presents English/Greek/Auto as preferences and labels local recognition truthfully as auto-detecting.

## Storage and secrets

Neither artifact is checked into Git. Download archives, extracted DLLs, model files, temporary WAV files, logs, credentials, and local settings are excluded by `.gitignore` or stored through OS-backed secret storage.

## Optional streaming model

- Model: `nemotron-speech-streaming-en-0.6b.q8_0.gguf`
- Pinned revision: `ebe59e5a817142986528bbbee5dba8db7b38ed50`
- File size: 699,872,960 bytes
- SHA-256: `d9a01898d2a611c8764e23a1c2f45e70bbd5a425dc4de93692ac951dd603812d`
- License: NVIDIA Open Model License
- Language: English

Nemotron is installed and marked independently from Parakeet. Its streaming ABI is loaded only when the complete optional symbol set is available. Auto/Greek requests stay on Parakeet; a stream failure replays the same in-memory PCM through Parakeet before any explicitly permitted cloud fallback.

Lyra passes enabled Words through the native speech-context interface for Nemotron. The pinned GGUF currently has no embedded SentencePiece tokenizer, however, so the runtime cannot tokenize those phrases and disables word boosting. Words are persisted and passed to the backend, but this model/runtime combination does not currently apply the recognition-bias benefit.
