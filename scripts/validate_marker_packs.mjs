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
const foodRequirementPrompts = readJson(path.join(sourceDir, 'food_requirement_prompts.json'));
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
  if (rule.marker_ids !== undefined && (!isStringArray(rule.marker_ids) || rule.marker_ids.length === 0)) {
    errors.push(`actionability rule ${rule.id || '(unnamed)'}: marker_ids must be a non-empty string array when present`);
  }
}
const conditionalPromptIds = new Set(Object.keys(foodRequirementPrompts?.conditional_prompts || {}));
const conditionalPromptSignalIds = new Set(Object.keys(foodRequirementPrompts?.conditional_prompt_signals || {}));
for (const promptId of conditionalPromptIds) {
  const signals = foodRequirementPrompts?.conditional_prompt_signals?.[promptId];
  if (!Array.isArray(signals) || signals.length === 0 || !signals.every((signal) => typeof signal === 'string' && signal.trim() !== '')) {
    errors.push(`food_requirement_prompts.json: ${promptId} needs non-empty conditional_prompt_signals`);
  }
}
for (const promptId of conditionalPromptSignalIds) {
  if (!conditionalPromptIds.has(promptId)) {
    errors.push(`food_requirement_prompts.json: conditional_prompt_signals references unknown prompt ${promptId}`);
  }
}

