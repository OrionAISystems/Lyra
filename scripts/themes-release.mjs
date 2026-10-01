import { createHash, createPublicKey, verify } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { basename, dirname, extname, isAbsolute, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const THEME_CATALOG_ENDPOINT = "https://raw.githubusercontent.com/OrionAISystems/Lyra-Releases/main/themes/catalog.json";
const APP_UPDATE_ENDPOINT = "https://github.com/OrionAISystems/Lyra-Releases/releases/latest/download/latest.json";
const DISCRIMINATOR = "lyra-seasonal-themes";
const SCHEMA_VERSION = 1;
const ENGINE_VERSION = 1;
const MAX_CATALOG_BYTES = 64 * 1024;
const MAX_ASSET_BYTES = 2 * 1024 * 1024;
const MAX_DECODED_IMAGE_BYTES = 4 * 1024 * 1024;
const MAX_PACK_BYTES = 8 * 1024 * 1024;
const MAX_DIMENSION = 1024;
const MAX_THEMES = 64;
const THEME_SLOTS = ["idle", "listening", "processing", "success", "error", "bar", "empty", "ambience"];
const PALETTE_TOKENS = ["bg", "panel", "panel2", "panel3", "line", "text", "muted", "chrome", "sidebar", "surface", "control", "accent", "accent2", "glow"];
const SURFACE_TOKENS = ["bg", "panel", "panel2", "panel3", "chrome", "sidebar", "surface", "control"];
const SIGNED_PALETTE_TOKENS = ["text", "muted", "accent", "accent2"];

const crcTable = (() => {
  const table = new Uint32Array(256);
  for (let value = 0; value < table.length; value += 1) {
    let crc = value;
    for (let bit = 0; bit < 8; bit += 1) crc = (crc & 1) ? (0xedb88320 ^ (crc >>> 1)) : (crc >>> 1);
    table[value] = crc >>> 0;
  }
  return table;
})();

function fail(message) {
  throw new Error(message);
}

function sameKeys(value, expected, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(`${label} must contain exactly: ${expected.join(", ")}`);
  }
}

function stringValue(value, label, maximum = 200) {
  if (typeof value !== "string" || value.trim().length === 0 || [...value].length > maximum || /[\u0000-\u001f\u007f-\u009f]/u.test(value)) {
    fail(`${label} must be a non-empty string no longer than ${maximum} characters`);
  }
  return value;
}

function positiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value < 1) fail(`${label} must be a positive safe integer`);
  return value;
}

function validateAssetFileName(fileName, label = "theme asset filename") {
  if (typeof fileName !== "string" || fileName.length > 128 || !/^[A-Za-z0-9][A-Za-z0-9._-]*\.png$/.test(fileName) || fileName.includes("..")) {
    fail(`${label} must be a safe ASCII filename ending in lowercase .png`);
  }
  return fileName;
}

function parseArguments(args) {
  const result = {};
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (!token.startsWith("--")) fail(`Unexpected argument: ${token}`);
    const key = token.slice(2);
    if (!key || Object.hasOwn(result, key)) fail(`Invalid or repeated option: ${token}`);
    const value = args[index + 1];
    if (!value || value.startsWith("--")) fail(`Option ${token} requires a value`);
    result[key] = value;
    index += 1;
  }
  return result;
}

function parseJson(bytes, label) {
  try {
    return JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : bytes);
  } catch (error) {
    fail(`${label} is not valid JSON: ${error.message}`);
  }
}

function decodeBase64(value, label, maxBytes = Infinity) {
  if (typeof value !== "string" || value.length === 0 || value.length % 4 !== 0 || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) {
    fail(`${label} is not valid padded base64`);
  }
  const bytes = Buffer.from(value, "base64");
  if (bytes.length === 0 || bytes.length > maxBytes || bytes.toString("base64") !== value) fail(`${label} has an invalid decoded size or encoding`);
  return bytes;
}

