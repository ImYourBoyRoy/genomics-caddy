#!/usr/bin/env node
// ./scripts/run_tsc.mjs
/*
Purpose: Invoke the TypeScript 6 compiler required by SvelteKit 3.
How to run: pnpm run typecheck  or  pnpm run build (calls this before Vite).
Side effect: runs `svelte-kit sync` first so SvelteKit's generated types and
package-import mappings are available on a fresh checkout.
*/

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const repoRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(path.join(repoRoot, "package.json"));

function runNodeScript(scriptPath, args, label) {
  const result = spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  if (result.error) {
    console.error(`Failed to run ${label}: ${result.error.message}`);
    process.exit(1);
  }
  if ((result.status ?? 1) !== 0) process.exit(result.status ?? 1);
}

let svelteKitJs;
try {
  svelteKitJs = path.join(path.dirname(require.resolve("@sveltejs/kit/package.json")), "svelte-kit.js");
} catch {
  console.error("SvelteKit is not installed (devDependency `@sveltejs/kit`).");
  process.exit(1);
}
runNodeScript(svelteKitJs, ["sync"], "svelte-kit sync");

let tscJs;
try {
  const pkgJson = require.resolve("typescript/package.json");
  tscJs = path.join(path.dirname(pkgJson), "lib", "tsc.js");
  if (!fs.existsSync(tscJs)) {
    tscJs = path.join(path.dirname(pkgJson), "bin", "tsc");
  }
} catch {
  console.error("TypeScript 6 is not installed (devDependency `typescript`).");
  process.exit(1);
}

runNodeScript(tscJs, process.argv.slice(2), "tsc");
