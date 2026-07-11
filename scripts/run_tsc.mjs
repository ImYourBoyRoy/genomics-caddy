#!/usr/bin/env node
// ./scripts/run_tsc.mjs
/*
Purpose: Invoke the project TypeScript 7 `tsc` even when npm's `.bin/tsc` was
stolen by `@typescript/old` (pulled in by `@typescript/typescript6`).
*/

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const repoRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(path.join(repoRoot, "package.json"));

let tscJs;
try {
  const pkgJson = require.resolve("typescript/package.json");
  tscJs = path.join(path.dirname(pkgJson), "lib", "tsc.js");
  if (!fs.existsSync(tscJs)) {
    tscJs = path.join(path.dirname(pkgJson), "bin", "tsc");
  }
} catch {
  console.error("TypeScript is not installed (devDependency `typescript`).");
  process.exit(1);
}

const result = spawnSync(process.execPath, [tscJs, ...process.argv.slice(2)], {
  cwd: repoRoot,
  stdio: "inherit",
});
process.exit(result.status ?? 1);
