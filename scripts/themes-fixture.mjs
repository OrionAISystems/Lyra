import assert from "node:assert/strict";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import {
  APP_UPDATE_ENDPOINT,
  THEME_CATALOG_ENDPOINT,
  decodeBase64,
  releaseTagFor,
  validateAssetFileName,
  validatePng,
  validateRemoteCatalog,
} from "./themes-release.mjs";

const root = process.cwd();
const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const fixtureDirectory = await mkdtemp(join(tmpdir(), "lyra-themes-fixture-"));
let rustTestExecutable;

function fixtureExecutable() {
  if (rustTestExecutable) return rustTestExecutable;
  const features = process.env.LYRA_FIXTURE_FEATURES;
  const output = run("cargo", [
    "test", "--release", "--locked", "--manifest-path", "src-tauri/Cargo.toml",
    "--lib", "--no-run", "--message-format=json",
    ...(features ? ["--features", features] : []),
  ], { stdio: ["ignore", "pipe", "inherit"], maxBuffer: 16 * 1024 * 1024 });
  const artifacts = output.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line))
    .filter((message) => message.reason === "compiler-artifact"
      && message.target.name === "lyra_lib" && message.profile.test && message.executable);
  assert.equal(artifacts.length, 1, "Expected exactly one Lyra library test executable");
  rustTestExecutable = artifacts[0].executable;
  return rustTestExecutable;
}

function invoke(command, args, options = {}) {
  const windowsShell = process.platform === "win32" && command.endsWith(".cmd");
  const actualCommand = windowsShell ? (process.env.ComSpec ?? "cmd.exe") : command;
  const actualArgs = windowsShell
    ? ["/d", "/s", "/c", [command, ...args].map((value) => {
      const text = String(value);
      return /^[A-Za-z0-9_./:+-]+$/.test(text) ? text : `"${text.replaceAll('"', '""')}"`;
    }).join(" ")]
    : args;
  const result = spawnSync(actualCommand, actualArgs, {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    windowsVerbatimArguments: windowsShell,
    ...options,
  });
  return { ...result, actualCommand, actualArgs };
}

