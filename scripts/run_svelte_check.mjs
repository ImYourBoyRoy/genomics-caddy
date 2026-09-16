#!/usr/bin/env node
// ./scripts/run_svelte_check.mjs
/*
Purpose: Run svelte-kit sync + svelte-check against a TypeScript 7 project.
Responsibilities:
  - Preload the TS6 API shim for Svelte embedders.
  - Forward all CLI args to svelte-check.
How to run: pnpm run check  (wired in package.json)
*/

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(scriptDir, "..");
const preload = path.join(scriptDir, "preload-typescript6-for-svelte.cjs");

const forwarded = process.argv.slice(2);
const watch = forwarded.includes("--watch");

function run(cmd, args) {
  const result = spawnSync(cmd, args, {
    cwd: repoRoot,
    stdio: "inherit",
    env: {
      ...process.env,
      GENOMICS_TS6_SHIM_QUIET: process.env.GENOMICS_TS6_SHIM_QUIET ?? "1",
      NODE_OPTIONS: [process.env.NODE_OPTIONS, `--require ${preload}`].filter(Boolean).join(" "),
    },
    shell: false,
  });
  if ((result.status ?? 1) !== 0) process.exit(result.status ?? 1);
}

run("npx", ["svelte-kit", "sync"]);
run("npx", ["svelte-check", "--tsconfig", "./tsconfig.json", ...forwarded.filter((a) => a !== "--watch"), ...(watch ? ["--watch"] : [])]);
