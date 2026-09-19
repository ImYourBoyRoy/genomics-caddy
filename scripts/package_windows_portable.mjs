#!/usr/bin/env node
// ./scripts/package_windows_portable.mjs
/**
 * Purpose: Build a USB-clean Windows zip (exe + Data/.keep) for GitHub.
 * How to run: `node ./scripts/package_windows_portable.mjs`
 * Optional: `--upload --tag v0.2.1` attaches the zip to the GitHub release.
 *           `--self-test` stages a fake exe, zips it, and asserts Data/.keep.
 * Inputs: DNA-Tools.exe (and sibling DLLs) under src-tauri/target release dirs.
 * Outputs: `builds/windows/GenomicsCaddy-portable-windows.zip` (gitignored).
 * Operational notes: Does not sign. The NSIS installer remains the updater path.
 */

import {
  mkdirSync,
  existsSync,
  readdirSync,
  statSync,
  copyFileSync,
  rmSync,
  writeFileSync,
  mkdtempSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Set(process.argv.slice(2));
const tagIndex = process.argv.indexOf("--tag");
const tag = tagIndex >= 0 ? process.argv[tagIndex + 1] : process.env.GITHUB_REF_NAME;
const upload = args.has("--upload");
const selfTest = args.has("--self-test");

export function findReleaseDir(projectRoot = root) {
  const envDir = process.env.GENOMICS_RELEASE_DIR;
  const candidates = [
    envDir,
    resolve(projectRoot, "src-tauri/target/release"),
    resolve(projectRoot, "src-tauri/target/x86_64-pc-windows-msvc/release"),
    resolve(projectRoot, "src-tauri/target/x86_64-pc-windows-gnu/release"),
  ].filter(Boolean);
  for (const dir of candidates) {
    if (existsSync(join(dir, "DNA-Tools.exe")) || existsSync(join(dir, "DNA-Tools"))) {
      return dir;
    }
  }
  return "";
}

export function exeNameIn(releaseDir) {
  if (existsSync(join(releaseDir, "DNA-Tools.exe"))) return "DNA-Tools.exe";
  if (existsSync(join(releaseDir, "DNA-Tools"))) return "DNA-Tools";
  return "";
}

export function stagePortable(releaseDir, staging) {
  rmSync(staging, { recursive: true, force: true });
  mkdirSync(join(staging, "Data"), { recursive: true });
  writeFileSync(join(staging, "Data", ".keep"), "");
  const exeName = exeNameIn(releaseDir);
  if (!exeName) {
    throw new Error(`package_windows_portable: no DNA-Tools.exe in ${releaseDir}`);
  }
  copyFileSync(join(releaseDir, exeName), join(staging, exeName));

  for (const name of readdirSync(releaseDir)) {
    if (!name.toLowerCase().endsWith(".dll")) continue;
    const from = join(releaseDir, name);
    if (statSync(from).isFile()) {
      copyFileSync(from, join(staging, name));
    }
  }

  const resources = join(releaseDir, "resources");
  if (existsSync(resources) && statSync(resources).isDirectory()) {
    copyTree(resources, join(staging, "resources"));
  }
}

function copyTree(from, to) {
  mkdirSync(to, { recursive: true });
  for (const name of readdirSync(from)) {
    const src = join(from, name);
    const dest = join(to, name);
    if (statSync(src).isDirectory()) copyTree(src, dest);
    else copyFileSync(src, dest);
  }
}

export function zipStaging(staging, zipPath) {
  mkdirSync(dirname(zipPath), { recursive: true });
  rmSync(zipPath, { force: true });
  const zip = spawnSync("tar", ["-a", "-c", "-f", zipPath, "-C", staging, "."], {
    stdio: "inherit",
  });
  if (zip.status !== 0) {
    throw new Error("package_windows_portable: failed to create zip");
  }
}

export function listZipEntries(zipPath) {
  const listed = spawnSync("tar", ["-tf", zipPath], { encoding: "utf8" });
  if (listed.status !== 0) {
    throw new Error(listed.stderr || "package_windows_portable: failed to list zip");
  }
  return listed.stdout.split(/\r?\n/).filter(Boolean);
}

function runSelfTest() {
  const work = mkdtempSync(join(tmpdir(), "genomics-caddy-zip-"));
  const fakeRelease = join(work, "release");
  const staging = join(work, "staging");
  const zipPath = join(work, "GenomicsCaddy-portable-windows.zip");
  mkdirSync(fakeRelease, { recursive: true });
  writeFileSync(join(fakeRelease, "DNA-Tools.exe"), "fake");
  stagePortable(fakeRelease, staging);
  zipStaging(staging, zipPath);
  const entries = listZipEntries(zipPath);
  const hasKeep = entries.some((entry) => entry.replace(/\\/g, "/").includes("Data/.keep"));
  if (!hasKeep) {
    throw new Error(`zip missing Data/.keep: ${entries.join(", ")}`);
  }
  rmSync(work, { recursive: true, force: true });
  console.log("package_windows_portable self-test: Data/.keep present");
}

function main() {
  if (selfTest) {
    runSelfTest();
    return;
  }

  const releaseDir = findReleaseDir();
  const exeName = exeNameIn(releaseDir);
  if (!exeName) {
    console.error("package_windows_portable: no DNA-Tools.exe in src-tauri/target/release");
    process.exit(1);
  }

  const staging = resolve(root, "builds/windows/GenomicsCaddy-portable");
  const zipPath = resolve(root, "builds/windows/GenomicsCaddy-portable-windows.zip");
  stagePortable(releaseDir, staging);
  zipStaging(staging, zipPath);
  const entries = listZipEntries(zipPath);
  if (!entries.some((entry) => entry.replace(/\\/g, "/").includes("Data/.keep"))) {
    console.error("package_windows_portable: zip is missing Data/.keep");
    process.exit(1);
  }
  console.log(`Wrote ${zipPath}`);

  if (upload) {
    if (!tag) {
      console.error("package_windows_portable: --upload needs --tag or GITHUB_REF_NAME");
      process.exit(1);
    }
    const gh = spawnSync("gh", ["release", "upload", tag, zipPath, "--clobber"], {
      stdio: "inherit",
    });
    if (gh.status !== 0) {
      console.error("package_windows_portable: gh release upload failed");
      process.exit(gh.status ?? 1);
    }
  }
}

const invokedDirectly = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (invokedDirectly) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}
