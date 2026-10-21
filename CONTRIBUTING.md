# Contributing to Lyra

Lyra is a Windows-first local-first dictation application. Start with the
[README](README.md), [architecture](ARCHITECTURE.md), and [repository guidance](AGENTS.md).
Discuss substantial changes in an issue before implementation. Small fixes can
arrive directly as a pull request. Include the problem, scope, and validation
actually performed; distinguish synthetic replay from microphone, focus/paste,
and installed-updater verification.

## Development

Use Windows x64, Node.js 22, the pinned Rust toolchain, the MSVC C++ build tools
and Windows SDK, and WebView2. Run `npm ci`; see [SETUP](docs/SETUP.md) for native
runtime/model installation. Model downloads are separate and explicit. Tests
that do not require hardware should run without a microphone or installed model.

Use the verification commands in the README and the nearest AGENTS.md. After
dependency changes, regenerate and check the third-party notices. Keep source
and dependency licenses/notices in downstream distributions. Follow the
[commit](docs/engineering/commits.md) and [PR](docs/engineering/pull-requests.md)
conventions; maintainers control official signing and release publication.

## Privacy and contribution terms

Submit only code and assets you have the right to contribute. Contributions are
provided under the project MIT license unless a file explicitly retains another
upstream license. Identify copied/adapted material, its source/version, changes,
and applicable notices in the pull request. Do not remove dependency copyright
or license headers.

Never commit personal dictation, recordings, transcripts, credentials, clipboard
contents, local app data, or private reports. Use invented synthetic examples;
store local benchmark audio/reports in ignored `audio/` or `local/` folders.
Sanitize screenshots and diagnostic exports before sharing them. Do not post
security vulnerabilities publicly; use [SECURITY.md](SECURITY.md).
