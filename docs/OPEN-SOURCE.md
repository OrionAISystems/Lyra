# Source publication and redistribution

Lyra's own code and documentation are licensed under MIT. The copyright and
permission notice must travel with copies or substantial portions. Dependency,
native-interface, model, runtime, and asset attribution is documented in
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). `package.json` intentionally
keeps `private: true`: this desktop application is not an npm package.

## Publication status and boundaries

This documentation describes source publication and downstream redistribution.
Repository visibility and publication history are recorded on GitHub.
`OrionAISystems/Lyra` is the source repository. `OrionAISystems/Lyra-Releases`
continues to host official installer/updater/theme assets. Preserve those URLs,
verification keys, protected release tags, and installed-client compatibility.
Forks should configure their own identity, feeds, and signing keys before
distributing updates under a different publisher.

The developer benchmark, reviewed invented prompts, and synthetic reports may
be published with the source. They remain excluded from normal application
builds and installer assets. Real recordings, private reports, transcripts,
models, native binaries, secrets, and local application data remain outside Git.

## Maintainer publication review

Before changing source visibility, review the complete branch/tag history with
a dedicated secret scanner and inspect hosted PR/issue discussions and workflow
logs for private data. The earlier source review found no confirmed secret leak;
that is not a substitute for an ongoing scanning process. Current personal
machine paths have been generalized. Author names and development email addresses
are deliberately retained. Earlier commits still contain the old machine-path
examples; this preparation does not rewrite history or signed release tags.

Keep private vulnerability reporting enabled when the public repository supports
it; the security policy also provides a maintainer email fallback. Preserve the
current distinction between read-only pull-request CI and trusted signing/release
workflows. Review notices whenever the lockfiles or bundled assets change.

## Reproducible notices

After installing the locked npm and Rust dependencies:

```powershell
npm run notices:generate
npm run notices:check
```

Generation is offline and reads installed production npm packages plus Cargo's
locked Windows x64 resolution, including build dependencies. It preserves full
upstream license/NOTICE texts and links exact-version source archives, including
MPL-covered sources. Packages that omit license files from their archive use
reviewed upstream copies recorded in `licenses/upstream-sources.json`. If a new
package has no license declaration or notice, generation fails rather than
silently omitting it. Review any obsolete notices before deleting them.

The Tauri bundle includes the MIT license, third-party overview, inventory, and
license texts as resources. CI and tagged release workflows check that the
notices match the lockfiles before building. Redistributors that alter dependency
features, target platforms, versions, native runtimes, or models must regenerate
and review their own notice/source obligations.

## Preparation verification (2026-10-21)

- MIT metadata is set in npm, Cargo, and Tauri. Generated notices cover 7
  production npm packages and 350 resolved Windows Cargo packages; the offline
  notice check passes. Local documentation links and all bundle resource paths
  resolve. Release configuration verification checks the required license and
  resource mappings.
- Frontend type checking, all 38 tests, and the production build passed. Rust
  formatting, the default-feature release check, and all 7 focused usage tests
  passed. Existing Rust dead-code warnings remain. Version/changelog validation
  passed at v0.5.13; no application release/version bump is part of this change.
- Gitleaks v8.30.1, downloaded with its published SHA-256 checksum verified,
  reported no leaks in the committed history (`--all`, 77 scanned non-merge
  commits; the repository has 80 total reachable commits). A second scan of
  the prepared source and fetched hosted text also reported no leaks: 12 issue/PR
  records, 19 issue comments, and 92 review comments. These are secret-scanner
  results, not a claim that every privacy or security risk has been excluded.
- Historical Actions logs and unavailable/deleted GitHub records were not
  exhaustively reviewed. The installer resource mapping was checked against the
  local Tauri schema and compiled configuration; a new installer was not built
  or installed. Production signing/publication and hardware dictation were not
  exercised for this documentation/attribution preparation.
- These checks were performed during source preparation before publication.
  Author metadata, signed tags, updater endpoints/keys, and official release
  assets were preserved. Source publication does not create a new app release.
