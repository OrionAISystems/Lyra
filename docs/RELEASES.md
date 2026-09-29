# Lyra releases

Lyra keeps source private in [`OrionAISystems/Lyra`](https://github.com/OrionAISystems/Lyra) and publishes release-only files plus the public [`CHANGELOG.md`](https://github.com/OrionAISystems/Lyra-Releases/blob/main/CHANGELOG.md) to the [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients therefore need no GitHub authentication. v0.3.0 was the first release whose installed v0.2.0 client exercised this path end to end; v0.3.1 kept long settings navigation anchored while the main content scrolls; v0.3.2 fixed local recognition reliability. The current published release is [v0.5.3](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.3) (2026-09-28 local time), which groups Insights by the computer's local calendar day and migrates retained UTC usage metadata. See [ACCEPTANCE-V0.5.3.md](ACCEPTANCE-V0.5.3.md) for release evidence and installed-app checks. The previous [v0.5.2 release](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.2) fixed the dictation bar context menu placement and paste after releasing the push-to-talk Space key; its evidence is in [ACCEPTANCE-V0.5.2.md](ACCEPTANCE-V0.5.2.md).

## Release contract

1. Update all version files together, add the new top `CHANGELOG.md` section, and run `npm run verify:version` plus `npm run verify:changelog`.
2. Confirm the committed updater public key in `src-tauri/tauri.conf.json`, then run `npm run verify:release-config`.
3. Pull requests and pushes to `main` run the frontend checks, release-mode Rust tests, and formatting checks. A trusted push to `main` also performs the unbundled Tauri release build and warms the default-branch Cargo/release-target cache used by tagged releases.
4. Push a protected `vX.Y.Z` tag to the private source repository after the relevant `main` checks have passed.
5. The tag workflow restores the Cargo/release-target cache, repeats the release-mode Rust checks, and runs the single authoritative NSIS/updater build. Authenticode signing is applied when the optional Windows certificate is configured.
6. CI mirrors the canonical changelog to `Lyra-Releases` and uses the current version section as the public release notes.
7. The pinned Tauri Action publishes the installer, updater artifact and signature, and `latest.json` to `Lyra-Releases`.
8. Inspect the public changelog, release assets, and manifest before treating the release as production-ready.
9. Historical acceptance: from an installed v0.2.0 client, check, download, passively install, restart, and verify v0.3.0. Current installed-hardware and v0.3.2-to-v0.4.x updater scenarios are tracked in [ACCEPTANCE-V0.4.md](ACCEPTANCE-V0.4.md); record observed results there before marking them passed.

The public repository must remain release-only. Its root may contain the public README and changelog, alongside release assets and metadata. Do not add source code, models, native runtime archives, credentials, CI logs, or private organization documentation there.

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

## Local fixture

`npm run test:updater-fixture` does not contact or mutate `OrionAISystems/Lyra-Releases`. It creates an ephemeral signing key with the Tauri signer, serves a dynamic local endpoint from a temporary HTTP server, verifies the minisign artifact and trusted-comment signatures, corrupts the signature, stops the endpoint for an offline check, and simulates a staged install failure while asserting the existing application bytes are unchanged. `src-tauri/tauri.fixture.conf.json` is a template only; the production endpoint, production public key, passive install mode, and release-only repository remain unchanged.

The manual release evidence, including the real installed v0.2.0 → public v0.3.0 upgrade, is maintained in [ACCEPTANCE-V0.3.md](ACCEPTANCE-V0.3.md).
