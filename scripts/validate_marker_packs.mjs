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
const curatedStandardRsids = new Set();
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
      if (typeof marker.rsid === 'string' && /^rs\d+$/i.test(marker.rsid.trim())) {
        curatedStandardRsids.add(marker.rsid.trim().toLowerCase());
      }
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
const sourceRegistry = readJson(path.join(sourceDir, 'source_registry.json'));
const cycleSupport = readJson(path.join(sourceDir, 'cycle_support_guidance.json'));
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
  agent_research_prompts: { objects: ['templates'] },
  ai_prompt_policy: { arrays: ['default_instructions', 'payload_rules', 'forbidden_actions'], objects: ['safety_review_prompt'] },
  activity_guardrails: { arrays: ['principles', 'stop_and_escalate', 'domains', 'sources'] },
  callability_rules: { arrays: ['rules'] },
  consultation_modes: { arrays: ['modes'] },
  cycle_support_guidance: { arrays: ['context_keywords', 'context_options', 'principles', 'domains', 'do_not_do', 'evidence_layers'], objects: ['marker_contexts', 'intake_schema', 'diary_schema', 'review_schema'] },
  diet_pattern_profiles: { arrays: ['profiles'] },
  dietary_requirements: { arrays: ['priority_order', 'rules'] },
  evidence_policy: { objects: ['tiers', 'claim_policy', 'display'] },
  food_nutrient_matrix: { arrays: ['food_groups', 'sources'] },
  food_requirement_prompts: { arrays: ['global_first_run_questions'], objects: ['conditional_prompts', 'conditional_prompt_signals'] },
  lab_overlays: { arrays: ['overlays'] },
  layperson_translations: { arrays: ['translations'], objects: ['fallback'] },
  meal_planning_rules: { arrays: ['decision_pipeline', 'do_not_do'], objects: ['priority_weights'] },
  phenotype_prompts: { arrays: ['domains'] },
  pgx_diplotype_guidance: { arrays: ['genes', 'do_not_do'], objects: ['policy'] },
  prs_registry: { arrays: ['prs_modules'] },
  research_taxonomy: { arrays: ['categories', 'discovery_categories'] },
  safety_guardrails: { arrays: ['rules'], objects: ['medication_context', 'personal_context_notes'], nested_arrays: { medication_context: ['base_rule_ids', 'pgx_rule_ids', 'reproductive_intake_field_ids', 'ask_for', 'do_not_do', 'pgx_context_keywords', 'hormone_context_keywords', 'hormone_medication_keywords', 'contraceptive_medication_keywords'] } },
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
    if (!Number.isInteger(resource.policy?.confirm_with_cap) || resource.policy.confirm_with_cap < 1 || resource.policy.confirm_with_cap > 100) {
      errors.push('actionability_guidance.json: policy.confirm_with_cap must be an integer from 1 through 100');
    }
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
  if (resourceId === 'phenotype_prompts') {
    const domainIds = new Set();
    const registeredSourceIds = new Set(Object.keys(sourceRegistry?.sources || {}));
    for (const [index, domain] of (resource.domains || []).entries()) {
      const location = `phenotype_prompts.json domain ${index + 1}`;
      for (const field of ['id']) {
        if (typeof domain[field] !== 'string' || domain[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['trigger', 'questions', 'red_flags', 'confirmations', 'sources']) {
        if (!isStringArray(domain[field]) || domain[field].length === 0) {
          errors.push(`${location}: ${field} must be a non-empty string array`);
        }
      }
      for (const sourceId of domain.sources || []) {
        if (!registeredSourceIds.has(sourceId)) {
          errors.push(`${location}: sources references unknown source ${sourceId}`);
        }
      }
      if (domainIds.has(domain.id)) errors.push(`${location}: duplicate id ${domain.id}`);
      domainIds.add(domain.id);
    }
  }
  if (resourceId === 'safety_guardrails') {
    const medicationRuleIds = new Set((resource.rules || []).map((rule) => rule.id));
    for (const field of ['base_rule_ids', 'pgx_rule_ids']) {
      for (const ruleId of resource.medication_context?.[field] || []) {
        if (!medicationRuleIds.has(ruleId)) {
          errors.push(`safety_guardrails.json: medication_context.${field} references unknown rule ${ruleId}`);
        }
      }
    }
    const intakeFieldIds = new Set((cycleSupport?.intake_schema?.fields || []).map((field) => field.id));
    for (const fieldId of resource.medication_context?.reproductive_intake_field_ids || []) {
      if (!intakeFieldIds.has(fieldId)) {
        errors.push(`safety_guardrails.json: medication_context.reproductive_intake_field_ids references unknown intake field ${fieldId}`);
      }
    }
    for (const field of ['allergies', 'medications', 'supplements', 'symptoms', 'labs', 'reproductive_intake', 'cycle_diary']) {
      if (typeof resource.personal_context_notes?.[field] !== 'string' || resource.personal_context_notes[field].trim() === '') {
        errors.push(`safety_guardrails.json: personal_context_notes.${field} must be a non-empty string`);
      }
    }
  }
  if (resourceId === 'cycle_support_guidance') {
    const contextIds = new Set((resource.context_options || []).map((option) => option.id));
    const domainIds = new Set((resource.domains || []).map((domain) => domain.id));
    const intakeFieldIds = new Set((resource.intake_schema?.fields || []).map((field) => field.id));
    const checkContextIds = (values, location) => {
      if (values === undefined) return;
      if (!isStringArray(values)) {
        errors.push(`${location}: context_ids must be a string array when present`);
        return;
      }
      for (const contextId of values) {
        if (!contextIds.has(contextId)) errors.push(`${location}: references unknown context ${contextId}`);
      }
    };
    for (const [index, option] of (resource.context_options || []).entries()) {
      const location = `cycle_support_guidance.json context option ${index + 1}`;
      for (const field of ['id', 'label', 'description']) {
        if (typeof option[field] !== 'string' || option[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['domain_ids', 'medication_rule_ids']) {
        if (!isStringArray(option[field])) errors.push(`${location}: ${field} must be a string array`);
      }
      if (option.id !== 'none_or_unknown' && (!isStringArray(option.profile_keywords) || option.profile_keywords.length === 0 || option.profile_keywords.some((keyword) => keyword.trim() === ''))) {
        errors.push(`${location}: profile_keywords must be a non-empty string array for routable contexts`);
      }
      for (const domainId of option.domain_ids || []) {
        if (!domainIds.has(domainId)) errors.push(`${location}: references unknown domain ${domainId}`);
      }
    }
    for (const [index, group] of (resource.intake_schema?.groups || []).entries()) {
      const location = `cycle_support_guidance.json intake group ${index + 1}`;
      for (const field of ['id', 'title', 'description']) {
        if (typeof group[field] !== 'string' || group[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      if (!isStringArray(group.field_ids)) errors.push(`${location}: field_ids must be a string array`);
      for (const fieldId of group.field_ids || []) {
        if (!intakeFieldIds.has(fieldId)) errors.push(`${location}: references unknown intake field ${fieldId}`);
      }
      checkContextIds(group.context_ids, location);
    }
    checkContextIds(resource.diary_schema?.context_ids, 'cycle_support_guidance.json diary_schema');
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
  if (resourceId === 'ai_prompt_policy') {
    for (const field of ['default_instructions', 'payload_rules', 'forbidden_actions']) {
      if (!isStringArray(resource[field]) || resource[field].length === 0) {
        errors.push(`ai_prompt_policy.json: ${field} must be a non-empty string array`);
      }
    }
    if (typeof resource.date_warning !== 'string' || resource.date_warning.trim() === '') {
      errors.push('ai_prompt_policy.json: date_warning must be a non-empty string');
    }
    const safetyReview = resource.safety_review_prompt;
    if (!isStringArray(safetyReview?.required_placeholders) || safetyReview.required_placeholders.length === 0) {
      errors.push('ai_prompt_policy.json: safety_review_prompt.required_placeholders must be a non-empty string array');
    }
    if (typeof safetyReview?.template !== 'string' || safetyReview.template.trim() === '') {
      errors.push('ai_prompt_policy.json: safety_review_prompt.template must be a non-empty string');
    }
    if (typeof safetyReview?.template === 'string') {
      for (const placeholder of safetyReview.required_placeholders || []) {
        if (!safetyReview.template.includes(`{{${placeholder}}}`)) {
          errors.push(`ai_prompt_policy.json: safety_review_prompt.template missing placeholder {{${placeholder}}}`);
        }
      }
    }
  }
  if (resourceId === 'agent_research_prompts') {
    for (const templateId of ['critique', 'synthesis', 'validation']) {
      const prompt = resource.templates?.[templateId];
      const location = `agent_research_prompts.json templates.${templateId}`;
      if (!prompt || typeof prompt !== 'object' || Array.isArray(prompt)) {
        errors.push(`${location} must be an object`);
        continue;
      }
      if (!isStringArray(prompt.required_placeholders) || prompt.required_placeholders.length === 0) {
        errors.push(`${location}.required_placeholders must be a non-empty string array`);
      }
      if (typeof prompt.template !== 'string' || prompt.template.trim() === '') {
        errors.push(`${location}.template must be a non-empty string`);
        continue;
      }
      for (const placeholder of prompt.required_placeholders || []) {
        if (!prompt.template.includes(`{{${placeholder}}}`)) {
          errors.push(`${location}.template missing placeholder {{${placeholder}}}`);
        }
      }
    }
  }
  if (resourceId === 'cycle_support_guidance') {
    const schema = resource.intake_schema;
    const fields = schema?.fields || [];
    const fieldIds = new Set();
    const allowedInputTypes = new Set(['date', 'number', 'text', 'textarea']);
    for (const [index, field] of fields.entries()) {
      const location = `cycle_support_guidance.json intake field ${index + 1}`;
      for (const key of ['id', 'label', 'input_type', 'help']) {
        if (typeof field[key] !== 'string' || field[key].trim() === '') {
          errors.push(`${location}: ${key} must be a non-empty string`);
        }
      }
      if (!allowedInputTypes.has(field.input_type)) {
        errors.push(`${location}: unsupported input_type ${field.input_type}`);
      }
      if (fieldIds.has(field.id)) errors.push(`${location}: duplicate id ${field.id}`);
      fieldIds.add(field.id);
    }
    const groupIds = new Set();
    const contextIds = new Set((resource.context_options || []).map((option) => option.id));

    const evidenceLayerIds = new Set();
    for (const [index, layer] of (resource.evidence_layers || []).entries()) {
      const location = `cycle_support_guidance.json evidence layer ${index + 1}`;
      for (const key of ['id', 'title', 'summary', 'next_step']) {
        if (typeof layer[key] !== 'string' || layer[key].trim() === '') {
          errors.push(`${location}: ${key} must be a non-empty string`);
        }
      }
      for (const field of ['context_ids', 'do_not_claim', 'sources']) {
        if (!isStringArray(layer[field]) || layer[field].length === 0) {
          errors.push(`${location}: ${field} must be a non-empty string array`);
        }
      }
      if (layer.marker_pack_ids !== undefined && !isStringArray(layer.marker_pack_ids)) {
        errors.push(`${location}: marker_pack_ids must be a string array when provided`);
      }
      for (const contextId of layer.context_ids || []) {
        if (!contextIds.has(contextId)) errors.push(`${location}: context_ids references unknown context ${contextId}`);
      }
      if (evidenceLayerIds.has(layer.id)) errors.push(`${location}: duplicate id ${layer.id}`);
      evidenceLayerIds.add(layer.id);
    }
    if (evidenceLayerIds.size === 0) {
      errors.push('cycle_support_guidance.json: evidence_layers must contain at least one layer');
    }

    for (const [index, group] of (schema?.groups || []).entries()) {
      const location = `cycle_support_guidance.json intake group ${index + 1}`;
      for (const key of ['id', 'title', 'description']) {
        if (typeof group[key] !== 'string' || group[key].trim() === '') {
          errors.push(`${location}: ${key} must be a non-empty string`);
        }
      }
      if (!isStringArray(group.field_ids) || group.field_ids.length === 0) {
        errors.push(`${location}: field_ids must be a non-empty string array`);
      }
      if (group.context_ids !== undefined) {
        if (!isStringArray(group.context_ids) || group.context_ids.length === 0) {
          errors.push(`${location}: context_ids must be a non-empty string array when provided`);
        }
        for (const contextId of group.context_ids || []) {
          if (!contextIds.has(contextId)) errors.push(`${location}: context_ids references unknown context ${contextId}`);
        }
      }
      for (const fieldId of group.field_ids || []) {
        if (!fieldIds.has(fieldId)) errors.push(`${location}: field_ids references unknown field ${fieldId}`);
      }
      if (groupIds.has(group.id)) errors.push(`${location}: duplicate id ${group.id}`);
      groupIds.add(group.id);
    }
    for (const key of ['id', 'title', 'description', 'privacy_note']) {
      if (typeof schema?.[key] !== 'string' || schema[key].trim() === '') {
        errors.push(`cycle_support_guidance.json intake_schema.${key} must be a non-empty string`);
      }
    }
    if (!isStringArray(schema?.do_not_infer) || schema.do_not_infer.length === 0) {
      errors.push('cycle_support_guidance.json intake_schema.do_not_infer must be a non-empty string array');
    }

    const diary = resource.diary_schema;
    const diaryFields = diary?.fields || [];
    const diaryFieldIds = new Set();
    for (const [index, field] of diaryFields.entries()) {
      const location = `cycle_support_guidance.json diary field ${index + 1}`;
      for (const key of ['id', 'label', 'input_type', 'help']) {
        if (typeof field[key] !== 'string' || field[key].trim() === '') {
          errors.push(`${location}: ${key} must be a non-empty string`);
        }
      }
      if (!allowedInputTypes.has(field.input_type)) {
        errors.push(`${location}: unsupported input_type ${field.input_type}`);
      }
      if (diaryFieldIds.has(field.id)) errors.push(`${location}: duplicate id ${field.id}`);
      diaryFieldIds.add(field.id);
      for (const bound of ['min', 'max', 'step']) {
        if (field[bound] !== undefined && (typeof field[bound] !== 'number' || !Number.isFinite(field[bound]))) {
          errors.push(`${location}: ${bound} must be a finite number when provided`);
        }
      }
      if (field.min !== undefined && field.max !== undefined && field.min > field.max) {
        errors.push(`${location}: min cannot exceed max`);
      }
    }
    if (!diaryFieldIds.has('entry_date')) {
      errors.push('cycle_support_guidance.json diary_schema requires an entry_date field');
    }
    if (!Number.isInteger(diary?.retention_limit) || diary.retention_limit < 1) {
      errors.push('cycle_support_guidance.json diary_schema.retention_limit must be a positive integer');
    }
    if (!isStringArray(diary?.context_ids) || diary.context_ids.length === 0) {
      errors.push('cycle_support_guidance.json diary_schema.context_ids must be a non-empty string array');
    } else {
      for (const contextId of diary.context_ids) {
        if (!contextIds.has(contextId)) errors.push(`cycle_support_guidance.json diary_schema.context_ids references unknown context ${contextId}`);
      }
    }
    for (const key of ['id', 'title', 'description', 'privacy_note']) {
      if (typeof diary?.[key] !== 'string' || diary[key].trim() === '') {
        errors.push(`cycle_support_guidance.json diary_schema.${key} must be a non-empty string`);
      }
    }
    if (!isStringArray(diary?.do_not_infer) || diary.do_not_infer.length === 0) {
      errors.push('cycle_support_guidance.json diary_schema.do_not_infer must be a non-empty string array');
    }

    const review = resource.review_schema;
    for (const key of ['id', 'title', 'description']) {
      if (typeof review?.[key] !== 'string' || review[key].trim() === '') {
        errors.push(`cycle_support_guidance.json review_schema.${key} must be a non-empty string`);
      }
    }
    if (!review?.metric_thresholds || typeof review.metric_thresholds !== 'object' || Array.isArray(review.metric_thresholds)) {
      errors.push('cycle_support_guidance.json review_schema.metric_thresholds must be an object');
    } else {
      for (const [key, value] of Object.entries(review.metric_thresholds)) {
        if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) {
          errors.push(`cycle_support_guidance.json review_schema.metric_thresholds.${key} must be a non-negative finite number`);
        }
      }
    }
    if (!Number.isInteger(review?.minimum_observations_for_cycle_day_comparison)
      || review.minimum_observations_for_cycle_day_comparison < 2) {
      errors.push('cycle_support_guidance.json review_schema.minimum_observations_for_cycle_day_comparison must be an integer of at least 2');
    }
    if (!Array.isArray(review?.metrics) || review.metrics.length === 0) {
      errors.push('cycle_support_guidance.json review_schema.metrics must be a non-empty array');
    } else {
      const reviewMetricIds = new Set();
      for (const [index, metric] of review.metrics.entries()) {
        const location = `cycle_support_guidance.json review metric ${index + 1}`;
        for (const key of ['id', 'field_id', 'label', 'threshold_key']) {
          if (typeof metric[key] !== 'string' || metric[key].trim() === '') {
            errors.push(`${location}: ${key} must be a non-empty string`);
          }
        }
        if (reviewMetricIds.has(metric.id)) errors.push(`${location}: duplicate id ${metric.id}`);
        reviewMetricIds.add(metric.id);
        if (!diaryFieldIds.has(metric.field_id)) errors.push(`${location}: references unknown diary field ${metric.field_id}`);
        if (!Object.prototype.hasOwnProperty.call(review.metric_thresholds || {}, metric.threshold_key)) {
          errors.push(`${location}: references unknown metric threshold ${metric.threshold_key}`);
        }
      }
    }
    if (!isStringArray(review?.review_notes) || review.review_notes.length === 0) {
      errors.push('cycle_support_guidance.json review_schema.review_notes must be a non-empty string array');
    }
  }
  if (resourceId === 'pgx_diplotype_guidance') {
    const policy = resource.policy;
    for (const field of ['summary', 'display_rule']) {
      if (typeof policy?.[field] !== 'string' || policy[field].trim() === '') {
        errors.push(`pgx_diplotype_guidance.json: policy.${field} must be a non-empty string`);
      }
    }
    for (const field of ['required_inputs', 'do_not_claim']) {
      if (!isStringArray(policy?.[field]) || policy[field].length === 0) {
        errors.push(`pgx_diplotype_guidance.json: policy.${field} must be a non-empty string array`);
      }
    }
    const geneIds = new Set();
    for (const [index, gene] of (resource.genes || []).entries()) {
      const location = `pgx_diplotype_guidance.json gene ${index + 1}`;
      for (const field of ['id', 'label', 'limitation', 'clinical_next_step']) {
        if (typeof gene[field] !== 'string' || gene[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      for (const field of ['gene_symbols', 'marker_ids', 'required_inputs', 'sources']) {
        if (!isStringArray(gene[field]) || gene[field].length === 0) {
          errors.push(`${location}: ${field} must be a non-empty string array`);
        }
      }
      for (const sourceId of gene.sources || []) {
        if (!sourceRegistry?.sources?.[sourceId]) {
          errors.push(`${location}: unregistered source ${sourceId}`);
        }
      }
      if (geneIds.has(gene.id)) errors.push(`${location}: duplicate id ${gene.id}`);
      geneIds.add(gene.id);
    }
    if (geneIds.size === 0) errors.push('pgx_diplotype_guidance.json: genes must contain at least one gene group');
    if (!isStringArray(resource.do_not_do) || resource.do_not_do.length === 0) {
      errors.push('pgx_diplotype_guidance.json: do_not_do must be a non-empty string array');
    }
  }
  if (resourceId === 'evidence_policy') {
    const display = resource.display;
    if (!display || typeof display !== 'object' || Array.isArray(display)) {
      errors.push('evidence_policy.json: display must be an object');
    } else {
      const displayTextFields = ['label', 'description', 'color_class', 'confidence_label'];
      const tierKeys = ['A', 'B', 'C', 'D', 'E'];
      for (const key of ['unknown_tier', ...tierKeys]) {
        const tier = key === 'unknown_tier' ? display.unknown_tier : display.tiers?.[key];
        for (const field of displayTextFields) {
          if (typeof tier?.[field] !== 'string' || tier[field].trim() === '') {
            errors.push(`evidence_policy.json: display.${key}.${field} must be a non-empty string`);
          }
        }
      }
      const claimFrameKeys = ['interpretation_blocked', 'clinical_confirmation', 'B', 'C', 'D', 'E', 'unknown'];
      for (const key of claimFrameKeys) {
        if (typeof display.claim_frames?.[key] !== 'string' || display.claim_frames[key].trim() === '') {
          errors.push(`evidence_policy.json: display.claim_frames.${key} must be a non-empty string`);
        }
      }
      for (const key of ['risk', 'protective', 'context_dependent', 'trait', 'not_applicable', 'no_claim', 'unknown']) {
        const direction = display.directions?.[key];
        for (const field of ['label', 'plain_label', 'description', 'color_class']) {
          if (typeof direction?.[field] !== 'string' || direction[field].trim() === '') {
            errors.push(`evidence_policy.json: display.directions.${key}.${field} must be a non-empty string`);
          }
        }
      }
      for (const key of ['high_risk', 'moderate_risk', 'low_risk', 'protective', 'trait', 'context_dependent', 'confirmation_required', 'no_data', 'benign']) {
        const severity = display.severity?.[key];
        for (const field of ['css_class', 'emoji', 'glyph', 'label', 'plain_label', 'legend_label', 'legend_description', 'description']) {
          if (typeof severity?.[field] !== 'string' || severity[field].trim() === '') {
            errors.push(`evidence_policy.json: display.severity.${key}.${field} must be a non-empty string`);
          }
        }
      }
      for (const key of [
        'xx_reproductive',
        'xy_reproductive',
        'x_linked',
        'y_linked',
        'menstrual_cycle_context',
        'ovarian_context',
        'uterine_context',
        'androgen_reproductive_context',
        'all_bodies_preconception_fertility_context',
        'all_bodies_hormone_therapy_context',
        'all_bodies_pregnancy_lactation_context',
      ]) {
        if (typeof display.scope_labels?.[key] !== 'string' || display.scope_labels[key].trim() === '') {
          errors.push(`evidence_policy.json: display.scope_labels.${key} must be a non-empty string`);
        }
      }
    }
  }
  if (resourceId === 'layperson_translations') {
    for (const field of ['simpleImpact', 'simpleMeaning']) {
      if (typeof resource.fallback?.[field] !== 'string' || resource.fallback[field].trim() === '') {
        errors.push(`layperson_translations.json: fallback.${field} must be a non-empty string`);
      }
    }
    const translationIds = new Set();
    for (const [index, translation] of (resource.translations || []).entries()) {
      const location = `layperson_translations.json translation ${index + 1}`;
      if (typeof translation.rsid !== 'string' || !/^rs\d+$/i.test(translation.rsid.trim())) {
        errors.push(`${location}: rsid must be a numeric rsID such as rs12345`);
      }
      for (const field of ['simpleImpact', 'simpleMeaning']) {
        if (typeof translation[field] !== 'string' || translation[field].trim() === '') {
          errors.push(`${location}: ${field} must be a non-empty string`);
        }
      }
      const normalizedRsid = String(translation.rsid || '').trim().toLowerCase();
      if (translationIds.has(normalizedRsid)) errors.push(`${location}: duplicate rsid ${translation.rsid}`);
      if (normalizedRsid && !curatedStandardRsids.has(normalizedRsid)) {
        errors.push(`${location}: rsid ${translation.rsid} is not present in curated marker packs`);
      }
      translationIds.add(normalizedRsid);
    }
    const translatedCount = [...translationIds].filter((rsid) => curatedStandardRsids.has(rsid)).length;
    warnings.push(`layperson_translations.json: ${translatedCount}/${curatedStandardRsids.size} curated standard rsIDs have dedicated plain-English translations`);
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

const safetyGuardrails = readJson(path.join(sourceDir, 'safety_guardrails.json'));
const cycleDomainIds = new Set((cycleSupport?.domains || []).map((domain) => domain.id));
const reproductiveContextIds = new Set((cycleSupport?.context_options || []).map((option) => option.id));
const safetyRuleIds = new Set((safetyGuardrails?.rules || []).map((rule) => rule.id));
const activityGuardrails = readJson(path.join(sourceDir, 'activity_guardrails.json'));
const supplementSafety = readJson(path.join(sourceDir, 'supplement_safety.json'));

const activityDomainIds = new Set();
for (const [index, domain] of (activityGuardrails?.domains || []).entries()) {
  const location = `activity_guardrails.json domain ${index + 1}`;
  for (const field of ['id', 'context']) {
    if (typeof domain[field] !== 'string' || domain[field].trim() === '') {
      errors.push(`${location}: ${field} must be a non-empty string`);
    }
  }
  for (const field of ['section_keywords', 'favor', 'avoid', 'confirm_with', 'personal_context_keywords']) {
    if (!isStringArray(domain[field]) || domain[field].length === 0) {
      errors.push(`${location}: ${field} must be a non-empty string array`);
    }
  }
  for (const field of ['relevant_pack_signals', 'context_ids']) {
    if (domain[field] !== undefined && !isStringArray(domain[field])) {
      errors.push(`${location}: optional ${field} must be a string array`);
    }
  }
  for (const packId of domain.relevant_pack_signals || []) {
    if (!manifestIds.has(packId)) {
      errors.push(`${location}: relevant_pack_signals references unknown manifest pack ${packId}`);
    }
  }
  for (const contextId of domain.context_ids || []) {
    if (!reproductiveContextIds.has(contextId)) {
      errors.push(`${location}: context_ids references unknown reproductive context ${contextId}`);
    }
  }
  if (activityDomainIds.has(domain.id)) errors.push(`${location}: duplicate id ${domain.id}`);
  activityDomainIds.add(domain.id);
}
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
