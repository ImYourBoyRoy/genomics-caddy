#!/usr/bin/env node
/**
 * Audit curated marker and support resources for evidence, claim-boundary,
 * callability, actionability, and source-registry coverage.
 *
 * This is a review tool, not a clinical validity engine. Warnings identify
 * text or coverage that deserves human review; they do not upgrade a finding.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const resourceDir = path.join(root, 'src', 'lib', 'marker-packs');
const runtimeDir = path.join(root, 'src-tauri', 'App', 'Data', 'marker-packs');
const runtimeSummaryPath = path.join(resourceDir, 'runtime_validation_summary.json');
const outputJson = process.argv.includes('--json');

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

function isNonEmptyStringArray(value) {
  return Array.isArray(value) && value.length > 0 && value.every((item) => typeof item === 'string' && item.trim() !== '');
}

function markerGenes(gene) {
  return String(gene || '')
    .split(/[\s/]+/)
    .map((value) => value.trim().toUpperCase())
    .filter(Boolean);
}

function collectStringSourceRefs(value, refs, key = '') {
  if (Array.isArray(value)) {
    if (key === 'sources' && value.every((item) => typeof item === 'string')) {
      for (const item of value) refs.add(item);
      return;
    }
    for (const item of value) collectStringSourceRefs(item, refs, key);
    return;
  }
  if (!value || typeof value !== 'object') return;
  for (const [childKey, childValue] of Object.entries(value)) {
    collectStringSourceRefs(childValue, refs, childKey);
  }
}

function ratio(present, total) {
  return total === 0 ? 0 : Math.round((present / total) * 1000) / 10;
}

const strongClaimPattern = /\b(?:will|always|guarantee(?:s)?|definitive(?:ly)?|causes?|results?\s+in|leads?\s+to|master\s+regulator|primary\s+(?:enzyme|transport|protein|regulator|driver|locus|marker|flag)|is\s+responsible\s+for|determines?|defines?|drives?|explains?|eliminates?|complete\s+deficiency|most\s+robust|strongly\s+modifiable)\b/i;
const qualifiedClaimPattern = /\b(?:may|might|could|can|associated\s+with|linked\s+to|contribute(?:s)?\s+to|helps?|important|major|one\s+of|among|some\s+(?:studies|cohorts|populations|models)|in\s+some|often|sometimes|typically|usually|generally|var(?:y|ies|iation)|depends?|requires?|consistent\s+with|little\s+or\s+no|rather\s+than|not|cannot|does\s+not|do\s+not|no\s+longer|research(?:-grade)?|context)\b/i;

function sentences(text) {
  return String(text || '').match(/[^.!?]+[.!?]?/g) || [];
}

function hasUnqualifiedStrongLanguage(text) {
  return sentences(text).some((sentence) => {
    if (!strongClaimPattern.test(sentence) || qualifiedClaimPattern.test(sentence)) return false;

    // Allele/isoform naming is a molecular annotation, not a claim that a
    // person's health outcome is determined. Keep those exact definitions out
    // of the health-language review queue when no phenotype is asserted.
    if (/\b(?:determines?|defines?)\b/i.test(sentence)
      && /\b(?:allele|haplotype|isoform|star\s+allele)\b/i.test(sentence)
      && !/\b(?:risk|phenotype|disease|symptom|activity|clearance|level|susceptibility)\b/i.test(sentence)) {
      return false;
    }
    return true;
  });
}

const manifest = readJson(path.join(resourceDir, 'manifest.json'));
const runtimeSummary = readJson(runtimeSummaryPath);
const sourceRegistry = readJson(path.join(resourceDir, 'source_registry.json'));
const actionability = readJson(path.join(resourceDir, 'actionability_guidance.json'));
const evidencePolicy = readJson(path.join(resourceDir, 'evidence_policy.json'));
const callabilityRules = readJson(path.join(resourceDir, 'callability_rules.json'));
const cycleSupport = readJson(path.join(resourceDir, 'cycle_support_guidance.json'));
const safetyGuardrails = readJson(path.join(resourceDir, 'safety_guardrails.json'));
const discoveryCatalog = readJson(path.join(resourceDir, 'discovery_catalog.json'));
const runtimeDiscoveryCatalogPath = path.join(runtimeDir, 'discovery_catalog.json');

if (!manifest || !Array.isArray(manifest.packs)) errors.push('manifest.json: packs must be an array');
if (!runtimeSummary || !Array.isArray(runtimeSummary.support_files)) {
  errors.push('runtime_validation_summary.json: support_files must be an array');
}
if (!sourceRegistry?.sources || typeof sourceRegistry.sources !== 'object') {
  errors.push('source_registry.json: sources must be an object');
}
if (!evidencePolicy?.claim_policy || typeof evidencePolicy.claim_policy !== 'object') {
  errors.push('evidence_policy.json: claim_policy is required');
}
if (!Array.isArray(callabilityRules?.rules) || callabilityRules.rules.length === 0) {
  errors.push('callability_rules.json: rules must be a non-empty array');
}
if (!cycleSupport?.marker_contexts || typeof cycleSupport.marker_contexts !== 'object' || Array.isArray(cycleSupport.marker_contexts)) {
  errors.push('cycle_support_guidance.json: marker_contexts must be an object');
}
if (!discoveryCatalog || !Array.isArray(discoveryCatalog.markers)) {
  errors.push('discovery_catalog.json: markers must be an array');
} else if (!fs.existsSync(runtimeDiscoveryCatalogPath)) {
  errors.push('discovery_catalog.json: runtime mirror missing');
} else if (fs.readFileSync(path.join(resourceDir, 'discovery_catalog.json'), 'utf8') !== fs.readFileSync(runtimeDiscoveryCatalogPath, 'utf8')) {
  warnings.push('discovery_catalog: source/runtime content differs');
}

const packSummaries = [];
const allMarkers = [];
const supportRefs = new Set();
const markerFiles = new Set();
const boundaryGaps = [];
const absoluteLanguageReview = [];
const discoveryLanguageReview = [];
const laypersonLanguageReview = [];
const actionabilitySourceGaps = [];

for (const pack of manifest?.packs || []) {
  const packId = pack?.id;
  if (typeof packId !== 'string' || !packId) {
    errors.push('manifest.json: every pack must have a non-empty id');
    continue;
  }
  const sourceFile = path.join(resourceDir, `${packId}.json`);
  const runtimeFile = path.join(runtimeDir, `${packId}.json`);
  markerFiles.add(`${packId}.json`);
  const doc = readJson(sourceFile);
  if (!doc || !Array.isArray(doc.markers)) {
    errors.push(`${packId}.json: markers must be an array`);
    continue;
  }
  if (!fs.existsSync(runtimeFile)) errors.push(`${packId}: runtime mirror missing`);
  else if (fs.readFileSync(sourceFile, 'utf8') !== fs.readFileSync(runtimeFile, 'utf8')) {
    warnings.push(`${packId}: source/runtime content differs`);
  }

  const tiers = {};
  const variants = {};
  let scoped = 0;
  let guardrails = 0;
  let clinicalConfirmation = 0;
  let sourceBacked = 0;
  for (const marker of doc.markers) {
    allMarkers.push({ packId, marker });
    tiers[marker.evidence_tier] = (tiers[marker.evidence_tier] || 0) + 1;
    variants[marker.variant_type || 'missing'] = (variants[marker.variant_type || 'missing'] || 0) + 1;
    if (marker.sex_scope) scoped += 1;
    if (marker.variant_type === 'guardrail') guardrails += 1;
    if (marker.clinical_confirmation_required === true) clinicalConfirmation += 1;
    if (Array.isArray(marker.sources) && marker.sources.length > 0) sourceBacked += 1;

    const missing = ['do_not_claim', 'confirm_with', 'raw_dna_limitation', 'evidence_tier', 'effect_direction']
      .filter((field) => field === 'raw_dna_limitation'
        ? typeof marker[field] !== 'string' || marker[field].trim() === ''
        : field === 'evidence_tier' || field === 'effect_direction'
          ? typeof marker[field] !== 'string' || marker[field].trim() === ''
          : !isNonEmptyStringArray(marker[field]));
    if (marker.sources === undefined || !Array.isArray(marker.sources) || marker.sources.length === 0) missing.push('sources');
    if (marker.clinical_confirmation_required === true && !isNonEmptyStringArray(marker.confirm_with)) {
      missing.push('clinical_confirmation.confirm_with');
    }
    if (missing.length > 0) boundaryGaps.push({ pack: packId, rsid: marker.rsid || '(missing rsid)', missing });

    const interpretation = String(marker.interpretation || '');
    if (marker.variant_type !== 'guardrail' && hasUnqualifiedStrongLanguage(interpretation)) {
      absoluteLanguageReview.push({
        pack: packId,
        rsid: marker.rsid || '(missing rsid)',
        evidence_tier: marker.evidence_tier || '(missing)',
        interpretation,
      });
    }
  }
  packSummaries.push({
    id: packId,
    markers: doc.markers.length,
    source_backed: sourceBacked,
    source_backed_percent: ratio(sourceBacked, doc.markers.length),
    clinical_confirmation: clinicalConfirmation,
    guardrails,
    sex_scoped: scoped,
    evidence_tiers: tiers,
    variant_types: variants,
  });
}

for (const marker of discoveryCatalog?.markers || []) {
  const description = String(marker.description || '');
  if (hasUnqualifiedStrongLanguage(description)) {
    discoveryLanguageReview.push({
      category: marker.category || '(missing)',
      rsid: marker.rsid || '(missing rsid)',
      description,
    });
  }
}

const supportFiles = runtimeSummary?.support_files || [];
for (const fileName of supportFiles) {
  const file = path.join(resourceDir, fileName);
  if (!fs.existsSync(file)) {
    errors.push(`support resource missing: ${fileName}`);
    continue;
  }
  const resource = readJson(file);
  if (resource) collectStringSourceRefs(resource, supportRefs);
}

const registeredSourceIds = new Set(Object.keys(sourceRegistry?.sources || {}));
const missingSourceIds = Array.from(supportRefs).filter((sourceId) => !registeredSourceIds.has(sourceId)).sort();
if (missingSourceIds.length > 0) errors.push(`unregistered support source ids: ${missingSourceIds.join(', ')}`);

const hormonePack = allMarkers.filter(({ packId }) => packId === 'hormones_reproductive');
const hormoneMarkerIds = new Set(hormonePack.map(({ marker }) => marker.rsid));
const contextOptionIds = new Set((cycleSupport?.context_options || []).map((option) => option.id));
const cycleDomainIds = new Set((cycleSupport?.domains || []).map((domain) => domain.id));
const safetyRuleIds = new Set((safetyGuardrails?.rules || []).map((rule) => rule.id));
for (const option of cycleSupport?.context_options || []) {
  for (const domainId of option.domain_ids || []) {
    if (!cycleDomainIds.has(domainId)) {
      errors.push(`cycle_support_guidance.json: context ${option.id} references unknown domain ${domainId}`);
    }
  }
  for (const ruleId of option.medication_rule_ids || []) {
    if (!safetyRuleIds.has(ruleId)) {
      errors.push(`cycle_support_guidance.json: context ${option.id} references unknown medication rule ${ruleId}`);
    }
  }
}
for (const [contextId, markerIds] of Object.entries(cycleSupport?.marker_contexts || {})) {
  if (contextId !== 'shared_reproductive' && !contextOptionIds.has(contextId)) {
    errors.push(`cycle_support_guidance.json: marker_contexts references unknown context ${contextId}`);
  }
  if (!Array.isArray(markerIds)) {
    errors.push(`cycle_support_guidance.json: marker_contexts.${contextId} must be an array`);
    continue;
  }
  for (const markerId of markerIds) {
    if (!hormoneMarkerIds.has(markerId)) {
      errors.push(`cycle_support_guidance.json: marker_contexts.${contextId} references missing hormone marker ${markerId}`);
    }
  }
}

const actionabilityGenes = new Set();
const actionabilityCoverage = [];
const actionabilityMarkerReferenceGaps = [];
for (const rule of actionability?.rules || []) {
  const genes = (rule.genes || []).map((gene) => String(gene).toUpperCase());
  const markerIds = Array.isArray(rule.marker_ids)
    ? rule.marker_ids.map((markerId) => String(markerId).trim().toLowerCase()).filter(Boolean)
    : [];
  genes.forEach((gene) => actionabilityGenes.add(gene));
  if (!isNonEmptyStringArray(rule.sources)) {
    actionabilitySourceGaps.push({ id: rule.id || '(unnamed)', reason: 'sources_required' });
  }
  if ((rule.actionability_class === 'clinical_confirmation' || rule.severity_classes?.includes('confirmation_required'))
    && !isNonEmptyStringArray(rule.sources)) {
    actionabilitySourceGaps.push({ id: rule.id || '(unnamed)', reason: 'clinical_rule_requires_registered_sources' });
  }
  const matchingMarkers = allMarkers.filter(({ marker }) => {
    if (!markerGenes(marker.gene).some((gene) => genes.includes(gene))) return false;
    return markerIds.length === 0 || markerIds.includes(String(marker.rsid || '').trim().toLowerCase());
  });
  if (markerIds.length > 0) {
    const curatedMarkersById = new Map(
      allMarkers.map(({ marker }) => [
        String(marker.rsid || '').trim().toLowerCase(),
        markerGenes(marker.gene),
      ])
    );
    for (const markerId of markerIds) {
      const markerGeneSymbols = curatedMarkersById.get(markerId);
      if (!markerGeneSymbols) {
        actionabilityMarkerReferenceGaps.push({ id: rule.id || '(unnamed)', marker_id: markerId });
      } else if (!markerGeneSymbols.some((gene) => genes.includes(gene))) {
        actionabilityMarkerReferenceGaps.push({ id: rule.id || '(unnamed)', marker_id: markerId, reason: 'gene_mismatch' });
      }
    }
  }
  actionabilityCoverage.push({
    id: rule.id || '(unnamed)',
    genes,
    marker_ids: markerIds,
    marker_matches: matchingMarkers.length,
    matched_packs: Array.from(new Set(matchingMarkers.map(({ packId }) => packId))),
  });
}

const uncoveredActionabilityGenes = Array.from(actionabilityGenes)
  .filter((gene) => !allMarkers.some(({ marker }) => markerGenes(marker.gene).includes(gene)))
  .sort();
if (uncoveredActionabilityGenes.length > 0) {
  warnings.push(`actionability genes without curated markers: ${uncoveredActionabilityGenes.join(', ')}`);
}

const laypersonPath = path.join(root, 'src', 'lib', 'utils', 'layperson.ts');
if (fs.existsSync(laypersonPath)) {
  const laypersonText = fs.readFileSync(laypersonPath, 'utf8');
  const simpleMeaningPattern = /simpleMeaning:\s*"((?:\\.|[^"\\])*)"/g;
  for (const match of laypersonText.matchAll(simpleMeaningPattern)) {
    let meaning = match[1];
    try {
      meaning = JSON.parse(`"${meaning}"`);
    } catch {
      // The TypeScript parser/checker is the authority for syntax; retain the
      // raw text if this lightweight review scanner cannot decode one string.
    }
    if (hasUnqualifiedStrongLanguage(meaning)) laypersonLanguageReview.push(meaning);
  }
  if (laypersonLanguageReview.length > 0) {
    warnings.push(`plain-English interpretations needing wording review: ${laypersonLanguageReview.length}`);
  }
}

const summary = {
  marker_packs: packSummaries.length,
  curated_markers: allMarkers.length,
  support_resources: supportFiles.length,
  registered_sources: registeredSourceIds.size,
  referenced_support_sources: supportRefs.size,
  gates: {
    probability: {
      status: evidencePolicy?.claim_policy ? 'present' : 'failed',
      note: 'Evidence tiers and claim policy are loaded; this audit does not infer personal probability from a marker.',
    },
    callability: {
      status: Array.isArray(callabilityRules?.rules) && callabilityRules.rules.length > 0 ? 'present' : 'failed',
      note: 'Callability rules are loaded; fixture coverage is audited separately.',
    },
    actionability: {
      status: Array.isArray(actionability?.rules) && actionability.rules.length > 0 ? 'present' : 'failed',
      rules: actionabilityCoverage.length,
      marker_matches: actionabilityCoverage.reduce((total, rule) => total + rule.marker_matches, 0),
    },
  },
  boundary_gaps: boundaryGaps,
  absolute_language_review: absoluteLanguageReview,
  discovery_language_review: discoveryLanguageReview,
  layperson_language_review: laypersonLanguageReview,
  actionability_source_gaps: actionabilitySourceGaps,
  actionability_marker_reference_gaps: actionabilityMarkerReferenceGaps,
  uncovered_actionability_genes: uncoveredActionabilityGenes,
  pack_summaries: packSummaries,
  errors,
  warnings,
};

if (outputJson) {
  console.log(JSON.stringify(summary, null, 2));
  process.exit(errors.length > 0 || boundaryGaps.length > 0 || actionabilitySourceGaps.length > 0 || actionabilityMarkerReferenceGaps.length > 0 ? 1 : 0);
} else {
  console.log(`Audited ${summary.marker_packs} marker packs / ${summary.curated_markers} curated markers.`);
  console.log(`Audited ${summary.support_resources} support resources / ${summary.registered_sources} registered sources.`);
  console.log(`Gates: probability=${summary.gates.probability.status} callability=${summary.gates.callability.status} actionability=${summary.gates.actionability.status}`);
  console.log(`Actionability coverage: ${summary.gates.actionability.marker_matches} marker matches across ${summary.gates.actionability.rules} rules.`);
  console.log(`Claim-boundary gaps: ${boundaryGaps.length}; actionability source gaps: ${actionabilitySourceGaps.length}; actionability marker-reference gaps: ${actionabilityMarkerReferenceGaps.length}; marker wording review queue: ${absoluteLanguageReview.length}; discovery wording review queue: ${discoveryLanguageReview.length}; plain-English wording review queue: ${laypersonLanguageReview.length}.`);
  for (const pack of packSummaries) {
    console.log(`  ${pack.id}: markers=${pack.markers} sources=${pack.source_backed_percent}% clinical_confirmation=${pack.clinical_confirmation} guardrails=${pack.guardrails} sex_scoped=${pack.sex_scoped}`);
  }
  for (const warning of warnings) console.log(`WARN: ${warning}`);
  for (const error of errors) console.error(`ERROR: ${error}`);
}

if (errors.length > 0 || boundaryGaps.length > 0 || actionabilitySourceGaps.length > 0 || actionabilityMarkerReferenceGaps.length > 0) process.exit(1);
console.log('Resource-quality audit passed; review warnings before treating coverage as complete.');
