import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const changelogPath = path.join(root, "CHANGELOG.md");
const packagePath = path.join(root, "package.json");
const headingPattern = /^## \[([0-9]+\.[0-9]+\.[0-9]+)\] - ([0-9]{4}-[0-9]{2}-[0-9]{2})\r?$/gm;

function readChangelog() {
  return fs.readFileSync(changelogPath, "utf8");
}

function readPackageVersion() {
  return JSON.parse(fs.readFileSync(packagePath, "utf8")).version;
}

function parseSections(markdown) {
  const headings = [...markdown.matchAll(headingPattern)];
  return headings.map((heading, index) => {
    const bodyStart = heading.index + heading[0].length;
    const bodyEnd = headings[index + 1]?.index ?? markdown.length;
    return {
      version: heading[1],
      date: heading[2],
      body: markdown.slice(bodyStart, bodyEnd).trim(),
    };
  });
}

function verify() {
  const markdown = readChangelog();
  const currentVersion = readPackageVersion();
  const sections = parseSections(markdown);
  const errors = [];

  if (!markdown.startsWith("# Lyra changelog")) {
    errors.push("the file must start with '# Lyra changelog'");
  }
  if (sections.length === 0) {
    errors.push("no dated release sections were found");
  }
  if (sections[0]?.version !== currentVersion) {
    errors.push(`the first release section must match package version ${currentVersion}`);
  }

  const seenVersions = new Set();
  for (const section of sections) {
    if (seenVersions.has(section.version)) {
      errors.push(`duplicate release section: ${section.version}`);
    }
    seenVersions.add(section.version);
    if (!section.body || !/^### /m.test(section.body)) {
      errors.push(`release section ${section.version} must contain a categorized body`);
    }
  }

  if (errors.length > 0) {
    console.error("Changelog validation failed:");
    for (const error of errors) console.error(`- ${error}`);
    process.exitCode = 1;
    return;
  }

  console.log(`Lyra changelog verified: ${sections.length} release entries; current ${currentVersion}`);
}

function printNotes(version) {
  const normalizedVersion = version?.replace(/^v/, "");
  if (!normalizedVersion) {
    console.error("Usage: node scripts/changelog.mjs notes <version>");
    process.exitCode = 1;
    return;
  }

  const section = parseSections(readChangelog()).find((entry) => entry.version === normalizedVersion);
  if (!section) {
    console.error(`No changelog section found for ${normalizedVersion}`);
    process.exitCode = 1;
    return;
  }

  process.stdout.write(section.body);
}

const [command, version] = process.argv.slice(2);
if (command === "verify") {
  verify();
} else if (command === "notes") {
  printNotes(version);
} else {
  console.error("Usage: node scripts/changelog.mjs <verify|notes> [version]");
  process.exitCode = 1;
}
