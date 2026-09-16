#!/usr/bin/env node
/**
 * Audit the inflammation support layer.
 *
 * This is a coverage and parity check, not a clinical validity engine. It
 * verifies that the canonical IL6/TNF/CRP copies agree, that canonical lab
 * IDs are shared by the support and actionability resources, and that every
 * inflammation-related marker found by the resource's detection terms has
 * either a marker-linked route or an explicit no-lifestyle fallback.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sourceDir = path.join(root, 'src', 'lib', 'marker-packs');
const runtimeDir = path.join(root, 'src-tauri', 'App', 'Data', 'marker-packs');
const resourcePath = path.join(sourceDir, 'inflammation_support_guidance.json');
const actionabilityPath = path.join(sourceDir, 'actionability_guidance.json');
const dbPath = path.join(root, 'src-tauri', 'src', 'db.rs');
const errors = [];
const outputJson = process.argv.includes('--json');

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(path.relative(root, file) + ': invalid JSON (' + error.message + ')');
    return null;
  }
}

function normalized(value) {
  return String(value || '').trim().toLowerCase();
}

function normalizedGene(value) {
  return String(value || '').trim().toUpperCase().replace(/[^A-Z0-9]/g, '');
}

const resource = readJson(resourcePath);
const actionability = readJson(actionabilityPath);
const manifest = readJson(path.join(sourceDir, 'manifest.json'));
const dbSource = fs.existsSync(dbPath) ? fs.readFileSync(dbPath, 'utf8') : '';
const allMarkers = [];
const sourceRuntimeParity = [];

for (const pack of manifest?.packs || []) {
  const packId = pack?.id;
  if (!packId) continue;
  const sourceFile = path.join(sourceDir, packId + '.json');
  const runtimeFile = path.join(runtimeDir, packId + '.json');
  const sourceText = fs.existsSync(sourceFile) ? fs.readFileSync(sourceFile, 'utf8') : null;
  const runtimeText = fs.existsSync(runtimeFile) ? fs.readFileSync(runtimeFile, 'utf8') : null;
  if (sourceText === null || runtimeText === null || sourceText !== runtimeText) {
    sourceRuntimeParity.push(packId);
  }
  const doc = readJson(sourceFile);
  for (const marker of doc?.markers || []) {
    allMarkers.push({ packId, marker });
  }
}
if (sourceRuntimeParity.length > 0) {
  errors.push('source/runtime marker-pack parity failed: ' + sourceRuntimeParity.join(', '));
}

const canonical = {
  rs1800795: { gene: 'IL6', effect_allele: 'G' },
  rs1800629: { gene: 'TNF', effect_allele: 'A' },
  rs1205: { gene: 'CRP', effect_allele: 'T' },
};
const canonicalRows = [];
const canonicalConflicts = [];
for (const [rsid, expected] of Object.entries(canonical)) {
  const rows = allMarkers.filter(({ marker }) => normalized(marker.rsid) === rsid);
  if (rows.length === 0) {
    errors.push('canonical inflammation marker missing from curated packs: ' + rsid);
    continue;
  }
  for (const { packId, marker } of rows) {
    const observed = {
      pack: packId,
      gene: normalizedGene(marker.gene),
      effect_allele: String(marker.effect_allele || '').trim().toUpperCase(),
    };
    canonicalRows.push({ rsid, ...observed });
    if (observed.gene !== expected.gene || observed.effect_allele !== expected.effect_allele) {
      canonicalConflicts.push({ rsid, ...observed, expected });
    }
  }
}
if (canonicalConflicts.length > 0) {
  errors.push('canonical inflammation marker conflicts: ' + canonicalConflicts.map((item) => item.rsid + '@' + item.pack).join(', '));
}

const labIds = new Set((resource?.lab_ids || []).map((lab) => normalized(lab.id)));
const actionabilityLabIds = new Set((actionability?.lab_catalog || []).map((lab) => normalized(lab.id)));
const missingActionabilityLabIds = [...labIds].filter((id) => !actionabilityLabIds.has(id));
if (missingActionabilityLabIds.length > 0) {
  errors.push('inflammation lab IDs missing from actionability lab catalog: ' + missingActionabilityLabIds.join(', '));
}

const routeByMarker = new Map();
for (const domain of resource?.domains || []) {
  for (const markerId of domain.marker_ids || []) {
    const key = normalized(markerId);
    const routes = routeByMarker.get(key) || [];
    routes.push(domain);
    routeByMarker.set(key, routes);
  }
}
const curatedIds = new Set(allMarkers.map(({ marker }) => normalized(marker.rsid)).filter(Boolean));
for (const markerId of routeByMarker.keys()) {
  if (!curatedIds.has(markerId)) errors.push('inflammation support references missing curated marker: ' + markerId);
}

const detectedTerms = resource?.coverage_policy?.detected_terms || [];
const escapedTerms = detectedTerms.map((term) => String(term).replace(/[\^$.*+?()[\]{}|]/g, '\\$&'));
const detector = new RegExp(escapedTerms.join('|'), 'i');
const detectedIds = new Set(
  allMarkers
    .filter(({ marker }) => detector.test([marker.variant_name, marker.impact, marker.interpretation].join(' ')))
    .map(({ marker }) => normalized(marker.rsid))
    .filter(Boolean),
);
const fallback = resource?.coverage_policy?.fallback;
const coverage = { marker_linked: [], explicit_no_lifestyle: [], fallback_no_lifestyle: [] };
for (const markerId of detectedIds) {
  const routes = routeByMarker.get(markerId) || [];
  if (routes.some((domain) => domain.lifestyle_recommendation !== null && domain.foundational_lifestyle)) {
    coverage.marker_linked.push(markerId);
  } else if (routes.some((domain) => domain.lifestyle_recommendation === null && domain.no_lifestyle_reason)) {
    coverage.explicit_no_lifestyle.push(markerId);
  } else if (fallback?.lifestyle_recommendation === null && fallback?.label && fallback?.reason) {
    coverage.fallback_no_lifestyle.push(markerId);
  } else {
    errors.push('inflammation marker has no guidance or explicit no-lifestyle route: ' + markerId);
  }
}

const directRule = (actionability?.rules || []).find((rule) => rule.id === 'inflammation_foundational_context');
if (!directRule) {
  errors.push('actionability_guidance.json: inflammation_foundational_context rule missing');
} else {
  const directMarkerIds = new Set((directRule.marker_ids || []).map(normalized));
  for (const markerId of coverage.marker_linked) {
    if (!directMarkerIds.has(markerId)) errors.push('marker-linked inflammation route is not wired to actionability rule: ' + markerId);
  }
  for (const field of ['dietary_favor', 'favor', 'lab_tests', 'sources']) {
    if (!Array.isArray(directRule[field]) || directRule[field].length === 0) {
      errors.push('inflammation_foundational_context: ' + field + ' must be non-empty');
    }
  }
}

if (!dbSource.includes('"inflammation_support_guidance"')
  || !dbSource.includes('include_str!("../../src/lib/marker-packs/inflammation_support_guidance.json")')) {
  errors.push('Rust support-resource bridge does not compile inflammation_support_guidance.json');
}

const summary = {
  canonical_components: Object.keys(canonical).length,
  canonical_rows: canonicalRows.length,
  detected_inflammation_marker_ids: detectedIds.size,
  coverage: {
    marker_linked: coverage.marker_linked.sort(),
    explicit_no_lifestyle: coverage.explicit_no_lifestyle.sort(),
    fallback_no_lifestyle: coverage.fallback_no_lifestyle.sort(),
  },
  lab_ids: [...labIds].sort(),
  source_runtime_parity: sourceRuntimeParity.length === 0,
  errors,
};

if (outputJson) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log('Canonical IL6/TNF/CRP components: ' + summary.canonical_components + '; rows checked: ' + summary.canonical_rows + '.');
  console.log('Inflammation-related marker IDs detected: ' + summary.detected_inflammation_marker_ids + '.');
  console.log('Guidance coverage: marker-linked=' + coverage.marker_linked.length + '; explicit no lifestyle=' + coverage.explicit_no_lifestyle.length + '; fallback no lifestyle=' + coverage.fallback_no_lifestyle.length + '.');
  console.log('Canonical lab IDs: ' + summary.lab_ids.join(', ') + '.');
  console.log('Source/runtime marker-pack parity: ' + (summary.source_runtime_parity ? 'pass' : 'fail') + '.');
  for (const error of errors) console.error('ERROR: ' + error);
}

process.exit(errors.length > 0 ? 1 : 0);
