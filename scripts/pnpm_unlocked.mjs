#!/usr/bin/env node
// ./scripts/pnpm_unlocked.mjs
/*
Purpose: Resolve pnpm dependencies without retaining a project lockfile.
How to run: node ./scripts/pnpm_unlocked.mjs install

pnpm 12 does not support the historical lockfile=false setting. It does
support --lockfile-dir, so keep the resolver's transient lockfile in a
temporary directory outside the repository while keeping the virtual store
in the normal project node_modules directory. Remove the transient directory
when the command exits so project symlinks remain valid.
*/

import { spawnSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const forwarded = process.argv.slice(2);

if (forwarded.length === 0) {
  console.error("Usage: node ./scripts/pnpm_unlocked.mjs <pnpm command> [args…]");
  process.exit(2);
}

const lockDir = await mkdtemp(path.join(os.tmpdir(), "genomics-caddy-pnpm-lock-"));
const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const virtualStoreDir = path.join(repoRoot, "node_modules", ".pnpm");
let status = 1;

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
      env: process.env,
      shell: false,
      stdio: "inherit",
    },
  );
  status = result.status ?? 1;
} finally {
  await rm(lockDir, { recursive: true, force: true });
}

process.exit(status);
