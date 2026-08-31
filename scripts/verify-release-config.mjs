import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const config = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const updater = config.plugins?.updater;
const expectedEndpoint = "https://github.com/OrionAISystems/Lyra-Releases/releases/latest/download/latest.json";
const errors = [];

if (!updater?.pubkey || updater.pubkey === "REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY") {
  errors.push("src-tauri/tauri.conf.json still contains the updater public-key placeholder");
}
if (!updater?.endpoints?.includes(expectedEndpoint)) {
  errors.push(`the updater endpoint must include ${expectedEndpoint}`);
}
if (config.bundle?.targets?.length !== 1 || config.bundle.targets[0] !== "nsis") {
  errors.push("the release bundle target must be NSIS only");
}
if (config.bundle?.createUpdaterArtifacts !== true) {
  errors.push("bundle.createUpdaterArtifacts must remain enabled for production releases");
}

if (errors.length > 0) {
  console.error("Release configuration is not ready:");
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}

console.log("Lyra release configuration is ready for updater-signed publication; Windows Authenticode signing is optional.");
