#!/usr/bin/env node
// ./scripts/pnpm_unlocked.mjs
/*
Purpose: Resolve pnpm dependencies without retaining a project lockfile.
How to run: node ./scripts/pnpm_unlocked.mjs install

pnpm 12 does not support the historical lockfile=false setting. It does
support --lockfile-dir, so keep the resolver's transient lockfile under
node_modules (gitignored) while keeping the virtual store in the normal
project node_modules directory. Do not use os.tmpdir(): on macOS pnpm
computes importer paths relative to the lockfile dir, and /var/folders
walks produce Permission denied. Remove the transient directory when the
command exits so project symlinks remain valid. Resolve the pnpm binary
from PNPM_HOME or node_modules/.bin instead of assuming `pnpm.cmd` is on
PATH. GitHub's Windows cmd.exe child shells do not see that shim.
*/

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const forwarded = process.argv.slice(2);

if (forwarded.length === 0) {
  console.error("Usage: node ./scripts/pnpm_unlocked.mjs <pnpm command> [args…]");
  process.exit(2);
}

function resolvePnpmExecutable() {
  const names = process.platform === "win32" ? ["pnpm.exe", "pnpm.cmd", "pnpm"] : ["pnpm"];
  const home = process.env.HOME || process.env.USERPROFILE || "";
  const searchDirs = [
    process.env.PNPM_HOME,
    path.join(repoRoot, "node_modules", ".bin"),
    home && path.join(home, ".local", "share", "pnpm"),
    home && path.join(home, ".local", "bin"),
    home && path.join(home, ".npm-global", "bin"),
    ...String(process.env.PATH || process.env.Path || "").split(path.delimiter),
  ].filter(Boolean);
  for (const dir of searchDirs) {
    for (const name of names) {
      const candidate = path.join(dir, name);
      if (existsSync(candidate)) return candidate;
    }
  }
  return names[0];
}

function envWithPnpmHome() {
  const env = { ...process.env };
  if (!env.PNPM_HOME) return env;
  for (const key of ["PATH", "Path"]) {
    if (env[key]) env[key] = `${env.PNPM_HOME}${path.delimiter}${env[key]}`;
  }
  return env;
}

const lockParent = path.join(repoRoot, "node_modules");
await mkdir(lockParent, { recursive: true });
const lockDir = await mkdtemp(path.join(lockParent, ".pnpm-lock-"));
const pnpm = resolvePnpmExecutable();
const virtualStoreDir = path.join(repoRoot, "node_modules", ".pnpm");
let status = 1;
console.log(`Using pnpm executable: ${pnpm}`);

try {
  const result = spawnSync(
    pnpm,
    [
      forwarded[0],
      "--lockfile-dir",
      lockDir,
      "--virtual-store-dir",
      virtualStoreDir,
      ...forwarded.slice(1),
    ],
    {
      cwd: repoRoot,
      env: envWithPnpmHome(),
      shell: process.platform === "win32" && pnpm.toLowerCase().endsWith(".cmd"),
      stdio: "inherit",
    },
  );
  if (result.error) {
    console.error(`Failed to run ${pnpm}: ${result.error.message}`);
    status = 1;
  } else {
    status = result.status ?? 1;
  }
} finally {
  await rm(lockDir, { recursive: true, force: true });
}

process.exit(status);
