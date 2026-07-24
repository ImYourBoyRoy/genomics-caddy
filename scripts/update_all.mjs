#!/usr/bin/env node
// ./scripts/update_all.mjs
/*
Purpose: One-shot “Update All” for Genomics Caddy toolchains and project dependencies.
Responsibilities:
  - Refresh package managers / compilers when possible (npm, rustup/stable Rust).
  - Bump npm package.json deps to latest stable and reinstall (lockfile), including TypeScript majors.
  - Svelte check remains usable on TS 7 via scripts/run_svelte_check.mjs (TS6 API shim until Svelte supports TS7 natively).
  - Refresh Cargo.lock and incompatible crate bumps in src-tauri.
  - Optionally refresh Node via fnm/nvm/n when --update-node is set.
  - Run light verification (npm check + cargo check) unless skipped.
How to run:
  npm run update:all
  npm run update:all -- --dry-run
  npm run update:all -- --skip-toolchains
  npm run update:all -- --update-node
Key inputs: CLI flags (see --help).
Key outputs: Updated package.json / package-lock.json / Cargo.toml / Cargo.lock; console report.
Assumptions: Network access; rustup for Rust updates; write access for global npm when updating npm itself.
*/

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(scriptDir, "..");
const cargoToml = path.join(repoRoot, "src-tauri", "Cargo.toml");
const cargoLock = path.join(repoRoot, "src-tauri", "Cargo.lock");

const args = process.argv.slice(2).map((a) => a.trim()).filter(Boolean);

function normalizeToken(raw) {
  return raw.replace(/^--?/, "").replace(/_/g, "-").toLowerCase().replace(/-/g, "");
}

const flags = new Set(args.map(normalizeToken));

function hasFlag(...aliases) {
  return aliases.some((alias) => flags.has(normalizeToken(alias)));
}

if (hasFlag("help", "h")) {
  console.log(`Usage: node ./scripts/update_all.mjs [flags]

  (default)         Apply toolchain + dependency updates, then verify
  --dry-run         Print the plan; do not mutate
  --skip-toolchains Skip npm self-update and rustup
  --skip-npm        Skip npm-check-updates + npm install
  --skip-cargo      Skip cargo update / upgrade
  --skip-verify     Skip npm run check + cargo check
  --update-node     Also try fnm/nvm/n to install latest Node (opt-in)
  --help            Show this help
`);
  process.exit(0);
}

const dryRun = hasFlag("dry-run", "DryRun");
const skipToolchains = hasFlag("skip-toolchains", "SkipToolchains");
const skipNpm = hasFlag("skip-npm", "SkipNpm");
const skipCargo = hasFlag("skip-cargo", "SkipCargo");
const skipVerify = hasFlag("skip-verify", "SkipVerify");
const updateNode = hasFlag("update-node", "UpdateNode");

const report = {
  startedAt: new Date().toISOString(),
  dryRun,
  steps: /** @type {{ name: string, ok: boolean, detail?: string }[]} */ ([]),
};

function log(msg) {
  console.log(msg);
}

function step(title) {
  console.log(`\n==> ${title}`);
}

function which(cmd) {
  if (process.platform === "win32") {
    const r = spawnSync("where.exe", [cmd], { encoding: "utf8", shell: false });
    if (r.status !== 0) return null;
    return (r.stdout || "").split(/\r?\n/).map((s) => s.trim()).find(Boolean) || null;
  }
  const r = spawnSync("sh", ["-c", 'command -v "$1"', "sh", cmd], { encoding: "utf8" });
  const out = (r.stdout || "").trim();
  return r.status === 0 && out ? out : null;
}

function run(cmd, cmdArgs, opts = {}) {
  const label = `${cmd} ${cmdArgs.join(" ")}`.trim();
  if (dryRun) {
    log(`  [dry-run] ${label}`);
    report.steps.push({ name: label, ok: true, detail: "dry-run" });
    return { status: 0, dryRun: true };
  }
  log(`  $ ${label}`);
  const result = spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd ?? repoRoot,
    stdio: "inherit",
    env: { ...process.env, ...(opts.env || {}) },
    shell: opts.shell ?? false,
  });
  const ok = (result.status ?? 1) === 0;
  report.steps.push({ name: label, ok, detail: ok ? undefined : `exit ${result.status}` });
  if (!ok && !opts.allowFail) {
    throw new Error(`Command failed (${result.status}): ${label}`);
  }
  return result;
}