function luminance(hex) {
  const channels = hex.slice(1).match(/../g).map((part) => Number.parseInt(part, 16) / 255);
  const linear = channels.map((channel) => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
}

function contrastRatio(first, second) {
  const left = luminance(first);
  const right = luminance(second);
  return (Math.max(left, right) + 0.05) / (Math.min(left, right) + 0.05);
}

function validatePalettes(palettes, label) {
  sameKeys(palettes, ["dark", "light"], `${label}.palettes`);
  for (const [mode, palette] of Object.entries(palettes)) {
    sameKeys(palette, PALETTE_TOKENS, `${label}.palettes.${mode}`);
    for (const token of PALETTE_TOKENS) {
      if (typeof palette[token] !== "string" || !/^#[0-9a-fA-F]{6}$/.test(palette[token])) {
        fail(`${label}.palettes.${mode}.${token} must be a six-digit hex color`);
      }
    }
    for (const foreground of SIGNED_PALETTE_TOKENS) {
      for (const background of SURFACE_TOKENS) {
        const ratio = contrastRatio(palette[foreground], palette[background]);
        if (ratio < 4.5) {
          fail(`${label}.palettes.${mode}.${foreground} has ${ratio.toFixed(2)}:1 contrast on ${background}; at least 4.5:1 is required`);
        }
      }
    }
  }
}

function validateThemeMetadata(theme, label) {
  sameKeys(theme, ["id", "name", "description", "startMonthDay", "endMonthDay", "revision", "palettes", "artwork"], label);
  if (typeof theme.id !== "string" || theme.id.length > 40 || !/^[a-z](?:[a-z0-9-]*[a-z])?$/.test(theme.id)) fail(`${label}.id must be a lowercase slug starting and ending with a letter`);
  stringValue(theme.name, `${label}.name`, 80);
  stringValue(theme.description, `${label}.description`, 300);
  positiveInteger(theme.revision, `${label}.revision`);
  for (const field of ["startMonthDay", "endMonthDay"]) {
    const value = theme[field];
    const match = typeof value === "string" ? /^(\d{2})-(\d{2})$/.exec(value) : null;
    if (!match) fail(`${label}.${field} must use MM-DD format`);
    const month = Number(match[1]);
    const day = Number(match[2]);
    const date = new Date(Date.UTC(2000, month - 1, day));
    if (date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) fail(`${label}.${field} is not a valid annual date`);
  }
  if (theme.startMonthDay === theme.endMonthDay) fail(`${label} annual date range must not be empty`);
  validatePalettes(theme.palettes, label);
}

function annualDayIndex(value, label) {
  const match = typeof value === "string" ? /^(\d{2})-(\d{2})$/.exec(value) : null;
  if (!match) fail(`${label} must use MM-DD format`);
  const month = Number(match[1]);
  const day = Number(match[2]);
  const date = new Date(Date.UTC(2000, month - 1, day));
  if (date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) fail(`${label} is not a valid annual date`);
  return Math.trunc((date.getTime() - Date.UTC(2000, 0, 1)) / 86_400_000);
}

function annualSegments(start, end) {
  return start < end ? [[start, end]] : [[start, 366], [0, end]];
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function validatePng(bytes, label) {
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  if (bytes.length > MAX_ASSET_BYTES) fail(`${label} is ${bytes.length} bytes; the limit is ${MAX_ASSET_BYTES}`);
  if (bytes.length < 45 || !bytes.subarray(0, 8).equals(signature)) fail(`${label} is not a complete PNG file`);

  let offset = 8;
  let header;
  let paletteSeen = false;
  let imageDataStarted = false;
  let imageDataEnded = false;
  let endSeen = false;
  const imageChunks = [];
  while (offset < bytes.length) {
    if (offset + 12 > bytes.length) fail(`${label} has a truncated PNG chunk`);
    const length = bytes.readUInt32BE(offset);
    const typeStart = offset + 4;
    const dataStart = offset + 8;
    const chunkEnd = dataStart + length;
    const nextOffset = chunkEnd + 4;
    if (nextOffset > bytes.length) fail(`${label} has a truncated PNG chunk`);
    const typeBytes = bytes.subarray(typeStart, dataStart);
    const type = typeBytes.toString("ascii");
    if (!/^[A-Za-z]{4}$/.test(type)) fail(`${label} has an invalid PNG chunk type`);
    if (["acTL", "fcTL", "fdAT"].includes(type)) fail(`${label} contains APNG animation chunks; only static PNG artwork is accepted`);
    const storedCrc = bytes.readUInt32BE(chunkEnd);
    const actualCrc = crc32(bytes.subarray(typeStart, chunkEnd));
    if (storedCrc !== actualCrc) fail(`${label} has a bad CRC in its ${type} chunk`);
    const data = bytes.subarray(dataStart, chunkEnd);
    if (endSeen) fail(`${label} has bytes after IEND`);

    if (!header && type !== "IHDR") fail(`${label} must begin with IHDR`);
    if (type === "IHDR") {
      if (header || length !== 13) fail(`${label} has an invalid IHDR chunk`);
      const width = data.readUInt32BE(0);
      const height = data.readUInt32BE(4);
      const bitDepth = data[8];
      const colorType = data[9];
      const channelsByColor = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };
      const legalDepths = { 0: [1, 2, 4, 8, 16], 2: [8, 16], 3: [1, 2, 4, 8], 4: [8, 16], 6: [8, 16] };
      if (width < 1 || height < 1 || width > MAX_DIMENSION || height > MAX_DIMENSION) fail(`${label} dimensions must be from 1 through ${MAX_DIMENSION} pixels`);
      if (!channelsByColor[colorType] || !legalDepths[colorType].includes(bitDepth)) fail(`${label} uses an unsupported PNG color type or bit depth`);
      if (data[10] !== 0 || data[11] !== 0 || data[12] !== 0) fail(`${label} must use standard PNG compression, filters, and non-interlaced rows`);
      const rowBytes = Math.ceil((width * channelsByColor[colorType] * bitDepth) / 8);
      header = { width, height, bitDepth, colorType, rowBytes, channels: channelsByColor[colorType] };
    } else if (type === "PLTE") {
      if (imageDataStarted || paletteSeen || length === 0 || length > 768 || length % 3 !== 0) fail(`${label} has an invalid PLTE chunk`);
      if (header.colorType === 0 || header.colorType === 4) fail(`${label} must not have a PLTE chunk for its grayscale color type`);
      paletteSeen = true;
      if (header.colorType === 3 && length / 3 > 2 ** header.bitDepth) fail(`${label} has more palette entries than its bit depth permits`);
    } else if (type === "IDAT") {
      if (imageDataEnded) fail(`${label} has non-contiguous IDAT chunks`);
      if (header.colorType === 3 && !paletteSeen) fail(`${label} is indexed but has no PLTE chunk`);
      imageDataStarted = true;
      imageChunks.push(data);
    } else {
      if (imageDataStarted && type !== "IEND") imageDataEnded = true;
      if ((type.charCodeAt(0) & 0x20) === 0 && !["IHDR", "PLTE", "IDAT", "IEND"].includes(type)) {
        fail(`${label} contains an unsupported critical ${type} chunk`);
      }
      if (type === "IEND") {
        if (length !== 0 || !imageDataStarted) fail(`${label} has an invalid IEND chunk`);
        endSeen = true;
      }
    }
    offset = nextOffset;
  }
  if (!header || !imageDataStarted || !endSeen || offset !== bytes.length) fail(`${label} is missing a required PNG chunk`);
  if (header.colorType === 3 && !paletteSeen) fail(`${label} is indexed but has no PLTE chunk`);

  const decodedLength = header.rowBytes * header.height;
  if (decodedLength > MAX_DECODED_IMAGE_BYTES) {
    fail(`${label} decoded pixel buffer is ${decodedLength} bytes; the limit is ${MAX_DECODED_IMAGE_BYTES}`);
  }
  const expectedLength = decodedLength + header.height;
  const compressed = Buffer.concat(imageChunks);
  let scanlines;
  try {
    scanlines = inflateSync(compressed, { maxOutputLength: expectedLength });
  } catch (error) {
    fail(`${label} image data could not be decompressed within its declared dimensions: ${error.message}`);
  }
  if (scanlines.length !== expectedLength) fail(`${label} decompressed to ${scanlines.length} bytes; expected ${expectedLength}`);
  for (let row = 0; row < header.height; row += 1) {
    if (scanlines[row * (header.rowBytes + 1)] > 4) fail(`${label} has an invalid PNG row filter`);
  }
  return { width: header.width, height: header.height };
}

function releaseTagFor(id, revision) {
  const tag = `themes-${id}-v${revision}`;
  if (/^v\d+\.\d+\.\d+$/.test(tag) || !/^themes-[a-z0-9]+(?:-[a-z0-9]+)*-v[1-9]\d*$/.test(tag)) {
    fail(`Theme tag ${tag} must remain outside the app's stable vX.Y.Z release pattern`);
  }
  return tag;
}

function releaseAssetUrl(tag, fileName) {
  return `https://github.com/OrionAISystems/Lyra-Releases/releases/download/${tag}/${encodeURIComponent(fileName)}`;
}

function validateRemoteCatalog(catalog, label) {
  sameKeys(catalog, ["discriminator", "schemaVersion", "engineVersion", "revision", "themes"], label);
  if (catalog.discriminator !== DISCRIMINATOR) fail(`${label}.discriminator must be ${DISCRIMINATOR}`);
  if (catalog.schemaVersion !== SCHEMA_VERSION) fail(`${label}.schemaVersion must be ${SCHEMA_VERSION}`);
  if (catalog.engineVersion !== ENGINE_VERSION) fail(`${label}.engineVersion must be ${ENGINE_VERSION}`);
  positiveInteger(catalog.revision, `${label}.revision`);
  if (!Array.isArray(catalog.themes) || catalog.themes.length === 0 || catalog.themes.length > MAX_THEMES) fail(`${label}.themes must be a non-empty array of at most ${MAX_THEMES} entries`);
  const annualRanges = [];
  const catalogAssets = new Map();
  for (const [index, theme] of catalog.themes.entries()) {
    const themeLabel = `${label}.themes[${index}]`;
    sameKeys(theme, ["id", "name", "description", "startMonthDay", "endMonthDay", "revision", "palettes", "artwork"], themeLabel);
    validateThemeMetadata({ ...theme, artwork: undefined }, themeLabel);
    const rangeStart = annualDayIndex(theme.startMonthDay, `${themeLabel}.startMonthDay`);
    const rangeEnd = annualDayIndex(theme.endMonthDay, `${themeLabel}.endMonthDay`);
    for (const [otherStart, otherEnd] of annualRanges) {
      const overlaps = annualSegments(rangeStart, rangeEnd).some(([start, end]) =>
        annualSegments(otherStart, otherEnd).some(([otherA, otherB]) => start < otherB && otherA < end));
      if (overlaps) fail(`${themeLabel} annual range overlaps another catalog theme`);
    }
    annualRanges.push([rangeStart, rangeEnd]);
    sameKeys(theme.artwork, THEME_SLOTS, `${themeLabel}.artwork`);
    const tag = releaseTagFor(theme.id, theme.revision);
    const distinct = new Map();
    const themeAssets = new Map();
    for (const slot of THEME_SLOTS) {
      const asset = theme.artwork[slot];
      sameKeys(asset, ["url", "sha256", "sizeBytes"], `${themeLabel}.artwork.${slot}`);
      stringValue(asset.url, `${themeLabel}.artwork.${slot}.url`, 500);
      if (!asset.url.startsWith(`${releaseAssetUrl(tag, "")}`)) fail(`${themeLabel}.artwork.${slot}.url must point to release ${tag}`);
      const parsedUrl = new URL(asset.url);
      if (parsedUrl.protocol !== "https:" || parsedUrl.hostname !== "github.com" || parsedUrl.port || parsedUrl.username || parsedUrl.password || parsedUrl.search || parsedUrl.hash) {
        fail(`${themeLabel}.artwork.${slot}.url must be a plain HTTPS GitHub release-asset URL`);
      }
      if (parsedUrl.pathname.includes("%") || parsedUrl.pathname.includes("\\")) fail(`${themeLabel}.artwork.${slot}.url must not encode or escape its release path`);
      if (!/^[a-f0-9]{64}$/.test(asset.sha256)) fail(`${themeLabel}.artwork.${slot}.sha256 must be lowercase SHA-256 hex`);
      if (!Number.isSafeInteger(asset.sizeBytes) || asset.sizeBytes < 45 || asset.sizeBytes > MAX_ASSET_BYTES) {
        fail(`${themeLabel}.artwork.${slot}.sizeBytes must be from 45 through ${MAX_ASSET_BYTES}`);
      }
      const pathFile = parsedUrl.pathname.slice(parsedUrl.pathname.lastIndexOf("/") + 1);
      validateAssetFileName(pathFile, `${themeLabel}.artwork.${slot}.url filename`);
      if (asset.url !== releaseAssetUrl(tag, pathFile)) fail(`${themeLabel}.artwork.${slot}.url must use the canonical release URL`);
      const prior = distinct.get(asset.url);
      if (prior && (prior.sha256 !== asset.sha256 || prior.sizeBytes !== asset.sizeBytes)) fail(`${themeLabel}.artwork repeats one URL with conflicting metadata`);
      distinct.set(asset.url, asset);
      const previousSize = catalogAssets.get(asset.sha256);
      if (previousSize !== undefined && previousSize !== asset.sizeBytes) fail(`${themeLabel}.artwork repeats a SHA-256 with conflicting sizes`);
      catalogAssets.set(asset.sha256, asset.sizeBytes);
      themeAssets.set(asset.sha256, asset.sizeBytes);
    }
    if (distinct.size > 8) fail(`${themeLabel} references more than eight distinct assets`);
    const totalBytes = [...themeAssets.values()].reduce((sum, sizeBytes) => sum + sizeBytes, 0);
    if (totalBytes > MAX_PACK_BYTES) fail(`${themeLabel} assets total ${totalBytes} bytes; the limit is ${MAX_PACK_BYTES}`);
  }
  const ids = catalog.themes.map((theme) => theme.id);
  if (new Set(ids).size !== ids.length) fail(`${label}.themes contains duplicate theme IDs`);
  return catalog;
}

async function readPublicKey(overridePath) {
  if (overridePath) {
    const value = (await readFile(resolve(root, overridePath), "utf8")).trim();
    if (value.startsWith("untrusted comment:")) return value;
    return decodeBase64(value, "Tauri signer public-key file", 4096).toString("utf8").trim();
  }
  const config = parseJson(await readFile(join(root, "src-tauri", "tauri.conf.json")), "src-tauri/tauri.conf.json");
  const encoded = config.plugins?.updater?.pubkey;
  if (typeof encoded !== "string" || encoded === "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY") fail("The committed Tauri updater public key is unavailable");
  return decodeBase64(encoded, "committed Tauri updater public key").toString("utf8").trim();
}

function decodeTauriSignatureFile(signatureFileBytes, label) {
  const fileText = signatureFileBytes.toString("utf8").trim();
  const signatureText = fileText.startsWith("untrusted comment:")
    ? fileText
    : decodeBase64(fileText, `${label} Tauri .sig file`, 4096).toString("utf8").trim();
  if (!signatureText.startsWith("untrusted comment:")) fail(`${label} Tauri .sig file does not contain a minisign signature`);
  return signatureText;
}

function verifyMinisign(data, signatureText, publicKeyText, label) {
  const publicLines = publicKeyText.trim().split(/\r?\n/);
  const signatureLines = signatureText.trim().split(/\r?\n/);
  if (publicLines.length < 2 || !publicLines[0].startsWith("untrusted comment:")) fail(`${label} public key is not a minisign public key`);
  if (signatureLines.length < 4 || !signatureLines[0].startsWith("untrusted comment:") || !signatureLines[2].startsWith("trusted comment:")) fail(`${label} signature is not a minisign signature`);
  const publicBytes = decodeBase64(publicLines[1], `${label} public-key payload`);
  const signatureBytes = decodeBase64(signatureLines[1], `${label} signature payload`);
  const globalSignature = decodeBase64(signatureLines[3], `${label} trusted-comment signature`);
  if (publicBytes.length !== 42 || !["Ed", "ED"].includes(publicBytes.toString("ascii", 0, 2))) fail(`${label} public key payload is invalid`);
  if (signatureBytes.length !== 74 || globalSignature.length !== 64 || signatureBytes.toString("ascii", 0, 2) !== "ED") fail(`${label} signature payload is invalid`);
  if (!signatureBytes.subarray(2, 10).equals(publicBytes.subarray(2, 10))) fail(`${label} signature key ID does not match the public key`);
  const der = Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), publicBytes.subarray(10, 42)]);
  const key = createPublicKey({ key: der, format: "der", type: "spki" });
  const messageSignature = signatureBytes.subarray(10, 74);
  const signedMessage = createHash("blake2b512").update(data).digest();
  if (!verify(null, signedMessage, key, messageSignature)) fail(`${label} payload signature did not verify`);
  const trustedComment = Buffer.from(signatureLines[2].slice("trusted comment: ".length), "utf8");
  if (!verify(null, Buffer.concat([messageSignature, trustedComment]), key, globalSignature)) fail(`${label} trusted-comment signature did not verify`);
}

