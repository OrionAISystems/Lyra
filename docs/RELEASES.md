# Lyra releases

Source licensing and redistribution are described in [OPEN-SOURCE.md](OPEN-SOURCE.md).
Installer bundles include `LICENSE`, `THIRD_PARTY_NOTICES.md`, and `licenses/`.
After dependency updates, run `npm run notices:generate` and review the inventory;
CI and releases verify it with `npm run notices:check` before packaging.

Lyra maintains its MIT-licensed source in [`OrionAISystems/Lyra`](https://github.com/OrionAISystems/Lyra) and publishes release-only files plus the public [`CHANGELOG.md`](https://github.com/OrionAISystems/Lyra-Releases/blob/main/CHANGELOG.md) to the [`OrionAISystems/Lyra-Releases`](https://github.com/OrionAISystems/Lyra-Releases) repository. Installed clients therefore need no GitHub authentication. Release [v0.5.13](https://github.com/OrionAISystems/Lyra-Releases/releases/tag/v0.5.13) (2026-10-18 local time) improves History listing responsiveness and runtime download failure handling. See [ACCEPTANCE-V0.5.13.md](ACCEPTANCE-V0.5.13.md) for validation and publication status. Earlier versioned acceptance documents preserve historical updater and hardware evidence.

Signed seasonal themes use a separate publication path; their catalog and versioned artwork are described in [THEMES.md](THEMES.md), with acceptance evidence in [ACCEPTANCE-V0.5.5.md](ACCEPTANCE-V0.5.5.md).

## Release contract

1. Update all version files together, add the new top `CHANGELOG.md` section, and run `npm run verify:version` plus `npm run verify:changelog`.
2. Confirm the committed updater public key in `src-tauri/tauri.conf.json`, then run `npm run verify:release-config`.
3. Pull requests and pushes to `main` run the frontend checks, release-mode Rust tests, and formatting checks. A trusted push to `main` also performs the unbundled Tauri release build and warms the default-branch Cargo/release-target cache used by tagged releases.
4. Push a protected `vX.Y.Z` tag to the source repository after the relevant `main` checks have passed.
5. The tag workflow restores the Cargo/release-target cache, repeats the release-mode Rust checks, and runs the single authoritative NSIS/updater build. Authenticode signing is applied when the optional Windows certificate is configured.
6. CI mirrors the canonical changelog to `Lyra-Releases` and uses the current version section as the public release notes.
7. The pinned Tauri Action publishes the installer, updater artifact and signature, and `latest.json` to `Lyra-Releases`.
8. Inspect the public changelog, release assets, and manifest before treating the release as production-ready.
9. Historical acceptance: from an installed v0.2.0 client, check, download, passively install, restart, and verify v0.3.0. Current installed-hardware and v0.3.2-to-v0.4.x updater scenarios are tracked in [ACCEPTANCE-V0.4.md](ACCEPTANCE-V0.4.md); record observed results there before marking them passed.

Seasonal art uses a separate `workflow_dispatch` path. It signs decoded catalog bytes with the existing Tauri updater key, publishes PNGs to a `themes-<id>-v<revision>` prerelease with `make_latest=false`, verifies each uploaded asset, then commits only `themes/catalog.json` to `Lyra-Releases/main`. This does not invoke or change the app updater feed. See [THEMES.md](THEMES.md) for input requirements and acceptance checks.

`OrionAISystems/Lyra-Releases` must remain release-only. Its root may contain the public README and changelog, alongside release assets and metadata. Do not add source code, models, native runtime archives, credentials, CI logs, or private organization documentation there.

## Workflow toolchain and build reuse

Both verification and tagged releases use the shared
`.github/actions/setup-rust` action and the exact version in
[`rust-toolchain.toml`](../rust-toolchain.toml), currently Rust 1.99.1 (the latest
stable release when adopted). Update that file to adopt future stable versions;
the workflows do not depend on whichever Rust happens to ship on the runner.
Local Cargo commands use the same file automatically.

The shared cache includes the runner image, toolchain, Cargo manifests/lockfile,
and source commit. Each new commit can save refreshed artifacts instead of
repeatedly restoring an immutable, stale cache. Compatible prior commits are
fallbacks, including across dependency/version bumps, but different runner
images or toolchains never share compiled artifacts. A tag on the verified main
commit reuses its exact cache when the runner image matches. Installer bundles
are excluded. Cache writes happen after successful jobs; pull-request caches
remain scoped by GitHub and cannot replace the default-branch cache.

CI keeps benchmark-enabled Rust tests and passes the same features to the theme
fixture via `LYRA_FIXTURE_FEATURES=regression-bench`. The fixture compiles the
library test executable once, then runs its four cases directly with separate
environments. Exact test names and one-test assertions prevent a renamed or
missing test from silently passing. Releases exercise default features and still
run the complete frontend, Rust, signed-theme, signing and publication gates.
Production optimization settings are unchanged.

The first job on a new toolchain or runner image must populate a cold cache.
Compare subsequent compatible runs using job/step durations, cache restore/save
times, compiler versions and image versions in Actions logs. Track Rust test
compilation, theme fixtures, the unbundled CI build and installer publication
separately; a successful cache download alone does not prove Cargo reused it.
The previous v0.5.10 reference was 13m30s for CI and 11m16s for release; this
change needs hosted-run measurements before claiming a restored timing target.

Local verification on 2026-10-18 with Rust 1.99.1 passed formatting, 154 Rust
tests (four hardware/provider cases ignored), the default-feature release check,
both default and benchmark-feature signed-theme fixtures, 38 frontend tests,
the frontend build, release metadata checks and actionlint. A repeated warm
benchmark-feature fixture completed in 4.65 seconds overall; Cargo reported
1.02 seconds and no recompilation. This is local reuse evidence, not a hosted
before/after comparison or an installer publication measurement.

When the user explicitly requests skipping main CI for a hotfix, run all CI-equivalent gates locally first and record them in the versioned acceptance document. Put `[skip ci]` on a separate documentation-only main push tip, and tag the tested release source commit whose message contains no skip directive. A skip directive on the tagged commit can suppress the tag workflow too. The signed tag publication and public-artifact checks remain required.

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
