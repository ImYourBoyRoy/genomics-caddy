#!/usr/bin/env node
// ./scripts/audit_updater_config.mjs
/**
 * Purpose: Fail CI when the signed in-app updater contract drifts.
 * How to run: `pnpm run audit:updater-config` (no network, no secrets).
 * Inputs: tauri.conf.json, capabilities, Cargo.toml, package.json, release.yml, lib.rs.
 * Outputs: OK line or a list of missing wiring. Never prints private keys.
 */

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const problems = [];

function read(relativePath) {
  return readFileSync(resolve(root, relativePath), "utf8");
}

function mustContain(relativePath, needle, label = needle) {
  const text = read(relativePath);
  if (!text.includes(needle)) {
    problems.push(`${relativePath} is missing ${label}`);
  }
}

const tauriConf = JSON.parse(read("src-tauri/tauri.conf.json"));
const capabilities = JSON.parse(read("src-tauri/capabilities/default.json"));
const pkg = JSON.parse(read("package.json"));

const pubkey = tauriConf?.plugins?.updater?.pubkey?.trim() ?? "";
if (pubkey.length < 80) {
  problems.push("src-tauri/tauri.conf.json plugins.updater.pubkey is missing or too short");
}
if (pubkey.includes("BEGIN") || pubkey.includes("PRIVATE")) {
  problems.push("src-tauri/tauri.conf.json pubkey looks like a private key; commit only the public key");
}

const endpoints = tauriConf?.plugins?.updater?.endpoints ?? [];
if (!endpoints.includes("https://github.com/ImYourBoyRoy/genomics-caddy/releases/latest/download/latest.json")) {
  problems.push("updater endpoints must include GitHub latest.json for genomics-caddy");
}

if (tauriConf?.bundle?.createUpdaterArtifacts !== true) {
  problems.push("bundle.createUpdaterArtifacts must be true so GitHub releases get signatures");
}

const permissions = capabilities?.permissions ?? [];
for (const permission of ["updater:default", "process:allow-restart"]) {
  if (!permissions.includes(permission)) {
    problems.push(`src-tauri/capabilities/default.json is missing ${permission}`);
  }
}

if (!pkg.dependencies?.["@tauri-apps/plugin-updater"]) {
  problems.push("package.json is missing @tauri-apps/plugin-updater");
}
if (!pkg.dependencies?.["@tauri-apps/plugin-process"]) {
  problems.push("package.json is missing @tauri-apps/plugin-process");
}

mustContain("src-tauri/Cargo.toml", "tauri-plugin-updater");
mustContain("src-tauri/Cargo.toml", "tauri-plugin-process");
mustContain("src-tauri/src/lib.rs", "tauri_plugin_updater::Builder");
mustContain("src-tauri/src/lib.rs", "tauri_plugin_process::init");
mustContain(".github/workflows/release.yml", "TAURI_SIGNING_PRIVATE_KEY");
mustContain(".github/workflows/release.yml", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD");
mustContain(".github/workflows/release.yml", "tauri-apps/tauri-action@v1");
mustContain(".github/workflows/release.yml", "uploadUpdaterJson: true");
mustContain(".github/workflows/release.yml", "updaterJsonPreferNsis: true");
if (read(".github/workflows/release.yml").includes("tauri-apps/tauri-action@v2")) {
  problems.push(".github/workflows/release.yml must not use tauri-action@v2; that tag does not exist");
}
mustContain("scripts/run_tsc.mjs", "svelte-kit sync");
mustContain("src/lib/utils/updater.ts", "checkForAppUpdate");
mustContain("src/lib/components/layout/AppShell.svelte", "AppUpdateHost");

if (problems.length) {
  console.error("Updater config audit failed:");
  for (const problem of problems) console.error(`- ${problem}`);
  process.exit(1);
}

console.log("Updater config audit OK: signed GitHub latest.json path is wired.");
