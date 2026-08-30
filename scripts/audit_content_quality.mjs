#!/usr/bin/env node
/**
 * Privacy-safe aggregate audit for report-content quality.
 *
 * This complements audit_resource_quality.mjs. It measures repetition,
 * vagueness signals, missing Simple-mode fields, recommendation provenance,
 * source IDs, and multi-marker link integrity without loading DNA fixtures or
 * printing genotype values.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { analyzeContentQuality } from './content_quality_metrics.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const resourceDir = path.join(root, 'src', 'lib', 'marker-packs');
const runtimeDir = path.join(root, 'src-tauri', 'App', 'Data', 'marker-packs');
const jsonOutput = process.argv.includes('--json');

function relative(file) {
  return path.relative(root, file) || path.basename(file);
}

function readJson(file, errors) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(`${relative(file)} could not be read as JSON`);
    return null;
  }
}

const inputErrors = [];
const manifest = readJson(path.join(resourceDir, 'manifest.json'), inputErrors);
const runtimeSummary = readJson(path.join(resourceDir, 'runtime_validation_summary.json'), inputErrors);
const actionability = readJson(path.join(resourceDir, 'actionability_guidance.json'), inputErrors) || {};
const laypersonTranslations = readJson(path.join(resourceDir, 'layperson_translations.json'), inputErrors) || {};
const discoveryCatalog = readJson(path.join(resourceDir, 'discovery_catalog.json'), inputErrors) || {};
const sourceRegistry = readJson(path.join(resourceDir, 'source_registry.json'), inputErrors) || {};

const packDocs = [];
for (const pack of manifest?.packs || []) {
  const packId = typeof pack?.id === 'string' ? pack.id : '';
  if (!packId) {
    inputErrors.push('manifest contains a pack without an ID');
    continue;
  }
  const sourceFile = path.join(resourceDir, `${packId}.json`);
  const doc = readJson(sourceFile, inputErrors);
  if (!Array.isArray(doc?.markers)) {
    inputErrors.push(`${relative(sourceFile)} does not contain a marker array`);
    continue;
  }
  packDocs.push({ id: packId, markers: doc.markers });
}

const supportResources = [];
for (const fileName of runtimeSummary?.support_files || []) {
  if (typeof fileName !== 'string' || !fileName.trim()) continue;
  const file = path.join(resourceDir, fileName);
  const value = readJson(file, inputErrors);
  if (value) supportResources.push({ id: fileName, value });
}

const summary = analyzeContentQuality({
  packDocs,
  supportResources,
  actionability,
  laypersonTranslations,
  discoveryCatalog,
  sourceRegistry,
});
summary.errors = [...inputErrors, ...summary.errors];

if (jsonOutput) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log('Content-quality audit (aggregate only; no DNA fixtures loaded).');
  console.log(`Marker rows: ${summary.marker_rows}; unique standard rsIDs: ${summary.unique_standard_rsids}; duplicate standard rsIDs: ${summary.duplicate_standard_rsids}; duplicate rows: ${summary.duplicate_standard_rows} (${summary.duplicate_standard_excess_rows} extra rows).`);
  console.log(`Copy entries: ${summary.copy.candidate_entries}; repeated visible phrases: ${summary.copy.repeated_visible_phrases.repeated_values}; generic interpretation phrases: ${summary.copy.generic_interpretation_phrases.repeated_values}.`);
  console.log(`Simple meaning: authored=${summary.plain_meaning.authored}; fallback=${summary.plain_meaning.fallback}; missing=${summary.plain_meaning.missing}; translated standard rsIDs=${summary.plain_meaning.translated_standard_rsids}.`);
  console.log(`Simple copy contract: ${summary.simple_copy_contract.fully_structured_entries}/${summary.simple_copy_contract.authored_entries} fully structured; vague=${summary.simple_copy_contract.vague_entries}; overlong=${summary.simple_copy_contract.overlong_entries}.`);
  console.log(`Findings without action: ${summary.findings_without_action}; recommendations: ${summary.recommendations.total}; rules without marker/gene basis: ${summary.recommendations.rules_without_marker_basis}.`);
  console.log(`Topics: ${summary.topics.unique} unique; repeated topic labels: ${summary.topics.repeated_values}.`);
  console.log(`References: ${summary.references.registered_source_ids} registered IDs checked across ${summary.references.checked_arrays} arrays; invalid IDs: ${summary.references.invalid_ids}.`);
  console.log(`Multi-marker groups: ${summary.multi_marker.groups}; duplicate references: ${summary.multi_marker.duplicate_references}; link/count mismatches: ${summary.multi_marker.link_count_mismatches}.`);
  for (const pack of summary.pack_summaries) {
    console.log(`  ${pack.id}: rows=${pack.marker_rows} unique_rsids=${pack.unique_standard_rsids} duplicate_rows=${pack.duplicate_standard_rows} fallback_meaning=${pack.plain_meaning.fallback} missing_meaning=${pack.plain_meaning.missing} missing_action=${pack.findings_without_action}`);
  }
  for (const warning of summary.warnings) console.log(`WARN: ${warning}`);
  for (const error of summary.errors) console.error(`ERROR: ${error}`);
  console.log(summary.errors.length > 0
    ? 'Content-quality audit failed; resolve structural content gaps before marking Task 01 complete.'
    : 'Content-quality audit passed; review aggregate findings before changing content.');
}

process.exitCode = summary.errors.length > 0 ? 1 : 0;
