import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import {
  MAX_ASSET_BYTES,
  THEME_CATALOG_ENDPOINT,
  decodeTauriSignatureFile,
  decodeEnvelope,
  readPublicKey,
  validatePng,
  verifyMinisign,
} from "./themes-release.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const REPOSITORY = "OrionAISystems/Lyra-Releases";
const MAX_ENVELOPE_BYTES = 128 * 1024;
const MAX_LATEST_BYTES = 512 * 1024;
const MAX_CHANGELOG_BYTES = 1024 * 1024;
const MAX_RELEASE_METADATA_BYTES = 4 * 1024 * 1024;
const MAX_INSTALLER_BYTES = 256 * 1024 * 1024;
const MAX_CATALOG_ARTWORK_BYTES = 64 * 8 * 1024 * 1024;
const SAFE_REDIRECT_HOSTS = new Set([
  "api.github.com",
  "github.com",
  "raw.githubusercontent.com",
  "release-assets.githubusercontent.com",
  "objects.githubusercontent.com",
]);

function fail(message) {
  throw new Error(message);
}

function parseArgs(args) {
  if (args.length === 2 && args[0] === "--version" && /^\d+\.\d+\.\d+$/.test(args[1])) {
    return { mode: "full", version: args[1] };
  }
  if (args.length === 3 && args[0] === "--themes-only" && args[1] === "--latest-version" && /^\d+\.\d+\.\d+$/.test(args[2])) {
    return { mode: "themes", version: args[2] };
  }
  fail("Usage: npm run verify:public-release -- --version <X.Y.Z>\n   npm run verify:public-themes -- --latest-version <stable X.Y.Z>");
}

async function readBounded(response, maxBytes, label) {
  const announcedLength = Number(response.headers.get("content-length"));
  if (Number.isFinite(announcedLength) && announcedLength > maxBytes) fail(`${label} exceeds the ${maxBytes}-byte download limit`);
  if (!response.body) fail(`${label} response has no body`);
  const reader = response.body.getReader();
  const chunks = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > maxBytes) {
      await reader.cancel();
      fail(`${label} exceeds the ${maxBytes}-byte download limit`);
    }
    chunks.push(Buffer.from(value));
  }
  return Buffer.concat(chunks, total);
}

async function fetchBounded(urlValue, maxBytes, label, { acceptedHosts = SAFE_REDIRECT_HOSTS, accept = "application/octet-stream" } = {}) {
  let url = new URL(urlValue);
  for (let redirectCount = 0; redirectCount <= 5; redirectCount += 1) {
    if (url.protocol !== "https:" || !acceptedHosts.has(url.hostname) || url.username || url.password) {
      fail(`${label} uses a disallowed download URL`);
    }
    const response = await fetch(url, {
      headers: { accept, "user-agent": "Lyra-public-release-verifier" },
      redirect: "manual",
      signal: AbortSignal.timeout(60_000),
    });
    if ([301, 302, 303, 307, 308].includes(response.status)) {
      const location = response.headers.get("location");
      if (!location || redirectCount === 5) fail(`${label} exceeded the redirect limit`);
      url = new URL(location, url);
      continue;
    }
    if (!response.ok) fail(`${label} returned HTTP ${response.status}`);
    return await readBounded(response, maxBytes, label);
  }
  fail(`${label} exceeded the redirect limit`);
}

function parseJson(bytes, label) {
  try {
    return JSON.parse(bytes.toString("utf8"));
  } catch (error) {
    fail(`${label} is not valid JSON: ${error.message}`);
  }
}

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function compareReleaseAsset(bytes, asset, label) {
  if (!asset || typeof asset.name !== "string") fail(`${label} is missing from the stable release`);
  if (bytes.length !== asset.size) fail(`${label} byte length differs from the release metadata`);
  if (asset.digest && asset.digest !== `sha256:${digest(bytes)}`) fail(`${label} SHA-256 differs from the release metadata`);
}

