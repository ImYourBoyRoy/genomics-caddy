// ./scripts/run_release_build.mjs
// Cross-platform entry for production Tauri release builds.
// Windows -> purge_and_build.ps1; Linux/macOS -> purge_and_build.sh
//
// npm scripts pass kebab flags (e.g. --skip-purge). Accept those plus
// PowerShell-style -SkipPurge so build:release-fast actually skips cargo clean.

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(scriptDir, "..");
const forwarded = process.argv.slice(2).map((a) => a.trim()).filter(Boolean);

/** Normalize --skip-purge / -SkipPurge / skip-purge → skippurge */
function normalizeToken(raw) {
  return raw.replace(/^--?/, "").replace(/_/g, "-").toLowerCase().replace(/-/g, "");
}

const normalized = new Set(forwarded.map(normalizeToken));

function hasFlag(...aliases) {
  return aliases.some((alias) => normalized.has(normalizeToken(alias)));
}

const purgeOnly = hasFlag("purge-only", "PurgeOnly");
const skipPurge = hasFlag("skip-purge", "SkipPurge");
const skipChecks = hasFlag("skip-checks", "SkipChecks");
const dryRun = hasFlag("dry-run", "DryRun");

if (process.platform === "win32") {
  const psArgs = [
    "-NoLogo",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    path.join(scriptDir, "purge_and_build.ps1"),
  ];
  if (purgeOnly) psArgs.push("-PurgeOnly");
  if (skipPurge) psArgs.push("-SkipPurge");
  if (skipChecks) psArgs.push("-SkipChecks");
  if (dryRun) psArgs.push("-DryRun");

  console.log(`[run_release_build] pwsh ${psArgs.slice(5).join(" ")}`);
  const result = spawnSync("pwsh", psArgs, { stdio: "inherit", cwd: repoRoot });
  process.exit(result.status ?? 1);
}

const shArgs = [path.join(scriptDir, "purge_and_build.sh")];
if (purgeOnly) shArgs.push("--purge-only");
if (skipPurge) shArgs.push("--skip-purge");
if (skipChecks) shArgs.push("--skip-checks");
if (dryRun) shArgs.push("--dry-run");

console.log(`[run_release_build] bash ${path.basename(shArgs[0])} ${shArgs.slice(1).join(" ")}`);
const result = spawnSync("bash", shArgs, { stdio: "inherit", cwd: repoRoot });
process.exit(result.status ?? 1);
