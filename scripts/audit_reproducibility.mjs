#!/usr/bin/env node
// ./scripts/audit_reproducibility.mjs
/*
Purpose: Enforce the repository's lock-free dependency contract.
Contract:
  - Direct package versions are recorded in package.json.
  - JavaScript and Cargo lockfiles are absent.
  - Cargo metadata resolves without --locked.
  - Dependency refreshes remain explicit update operations, not an implicit
    part of release or validation builds.
Output is aggregate-only and never includes application data or DNA values.
*/

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(scriptDir, "..");
const packagePath = path.join(repoRoot, "package.json");
const cargoManifestPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const exactVersion = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const failures = [];

function readJson(filePath, label) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    failures.push(`${label} could not be parsed`);
    return null;
  }
}

const packageJson = readJson(packagePath, "package.json");
const forbiddenLockfiles = [
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "bun.lock",
  "bun.lockb",
  "npm-shrinkwrap.json",
  path.join("src-tauri", "Cargo.lock"),
];

for (const relativePath of forbiddenLockfiles) {
  if (fs.existsSync(path.join(repoRoot, relativePath))) {
    failures.push(`${relativePath} must be absent in lock-free mode`);
  }
}

if (packageJson) {
  for (const group of ["dependencies", "devDependencies"]) {
    for (const [name, spec] of Object.entries(packageJson[group] ?? {})) {
      if (!exactVersion.test(String(spec))) {
        failures.push(`${group}/${name} is not pinned to an exact manifest version`);
      }
    }
  }
}

const metadata = spawnSync(
  "cargo",
  ["metadata", "--manifest-path", cargoManifestPath, "--no-deps", "--format-version", "1"],
  { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
);
if (metadata.status !== 0) {
  failures.push("Cargo metadata does not resolve without a lockfile");
}

if (failures.length > 0) {
  console.error(`FAIL: dependency reproducibility audit (${failures.length} issue(s))`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log("PASS: lock-free dependency audit");
console.log("direct_manifest_versions=pinned; package_lock=absent; cargo_lock=absent; cargo_resolution=unlocked");
