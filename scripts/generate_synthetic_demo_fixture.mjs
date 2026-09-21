#!/usr/bin/env node
// ./scripts/generate_synthetic_demo_fixture.mjs
/**
 * Purpose: Build a parser-valid AncestryDNA-format fixture from curated pack
 *          rsIDs so docs screenshots can use a genome that is not a real person.
 * How to run: `node ./scripts/generate_synthetic_demo_fixture.mjs`
 * Optional: `--self-test` writes to a temp file and asserts counts only.
 * Inputs: `src/lib/marker-packs/*.json` (standard rsIDs and effect alleles).
 * Outputs: `src-tauri/testdata/synthetic_demo_ancestry_grch37.csv`
 * Notes: Alleles are hashed from the rsID. Never reads App/Data or user files.
 *        Does not print genotype values.
 */

import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packDir = join(root, "src/lib/marker-packs");
const outPath = join(root, "src-tauri/testdata/synthetic_demo_ancestry_grch37.csv");
const selfTest = process.argv.includes("--self-test");
const RSID_RE = /^rs\d+$/i;
const BASES = ["A", "C", "G", "T"];

function hashString(value) {
  let hash = 2166136261;
  for (const char of value) {
    hash ^= char.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function otherBase(effect) {
  const upper = BASES.includes(effect) ? effect : "A";
  return BASES.find((base) => base !== upper) ?? "C";
}

function collectMarkers() {
  const manifest = JSON.parse(readFileSync(join(packDir, "manifest.json"), "utf8"));
  const byRsid = new Map();
  for (const pack of manifest.packs ?? []) {
    const id = String(pack.id || "").trim();
    if (!id || id === "research_found") continue;
    const path = join(packDir, `${id}.json`);
    let doc;
    try {
      doc = JSON.parse(readFileSync(path, "utf8"));
    } catch {
      continue;
    }
    for (const marker of doc.markers ?? []) {
      const rsid = String(marker.rsid || "").trim().toLowerCase();
      if (!RSID_RE.test(rsid) || byRsid.has(rsid)) continue;
      const effect = String(marker.effect_allele || "A").trim().toUpperCase().slice(0, 1);
      byRsid.set(rsid, BASES.includes(effect) ? effect : "A");
    }
  }
  return [...byRsid.entries()].sort(([a], [b]) => a.localeCompare(b));
}

function buildCsv(markers) {
  const lines = [
    "# Synthetic AncestryDNA-format fixture; no user data.",
    "# Example profile for README screenshots and local demos. Not a real person.",
    "# Alleles are deterministic hashes of each rsID, not copied from any genome.",
    "# build 37; forward strand",
    "rsid,chromosome,position,allele1,allele2",
  ];
  const perChromosome = new Map();
  for (const [rsid, effect] of markers) {
    const hash = hashString(rsid);
    const chromosome = String((hash % 22) + 1);
    const index = (perChromosome.get(chromosome) ?? 0) + 1;
    perChromosome.set(chromosome, index);
    const position = 100_000 + index * 5_000;
    const other = otherBase(effect);
    const zygosity = hash % 3;
    const allele1 = zygosity === 2 ? other : effect;
    const allele2 = zygosity === 0 ? allele1 : other;
    lines.push(`${rsid},${chromosome},${position},${allele1},${allele2}`);
  }
  return `${lines.join("\n")}\n`;
}

function assertFixture(text) {
  const rows = text.trim().split("\n").filter((line) => line && !line.startsWith("#") && !line.startsWith("rsid,"));
  if (!text.includes("Synthetic AncestryDNA-format fixture")) {
    throw new Error("demo fixture is missing the synthetic header");
  }
  if (rows.length < 200) {
    throw new Error(`demo fixture is too small (${rows.length} rows)`);
  }
  for (const row of rows) {
    const rsid = row.split(",")[0] ?? "";
    if (!RSID_RE.test(rsid)) {
      throw new Error("demo fixture contained a non-standard rsID");
    }
  }
  return rows.length;
}

function main() {
  const markers = collectMarkers();
  const csv = buildCsv(markers);
  const target = selfTest ? join(mkdtempSync(join(tmpdir(), "genomics-demo-")), "demo.csv") : outPath;
  writeFileSync(target, csv);
  const count = assertFixture(readFileSync(target, "utf8"));
  if (selfTest) {
    rmSync(dirname(target), { recursive: true, force: true });
    console.log(`generate_synthetic_demo_fixture self-test: ${count} marker rows`);
    return;
  }
  console.log(`Wrote ${count} synthetic marker rows to src-tauri/testdata/synthetic_demo_ancestry_grch37.csv`);
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
