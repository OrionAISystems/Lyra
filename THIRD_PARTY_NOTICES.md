# Third-party notices

Lyra's own source, documentation, and project-owned artwork are provided under
the [MIT License](LICENSE), copyright 2026 Orion AI Systems. Dependencies and
external runtime/model artifacts retain their own licenses. Lyra's MIT license
does not relicense those works or grant rights to third-party trademarks.

## Included dependencies

The versioned [dependency inventory](licenses/dependencies.json) identifies each
production npm dependency and every crate in the locked Windows x64 Cargo
resolution, including build tools and crates that may not survive linking.
It records the upstream license expression, repository, exact-version source
archive, and paths to preserved license/copyright/NOTICE texts. Providing more
notices than the final executable needs is intentional.

The installer carries this file, `LICENSE`, and the `licenses/` directory beside
the installed application. Redistributors must preserve the applicable notices
and satisfy each dependency's terms, rather than treating the whole dependency
tree as MIT. Source distributions must retain these files too.

Some Rust dependencies use MPL-2.0 (including CSS parser/selector components).
They remain unmodified upstream dependencies. Their exact-version source archives
are linked in the inventory; preserve access to that source and its MPL notices
when distributing executables. If you modify an MPL-covered file, make the
modified covered source available under MPL-2.0 and document how recipients can
obtain it. This does not change Lyra's license for its own files.

Run `npm ci`, install/fetch the locked Rust dependencies, then run
`npm run notices:generate` after dependency changes. `npm run notices:check`
verifies the generated inventory/texts offline. Packages missing notice files
use the reviewed, revision-pinned entries in
[`licenses/upstream-sources.json`](licenses/upstream-sources.json). The selectors
crate's source headers reference the included standard MPL-2.0 text. Preserve
upstream copyright statements and notices; do not replace them with Lyra's.
These commands do not install models or capture user data.

## NVIDIA native interface and optional runtime

Lyra's `src-tauri/src/native_nemo.rs` adapts the NeMo-Speech.cpp v0.1.0 C ABI into
Rust declarations and adds Lyra-owned loading, error handling, and lifecycle
code. The upstream interface is from NVIDIA NeMo-Speech.cpp, copyright
2026 NVIDIA CORPORATION & AFFILIATES, licensed under Apache-2.0. Its license is
preserved in [`licenses/upstream/NeMo-Speech.cpp-0.1.0-LICENSE`](licenses/upstream/NeMo-Speech.cpp-0.1.0-LICENSE).
The upstream [NOTICE](licenses/upstream/NeMo-Speech.cpp-0.1.0-NOTICE) and
[runtime source-tree notices](licenses/upstream/NeMo-Speech.cpp-0.1.0-THIRD_PARTY_NOTICES.txt)
are retained verbatim for context. Relative paths inside those upstream texts
refer to the NVIDIA source tree, not to bundled Lyra files. The runtime's
third-party components include LGPL-covered KenLM code; do not assume that
an Apache-licensed top-level runtime makes all of its contents Apache or MIT.
Upstream: <https://github.com/NVIDIA/NeMo-Speech.cpp/tree/v0.1.0>.

Runtime archives are downloaded separately after approval; no NVIDIA runtime,
CUDA/Vulkan SDK, model weights, or Windows runtime binaries are included in this
source repository. Downloaded runtime archives may contain additional dependency
notices; retain those with any redistributed runtime. A runtime/model downloader
does not grant permission to relicense its downloads.

## Separately downloaded speech models

| Artifact | Attribution and terms |
| --- | --- |
| NVIDIA Parakeet TDT 0.6B v3, including the pinned GGUF quantization | NVIDIA; [pinned model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/blob/541d1f99c6b0c3cd0b11a95167540bb8edefd82b/README.md); [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Lyra downloads the upstream artifact unchanged; it does not claim to have trained or quantized it. |
| NVIDIA Nemotron Speech Streaming EN 0.6B, including the pinned GGUF quantization | NVIDIA; [pinned model card](https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b/blob/ebe59e5a817142986528bbbee5dba8db7b38ed50/README.md); [NVIDIA Open Model License](https://www.nvidia.com/en-us/agreements/enterprise-software/nvidia-open-model-license/). Lyra downloads the upstream artifact unchanged. |

See [the runtime/model manifest](docs/MODELS.md) for filenames, pinned revisions,
checksums, and storage. Downstream model redistribution or modification needs
its own compliance review; these artifacts are not covered by Lyra's MIT license.

## Artwork and system components

October theme artwork was generated for Lyra; its provenance and prompts are in
[`themes/october/ARTWORK.md`](themes/october/ARTWORK.md). Project-owned artwork is
included under the project MIT license. Lucide icons retain their upstream ISC
notice in the dependency inventory. Windows APIs, installed speech voices, and
Microsoft Edge WebView2 remain Microsoft components under their respective
terms; Lyra does not include or relicense the operating system or voice assets.
