# Lyra releases

Lyra keeps source private in [`OrionAISystems/Lyra`](https://github.com/OrionAISystems/Lyra) and publishes release-only files to the public [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients therefore need no GitHub authentication.

## Release contract

1. Update all version files together and run `npm run verify:version`.
2. Confirm the committed updater public key in `src-tauri/tauri.conf.json`, then run `npm run verify:release-config`.
3. Push a protected `vX.Y.Z` tag to the private source repository.
4. GitHub Actions runs frontend checks, Rust checks/tests, an unbundled Tauri validation build, then the NSIS build. Authenticode signing is applied when the optional Windows certificate is configured.
5. The pinned Tauri Action publishes the installer, updater artifact and signature, `latest.json`, and the release notes to `Lyra-Releases`.
6. Inspect the public release assets and manifest before treating the release as production-ready.

The public repository must remain release-only. Do not add source code, models, native runtime archives, credentials, CI logs, or private organization documentation there.

## Required protected inputs

The release workflow expects:

- `TAURI_SIGNING_PRIVATE_KEY` for Tauri updater signatures. The v0.2.0 key is intentionally unencrypted; GitHub Secret storage protects the private key at rest and in workflow logs;
- `LYRA_RELEASES_APP_CLIENT_ID` as a repository variable and `LYRA_RELEASES_APP_PRIVATE_KEY` as a repository secret for the GitHub App installed only on `OrionAISystems/Lyra-Releases`;
- optionally, `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` for Authenticode signing.

The updater public key is intentionally committed in `src-tauri/tauri.conf.json`; the updater private key, Authenticode material, passwords, and GitHub App private key never are. The workflow validates the updater key immediately after dependency installation so malformed or unusable signing credentials fail quickly. It creates a short-lived GitHub App installation token for each release and derives the imported Authenticode certificate thumbprint on the runner when a certificate is available. If the Windows certificate secrets are omitted, the installer is intentionally unsigned by Authenticode while the updater artifacts remain Tauri-signed.

## Update manifest

Tauri's generated manifest is served from:

`https://github.com/OrionAISystems/Lyra-Releases/releases/latest/download/latest.json`

The client accepts only stable semantic versions newer than its current version. Notes are sanitized and rendered as text. An unavailable feed does not interrupt dictation or show an operating-system toast.
