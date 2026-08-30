#!/usr/bin/env node
/*
Purpose: Remove project-local, rebuildable frontend and Tauri artifacts.
Responsibilities:
  - Purge frontend installs, lockfile, caches, and generated build output.
  - Purge every project-local Rust/Tauri target.
  - Optionally remove Cargo.lock for an explicitly requested fresh dependency rebuild.
  - Remove staged portable binaries/resources produced by the release build.
  - Keep private app data, downloaded references, profiles, exports, and marker packs intact.
How to run:
  npm run clean:deep
  npm run clean:deep:fresh
  npm run clean:deep:dry
*/

import { lstat, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');

const artifacts = [
  ['frontend dependencies', 'node_modules'],
  ['frontend lockfile', 'package-lock.json'],
  ['SvelteKit build cache', '.svelte-kit'],
  ['frontend build output', 'build'],
  ['legacy frontend output', 'dist'],
  ['generated package output', 'package'],
  ['Vite cache', '.vite'],
  ['test coverage output', 'coverage'],
  ['Tauri generated schemas', 'src-tauri/gen/schemas'],
  ['Rust/Tauri build target', 'target'],
  ['Rust/Tauri build target', 'src-tauri/target'],
  ['staged Linux portable binary', 'App/DNA-Tools'],
  ['staged Windows portable binary', 'App/DNA-Tools.exe'],
  ['staged macOS portable binary', 'App/Genomics Caddy'],
  ['staged Windows named binary', 'App/Genomics Caddy.exe'],
  ['staged release resources', 'App/resources'],
];

const args = new Set(process.argv.slice(2).map((arg) => arg.trim()).filter(Boolean));
if (args.has('--help') || args.has('-h')) {
  console.log('Usage: npm run clean:deep [-- --dry-run]');
  console.log('Usage: npm run clean:deep:fresh [-- --dry-run]');
  console.log('Removes project-local rebuildable frontend/Tauri artifacts only.');
  console.log('Cargo.lock is preserved unless --fresh-dependencies is explicitly supplied.');
  process.exit(0);
}

const freshDependencies = args.has('--fresh-dependencies');
const unknownArgs = [...args].filter((arg) => arg !== '--dry-run' && arg !== '--fresh-dependencies');
if (unknownArgs.length > 0) {
  throw new Error(`Unknown option: ${unknownArgs.join(', ')}`);
}

const dryRun = args.has('--dry-run');
if (freshDependencies) {
  artifacts.push(['Rust dependency lockfile (fresh dependency rebuild)', 'src-tauri/Cargo.lock']);
}
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
console.log(`Cargo.lock: ${freshDependencies ? 'included by explicit fresh-dependencies request' : 'preserved'}.`);

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
  console.log('Run npm install, npm run update:all, and the Tauri build to rehydrate the project.');
}
