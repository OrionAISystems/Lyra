import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const packageJson = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const packageLock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
const tauriConfig = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const cargoManifest = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
const cargoVersion = cargoManifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const lockVersion = packageLock.packages?.[""]?.version;
const versions = {
  "package.json": packageJson.version,
  "package-lock.json": lockVersion,
  "src-tauri/Cargo.toml": cargoVersion,
  "src-tauri/tauri.conf.json": tauriConfig.version,
};
const tag = process.env.GITHUB_REF_NAME;
const tagVersion = tag?.match(/^v(\d+\.\d+\.\d+)$/)?.[1];
if (tag && !tagVersion) {
  console.error(`Release tags must be stable vX.Y.Z tags; received ${tag}.`);
  process.exit(1);
}
const expected = tagVersion ?? packageJson.version;
const drift = Object.entries(versions).filter(([, version]) => version !== expected);

if (drift.length > 0) {
  console.error(`Version drift detected; expected ${expected}.`);
  for (const [file, version] of drift) console.error(`  ${file}: ${version ?? "missing"}`);
  process.exit(1);
}

console.log(`Lyra release version verified: ${expected}`);