function getAppAssetUrl(manifest, version) {
  const platform = manifest.platforms?.["windows-x86_64-nsis"];
  if (!platform || typeof platform.url !== "string" || typeof platform.signature !== "string") {
    fail("latest.json does not contain the Windows NSIS installer and signature");
  }
  const url = new URL(platform.url);
  const expectedPrefix = `/OrionAISystems/Lyra-Releases/releases/download/v${version}/`;
  if (url.protocol !== "https:" || url.hostname !== "github.com" || url.port || url.username || url.password || url.pathname.slice(0, expectedPrefix.length) !== expectedPrefix) {
    fail(`latest.json installer URL must point to the stable v${version} release`);
  }
  const installerName = decodeURIComponent(url.pathname.slice(expectedPrefix.length));
  if (!/^Lyra_[0-9]+\.[0-9]+\.[0-9]+_x64-setup\.exe$/.test(installerName) || url.search || url.hash || url.href !== `https://github.com/${REPOSITORY}/releases/download/v${version}/${encodeURIComponent(installerName)}`) {
    fail("latest.json installer filename is not a canonical Lyra NSIS asset");
  }
  const alternate = manifest.platforms?.["windows-x86_64"];
  if (alternate && (alternate.url !== platform.url || alternate.signature !== platform.signature)) {
    fail("latest.json Windows installer entries disagree about the URL or signature");
  }
  return { platform, installerName, installerUrl: url.href };
}

async function verifyPublicAppRelease(version, publicKeyText) {
  const stableUrl = `https://api.github.com/repos/${REPOSITORY}/releases/latest`;
  const stableBeforeBytes = await fetchBounded(stableUrl, MAX_RELEASE_METADATA_BYTES, "stable release metadata", { accept: "application/vnd.github+json" });
  const stableBefore = parseJson(stableBeforeBytes, "stable release metadata");
  const expectedTag = `v${version}`;
  if (stableBefore.tag_name !== expectedTag || stableBefore.draft || stableBefore.prerelease) {
    fail(`GitHub latest stable release is ${stableBefore.tag_name}; expected ${expectedTag}`);
  }

  const releaseBytes = await fetchBounded(`https://api.github.com/repos/${REPOSITORY}/releases/tags/${expectedTag}`, MAX_RELEASE_METADATA_BYTES, `${expectedTag} release metadata`, { accept: "application/vnd.github+json" });
  const release = parseJson(releaseBytes, `${expectedTag} release metadata`);
  if (release.tag_name !== expectedTag || release.draft || release.prerelease || !Array.isArray(release.assets)) {
    fail(`${expectedTag} must be a published stable release`);
  }
  const assets = new Map(release.assets.map((asset) => [asset.name, asset]));
  const latestAsset = assets.get("latest.json");
  const latestBytes = await fetchBounded("https://github.com/OrionAISystems/Lyra-Releases/releases/latest/download/latest.json", MAX_LATEST_BYTES, "stable latest.json");
  compareReleaseAsset(latestBytes, latestAsset, "latest.json");
  const manifest = parseJson(latestBytes, "latest.json");
  if (manifest.version !== version) fail(`latest.json version is ${manifest.version}; expected ${version}`);

  const { platform, installerName, installerUrl } = getAppAssetUrl(manifest, version);
  const installerAsset = assets.get(installerName);
  const signatureAsset = assets.get(`${installerName}.sig`);
  const installer = await fetchBounded(installerUrl, MAX_INSTALLER_BYTES, `${installerName} installer`);
  compareReleaseAsset(installer, installerAsset, `${installerName} installer`);
  const signatureFileBytes = Buffer.from(platform.signature, "utf8");
  if (signatureFileBytes.length === 0 || signatureFileBytes.length > 4096 || signatureFileBytes.toString("utf8") !== platform.signature) fail("latest.json installer signature is not a bounded Tauri .sig value");
  const signatureAssetBytes = await fetchBounded(`https://github.com/OrionAISystems/Lyra-Releases/releases/download/${expectedTag}/${encodeURIComponent(installerName)}.sig`, 4096, `${installerName}.sig`);
  compareReleaseAsset(signatureAssetBytes, signatureAsset, `${installerName}.sig`);
  if (!signatureFileBytes.equals(signatureAssetBytes)) fail("latest.json signature does not match the published .sig asset");
  verifyMinisign(installer, decodeTauriSignatureFile(signatureAssetBytes, `${expectedTag} installer`), publicKeyText, `${expectedTag} installer`);

  const stableAfterBytes = await fetchBounded(stableUrl, MAX_RELEASE_METADATA_BYTES, "stable release metadata after asset verification", { accept: "application/vnd.github+json" });
  const stableAfter = parseJson(stableAfterBytes, "stable release metadata after asset verification");
  if (stableAfter.tag_name !== expectedTag || stableAfter.prerelease || stableAfter.draft) fail(`GitHub latest stable release changed during verification (now ${stableAfter.tag_name})`);
  return { expectedTag, installerName, installerBytes: installer.length, stableLatest: stableAfter.tag_name };
}

