# Seasonal themes

Lyra keeps seasonal artwork and colors behind a small, versioned theme contract. The October pack is bundled with the app so Appearance works offline. Later artwork packs can arrive independently through a signed public catalog; publishing a theme never updates the app installer or updater manifest.

## Runtime feeds and trust

The app updater continues to read the stable Tauri feed:

`https://github.com/OrionAISystems/Lyra-Releases/releases/latest/download/latest.json`

The seasonal engine reads a different raw-file endpoint:

`https://raw.githubusercontent.com/OrionAISystems/Lyra-Releases/main/themes/catalog.json`

The file is a JSON envelope containing base64 for the exact catalog payload bytes and base64 for the Tauri signer `.sig` file:

```json
{
  "payload": "<base64 of exact UTF-8 catalog JSON bytes>",
  "signature": "<base64 of exact Tauri .sig file bytes>"
}
```

The decoded payload has this top-level contract:

```json
{
  "discriminator": "lyra-seasonal-themes",
  "schemaVersion": 1,
  "engineVersion": 1,
  "revision": 1,
  "themes": []
}
```

`revision` is a positive, monotonically increasing catalog revision. Each theme has its own positive revision. The runtime verifies the envelope using the committed updater public key before parsing or using its payload. A malformed signature, unsupported contract version, stale catalog, disallowed URL, invalid palette, oversized image, or hash mismatch leaves the currently usable theme intact.

Theme artwork is served only from versioned prerelease assets named `themes-<id>-v<revision>`. The catalog uses exact GitHub release download URLs and includes the SHA-256 and byte length for each PNG. Release assets are uploaded and checked before the signed envelope is committed to the public repository. The app updater workflow listens only for stable `vX.Y.Z` tags; seasonal tags use the `themes-...` prefix, are prereleases, and set `make_latest=false`.

## Theme descriptors and rendering slots

Source descriptors live at `themes/<id>/theme.json`. Their palette tokens are exactly `bg`, `panel`, `panel2`, `panel3`, `line`, `text`, `muted`, `chrome`, `sidebar`, `surface`, `control`, `accent`, `accent2`, and `glow`. Colors use six-digit `#RRGGBB` hex. `text`, `muted`, `accent`, and `accent2` must each reach at least 4.5:1 contrast against every surface token (`bg`, `panel`, `panel2`, `panel3`, `chrome`, `sidebar`, `surface`, and `control`) in both dark and light palettes.

The renderer has eight named PNG slots: `idle`, `listening`, `processing`, `success`, `error`, `bar`, `empty`, and `ambience`. A bundled descriptor points to paths relative to its theme folder, such as `assets/pumpkin.png`; the engine loads and packages those files as data URLs. Remote catalog entries have the same metadata and palettes, but each artwork slot is an object with `url`, lowercase SHA-256 `sha256`, and `sizeBytes` fields. Multiple slots may share one image.

The accepted annual date interval is start-inclusive and end-exclusive, using `MM-DD` values. For recurring boundaries on February 29, Lyra uses March 1 in non-leap years. Lyra previews seasonal art throughout the year; automatic decoration follows the local calendar and saved seasonality preference. Reduced motion is a separate saved preference and also respects the operating-system motion preference.

The publisher and runtime enforce these limits:

- The decoded catalog payload is at most 64 KiB.
- A theme has eight named artwork slots and references no more than eight unique PNG assets.
- Each PNG is at most 2 MiB, dimensions are at most 1024 × 1024, and the decoded pixel buffer is at most 4 MiB. Unique assets total at most 8 MiB per theme pack. The engine keeps at most two remote packs in its bounded cache.
- The publisher accepts legal PNG color-type/bit-depth combinations with standard compression and filters and non-interlaced rows. PNG chunk CRCs and image data are validated before signing or caching. Remote bytes must match the signed length and SHA-256 metadata before use.
- Animated PNG chunks (`acTL`, `fcTL`, and `fdAT`) are rejected; app-level motion preferences control decorative movement.

The cache pointer is recoverable state, not the source of the rollback floor. If it is damaged, Lyra scans retained snapshots and restores the highest catalog revision whose signature and payload still verify, even when a cached PNG in that snapshot is corrupt. A same-revision refresh can then repair the damaged artwork. If no authenticated catalog revision survives, Lyra quarantines the bad pointer, starts fresh from a newly fetched signed catalog, and logs `cache-reset-no-authenticated-revision`. In that last case there is no authenticated local revision to compare against, so the prior rollback floor cannot be recovered.

## Authoring and publishing

Add a new descriptor and its PNGs under `themes/<id>/`. Increment the descriptor's theme `revision` whenever that pack changes; published version tags are immutable. Keep asset filenames unique within the pack. The global `catalog_revision` input must be greater than the revision in the current public catalog. The publisher preserves other theme entries from the previously signed catalog, replacing only the theme being published.

After the workflow is present on `main`, dispatch **Publish Lyra seasonal theme** from `main` with the theme ID and the next catalog revision. For the first October pack, the command is:

```powershell
gh workflow run publish-themes.yml --ref main -f theme_id=october -f catalog_revision=1
```

Use a catalog revision greater than the one currently published for later updates. The workflow validates all artwork and palette constraints, signs the exact generated payload with `TAURI_SIGNING_PRIVATE_KEY`, creates a short-lived GitHub App token from `LYRA_RELEASES_APP_CLIENT_ID` and `LYRA_RELEASES_APP_PRIVATE_KEY`, and publishes only PNG release assets plus the signed `themes/catalog.json` envelope. The token is limited to contents write access on `Lyra-Releases`. The workflow never copies source descriptors, source code, signing keys, or CI files to the public repository.

If a run stops after creating the theme prerelease, a retry can fill in missing assets with their original names and hashes. Existing assets with unexpected names, lengths, or hashes fail closed. The catalog commit happens only after all release assets are downloaded again and verified.

Local gates:

```powershell
npm run verify:themes
npm run test:themes-fixture
```

`verify:themes` checks that the app and catalog endpoints remain distinct and that the two release paths remain separate. `test:themes-fixture` validates the authored descriptor and PNGs, creates a temporary Tauri signing key, signs and wraps a fixture catalog, asks the Rust production verifier to accept it, confirms that bad signatures and image bytes are rejected, and exercises offline cache recovery and same-revision image repair. It does not contact or write to `Lyra-Releases`.

## Publication acceptance

Record a theme catalog publication in [ACCEPTANCE-V0.5.5.md](ACCEPTANCE-V0.5.5.md). After publishing the theme catalog but before tagging v0.5.5, verify its public signature and PNGs while confirming the stable app release remains at v0.5.4:

```powershell
npm run verify:public-themes -- --latest-version 0.5.4
```

Then run the opt-in backend smoke against the real public endpoint and pinned key:

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml live_published_theme_channel -- --ignored --nocapture
```

It downloads the active or upcoming published pack, verifies its signed metadata and PNGs through the production path, stages a unique temporary cache, then reloads it offline. It performs public read-only requests and does not update the repository or release. It is backend delivery evidence, not an installed-app or hardware test.

Once both the app release and theme catalog are public, run the full credential-free verifier:

```powershell
npm run verify:public-release -- --version 0.5.5
```

It checks GitHub's stable latest tag, the release's `latest.json`, the NSIS installer and its published Tauri signature, the raw signed theme catalog, each theme's prerelease metadata, and every referenced PNG's release metadata, size, SHA-256, and static PNG structure. The app's `latest.json` must remain attached to the current stable Lyra release after the theme prerelease is published.
