#!/usr/bin/env node
// ./scripts/run_svelte_check.mjs
/*
Purpose: Run svelte-kit sync + svelte-check with the TypeScript 6 API required
by SvelteKit 3.
Responsibilities:
  - Forward all CLI args to svelte-check.
How to run: pnpm run check  (wired in package.json)
*/

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(path.join(repoRoot, "package.json"));

const forwarded = process.argv.slice(2);
const watch = forwarded.includes("--watch");

function run(cmd, args) {
  const result = spawnSync(cmd, args, {
    cwd: repoRoot,
    stdio: "inherit",
    env: process.env,
    shell: false,
  });
  if (result.error) {
    console.error(`Failed to run ${cmd}: ${result.error.message}`);
    process.exit(1);
  }
  if ((result.status ?? 1) !== 0) process.exit(result.status ?? 1);
}

// Invoke the package entry points through Node directly. This keeps the
// wrapper shell-free and works on Windows, where npm's `npx.cmd` shim cannot
// be spawned reliably with `shell:false`.
const svelteKit = path.join(path.dirname(require.resolve("@sveltejs/kit/package.json")), "svelte-kit.js");
const svelteCheck = path.join(path.dirname(require.resolve("svelte-check/package.json")), "bin", "svelte-check");
run(process.execPath, [svelteKit, "sync"]);
run(process.execPath, [svelteCheck, "--tsconfig", "./tsconfig.json", ...forwarded.filter((a) => a !== "--watch"), ...(watch ? ["--watch"] : [])]);