async function verifyPublicThemeCatalog(publicKeyText) {
  const envelopeBytes = await fetchBounded(THEME_CATALOG_ENDPOINT, MAX_ENVELOPE_BYTES, "public signed seasonal catalog", { acceptedHosts: new Set(["raw.githubusercontent.com"]), accept: "application/json" });
  const { catalog } = decodeEnvelope(envelopeBytes, publicKeyText, "public seasonal catalog");
  const verified = new Map();
  const releaseListBytes = await fetchBounded(`https://api.github.com/repos/${REPOSITORY}/releases?per_page=100`, MAX_RELEASE_METADATA_BYTES, "theme release metadata", { accept: "application/vnd.github+json" });
  const releaseList = parseJson(releaseListBytes, "theme release metadata");
  if (!Array.isArray(releaseList)) fail("GitHub theme release metadata is not a release list");
  const releases = new Map(releaseList.filter((release) => typeof release.tag_name === "string").map((release) => [release.tag_name, release]));
  let downloadedBytes = 0;
  for (const theme of catalog.themes) {
    const tag = `themes-${theme.id}-v${theme.revision}`;
    let release = releases.get(tag);
    if (!release) {
      const releaseBytes = await fetchBounded(`https://api.github.com/repos/${REPOSITORY}/releases/tags/${tag}`, MAX_RELEASE_METADATA_BYTES, `${tag} theme prerelease metadata`, { accept: "application/vnd.github+json" });
      release = parseJson(releaseBytes, `${tag} theme prerelease metadata`);
      releases.set(tag, release);
    }
    if (release.tag_name !== tag || release.draft || !release.prerelease || !Array.isArray(release.assets)) fail(`${tag} must be a published prerelease`);
    const releaseAssets = new Map(release.assets.map((asset) => [asset.name, asset]));
    const themeHashes = new Map();
    for (const [slot, asset] of Object.entries(theme.artwork)) {
      const previousSize = themeHashes.get(asset.sha256);
      if (previousSize !== undefined) {
        if (previousSize !== asset.sizeBytes) fail(`${theme.id}.${slot} repeats a PNG hash with conflicting size metadata`);
        continue;
      }
      themeHashes.set(asset.sha256, asset.sizeBytes);
      const key = `${asset.url}:${asset.sha256}:${asset.sizeBytes}`;
      if (verified.has(key)) continue;
      if (asset.sizeBytes > MAX_ASSET_BYTES) fail(`${theme.id}.${slot} exceeds the PNG download limit`);
      const fileName = decodeURIComponent(new URL(asset.url).pathname.split("/").at(-1));
      const releaseAsset = releaseAssets.get(fileName);
      if (!releaseAsset || releaseAsset.size !== asset.sizeBytes || (releaseAsset.digest && releaseAsset.digest !== `sha256:${asset.sha256}`)) {
        fail(`${theme.id}.${slot} release asset metadata does not match the signed catalog`);
      }
      downloadedBytes += asset.sizeBytes;
      if (downloadedBytes > MAX_CATALOG_ARTWORK_BYTES) fail(`Public catalog artwork exceeds the ${MAX_CATALOG_ARTWORK_BYTES}-byte verification budget`);
      const bytes = await fetchBounded(asset.url, MAX_ASSET_BYTES, `${theme.id}.${slot} theme PNG`);
      if (bytes.length !== asset.sizeBytes || digest(bytes) !== asset.sha256) fail(`${theme.id}.${slot} PNG does not match its signed size and SHA-256`);
      validatePng(bytes, `${theme.id}.${slot} published PNG`);
      verified.set(key, bytes.length);
    }
    const themeBytes = [...themeHashes.values()].reduce((sum, size) => sum + size, 0);
    if (themeBytes > 8 * 1024 * 1024) fail(`${theme.id} references ${themeBytes} bytes of unique artwork; the per-theme limit is 8 MiB`);
  }
  const total = [...verified.values()].reduce((sum, size) => sum + size, 0);
  return { revision: catalog.revision, themes: catalog.themes.length, images: verified.size, bytes: total };
}