function runCapture(cmd, cmdArgs) {
  const result = spawnSync(cmd, cmdArgs, {
    cwd: repoRoot,
    encoding: "utf8",
    shell: false,
  });
  return {
    status: result.status ?? 1,
    stdout: (result.stdout || "").trim(),
    stderr: (result.stderr || "").trim(),
  };
}

function versionLine(cmd, args = ["--version"]) {
  const r = runCapture(cmd, args);
  if (r.status !== 0) return null;
  return (r.stdout || r.stderr).split(/\r?\n/)[0];
}

function syncRustVersionField(rustcVersion) {
  // rustc 1.96.1 → keep Cargo.toml rust-version in sync (major.minor.patch when present)
  const m = String(rustcVersion).match(/(\d+\.\d+(?:\.\d+)?)/);
  if (!m) return;
  const ver = m[1];
  const raw = fs.readFileSync(cargoToml, "utf8");
  if (!/rust-version\s*=/.test(raw)) return;
  const next = raw.replace(/rust-version\s*=\s*"[^"]*"/, `rust-version = "${ver}"`);
  if (next === raw) {
    log(`  rust-version already ${ver}`);
    return;
  }
  if (dryRun) {
    log(`  [dry-run] set rust-version = "${ver}" in src-tauri/Cargo.toml`);
    return;
  }
  fs.writeFileSync(cargoToml, next);
  log(`  Updated Cargo.toml rust-version → ${ver}`);
  report.steps.push({ name: `rust-version → ${ver}`, ok: true });
}

function tryUpdateNodeRuntime() {
  if (which("fnm")) {
    run("fnm", ["install", "--latest"], { allowFail: true });
    run("fnm", ["use", "--install-if-missing", "latest"], { allowFail: true });
    return;
  }
  if (which("n")) {
    run("n", ["latest"], { allowFail: true });
    return;
  }
  // nvm is a shell function — invoke via bash
  const nvmDir = process.env.NVM_DIR || path.join(process.env.HOME || "", ".nvm");
  const nvmSh = path.join(nvmDir, "nvm.sh");
  if (fs.existsSync(nvmSh)) {
    run(
      "bash",
      ["-lc", `source "${nvmSh}" && nvm install node && nvm alias default node`],
      { allowFail: true }
    );
    return;
  }
  log("  No fnm/n/nvm detected — skip Node runtime upgrade (install a Node version manager to enable --update-node).");
  report.steps.push({
    name: "update-node",
    ok: true,
    detail: "skipped: no version manager",
  });
}

function ensureCargoUpgrade() {
  const help = runCapture("cargo", ["upgrade", "--help"]);
  if (help.status === 0) return true;
  log("  cargo-upgrade missing — installing cargo-edit (provides `cargo upgrade`)…");
  run("cargo", ["install", "cargo-edit", "--locked"], { allowFail: false });
  return true;
}