function run(command, args, options = {}) {
  const result = invoke(command, args, options);
  if (result.status !== 0) {
    throw new Error(`${result.actualCommand} ${result.actualArgs.join(" ")} failed (${result.error?.message ?? `exit ${result.status}`}):\n${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  }
  return result.stdout ?? "";
}

function assertCommandFails(command, args, pattern) {
  const result = invoke(command, args);
  const output = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
  assert.notEqual(result.status, 0, `${command} ${args.join(" ")} should have failed`);
  assert.match(output, pattern, `${command} ${args.join(" ")} failed for an unexpected reason`);
}

function runRustFixture(envelopePath, publicKeyPath, { assetDirectory, expectedInvalid = false, expectedInvalidAsset = false } = {}) {
  const result = invoke(fixtureExecutable(), [
    "seasonal_themes::tests::catalog_fixture_from_environment_is_verified_by_production_pipeline",
    "--exact",
  ], {
    env: {
      ...process.env,
      LYRA_THEMES_FIXTURE_ENVELOPE: envelopePath,
      LYRA_THEMES_FIXTURE_PUBLIC_KEY: publicKeyPath,
      ...(assetDirectory ? { LYRA_THEMES_FIXTURE_ASSET_DIR: assetDirectory } : {}),
      LYRA_THEMES_FIXTURE_EXPECT_INVALID: expectedInvalid ? "1" : "0",
      LYRA_THEMES_FIXTURE_EXPECT_INVALID_ASSET: expectedInvalidAsset ? "1" : "0",
    },
  });
  if (result.status !== 0) {
    throw new Error(`Rust catalog fixture failed (${result.error?.message ?? `exit ${result.status}`}):\n${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  }
  process.stdout.write(result.stdout ?? "");
  assert.match(result.stdout ?? "", /1 passed;/, "Rust catalog fixture must execute one test");
}

function runRustCacheFixture(envelopePath, publicKeyPath, assetDirectory) {
  const result = invoke(fixtureExecutable(), [
    "seasonal_themes::tests::cache_recovers_and_repairs_signed_fixture_offline",
    "--exact",
  ], {
    env: {
      ...process.env,
      LYRA_THEMES_FIXTURE_ENVELOPE: envelopePath,
      LYRA_THEMES_FIXTURE_PUBLIC_KEY: publicKeyPath,
      LYRA_THEMES_FIXTURE_ASSET_DIR: assetDirectory,
      LYRA_THEMES_FIXTURE_EXPECT_INVALID: "0",
      LYRA_THEMES_FIXTURE_EXPECT_INVALID_ASSET: "0",
    },
  });
  if (result.status !== 0) {
    throw new Error(`Rust theme cache fixture failed (${result.error?.message ?? `exit ${result.status}`}):\n${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  }
  process.stdout.write(result.stdout ?? "");
  assert.match(result.stdout ?? "", /1 passed;/, "Rust cache fixture must execute one test");
}

function assertSharedValidationBoundary(payload) {
  for (const fileName of ["pumpkin art.png", "Pumpkin.PNG", "pumpkin..png"]) {
    assert.throws(() => validateAssetFileName(fileName), `publisher accepted unsafe filename ${fileName}`);
  }
  assert.equal(validateAssetFileName("pumpkin.png"), "pumpkin.png");

  const rejects = (mutate, message) => {
    const altered = structuredClone(payload);
    mutate(altered.themes[0]);
    assert.throws(() => validateRemoteCatalog(altered, "shared contract fixture"), message);
  };
  rejects((theme) => { theme.id = "1october"; }, /id must be/);
  rejects((theme) => { theme.id = "october2"; }, /id must be/);
  rejects((theme) => { theme.id = `a${"b".repeat(39)}a`; }, /id must be/);
  rejects((theme) => { theme.name = "October\u0000theme"; }, /name must be/);
  rejects((theme) => { theme.description = "October\u0085theme"; }, /description must be/);
  rejects((theme) => { theme.artwork.idle.sizeBytes = 44; }, /sizeBytes must be/);
}

function corruptEnvelopeSignature(envelope) {
  const signatureFile = Buffer.from(envelope.signature, "base64");
  const signerBase64 = signatureFile.toString("utf8");
  const offset = Math.min(100, signerBase64.length - 4);
  signatureFile[offset] = signatureFile[offset] === 65 ? 66 : 65;
  return { ...envelope, signature: signatureFile.toString("base64") };
}

try {
  const outputDirectory = join(fixtureDirectory, "prepared");
  const privateKeyPath = join(fixtureDirectory, "fixture-key");
  const publicKeyPath = `${privateKeyPath}.pub`;
  const publicKeyTextPath = join(fixtureDirectory, "fixture-key-public-text");
  const payloadPath = join(outputDirectory, "catalog.payload.json");
  const envelopePath = join(fixtureDirectory, "catalog.json");
  const corruptEnvelopePath = join(fixtureDirectory, "catalog-corrupt-signature.json");

  assert.equal(releaseTagFor("october", 1), "themes-october-v1");
  assert.notEqual(APP_UPDATE_ENDPOINT, THEME_CATALOG_ENDPOINT);
  assert.match(APP_UPDATE_ENDPOINT, /releases\/latest\/download\/latest\.json$/);
  assert.match(THEME_CATALOG_ENDPOINT, /raw\.githubusercontent\.com\/OrionAISystems\/Lyra-Releases\/main\/themes\/catalog\.json$/);

  run("node", ["scripts/themes-release.mjs", "verify-feed-separation"]);
  run(npmCommand, ["run", "tauri", "--", "signer", "generate", "--ci", "--force", "--write-keys", privateKeyPath], {
    env: { ...process.env, CI: "1" },
  });
  const publicKeyFile = (await readFile(publicKeyPath, "utf8")).trim();
  await writeFile(publicKeyTextPath, decodeBase64(publicKeyFile, "fixture signer public key").toString("utf8"), "utf8");

  run("node", [
    "scripts/themes-release.mjs",
    "prepare",
    "--theme-id",
    "october",
    "--catalog-revision",
    "1",
    "--output-dir",
    outputDirectory,
  ]);
  const payload = JSON.parse(await readFile(payloadPath, "utf8"));
  assert.equal(payload.discriminator, "lyra-seasonal-themes");
  assert.equal(payload.schemaVersion, 1);
  assert.equal(payload.engineVersion, 1);
  assert.equal(payload.revision, 1);
  assert.equal(payload.themes.length, 1);
  assert.deepEqual(Object.keys(payload.themes[0].artwork).sort(), ["ambience", "bar", "empty", "error", "idle", "listening", "processing", "success"]);
  assertSharedValidationBoundary(payload);

  const assetManifest = JSON.parse(await readFile(join(outputDirectory, "assets.json"), "utf8"));
  const manifestPath = join(outputDirectory, "assets.json");
  const preparedAssetDirectory = join(outputDirectory, "assets");
  run("node", [
    "scripts/themes-release.mjs",
    "compare-assets",
    "--manifest",
    manifestPath,
    "--asset-directory",
    preparedAssetDirectory,
  ]);
  run("node", [
    "scripts/themes-release.mjs",
    "compare-existing-assets",
    "--manifest",
    manifestPath,
    "--asset-directory",
    preparedAssetDirectory,
  ]);
  const fixtureAssetDirectory = join(fixtureDirectory, "assets");
  const corruptAssetDirectory = join(fixtureDirectory, "assets-corrupt");
  const corruptPublishedAssetDirectory = join(fixtureDirectory, "assets-published-corrupt");
  await mkdir(fixtureAssetDirectory, { recursive: true });
  await mkdir(corruptAssetDirectory, { recursive: true });
  await mkdir(corruptPublishedAssetDirectory, { recursive: true });
  let firstFixtureAsset;
  for (const asset of assetManifest.assets) {
    const sourcePath = join(preparedAssetDirectory, asset.fileName);
    const fixturePath = join(fixtureAssetDirectory, `${asset.sha256}.png`);
    await copyFile(sourcePath, fixturePath);
    await copyFile(fixturePath, join(corruptAssetDirectory, `${asset.sha256}.png`));
    await copyFile(sourcePath, join(corruptPublishedAssetDirectory, asset.fileName));
    firstFixtureAsset ??= { path: join(corruptAssetDirectory, `${asset.sha256}.png`), bytes: await readFile(fixturePath) };
  }
  const firstPublishedAsset = assetManifest.assets[0];
  const corruptedPublishedBytes = await readFile(join(corruptPublishedAssetDirectory, firstPublishedAsset.fileName));
  corruptedPublishedBytes[corruptedPublishedBytes.length - 1] ^= 1;
  await writeFile(join(corruptPublishedAssetDirectory, firstPublishedAsset.fileName), corruptedPublishedBytes);
  const compareAssetArgs = [
    "scripts/themes-release.mjs",
    "compare-assets",
    "--manifest",
    manifestPath,
    "--asset-directory",
    corruptPublishedAssetDirectory,
  ];
  assertCommandFails("node", compareAssetArgs, /does not match the prepared size and SHA-256/);
  assertCommandFails("node", [
    "scripts/themes-release.mjs",
    "compare-existing-assets",
    "--manifest",
    manifestPath,
    "--asset-directory",
    corruptPublishedAssetDirectory,
  ], /does not match the prepared size and SHA-256/);

  const partialReleasePath = join(fixtureDirectory, "release-partial.json");
  const releasePresentPath = join(fixtureDirectory, "release-present.txt");
  const releaseMissingPath = join(fixtureDirectory, "release-missing.txt");
  const partialRelease = {
    tag_name: assetManifest.tag,
    draft: false,
    prerelease: true,
    assets: [{
      name: firstPublishedAsset.fileName,
      size: firstPublishedAsset.sizeBytes,
      digest: `sha256:${firstPublishedAsset.sha256}`,
    }],
  };
  await writeFile(partialReleasePath, `${JSON.stringify(partialRelease)}\n`, "utf8");
  const compareReleaseArgs = [
    "scripts/themes-release.mjs",
    "compare-release",
    "--manifest",
    manifestPath,
    "--release-json",
    partialReleasePath,
    "--present-output",
    releasePresentPath,
    "--missing-output",
    releaseMissingPath,
  ];
  run("node", compareReleaseArgs);
  assert.equal(await readFile(releasePresentPath, "utf8"), `${firstPublishedAsset.fileName}\n`);
  assert.equal(await readFile(releaseMissingPath, "utf8"), `${assetManifest.assets.find((asset) => asset.fileName !== firstPublishedAsset.fileName).fileName}\n`);
  const wrongDigestRelease = structuredClone(partialRelease);
  wrongDigestRelease.assets[0].digest = `sha256:${"0".repeat(64)}`;
  await writeFile(partialReleasePath, `${JSON.stringify(wrongDigestRelease)}\n`, "utf8");
  assertCommandFails("node", compareReleaseArgs, /different SHA-256 digest/);

  const apngCandidate = Buffer.from(firstFixtureAsset.bytes);
  apngCandidate.write("acTL", 37, "ascii");
  assert.throws(() => validatePng(apngCandidate, "APNG boundary fixture"), /APNG animation chunks/);
  firstFixtureAsset.bytes[firstFixtureAsset.bytes.length - 1] ^= 1;
  await writeFile(firstFixtureAsset.path, firstFixtureAsset.bytes);

  run(npmCommand, ["run", "tauri", "--", "signer", "sign", "--password", "", payloadPath], {
    env: {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY_PATH: privateKeyPath,
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
    },
  });
  run("node", [
    "scripts/themes-release.mjs",
    "envelope",
    "--payload",
    payloadPath,
    "--signature",
    `${payloadPath}.sig`,
    "--output",
    envelopePath,
    "--public-key",
    publicKeyPath,
  ]);
  run("node", ["scripts/themes-release.mjs", "verify", "--envelope", envelopePath, "--public-key", publicKeyPath]);

  const envelope = JSON.parse(await readFile(envelopePath, "utf8"));
  await writeFile(corruptEnvelopePath, `${JSON.stringify(corruptEnvelopeSignature(envelope))}\n`, "utf8");
  runRustFixture(envelopePath, publicKeyTextPath, { assetDirectory: fixtureAssetDirectory });
  runRustFixture(envelopePath, publicKeyTextPath, { assetDirectory: corruptAssetDirectory, expectedInvalidAsset: true });
  runRustFixture(corruptEnvelopePath, publicKeyTextPath, { assetDirectory: fixtureAssetDirectory, expectedInvalid: true });
  runRustCacheFixture(envelopePath, publicKeyTextPath, fixtureAssetDirectory);
  process.stdout.write("Signed seasonal catalog publishing fixture: passed\n");
} finally {
  await rm(fixtureDirectory, { recursive: true, force: true });
}
