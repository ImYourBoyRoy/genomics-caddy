#!/usr/bin/env node
// ./scripts/package_windows_portable.mjs
/**
 * Purpose: Build a USB-clean Windows zip (exe + Data/.keep) for GitHub.
 * How to run: `node ./scripts/package_windows_portable.mjs`
 * Optional: `--upload --tag v0.2.3` signs the zip and attaches zip + `.sig`
 *           to the GitHub release. `--self-test` stages a fake exe, zips it,
 *           and asserts an Explorer-safe PKZip with Data/.keep.
 * Inputs: DNA-Tools.exe (and sibling DLLs) under src-tauri/target release dirs.
 * Outputs: `builds/windows/GenomicsCaddy-portable-windows.zip` (gitignored)
 *          and, on `--upload`, the matching `.sig`.
 * Operational notes: Upload signs with the same Tauri minisign key as NSIS.
 * In-app portable copies replace themselves from that zip; NSIS still uses
 * the plugin updater. Zip writer is Python zipfile (not `tar -a`). Windows
 * `tar` stores `./` prefixes that make Explorer show an empty archive; GNU
 * tar writes a tar named `.zip`. WinRAR then reports duplicate `./` vs
 * implied current-dir names.
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
  readFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const zipWriter = resolve(root, "scripts/lib/write_explorer_zip.py");
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

function pythonCmd() {
  const candidates =
    process.platform === "win32"
      ? [
          ["py", "-3"],
          ["python"],
          ["python3"],
        ]
      : [
          ["python3"],
          ["python"],
        ];
  for (const cmd of candidates) {
    const probe = spawnSync(cmd[0], [...cmd.slice(1), "-c", "import zipfile"], {
      encoding: "utf8",
    });
    if (probe.status === 0) return cmd;
  }
  throw new Error("package_windows_portable: Python 3 with zipfile is required");
}

function runZipWriter(args, options = {}) {
  const python = pythonCmd();
  return spawnSync(python[0], [...python.slice(1), zipWriter, ...args], {
    encoding: "utf8",
    ...options,
  });
}

export function zipStaging(staging, zipPath) {
  mkdirSync(dirname(zipPath), { recursive: true });
  rmSync(zipPath, { force: true });
  const zip = runZipWriter(["create", staging, zipPath], { stdio: "inherit" });
  if (zip.status !== 0) {
    throw new Error("package_windows_portable: failed to create zip");
  }
}

export function listZipEntries(zipPath) {
  const listed = runZipWriter(["audit", zipPath]);
  if (listed.status !== 0) {
    throw new Error(
      (listed.stderr || listed.stdout || "package_windows_portable: failed to list zip").trim()
    );
  }
  return listed.stdout.split(/\r?\n/).filter(Boolean);
}

export function assertExplorerSafeZip(zipPath) {
  const magic = readFileSync(zipPath).subarray(0, 2);
  if (magic[0] !== 0x50 || magic[1] !== 0x4b) {
    throw new Error("package_windows_portable: output is not a PKZip");
  }
  const entries = listZipEntries(zipPath);
  const dotted = entries.filter((entry) => entry === "." || entry === "./" || entry.startsWith("./"));
  if (dotted.length) {
    throw new Error(`package_windows_portable: Explorer-hostile ./ entries: ${dotted.join(", ")}`);
  }
  const keys = entries.map((entry) => entry.replace(/\\/g, "/").replace(/\/$/, "").toLowerCase());
  const duplicates = keys.filter((key, index) => keys.indexOf(key) !== index);
  if (duplicates.length) {
    throw new Error(`package_windows_portable: duplicate zip names: ${duplicates.join(", ")}`);
  }
  if (!entries.includes("Data/.keep")) {
    throw new Error(`package_windows_portable: zip is missing Data/.keep: ${entries.join(", ")}`);
  }
  if (!entries.includes("DNA-Tools.exe") && !entries.includes("DNA-Tools")) {
    throw new Error(`package_windows_portable: zip is missing DNA-Tools.exe: ${entries.join(", ")}`);
  }
  return entries;
}

function signPortableZip(zipPath) {
  const cli = resolve(root, "node_modules/@tauri-apps/cli/tauri.js");
  if (!existsSync(cli)) {
    throw new Error("package_windows_portable: Tauri CLI missing; cannot sign the zip");
  }
  const signed = spawnSync(process.execPath, [cli, "signer", "sign", zipPath], {
    stdio: "inherit",
  });
  if (signed.status !== 0) {
    throw new Error("package_windows_portable: failed to sign the portable zip");
  }
  if (!existsSync(`${zipPath}.sig`)) {
    throw new Error("package_windows_portable: expected a .sig next to the zip");
  }
}

function runSelfTest() {
  const writerTest = runZipWriter(["self-test"]);
  if (writerTest.status !== 0) {
    throw new Error((writerTest.stderr || writerTest.stdout || "write_explorer_zip self-test failed").trim());
  }
  process.stdout.write(writerTest.stdout);

  const work = mkdtempSync(join(tmpdir(), "genomics-caddy-zip-"));
  const fakeRelease = join(work, "release");
  const staging = join(work, "staging");
  const zipPath = join(work, "GenomicsCaddy-portable-windows.zip");
  mkdirSync(fakeRelease, { recursive: true });
  writeFileSync(join(fakeRelease, "DNA-Tools.exe"), "fake");
  stagePortable(fakeRelease, staging);
  zipStaging(staging, zipPath);
  assertExplorerSafeZip(zipPath);
  rmSync(work, { recursive: true, force: true });
  console.log("package_windows_portable self-test: Data/.keep present");
  console.log("package_windows_portable self-test: Windows Explorer-safe zip");
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
  assertExplorerSafeZip(zipPath);
  console.log(`Wrote ${zipPath}`);

  if (upload) {
    if (!tag) {
      console.error("package_windows_portable: --upload needs --tag or GITHUB_REF_NAME");
      process.exit(1);
    }
    signPortableZip(zipPath);
    const gh = spawnSync(
      "gh",
      ["release", "upload", tag, zipPath, `${zipPath}.sig`, "--clobber"],
      { stdio: "inherit" },
    );
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
