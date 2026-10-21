import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

// Offline and deterministic: use the locked dependencies already installed on disk.
// Include the complete Windows Cargo resolution (including build dependencies),
// rather than claiming to infer which crate bytes survive linking.
const root = resolve(import.meta.dirname, "..");
const check = process.argv.includes("--check");
const outputs = new Map();
const overrides = JSON.parse(readFileSync(join(root, "licenses/upstream-sources.json"), "utf8"));
const run = (command, args) => execFileSync(command, args, {
  cwd: root, encoding: "utf8", maxBuffer: 32 * 1024 * 1024,
});
const json = (path) => JSON.parse(readFileSync(path, "utf8"));
const text = (path) => readFileSync(path, "utf8").replace(/^\uFEFF/, "").replace(/\r\n/g, "\n").trimEnd() + "\n";

function noticeFiles(directory, prefix = "") {
  const found = [];
  for (const entry of readdirSync(join(directory, prefix), { withFileTypes: true })) {
    const path = join(prefix, entry.name);
    if (entry.isFile() && /^(licen[cs]e|copying|notice|copyright|authors(?:\.|$))/i.test(entry.name)) found.push(path);
    // Preserve nested vendored notices, but never traverse installed dependencies.
    if (entry.isDirectory() && !["node_modules", ".git", "target"].includes(entry.name)) {
      found.push(...noticeFiles(directory, path));
    }
  }
  return found.sort();
}

function collect(kind, name, version, license, repository, directory) {
  if (!license) throw new Error(`Missing license declaration: ${kind}:${name}@${version}`);
  const notices = [];
  for (const file of noticeFiles(directory)) {
    const content = text(join(directory, file));
    const hash = createHash("sha256").update(content).digest("hex");
    const destination = `licenses/texts/${hash}.txt`;
    outputs.set(destination, content);
    notices.push({ original: file.replaceAll("\\", "/"), path: destination });
  }
  for (const override of overrides[`${name}@${version}`] ?? []) {
    if (!existsSync(join(root, override.path))) throw new Error(`Missing upstream notice: ${override.path}`);
    notices.push(override);
  }
  if (!notices.length) throw new Error(`Missing notice text: ${kind}:${name}@${version}`);
  return { ecosystem: kind, name, version, license, repository, source: kind === "cargo"
    ? `https://crates.io/api/v1/crates/${name}/${version}/download`
    : `https://registry.npmjs.org/${name}/-/${name.split("/").at(-1)}-${version}.tgz`, notices };
}

const entries = [];
// npm's CLI is JavaScript: invoke it with Node to avoid platform shell quoting.
const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error("Run through npm run notices:generate or npm run notices:check");
const directories = run(process.execPath, [npmCli, "ls", "--omit=dev", "--all", "--parseable"])
  .trim().split(/\r?\n/).map(resolvePath => resolve(resolvePath)).filter(path => path !== root);
for (const directory of [...new Set(directories)].sort()) {
  const pkg = json(join(directory, "package.json"));
  entries.push(collect("npm", pkg.name, pkg.version, pkg.license,
    typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url ?? null, directory));
}
const metadata = JSON.parse(run("cargo", ["metadata", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline",
  "--filter-platform", "x86_64-pc-windows-msvc", "--format-version", "1"]));
const resolved = new Set(metadata.resolve.nodes.map(node => node.id));
for (const pkg of metadata.packages.filter(pkg => pkg.source && resolved.has(pkg.id))) {
  entries.push(collect("cargo", pkg.name, pkg.version, pkg.license, pkg.repository, dirname(pkg.manifest_path)));
}
entries.sort((a, b) => `${a.ecosystem}:${a.name}@${a.version}`.localeCompare(`${b.ecosystem}:${b.name}@${b.version}`, "en"));
outputs.set("licenses/dependencies.json", JSON.stringify({
  scope: "Production npm dependencies and Windows x64 Cargo resolution, including build dependencies. Unmodified upstream sources are linked per package.",
  packages: entries,
}, null, 2) + "\n");
for (const [path, content] of outputs) {
  const destination = join(root, path);
  if (check) {
    if (!existsSync(destination) || text(destination) !== content) throw new Error(`Stale or missing notice: ${path}`);
  } else {
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, content);
  }
}
// Stale texts can overstate the distributed dependency set. Refuse silent removal.
for (const file of readdirSync(join(root, "licenses/texts"))) {
  if (!outputs.has(`licenses/texts/${file}`)) throw new Error(`Review and remove obsolete notice: licenses/texts/${file}`);
}
console.log(`${check ? "Verified" : "Generated"} notices for ${entries.length} dependencies; ${outputs.size - 1} distinct license/notice texts.`);