// Support resources are not marker packs and are intentionally source-only.
// Validate their public shape here so prompt wiring cannot silently drift when
// a resource is expanded or renamed.
const supportContracts = {
  actionability_guidance: { arrays: ['rules', 'lab_categories', 'lab_tiers'], objects: ['policy', 'lab_confirmation_filter'] },
  ai_prompt_helpers: { arrays: ['helpers'] },
  activity_guardrails: { arrays: ['principles', 'stop_and_escalate', 'domains', 'sources'] },
  callability_rules: { arrays: ['rules'] },
  consultation_modes: { arrays: ['modes'] },
  cycle_support_guidance: { arrays: ['context_keywords', 'context_options', 'principles', 'domains', 'do_not_do'], objects: ['marker_contexts'] },
  diet_pattern_profiles: { arrays: ['profiles'] },
  dietary_requirements: { arrays: ['priority_order', 'rules'] },
  evidence_policy: { objects: ['tiers', 'claim_policy'] },
  food_nutrient_matrix: { arrays: ['food_groups', 'sources'] },
  food_requirement_prompts: { arrays: ['global_first_run_questions'], objects: ['conditional_prompts', 'conditional_prompt_signals'] },
  lab_overlays: { arrays: ['overlays'] },
  meal_planning_rules: { arrays: ['decision_pipeline', 'do_not_do'], objects: ['priority_weights'] },
  phenotype_prompts: { arrays: ['domains'] },
  prs_registry: { arrays: ['prs_modules'] },
  research_taxonomy: { arrays: ['categories', 'discovery_categories'] },
  safety_guardrails: { arrays: ['rules'], objects: ['medication_context'], nested_arrays: { medication_context: ['ask_for', 'do_not_do', 'pgx_context_keywords', 'hormone_context_keywords', 'hormone_medication_keywords', 'contraceptive_medication_keywords'] } },
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
  if (resourceId === 'supplement_safety') {
    for (const [index, rule] of (resource.rules || []).entries()) {
      const location = `supplement_safety.json rule ${index + 1}`;
      for (const field of ['id', 'label']) {
        if (typeof rule[field] !== 'string' || rule[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['signal_genes', 'match_terms', 'avoid', 'confirm_with', 'sources']) {
        if (!isStringArray(rule[field])) errors.push(`${location}: ${field} must be a string array`);
      }
      for (const field of ['context_ids', 'medication_terms', 'allergy_terms']) {
        if (rule[field] !== undefined && !isStringArray(rule[field])) {
          errors.push(`${location}: optional ${field} must be a string array`);
        }
      }
    }
  }
  if (resourceId === 'research_taxonomy') {
    for (const [index, category] of (resource.categories || []).entries()) {
      const location = `research_taxonomy.json category ${index + 1}`;
      for (const field of ['id', 'label', 'query_hint']) {
        if (typeof category[field] !== 'string' || category[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['keywords', 'packs', 'modes']) {
        if (!isStringArray(category[field]) || category[field].length === 0) {
          errors.push(`${location}: ${field} must be a non-empty string array`);
        }
      }
    }
    const discoveryCategories = resource.discovery_categories || [];
    const discoveryIds = new Set();
    for (const [index, category] of discoveryCategories.entries()) {
      const location = `research_taxonomy.json discovery category ${index + 1}`;
      for (const field of ['id', 'label']) {
        if (typeof category[field] !== 'string' || category[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      if (!Number.isInteger(category.order) || category.order < 0) {
        errors.push(`${location}: order must be a non-negative integer`);
      }
      if (typeof category.default_selected !== 'boolean') {
        errors.push(`${location}: default_selected must be boolean`);
      }
      if (discoveryIds.has(category.id)) errors.push(`${location}: duplicate id ${category.id}`);
      discoveryIds.add(category.id);
    }
  }
  if (resourceId === 'actionability_guidance') {
    const tiers = resource.lab_tiers || [];
    const tierIds = new Set();
    for (const [index, tier] of tiers.entries()) {
      const location = `actionability_guidance.json lab tier ${index + 1}`;
      for (const field of ['id', 'label', 'hint']) {
        if (typeof tier[field] !== 'string' || tier[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      if (!Number.isInteger(tier.order) || tier.order < 0) errors.push(`${location}: order must be a non-negative integer`);
      if (tierIds.has(tier.id)) errors.push(`${location}: duplicate id ${tier.id}`);
      tierIds.add(tier.id);
    }
    for (const requiredId of ['counselor', 'discuss', 'optional']) {
      if (!tierIds.has(requiredId)) errors.push(`actionability_guidance.json lab_tiers: missing ${requiredId}`);
    }
    for (const field of ['allowed_keywords', 'blocked_keywords']) {
      if (!isStringArray(resource.lab_confirmation_filter?.[field]) || resource.lab_confirmation_filter[field].length === 0) {
        errors.push(`actionability_guidance.json: lab_confirmation_filter.${field} must be a non-empty string array`);
      }
    }
  }
  if (resourceId === 'actionability_guidance') {
    const labCategoryIds = new Set();
    for (const [index, category] of (resource.lab_categories || []).entries()) {
      const location = `actionability_guidance.json lab category ${index + 1}`;
      for (const field of ['id', 'label']) {
        if (typeof category[field] !== 'string' || category[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      if (!Number.isInteger(category.order) || category.order < 0) {
        errors.push(`${location}: order must be a non-negative integer`);
      }
      if (!isStringArray(category.keywords)) errors.push(`${location}: keywords must be a string array`);
      if (category.fallback !== undefined && typeof category.fallback !== 'boolean') {
        errors.push(`${location}: fallback must be boolean when provided`);
      }
      if (labCategoryIds.has(category.id)) errors.push(`${location}: duplicate id ${category.id}`);
      labCategoryIds.add(category.id);
    }
    if (!(resource.lab_categories || []).some((category) => category.fallback === true)) {
      errors.push('actionability_guidance.json: lab_categories requires a fallback category');
    }
  }
  if (resourceId === 'consultation_modes') {
    const modeIds = new Set();
    for (const [index, mode] of (resource.modes || []).entries()) {
      const location = `consultation_modes.json mode ${index + 1}`;
      for (const field of ['id', 'label', 'icon', 'instructions']) {
        if (typeof mode[field] !== 'string' || mode[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      if (mode.pack_id !== undefined && mode.pack_id !== null
        && (typeof mode.pack_id !== 'string' || !manifestIds.has(mode.pack_id))) {
        errors.push(`${location}: pack_id must reference a manifest pack when provided`);
      }
      if (modeIds.has(mode.id)) errors.push(`${location}: duplicate id ${mode.id}`);
      modeIds.add(mode.id);
    }
    for (const requiredId of ['general', 'hormones_reproductive']) {
      if (!modeIds.has(requiredId)) errors.push(`consultation_modes.json: missing required mode ${requiredId}`);
    }
  }
  if (resourceId === 'ai_prompt_helpers') {
    const helperIds = new Set();
    let fallbackCount = 0;
    for (const [index, helper] of (resource.helpers || []).entries()) {
      const location = `ai_prompt_helpers.json helper ${index + 1}`;
      for (const field of ['id', 'label', 'text']) {
        if (typeof helper[field] !== 'string' || helper[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['pack_ids', 'gene_symbols', 'rsids']) {
        if (helper[field] !== undefined && !isStringArray(helper[field])) {
          errors.push(`${location}: optional ${field} must be a string array`);
        }
      }
      for (const packId of helper.pack_ids || []) {
        if (!manifestIds.has(packId)) errors.push(`${location}: pack_ids references unknown manifest pack ${packId}`);
      }
      for (const field of ['requires_clinical_confirmation', 'fallback']) {
        if (helper[field] !== undefined && typeof helper[field] !== 'boolean') {
          errors.push(`${location}: optional ${field} must be boolean when provided`);
        }
      }
      const hasSignal = (helper.pack_ids || []).length > 0
        || (helper.gene_symbols || []).length > 0
        || (helper.rsids || []).length > 0
        || helper.requires_clinical_confirmation === true
        || helper.fallback === true;
      if (!hasSignal) errors.push(`${location}: requires at least one relevance signal or fallback=true`);
      if (helper.fallback === true) fallbackCount += 1;
      if (helperIds.has(helper.id)) errors.push(`${location}: duplicate id ${helper.id}`);
      helperIds.add(helper.id);
    }
    if (fallbackCount !== 1) errors.push(`ai_prompt_helpers.json: requires exactly one fallback helper`);
    for (const requiredId of ['hormones_reproductive', 'food_supplement_safety', 'activity_recovery']) {
      if (!helperIds.has(requiredId)) errors.push(`ai_prompt_helpers.json: missing required helper ${requiredId}`);
    }
  }
  if (resourceId === 'lab_overlays') {
    for (const [index, overlay] of (resource.overlays || []).entries()) {
      const location = `lab_overlays.json overlay ${index + 1}`;
      if (typeof overlay.domain !== 'string' || overlay.domain.trim() === '') {
        errors.push(`${location}: domain must be a non-empty string`);
      }
      if (!isStringArray(overlay.markers_or_packs) || overlay.markers_or_packs.length === 0) {
        errors.push(`${location}: markers_or_packs must be a non-empty string array`);
      }
      const hasLabs = isStringArray(overlay.labs) && overlay.labs.length > 0;
      const hasConfirmations = isStringArray(overlay.confirmations) && overlay.confirmations.length > 0;
      if (!hasLabs && !hasConfirmations) {
        errors.push(`${location}: labs or confirmations must be a non-empty string array`);
      }
      if (Object.hasOwn(overlay, 'labs_tests')) {
        errors.push(`${location}: use labs, not labs_tests`);
      }
    }
  }
}

const cycleSupport = readJson(path.join(sourceDir, 'cycle_support_guidance.json'));
const safetyGuardrails = readJson(path.join(sourceDir, 'safety_guardrails.json'));
const cycleDomainIds = new Set((cycleSupport?.domains || []).map((domain) => domain.id));
const reproductiveContextIds = new Set((cycleSupport?.context_options || []).map((option) => option.id));
const safetyRuleIds = new Set((safetyGuardrails?.rules || []).map((rule) => rule.id));
const supplementSafety = readJson(path.join(sourceDir, 'supplement_safety.json'));
for (const [index, rule] of (supplementSafety?.rules || []).entries()) {
  for (const contextId of rule.context_ids || []) {
    if (!reproductiveContextIds.has(contextId)) {
      errors.push(`supplement_safety.json rule ${index + 1} references unknown reproductive context ${contextId}`);
    }
  }
}
for (const option of cycleSupport?.context_options || []) {
  if (!isStringArray(option.domain_ids)) {
    errors.push(`cycle_support_guidance.json context ${option.id || '(unnamed)'}: domain_ids must be a string array`);
  } else {
    for (const domainId of option.domain_ids) {
      if (!cycleDomainIds.has(domainId)) {
        errors.push(`cycle_support_guidance.json context ${option.id || '(unnamed)'} references unknown domain ${domainId}`);
      }
    }
  }
  if (!isStringArray(option.medication_rule_ids)) {
    errors.push(`cycle_support_guidance.json context ${option.id || '(unnamed)'}: medication_rule_ids must be a string array`);
  } else {
    for (const ruleId of option.medication_rule_ids) {
      if (!safetyRuleIds.has(ruleId)) {
        errors.push(`cycle_support_guidance.json context ${option.id || '(unnamed)'} references unknown medication rule ${ruleId}`);
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