async function verifyPublicChangelog(version) {
  const bytes = await fetchBounded(`https://raw.githubusercontent.com/${REPOSITORY}/main/CHANGELOG.md`, MAX_CHANGELOG_BYTES, "public changelog", { acceptedHosts: new Set(["raw.githubusercontent.com"]), accept: "text/plain" });
  const changelog = bytes.toString("utf8");
  if (!new RegExp(`^## \\[${version.replaceAll(".", "\\.")}\\](?:\\s|$)`, "m").test(changelog)) {
    fail(`Public CHANGELOG.md does not contain a v${version} section`);
  }
  return true;
}

async function readStableTag() {
  const bytes = await fetchBounded(`https://api.github.com/repos/${REPOSITORY}/releases/latest`, MAX_RELEASE_METADATA_BYTES, "stable release metadata", { accept: "application/vnd.github+json" });
  return parseJson(bytes, "stable release metadata").tag_name;
}

async function main() {
  const { mode, version } = parseArgs(process.argv.slice(2));
  const publicKeyText = await readPublicKey();
  if (mode === "themes") {
    const expectedTag = `v${version}`;
    const before = await readStableTag();
    if (before !== expectedTag) fail(`GitHub latest stable release is ${before}; expected ${expectedTag}`);
    const themes = await verifyPublicThemeCatalog(publicKeyText);
    const after = await readStableTag();
    if (after !== expectedTag) fail(`GitHub latest stable release changed during theme verification (now ${after})`);
    process.stdout.write(`Verified signed seasonal catalog revision ${themes.revision} (${themes.themes} themes, ${themes.images} unique PNGs, ${themes.bytes} bytes); stable latest remained ${after}.\n`);
    return;
  }
  const app = await verifyPublicAppRelease(version, publicKeyText);
  const themes = await verifyPublicThemeCatalog(publicKeyText);
  await verifyPublicChangelog(version);
  const finalStableTag = await readStableTag();
  if (finalStableTag !== app.expectedTag) fail(`Stable latest release changed during full verification (now ${finalStableTag})`);
  process.stdout.write([
    `Verified public changelog and stable app release ${app.expectedTag}; GitHub latest remained ${app.stableLatest}.`,
    `Verified ${app.installerName} (${app.installerBytes} bytes) and its published Tauri signature.`,
    `Verified signed seasonal catalog revision ${themes.revision} (${themes.themes} themes, ${themes.images} unique PNGs, ${themes.bytes} bytes).`,
  ].join("\n") + "\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Public release verification failed: ${error.message}`);
    process.exitCode = 1;
  });
}
