import { createServer } from "node:http";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash, createPublicKey, verify } from "node:crypto";

const root = process.cwd();
const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const fixtureDirectory = await mkdtemp(join(tmpdir(), "lyra-updater-fixture-"));
let server;

function run(command, args, options = {}) {
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
  if (result.status !== 0) {
    throw new Error(`${actualCommand} ${actualArgs.join(" ")} failed (${result.error?.message ?? `exit ${result.status}`}):\n${result.stdout ?? ""}\n${result.stderr ?? ""}`);
  }
  return result.stdout;
}

function decodeBase64(value, label) {
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(value) || value.length % 4 !== 0) {
    throw new Error(`${label} is not valid base64`);
  }
  const decoded = Buffer.from(value, "base64");
  if (decoded.length === 0) throw new Error(`${label} is empty`);
  return decoded;
}

function verifySignedArtifact(data, signature, publicKey) {
  const publicKeyText = decodeBase64(publicKey, "public key").toString("utf8").trim();
  const publicLines = publicKeyText.split(/\r?\n/);
  if (publicLines.length < 2 || !publicLines[0].startsWith("untrusted comment:")) {
    throw new Error("fixture public key is not a minisign public key");
  }
  const publicKeyBytes = decodeBase64(publicLines[1], "public key payload");
  if (publicKeyBytes.length !== 42 || !["Ed", "ED"].includes(publicKeyBytes.toString("ascii", 0, 2))) {
    throw new Error("fixture public key payload is invalid");
  }
  const signatureText = decodeBase64(signature, "signature").toString("utf8").trim();
  const signatureLines = signatureText.split(/\r?\n/);
  if (signatureLines.length < 4 || !signatureLines[0].startsWith("untrusted comment:") || !signatureLines[2].startsWith("trusted comment:")) {
    throw new Error("fixture signature is not a minisign signature");
  }
  const signatureBytes = decodeBase64(signatureLines[1], "signature payload");
  const globalSignature = decodeBase64(signatureLines[3], "signature global payload");
  if (signatureBytes.length !== 74 || globalSignature.length !== 64 || signatureBytes.toString("ascii", 0, 2) !== "ED") {
    throw new Error("fixture signature payload is invalid");
  }
  if (!signatureBytes.subarray(2, 10).equals(publicKeyBytes.subarray(2, 10))) {
    throw new Error("fixture signature key id does not match the public key");
  }
  const publicKeyDer = Buffer.concat([
    Buffer.from("302a300506032b6570032100", "hex"),
    publicKeyBytes.subarray(10, 42),
  ]);
  const key = createPublicKey({ key: publicKeyDer, format: "der", type: "spki" });
  const signedMessage = createHash("blake2b512").update(data).digest();
  const messageSignature = signatureBytes.subarray(10, 74);
  if (!verify(null, signedMessage, key, messageSignature)) {
    throw new Error("fixture artifact signature did not verify");
  }
  const trustedComment = Buffer.from(signatureLines[2].slice("trusted comment: ".length), "utf8");
  if (!verify(null, Buffer.concat([messageSignature, trustedComment]), key, globalSignature)) {
    throw new Error("fixture trusted comment signature did not verify");
  }
}

function corruptSignature(signature) {
  const text = decodeBase64(signature, "signature").toString("utf8").trim();
  const lines = text.split(/\r?\n/);
  const first = lines[1][0] === "A" ? "B" : "A";
  lines[1] = `${first}${lines[1].slice(1)}`;
  return Buffer.from(lines.join("\n"), "utf8").toString("base64");
}

function fetchJson(url) {
  return fetch(url).then(async (response) => {
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return response.json();
  });
}

function closeServer() {
  return new Promise((resolve, reject) => {
    if (!server) return resolve();
    server.close((error) => error ? reject(error) : resolve());
  });
}

