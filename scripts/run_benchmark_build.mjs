// ./scripts/run_benchmark_build.mjs
// Cross-platform entry for build purge/rebuild timing benchmarks.
// Windows -> benchmark_build.ps1; Linux/macOS -> benchmark_build.sh
// Accepts --flag and -Flag forms (pnpm passes leading dashes).

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(scriptDir, "..");
const forwarded = process.argv.slice(2).map((a) => a.trim()).filter(Boolean);

function normalizeToken(raw) {
  return raw.replace(/^--?/, "").replace(/_/g, "-").toLowerCase().replace(/-/g, "");
}

const normalized = new Set(forwarded.map(normalizeToken));

function hasFlag(...aliases) {
  return aliases.some((alias) => normalized.has(normalizeToken(alias)));
}

const purgeOnly = hasFlag("purge-only", "PurgeOnly");
const skipChecks = hasFlag("skip-checks", "SkipChecks");
const json = hasFlag("json", "Json");
const rebuild = hasFlag("rebuild", "full");

if (process.platform === "win32") {
  const psArgs = [
    "-NoLogo",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    path.join(scriptDir, "benchmark_build.ps1"),
  ];
  if (purgeOnly) psArgs.push("-PurgeOnly");
  if (skipChecks) psArgs.push("-SkipChecks");
  if (json) psArgs.push("-Json");
  const result = spawnSync("pwsh", psArgs, { stdio: "inherit", cwd: repoRoot });
  process.exit(result.status ?? 1);
}

const shArgs = [path.join(scriptDir, "benchmark_build.sh")];
if (purgeOnly) shArgs.push("--purge-only");
if (rebuild) shArgs.push("--rebuild");
if (skipChecks) shArgs.push("--skip-checks");
if (json) shArgs.push("--json");
const result = spawnSync("bash", shArgs, { stdio: "inherit", cwd: repoRoot });
process.exit(result.status ?? 1);
