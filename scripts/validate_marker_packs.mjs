#!/usr/bin/env node
/**
 * Validate the curated marker-pack contract and source/runtime parity.
 *
 * This is intentionally stricter than JSON parsing: an unclassified evidence
 * tier or missing claim boundary can silently turn a research hint into an
 * apparent medical conclusion. discovery_catalog.json is a separate catalog
 * format and is validated by its own ingestion path.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sourceDir = path.join(root, 'src', 'lib', 'marker-packs');
const runtimeDir = path.join(root, 'src-tauri', 'App', 'Data', 'marker-packs');
const allowedDirections = new Set([
  'risk',
  'protective',
  'context_dependent',
  'trait',
  'unknown',
  'not_applicable',
  'no_claim',
]);
const allowedActionability = new Set([
  'general_wellness',
  'symptom_or_lab_conditioned',
  'clinical_confirmation',
]);
const errors = [];
const warnings = [];

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(`${path.relative(root, file)}: invalid JSON (${error.message})`);
    return null;
  }
}

function isStringArray(value) {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

const manifestFile = path.join(sourceDir, 'manifest.json');
const manifest = readJson(manifestFile);
const manifestIds = new Set((manifest?.packs || []).map((pack) => pack.id));

if (!manifest || !Array.isArray(manifest.packs)) {
  errors.push('manifest.json: expected a packs array');
}

let markerCount = 0;
let packCount = 0;
for (const fileName of fs.readdirSync(sourceDir).filter((name) => name.endsWith('.json')).sort()) {
  if (fileName === 'discovery_catalog.json') continue;
  const file = path.join(sourceDir, fileName);
  const doc = readJson(file);
  if (!doc) continue;

  if (fileName !== 'manifest.json' && Array.isArray(doc.markers)) {
    packCount += 1;
    for (const [index, marker] of doc.markers.entries()) {
      const location = `${fileName} marker ${index + 1}`;
      markerCount += 1;
      for (const field of ['rsid', 'gene', 'effect_allele', 'impact', 'evidence_tier', 'interpretation', 'effect_direction']) {
        if (typeof marker[field] !== 'string' || marker[field].trim() === '') {
          errors.push(`${location}: missing string field ${field}`);
        }
      }
      if (!allowedDirections.has(marker.effect_direction)) {
        errors.push(`${location} ${marker.rsid || '(unknown)'}: invalid effect_direction ${marker.effect_direction}`);
      }
      if (typeof marker.evidence_tier !== 'string' || !/^[A-E]_/.test(marker.evidence_tier)) {
        errors.push(`${location} ${marker.rsid || '(unknown)'}: evidence_tier must begin with A_, B_, C_, D_, or E_`);
      }
      for (const field of ['do_not_claim', 'confirm_with']) {
        if (!isStringArray(marker[field])) {
          errors.push(`${location} ${marker.rsid || '(unknown)'}: ${field} must be a string array`);
        }
      }
      if (marker.sources !== undefined && marker.sources !== null && !Array.isArray(marker.sources)) {
        errors.push(`${location} ${marker.rsid || '(unknown)'}: sources must be an array when present`);
      }
      if (marker.sex_scope !== undefined && marker.sex_scope !== null && typeof marker.sex_scope !== 'string') {
        errors.push(`${location} ${marker.rsid || '(unknown)'}: sex_scope must be a string when present`);
      }
    }
  }
}

const actionability = readJson(path.join(sourceDir, 'actionability_guidance.json'));
if (!actionability?.policy?.safety_notes?.length) {
  errors.push('actionability_guidance.json: policy.safety_notes is required');
}
for (const rule of actionability?.rules || []) {
  const actionabilityClass = rule.actionability_class || actionability.policy?.default_actionability;
  if (!allowedActionability.has(actionabilityClass)) {
    errors.push(`actionability rule ${rule.id || '(unnamed)'}: invalid actionability_class ${actionabilityClass}`);
  }
  if (!Array.isArray(rule.genes) || rule.genes.length === 0) {
    errors.push(`actionability rule ${rule.id || '(unnamed)'}: genes must be non-empty`);
  }
}

// Support resources are not marker packs and are intentionally source-only.
// Validate their public shape here so prompt wiring cannot silently drift when
// a resource is expanded or renamed.
const supportContracts = {
  actionability_guidance: { arrays: ['rules'], objects: ['policy'] },
  activity_guardrails: { arrays: ['principles', 'stop_and_escalate', 'domains', 'sources'] },
  callability_rules: { arrays: ['rules'] },
  diet_pattern_profiles: { arrays: ['profiles'] },
  dietary_requirements: { arrays: ['priority_order', 'rules'] },
  evidence_policy: { objects: ['tiers', 'claim_policy'] },
  food_nutrient_matrix: { arrays: ['food_groups', 'sources'] },
  food_requirement_prompts: { arrays: ['global_first_run_questions'], objects: ['conditional_prompts'] },
  lab_overlays: { arrays: ['overlays'] },
  meal_planning_rules: { arrays: ['decision_pipeline', 'do_not_do'], objects: ['priority_weights'] },
  phenotype_prompts: { arrays: ['domains'] },
  prs_registry: { arrays: ['prs_modules'] },
  safety_guardrails: { arrays: ['rules'], objects: ['medication_context'], nested_arrays: { medication_context: ['ask_for', 'do_not_do'] } },
  supplement_safety: { arrays: ['principles', 'rules', 'do_not_do'] },
  source_registry: { objects: ['sources'] },
  user_diet_profile_schema: { arrays: ['minimum_required_for_food_advice', 'do_not_infer'], objects: ['schema'] },
};
let supportResourceCount = 0;
for (const [resourceId, contract] of Object.entries(supportContracts)) {
  const resource = readJson(path.join(sourceDir, `${resourceId}.json`));
  if (!resource) continue;
  supportResourceCount += 1;
  for (const field of ['name', 'version', 'last_updated']) {
    if (typeof resource[field] !== 'string' || resource[field].trim() === '') {
      errors.push(`${resourceId}.json: missing string field ${field}`);
    }
  }
  for (const field of contract.arrays || []) {
    if (!Array.isArray(resource[field])) errors.push(`${resourceId}.json: ${field} must be an array`);
  }
  for (const field of contract.objects || []) {
    if (!resource[field] || typeof resource[field] !== 'object' || Array.isArray(resource[field])) {
      errors.push(`${resourceId}.json: ${field} must be an object`);
    }
  }
  for (const [parent, fields] of Object.entries(contract.nested_arrays || {})) {
    for (const field of fields) {
      if (!Array.isArray(resource[parent]?.[field])) {
        errors.push(`${resourceId}.json: ${parent}.${field} must be an array`);
      }
    }
  }
}

for (const id of manifestIds) {
  const sourceFile = path.join(sourceDir, `${id}.json`);
  const runtimeFile = path.join(runtimeDir, `${id}.json`);
  if (!fs.existsSync(sourceFile)) errors.push(`manifest pack ${id}: source file missing`);
  if (!fs.existsSync(runtimeFile)) errors.push(`manifest pack ${id}: runtime mirror missing`);
  if (fs.existsSync(sourceFile) && fs.existsSync(runtimeFile)) {
    const sourceText = fs.readFileSync(sourceFile, 'utf8');
    const runtimeText = fs.readFileSync(runtimeFile, 'utf8');
    if (sourceText !== runtimeText) warnings.push(`${id}: source/runtime content differs`);
  }
}

if (fs.existsSync(path.join(sourceDir, 'manifest.json')) && fs.existsSync(path.join(runtimeDir, 'manifest.json'))) {
  if (fs.readFileSync(path.join(sourceDir, 'manifest.json'), 'utf8') !== fs.readFileSync(path.join(runtimeDir, 'manifest.json'), 'utf8')) {
    errors.push('manifest.json: source/runtime content differs');
  }
}

console.log(`Validated ${packCount} marker packs and ${markerCount} curated markers.`);
console.log(`Validated ${manifestIds.size} manifest runtime mirrors.`);
console.log(`Validated ${supportResourceCount} source support resources.`);
for (const warning of warnings) console.log(`WARN: ${warning}`);
for (const error of errors) console.error(`ERROR: ${error}`);
if (errors.length > 0) process.exit(1);
console.log('Marker-pack validation passed.');