function decodeEnvelope(envelopeBytes, publicKeyText, label) {
  const envelope = parseJson(envelopeBytes, label);
  sameKeys(envelope, ["payload", "signature"], `${label} envelope`);
  const payload = decodeBase64(envelope.payload, `${label} payload`, MAX_CATALOG_BYTES);
  const signature = decodeBase64(envelope.signature, `${label} signature`, 4096);
  verifyMinisign(payload, decodeTauriSignatureFile(signature, label), publicKeyText, label);
  const catalog = validateRemoteCatalog(parseJson(payload, `${label} payload`), `${label} payload`);
  return { payload, signature, catalog };
}

async function validateLocalTheme(themeId) {
  if (typeof themeId !== "string" || themeId.length > 40 || !/^[a-z](?:[a-z0-9-]*[a-z])?$/.test(themeId)) fail("--theme-id must be a lowercase slug starting and ending with a letter");
  const themeDirectory = resolve(root, "themes", themeId);
  const descriptorPath = join(themeDirectory, "theme.json");
  const descriptor = parseJson(await readFile(descriptorPath), `${descriptorPath}`);
  sameKeys(descriptor, ["id", "name", "description", "startMonthDay", "endMonthDay", "revision", "palettes", "artwork"], "theme descriptor");
  if (descriptor.id !== themeId) fail(`Theme directory ${themeId} does not match descriptor ID ${descriptor.id}`);
  validateThemeMetadata(descriptor, "theme descriptor");
  sameKeys(descriptor.artwork, THEME_SLOTS, "theme descriptor artwork");

  const distinctByPath = new Map();
  const assetsByName = new Map();
  const slotAsset = {};
  let totalBytes = 0;
  for (const slot of THEME_SLOTS) {
    const relativePath = stringValue(descriptor.artwork[slot], `theme descriptor artwork.${slot}`, 240);
    const assetPath = resolve(themeDirectory, relativePath);
    const relative = assetPath.slice(themeDirectory.length + 1);
    if (isAbsolute(relativePath) || relativePath.includes("\\") || relative.split(sep).includes("..") || !assetPath.startsWith(`${themeDirectory}${sep}`)) {
      fail(`theme descriptor artwork.${slot} must be a relative path inside themes/${themeId}`);
    }
    validateAssetFileName(basename(assetPath), `theme descriptor artwork.${slot} filename`);
    if (extname(assetPath) !== ".png") fail(`theme descriptor artwork.${slot} must refer to a lowercase .png file`);
    let asset = distinctByPath.get(assetPath);
    if (!asset) {
      const bytes = await readFile(assetPath);
      const fileStat = await stat(assetPath);
      if (!fileStat.isFile()) fail(`${assetPath} must be a regular file`);
      const dimensions = validatePng(bytes, `themes/${themeId}/${relativePath}`);
      const fileName = basename(assetPath);
      const collision = assetsByName.get(fileName);
      const hash = createHash("sha256").update(bytes).digest("hex");
      if (collision && collision.sha256 !== hash) fail(`Theme assets contain different files named ${fileName}`);
      if (collision) {
        asset = collision;
        distinctByPath.set(assetPath, asset);
        slotAsset[slot] = asset;
        continue;
      }
      asset = { assetPath, fileName, bytes, sizeBytes: bytes.length, sha256: hash, dimensions };
      distinctByPath.set(assetPath, asset);
      assetsByName.set(fileName, asset);
      totalBytes += bytes.length;
    }
    slotAsset[slot] = asset;
  }
  if (distinctByPath.size > 8) fail(`Theme ${themeId} references more than eight distinct PNG assets`);
  if (totalBytes > MAX_PACK_BYTES) fail(`Theme ${themeId} assets total ${totalBytes} bytes; the limit is ${MAX_PACK_BYTES}`);
  return { descriptor, themeDirectory, slotAsset, assets: [...assetsByName.values()], totalBytes };
}