function ensureTypescript6CompatDep() {
  const pkgPath = path.join(repoRoot, "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  const tsRange = String(pkg.devDependencies?.typescript || pkg.dependencies?.typescript || "");
  const needsTs6 =
    /\^?7\b|>=\s*7\b/.test(tsRange) || tsRange.startsWith("7.");
  if (!needsTs6) return;
  pkg.devDependencies = pkg.devDependencies || {};
  if (!pkg.devDependencies["@typescript/typescript6"]) {
    if (dryRun) {
      log('  [dry-run] add devDependency @typescript/typescript6@^6');
      return;
    }
    pkg.devDependencies["@typescript/typescript6"] = "^6.0.2";
    fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
    log("  Ensured @typescript/typescript6 for svelte-check shim");
    report.steps.push({ name: "ensure @typescript/typescript6", ok: true });
  }
}

async function main() {
  log(`Genomics Caddy — Update All${dryRun ? " (dry-run)" : ""}`);
  log(`Root: ${repoRoot}`);

  step("Current toolchain snapshot");
  const before = {
    node: versionLine("node", ["-v"]),
    npm: versionLine("npm", ["-v"]),
    rustc: versionLine("rustc"),
    cargo: versionLine("cargo"),
    rustup: versionLine("rustup", ["-V"]),
    tauriCli: versionLine("npx", ["--no-install", "tauri", "--version"]) || versionLine("cargo", ["tauri", "--version"]),
  };
  for (const [k, v] of Object.entries(before)) {
    log(`  ${k}: ${v || "(not found)"}`);
  }

  if (!skipToolchains) {
    step("1) Toolchains — npm + Rust (and optional Node)");
    if (updateNode) {
      tryUpdateNodeRuntime();
    } else {
      log("  Node runtime left unchanged (pass --update-node to use fnm/nvm/n).");
    }

    // Refresh npm itself (the package manager), not only project packages.
    if (which("npm")) {
      try {
        run("npm", ["install", "-g", "npm@latest"], { allowFail: true });
      } catch {
        log("  WARN: global npm update failed (permissions?). Continuing with project deps.");
      }
    }

    const homeDir = process.env.HOME || "";
    const cargoBinDir = path.join(homeDir, ".cargo", "bin");
    const cargoBinRustup = path.join(cargoBinDir, "rustup");
    const cargoBinCargo = path.join(cargoBinDir, "cargo");
    const cargoBinRustc = path.join(cargoBinDir, "rustc");

    // If the official rustup is not present in ~/.cargo/bin, install it
    if (!fs.existsSync(cargoBinRustup)) {
      log("  Official rustup not found in ~/.cargo/bin. Installing official non-sandboxed rustup...");
      run(
        "sh",
        ["-c", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path"],
        { allowFail: false }
      );
    }

    if (fs.existsSync(cargoBinRustup)) {
      run(cargoBinRustup, ["update", "stable"]);
      run(cargoBinRustup, ["default", "stable"], { allowFail: true });
      const rustcVer = versionLine(cargoBinRustc);
      if (rustcVer) syncRustVersionField(rustcVer);
    } else {
      log("  Failed to configure rustup in ~/.cargo/bin — skipping toolchain update.");
      report.steps.push({ name: "rustup update stable", ok: false, detail: "not found" });
    }
  } else {
    step("1) Toolchains — skipped");
  }

  if (!skipNpm) {
    step("2) npm packages — bump package.json to latest + reinstall");
    // Update everything to latest, including TypeScript majors. Fix breaks after.
    // Uses --legacy-peer-deps (also in .npmrc): Kit's peerOptional still says TS≤6.
    run("npx", ["--yes", "npm-check-updates@latest", "-u", "--target", "latest"], {
      allowFail: false,
    });
    ensureTypescript6CompatDep();
    run("npm", ["install", "--legacy-peer-deps"], { allowFail: false });
    run("npm", ["rebuild"], { allowFail: true });
  } else {
    step("2) npm packages — skipped");
  }

  if (!skipCargo) {
    step("3) Cargo / Rust crates — lockfile + incompatible upgrades");
    if (!fs.existsSync(cargoToml)) {
      throw new Error(`Missing ${cargoToml}`);
    }
    ensureCargoUpgrade();
    // Bump Cargo.toml deps past major barriers when crates.io has newer majors.
    run(
      "cargo",
      ["upgrade", "--incompatible", "--manifest-path", cargoToml],
      { allowFail: true }
    );
    run("cargo", ["update", "--manifest-path", cargoToml], { allowFail: false });
    if (fs.existsSync(cargoLock)) {
      log(`  Cargo.lock present (${path.relative(repoRoot, cargoLock)})`);
    }
  } else {
    step("3) Cargo — skipped");
  }

  if (!skipVerify) {
    step("4) Verify — frontend check + cargo check");
    run("npm", ["run", "check"], { allowFail: false });
    run("cargo", ["check", "--manifest-path", cargoToml], { allowFail: false });
  } else {
    step("4) Verify — skipped");
  }

  step("After snapshot");
  const after = {
    node: versionLine("node", ["-v"]),
    npm: versionLine("npm", ["-v"]),
    rustc: versionLine("rustc"),
    cargo: versionLine("cargo"),
  };
  for (const [k, v] of Object.entries(after)) {
    const prev = before[k];
    const changed = prev && v && prev !== v ? `  (was ${prev})` : "";
    log(`  ${k}: ${v || "(not found)"}${changed}`);
  }

  const failed = report.steps.filter((s) => !s.ok);
  console.log("\n==> Summary");
  log(`  Steps recorded: ${report.steps.length}`);
  log(`  Failures: ${failed.length}`);
  if (failed.length) {
    for (const f of failed) log(`    - ${f.name}${f.detail ? ` (${f.detail})` : ""}`);
  }
  log(
    dryRun
      ? "\nDry-run complete. Re-run without --dry-run to apply."
      : "\nUpdate All complete. Review git diff (package.json, lockfiles, Cargo.toml) before committing."
  );

  if (failed.length && !dryRun) process.exit(1);
}

main().catch((err) => {
  console.error(`\nUpdate All failed: ${err instanceof Error ? err.message : err}`);
  process.exit(1);
});