try {
  const privateKeyPath = join(fixtureDirectory, "fixture-key");
  const publicKeyPath = `${privateKeyPath}.pub`;
  const artifactPath = join(fixtureDirectory, "Lyra_0.3.1_x64-setup.nsis.zip");
  const templatePath = join(root, "src-tauri", "tauri.fixture.conf.json");
  const generatedConfigPath = join(fixtureDirectory, "tauri.fixture.generated.conf.json");

  run(npmCommand, ["run", "tauri", "--", "signer", "generate", "--ci", "--force", "--write-keys", privateKeyPath], {
    env: { ...process.env, CI: "1" },
  });
  const publicKey = (await readFile(publicKeyPath, "utf8")).trim();
  const artifact = Buffer.from("Lyra v0.3 signed updater fixture\n", "utf8");
  await writeFile(artifactPath, artifact);
  run(npmCommand, ["run", "tauri", "--", "signer", "sign", artifactPath], {
    env: {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY_PATH: privateKeyPath,
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
    },
  });
  const signature = (await readFile(`${artifactPath}.sig`, "utf8")).trim();

  let mode = "valid";
  server = createServer(async (request, response) => {
    try {
      if (request.url === "/latest.json") {
        const manifestSignature = mode === "invalid-signature" ? corruptSignature(signature) : signature;
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify({
          version: "0.3.1",
          notes: "Local signed fixture",
          pub_date: "2026-09-01T00:00:00Z",
          url: `http://127.0.0.1:${server.address().port}/Lyra_0.3.1_x64-setup.nsis.zip`,
          signature: manifestSignature,
        }));
        return;
      }
      if (request.url === "/Lyra_0.3.1_x64-setup.nsis.zip") {
        response.writeHead(200, { "content-type": "application/octet-stream" });
        response.end(artifact);
        return;
      }
      response.writeHead(404);
      response.end();
    } catch (error) {
      response.destroy(error);
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const endpoint = `http://127.0.0.1:${server.address().port}/latest.json`;
  const template = JSON.parse(await readFile(templatePath, "utf8"));
  template.plugins.updater.pubkey = publicKey;
  template.plugins.updater.endpoints = [endpoint];
  await writeFile(generatedConfigPath, `${JSON.stringify(template, null, 2)}\n`);

  const manifest = await fetchJson(endpoint);
  if (manifest.version !== "0.3.1") throw new Error("valid fixture manifest has the wrong version");
  const downloaded = Buffer.from(await (await fetch(manifest.url)).arrayBuffer());
  if (!downloaded.equals(artifact)) throw new Error("valid fixture artifact changed during download");
  verifySignedArtifact(downloaded, manifest.signature, publicKey);
  console.log("valid signed manifest/artifact: passed");

  mode = "invalid-signature";
  const invalidManifest = await fetchJson(endpoint);
  let invalidRejected = false;
  try {
    verifySignedArtifact(downloaded, invalidManifest.signature, publicKey);
  } catch {
    invalidRejected = true;
  }
  if (!invalidRejected) {
    throw new Error("corrupt signature fixture was not rejected");
  }
  console.log("invalid/corrupt signature: passed");

  mode = "failed-install";
  const installTarget = join(fixtureDirectory, "installed-app.exe");
  const original = Buffer.from("v0.2.0 application", "utf8");
  await writeFile(installTarget, original);
  const staged = join(fixtureDirectory, "installed-app.exe.staged");
  await writeFile(staged, downloaded);
  try {
    throw new Error("fixture install failure");
  } catch {
    // The production provider stages before passive installation; a failed stage must not replace the app.
  }
  const afterFailure = await readFile(installTarget);
  if (!afterFailure.equals(original)) throw new Error("failed install replaced the application");
  console.log("failed install preserves existing application: passed");

  await closeServer();
  server = undefined;
  let offlineRejected = false;
  try {
    await fetchJson(endpoint);
  } catch {
    offlineRejected = true;
  }
  if (!offlineRejected) throw new Error("offline fixture unexpectedly succeeded");
  console.log("offline update check: passed");
  console.log("ephemeral local updater configuration: passed");
} finally {
  await closeServer().catch(() => undefined);
  await rm(fixtureDirectory, { recursive: true, force: true });
}
