#!/usr/bin/env node
/*
Purpose: Remove project-local, rebuildable frontend and Tauri artifacts.
Responsibilities:
  - Purge frontend installs, lockfiles, caches, and generated build output.
  - Purge every project-local Rust/Tauri target.
  - Remove Cargo.lock and common JavaScript lockfiles for an unlocked rebuild.
  - Remove staged portable binaries/resources produced by the release build.
  - Keep private app data, downloaded references, profiles, exports, and marker packs intact.
How to run:
  node ./scripts/clean_deep.mjs
  node ./scripts/clean_deep.mjs --fresh-dependencies
  node ./scripts/clean_deep.mjs --dry-run
*/

import { lstat, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

const artifacts = [
  ['frontend dependencies', 'node_modules'],
  ['frontend lockfile', 'package-lock.json'],
  ['frontend lockfile', 'pnpm-lock.yaml'],
  ['frontend lockfile', 'yarn.lock'],
  ['frontend lockfile', 'bun.lock'],
  ['frontend lockfile', 'bun.lockb'],
  ['frontend shrinkwrap', 'npm-shrinkwrap.json'],
  ['SvelteKit build cache', '.svelte-kit'],
  ['frontend build output', 'build'],
  ['staged release outputs', 'builds'],
  ['QA output', 'output'],
  ['legacy frontend output', 'dist'],
  ['generated package output', 'package'],
  ['Vite cache', '.vite'],
  ['test coverage output', 'coverage'],
  ['Tauri generated schemas', 'src-tauri/gen/schemas'],
  ['Rust/Tauri build target', 'target'],
  ['Rust/Tauri build target', 'src-tauri/target'],
  ['Rust dependency lockfile', 'src-tauri/Cargo.lock'],
  ['staged Linux portable binary', 'App/DNA-Tools'],
  ['staged Windows portable binary', 'App/DNA-Tools.exe'],
  ['staged macOS portable binary', 'App/Genomics Caddy'],
  ['staged Windows named binary', 'App/Genomics Caddy.exe'],
  ['staged release resources', 'App/resources'],
];

const args = new Set(process.argv.slice(2).map((arg) => arg.trim()).filter(Boolean));
if (args.has('--help') || args.has('-h')) {
  console.log('Usage: node ./scripts/clean_deep.mjs [--dry-run]');
  console.log('Usage: node ./scripts/clean_deep.mjs --fresh-dependencies [--dry-run]');
  console.log('Removes project-local rebuildable frontend/Tauri artifacts only.');
  console.log('Removes JavaScript and Cargo lockfiles for unlocked dependency resolution.');
  process.exit(0);
}

const freshDependencies = args.has('--fresh-dependencies');
const unknownArgs = [...args].filter((arg) => arg !== '--dry-run' && arg !== '--fresh-dependencies');
if (unknownArgs.length > 0) {
  throw new Error(`Unknown option: ${unknownArgs.join(', ')}`);
}

const dryRun = args.has('--dry-run');
const protectedPaths = [
  path.resolve(repoRoot, 'App/Data'),
  path.resolve(repoRoot, 'src/lib/marker-packs'),
  path.resolve(repoRoot, 'src-tauri/App/Data/marker-packs'),
];

function isInside(candidate, parent) {
  const relative = path.relative(parent, candidate);
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
}

function assertSafeArtifact(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!isInside(fullPath, repoRoot) || fullPath === repoRoot) {
    throw new Error(`Refusing to clean path outside the repository: ${relativePath}`);
  }
  if (protectedPaths.some((protectedPath) => isInside(fullPath, protectedPath))) {
    throw new Error(`Refusing to clean protected data path: ${relativePath}`);
  }
  return fullPath;
}

async function exists(fullPath) {
  try {
    await lstat(fullPath);
    return true;
  } catch (error) {
    if (error?.code === 'ENOENT') return false;
    throw error;
  }
}

console.log(`Genomics Caddy — deep project cleanup${dryRun ? ' (dry run)' : ''}`);
console.log(`Repository: ${repoRoot}`);
console.log('Preserved: App/Data, private profiles/genomes/exports, downloaded references, and protected marker packs.');
console.log('Not touched: global npm/Cargo caches, Rust toolchains, or files outside this repository.');
console.log(`Dependency locks: removed${freshDependencies ? ' (fresh-dependencies alias accepted)' : ''}.`);

let removed = 0;
for (const [label, relativePath] of artifacts) {
  const fullPath = assertSafeArtifact(relativePath);
  if (!(await exists(fullPath))) {
    console.log(`  skip ${label}: ${relativePath} (not present)`);
    continue;
  }
  if (dryRun) {
    console.log(`  would remove ${label}: ${relativePath}`);
    continue;
  }
  await rm(fullPath, { recursive: true, force: true });
  removed += 1;
  console.log(`  removed ${label}: ${relativePath}`);
}

if (dryRun) {
  console.log('Dry run complete. No files were changed.');
} else {
  console.log(`Deep cleanup complete. Removed ${removed} project-local artifact(s).`);
  console.log('Run node ./scripts/pnpm_unlocked.mjs install, node ./scripts/update_all.mjs, and the Tauri build to rehydrate the project.');
}
