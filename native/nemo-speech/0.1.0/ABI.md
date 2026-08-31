# NeMo-Speech.cpp ABI pin

Lyra loads the official NeMo-Speech.cpp v0.1.0 Windows runtime dynamically. The Rust binding in `src-tauri/src/native_nemo.rs` mirrors the installed v1 stable C ABI rather than linking an import library at build time.

- ABI header: <https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/main/include/nemo_speech/asr.h>
- Runtime release: <https://github.com/NVIDIA/NeMo-Speech.cpp/releases/tag/v0.1.0>
- Required offline symbols: `nemo_speech_asr_create`, `nemo_speech_asr_destroy`, `nemo_speech_asr_recognition_options_default`, `nemo_speech_asr_recognize_f32`, result transcript/destroy accessors, `nemo_speech_asr_last_error`, and `nemo_speech_asr_version`.
- Ownership: recognizers and result handles are explicitly destroyed; the loaded DLL remains alive for the recognizer lifetime.
- Input: mono F32 samples, with the capture layer resampling to 16 kHz.
- Streaming: the official stream symbols remain intentionally documented as the next backend implementation; v1 uses `recognize_f32` for utterance-at-a-time recognition.

The runtime manager only considers a DLL loadable after the app-local archive has been verified against the adjacent official SHA-256 file and a local `runtime.json` marker has been written. No SDK, CUDA Toolkit, compiler, or PATH modification is required.