async function prepare(options) {
  const themeId = options["theme-id"];
  const catalogRevision = positiveInteger(Number(options["catalog-revision"]), "--catalog-revision");
  const outputDirectory = resolve(root, options["output-dir"] ?? "dist/seasonal-themes");
  const currentEnvelopePath = options["current-envelope"] ? resolve(root, options["current-envelope"]) : null;
  const local = await validateLocalTheme(themeId);
  const tag = releaseTagFor(themeId, local.descriptor.revision);
  const publicKeyText = await readPublicKey(options["public-key"]);
  let existingThemes = [];
  let previousRevision = 0;
  if (currentEnvelopePath) {
    try {
      const { catalog } = decodeEnvelope(await readFile(currentEnvelopePath), publicKeyText, "existing public theme catalog");
      previousRevision = catalog.revision;
      existingThemes = catalog.themes.filter((theme) => theme.id !== themeId);
      const previousTheme = catalog.themes.find((theme) => theme.id === themeId);
      if (previousTheme && local.descriptor.revision <= previousTheme.revision) {
        fail(`Theme ${themeId} revision ${local.descriptor.revision} must be newer than the published revision ${previousTheme.revision}`);
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  if (catalogRevision <= previousRevision) fail(`Catalog revision ${catalogRevision} must be newer than the published revision ${previousRevision}`);

  const assetBySlot = {};
  for (const slot of THEME_SLOTS) {
    const asset = local.slotAsset[slot];
    assetBySlot[slot] = {
      url: releaseAssetUrl(tag, asset.fileName),
      sha256: asset.sha256,
      sizeBytes: asset.sizeBytes,
    };
  }
  const themeEntry = {
    id: local.descriptor.id,
    name: local.descriptor.name,
    description: local.descriptor.description,
    startMonthDay: local.descriptor.startMonthDay,
    endMonthDay: local.descriptor.endMonthDay,
    revision: local.descriptor.revision,
    palettes: local.descriptor.palettes,
    artwork: assetBySlot,
  };
  const catalog = {
    discriminator: DISCRIMINATOR,
    schemaVersion: SCHEMA_VERSION,
    engineVersion: ENGINE_VERSION,
    revision: catalogRevision,
    themes: [...existingThemes, themeEntry].sort((left, right) => left.id.localeCompare(right.id)),
  };
  validateRemoteCatalog(catalog, "generated catalog");
  const payload = Buffer.from(JSON.stringify(catalog), "utf8");
  if (payload.length > MAX_CATALOG_BYTES) fail(`Decoded catalog is ${payload.length} bytes; the limit is ${MAX_CATALOG_BYTES}`);
  const assetDirectory = join(outputDirectory, "assets");
  await mkdir(assetDirectory, { recursive: true });
  for (const asset of local.assets) await copyFile(asset.assetPath, join(assetDirectory, asset.fileName));
  const manifest = {
    themeId,
    themeRevision: local.descriptor.revision,
    catalogRevision,
    tag,
    assets: local.assets.map(({ fileName, sizeBytes, sha256, dimensions }) => ({
      fileName,
      sizeBytes,
      sha256,
      width: dimensions.width,
      height: dimensions.height,
      url: releaseAssetUrl(tag, fileName),
    })),
  };
  await writeFile(join(outputDirectory, "catalog.payload.json"), payload);
  await writeFile(join(outputDirectory, "assets.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  process.stdout.write(`${JSON.stringify({ ...manifest, payloadBytes: payload.length })}\n`);
}

async function envelopeCommand(options) {
  if (!options.payload || !options.signature || !options.output) fail("envelope requires --payload, --signature, and --output");
  const payloadPath = resolve(root, options.payload);
  const signaturePath = resolve(root, options.signature);
  const outputPath = resolve(root, options.output);
  const payload = await readFile(payloadPath);
  if (payload.length > MAX_CATALOG_BYTES) fail(`Decoded catalog is ${payload.length} bytes; the limit is ${MAX_CATALOG_BYTES}`);
  validateRemoteCatalog(parseJson(payload, "signed catalog payload"), "signed catalog payload");
  const signature = await readFile(signaturePath);
  const publicKeyText = await readPublicKey(options["public-key"]);
  verifyMinisign(payload, decodeTauriSignatureFile(signature, "generated catalog"), publicKeyText, "generated catalog");
  const envelope = { payload: payload.toString("base64"), signature: signature.toString("base64") };
  await writeFile(outputPath, `${JSON.stringify(envelope)}\n`, "utf8");
  process.stdout.write(`Signed catalog envelope written: ${outputPath}\n`);
}

async function verifyCommand(options) {
  if (!options.envelope) fail("verify requires --envelope");
  const { payload, catalog } = decodeEnvelope(await readFile(resolve(root, options.envelope)), await readPublicKey(options["public-key"]), "theme catalog");
  process.stdout.write(`Theme catalog signature and schema verified (revision ${catalog.revision}, ${payload.length} decoded bytes).\n`);
}

async function compareAssetsCommand(options) {
  if (!options.manifest || !options["asset-directory"]) fail("compare-assets requires --manifest and --asset-directory");
  const manifest = parseJson(await readFile(resolve(root, options.manifest)), "theme asset manifest");
  if (!Array.isArray(manifest.assets)) fail("theme asset manifest has no assets array");
  const directory = resolve(root, options["asset-directory"]);
  const present = (await readdir(directory)).sort();
  const expectedNames = manifest.assets.map((asset) => asset.fileName).sort();
  if (present.length !== expectedNames.length || present.some((name, index) => name !== expectedNames[index])) {
    fail(`Published release assets differ from the prepared asset set (expected ${expectedNames.join(", ")}; found ${present.join(", ")})`);
  }
  for (const asset of manifest.assets) {
    const bytes = await readFile(join(directory, asset.fileName));
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (bytes.length !== asset.sizeBytes || hash !== asset.sha256) fail(`Published asset ${asset.fileName} does not match the prepared size and SHA-256`);
    validatePng(bytes, `published asset ${asset.fileName}`);
  }
  process.stdout.write(`Verified ${manifest.assets.length} published theme PNG assets against their prepared hashes.\n`);
}

async function compareExistingAssetsCommand(options) {
  if (!options.manifest || !options["asset-directory"]) fail("compare-existing-assets requires --manifest and --asset-directory");
  const manifest = parseJson(await readFile(resolve(root, options.manifest)), "theme asset manifest");
  if (!Array.isArray(manifest.assets)) fail("theme asset manifest has no assets array");
  const directory = resolve(root, options["asset-directory"]);
  const present = (await readdir(directory)).sort();
  const expectedNames = new Set(manifest.assets.map((asset) => asset.fileName));
  if (present.some((name) => !expectedNames.has(name))) fail(`Existing release contains unexpected assets: ${present.filter((name) => !expectedNames.has(name)).join(", ")}`);
  const byName = new Map(manifest.assets.map((asset) => [asset.fileName, asset]));
  for (const name of present) {
    const asset = byName.get(name);
    const bytes = await readFile(join(directory, name));
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (bytes.length !== asset.sizeBytes || hash !== asset.sha256) fail(`Existing release asset ${name} does not match the prepared size and SHA-256`);
    validatePng(bytes, `existing release asset ${name}`);
  }
  process.stdout.write(`Verified ${present.length} existing theme PNG assets against their prepared hashes.\n`);
}

async function compareReleaseCommand(options) {
  if (!options.manifest || !options["release-json"] || !options["present-output"] || !options["missing-output"]) {
    fail("compare-release requires --manifest, --release-json, --present-output, and --missing-output");
  }
  const manifest = parseJson(await readFile(resolve(root, options.manifest)), "theme asset manifest");
  const release = parseJson(await readFile(resolve(root, options["release-json"])), "GitHub theme release");
  if (release.tag_name !== manifest.tag || release.draft || !release.prerelease) fail(`Existing release must be the non-draft prerelease ${manifest.tag}`);
  if (!Array.isArray(release.assets)) fail("Existing GitHub release has no assets array");
  const expected = new Map(manifest.assets.map((asset) => [asset.fileName, asset]));
  const present = [];
  for (const asset of release.assets) {
    const wanted = expected.get(asset.name);
    if (!wanted) fail(`Existing release contains unexpected asset ${asset.name}`);
    if (asset.size !== wanted.sizeBytes) fail(`Existing release asset ${asset.name} has the wrong size`);
    if (asset.digest && asset.digest !== `sha256:${wanted.sha256}`) fail(`Existing release asset ${asset.name} has a different SHA-256 digest`);
    present.push(asset.name);
  }
  const missing = manifest.assets.map((asset) => asset.fileName).filter((name) => !present.includes(name));
  await writeFile(resolve(root, options["present-output"]), present.sort().join("\n") + (present.length ? "\n" : ""), "utf8");
  await writeFile(resolve(root, options["missing-output"]), missing.join("\n") + (missing.length ? "\n" : ""), "utf8");
  process.stdout.write(`Existing theme release contains ${present.length} matching assets and is missing ${missing.length}.\n`);
}

async function verifyFeedSeparation() {
  const config = parseJson(await readFile(join(root, "src-tauri", "tauri.conf.json")), "src-tauri/tauri.conf.json");
  const endpoints = config.plugins?.updater?.endpoints;
  if (!Array.isArray(endpoints) || endpoints.length !== 1 || endpoints[0] !== APP_UPDATE_ENDPOINT) {
    fail(`The Tauri app updater must continue to use only ${APP_UPDATE_ENDPOINT}`);
  }
  if (endpoints.includes(THEME_CATALOG_ENDPOINT)) fail("The seasonal catalog endpoint must never be used as the app updater feed");
  const releaseWorkflow = await readFile(join(root, ".github", "workflows", "release.yml"), "utf8");
  if (!/tags:\s*\r?\n\s*-\s*["']?v\*\.\*\.\*["']?/m.test(releaseWorkflow)) fail("The app release workflow must remain scoped to v*.*.* tags");
  const themeWorkflow = await readFile(join(root, ".github", "workflows", "publish-themes.yml"), "utf8");
  if (!/workflow_dispatch:/m.test(themeWorkflow)) fail("The seasonal theme workflow must use workflow_dispatch");
  if (!/make_latest\s*=\s*(?:["']false["']|false)/m.test(themeWorkflow) || !/(?:-F\s+prerelease=true|prerelease\s*=\s*\$true)/m.test(themeWorkflow)) {
    fail("Seasonal releases must remain prereleases and explicitly keep make_latest false");
  }
  if (/latest\.json/.test(themeWorkflow)) fail("The seasonal theme workflow must not write or publish the app updater manifest");
  const tag = releaseTagFor("october", 1);
  if (!tag.startsWith("themes-") || /^v\d+\.\d+\.\d+$/.test(tag)) fail("Seasonal release tags must remain outside stable app release tags");
  process.stdout.write(`App updater feed (${APP_UPDATE_ENDPOINT}) and seasonal catalog (${THEME_CATALOG_ENDPOINT}) remain distinct.\n`);
}

function usage() {
  process.stdout.write([
    "Usage:",
    "  node scripts/themes-release.mjs prepare --theme-id <id> --catalog-revision <n> [--current-envelope <path>] [--output-dir <path>]",
    "  node scripts/themes-release.mjs envelope --payload <path> --signature <path> --output <path> [--public-key <path>]",
    "  node scripts/themes-release.mjs verify --envelope <path> [--public-key <path>]",
    "  node scripts/themes-release.mjs compare-assets --manifest <path> --asset-directory <path>",
    "  node scripts/themes-release.mjs compare-existing-assets --manifest <path> --asset-directory <path>",
    "  node scripts/themes-release.mjs compare-release --manifest <path> --release-json <path> --present-output <path> --missing-output <path>",
    "  node scripts/themes-release.mjs verify-feed-separation",
  ].join("\n") + "\n");
}

async function main() {
  const [command, ...rawArgs] = process.argv.slice(2);
  if (!command || command === "--help" || command === "help") return usage();
  if (command === "verify-feed-separation") {
    if (rawArgs.length !== 0) fail("verify-feed-separation takes no options");
    return verifyFeedSeparation();
  }
  const options = parseArguments(rawArgs);
  if (command === "prepare") return prepare(options);
  if (command === "envelope") return envelopeCommand(options);
  if (command === "verify") return verifyCommand(options);
  if (command === "compare-assets") return compareAssetsCommand(options);
  if (command === "compare-existing-assets") return compareExistingAssetsCommand(options);
  if (command === "compare-release") return compareReleaseCommand(options);
  fail(`Unknown command ${command}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Theme release error: ${error.message}`);
    process.exitCode = 1;
  });
}

export {
  APP_UPDATE_ENDPOINT,
  DISCRIMINATOR,
  ENGINE_VERSION,
  MAX_ASSET_BYTES,
  MAX_CATALOG_BYTES,
  MAX_PACK_BYTES,
  SCHEMA_VERSION,
  THEME_CATALOG_ENDPOINT,
  THEME_SLOTS,
  contrastRatio,
  decodeTauriSignatureFile,
  decodeBase64,
  decodeEnvelope,
  releaseTagFor,
  readPublicKey,
  validateAssetFileName,
  validateRemoteCatalog,
  validatePng,
  verifyMinisign,
};
