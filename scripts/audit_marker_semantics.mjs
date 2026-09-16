#!/usr/bin/env node
/**
 * Read-only audit of curated marker assertion semantics.
 *
 * This intentionally reports gaps without guessing corrections. It never
 * reads genotype databases and never prints private calls.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sourceDir = path.join(root, 'src', 'lib', 'marker-packs');
const jsonOutput = process.argv.includes('--json');
const callabilityRules = JSON.parse(fs.readFileSync(path.join(sourceDir, 'callability_rules.json'), 'utf8'));
const callabilityRegistry = callabilityRules.variant_type_registry || {};
const errors = [];

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function classifyEffectAllele(marker) {
  const allele = String(marker.effect_allele || '').trim().toUpperCase();
  const type = String(marker.variant_type || '').trim().toLowerCase();
  if (/^[ACGT]$/.test(allele)) return 'literal_snp_allele';
  if ((type === 'indel' || type === 'deletion') && /^(?:[ACGT]|I|D|-)$/.test(allele)) {
    return 'literal_indel_allele';
  }
  if (/^(?:N\/A|NOT_APPLICABLE|NO_CLAIM)$/i.test(allele) || type.includes('guardrail')) {
    return 'guardrail_no_allele';
  }
  if (type.includes('prs') || type.includes('polygenic') || /(?:POLYGENIC|MULTI[_ ]?LOCUS|SCORE)/.test(allele)) {
    return 'polygenic_score_requirement';
  }
  if (['gene_panel', 'hla_panel', 'hla_tag', 'pgx_panel', 'star_allele', 'haplotype', 'pathway_panel', 'research_panel', 'variant_panel', 'copy_number_variant'].includes(type)
    || /(?:PATHOGENIC|DIPLOTYPE|HLA|PANEL|COPY[_ ]?NUMBER)/.test(allele)) {
    return 'panel_or_diplotype_requirement';
  }
  if (/(?:STUDY|REPORTED|RISK[_ ]?ALLELE|TRAIT[_ ]?ALLELE|AUTOIMMUNE|TSH|B12)/.test(allele)) {
    return 'study_reported_label';
  }
  return 'unknown';
}

function stableSemanticValue(value) {
  if (!value || typeof value !== 'object') return '';
  return JSON.stringify({
    condition_label: String(value.condition_label || '').trim(),
    interpretation_class: String(value.interpretation_class || '').trim(),
    inheritance_model: String(value.inheritance_model || '').trim(),
    clinical_state: String(value.clinical_state || '').trim(),
  });
}

function sourceAssertionValue(marker) {
  if (!Array.isArray(marker.sources)) return '';
  return JSON.stringify(marker.sources.map((source) => ({
    name: String(source?.name || '').trim(),
    url: String(source?.url || '').trim(),
    evidence_type: String(source?.evidence_type || '').trim(),
  })).sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right))));
}

const markerRows = [];
const duplicateMap = new Map();
const variantTypes = new Map();
const evidenceTiers = new Map();

for (const fileName of fs.readdirSync(sourceDir).filter((name) => name.endsWith('.json')).sort()) {
  if (fileName === 'manifest.json' || fileName === 'discovery_catalog.json') continue;
  const file = path.join(sourceDir, fileName);
  const doc = readJson(file);
  if (!Array.isArray(doc.markers)) continue;
  for (const [index, marker] of doc.markers.entries()) {
    const rsid = String(marker.rsid || '').trim();
    const variantType = String(marker.variant_type || 'unspecified').trim() || 'unspecified';
    const effectClass = classifyEffectAllele(marker);
    const variantTypeKey = variantType.toLowerCase();
    const callabilityPolicy = callabilityRegistry[variantTypeKey];
    if (!callabilityPolicy) errors.push(`unregistered variant type: ${variantType}`);
    const row = {
      file: fileName,
      pack: fileName.replace(/\.json$/i, ''),
      index: index + 1,
      rsid,
      marker_id: `${fileName}:${index + 1}:${rsid || 'unidentified'}`,
      variant_type: variantType,
      effect_class: effectClass,
      effect_allele: String(marker.effect_allele || '').trim(),
      effect_direction: String(marker.effect_direction || '').trim(),
      condition_or_trait: String(
        marker.clinical_semantics?.condition_label
          || marker.condition
          || marker.trait
          || '',
      ).trim(),
      clinical_semantics: stableSemanticValue(marker.clinical_semantics),
      source_assertion: sourceAssertionValue(marker),
      evidence_tier: String(marker.evidence_tier || '').trim(),
      clinical_confirmation_required: marker.clinical_confirmation_required === true,
      has_clinical_semantics: Boolean(marker.clinical_semantics),
      has_source_build: Boolean(String(marker.source_build || '').trim()),
      expected_plus_alleles: Array.isArray(marker.expected_plus_alleles)
        ? marker.expected_plus_alleles.map((allele) => String(allele || '').trim().toUpperCase()).filter(Boolean).sort().join(',')
        : '',
      has_orientation_metadata: marker.allele_orientation_verified === true
        && Boolean(String(marker.orientation_source || '').trim()),
      scoring_policy: callabilityPolicy?.scoring_policy || 'unregistered',
      callability_class: callabilityPolicy?.display_policy || 'unregistered',
      scoring_allowed: callabilityPolicy?.scoring_policy === 'snp_allele_count',
    };
    markerRows.push(row);
    variantTypes.set(variantType, (variantTypes.get(variantType) || 0) + 1);
    evidenceTiers.set(row.evidence_tier, (evidenceTiers.get(row.evidence_tier) || 0) + 1);
    if (rsid) {
      const key = rsid.toLowerCase();
      if (!duplicateMap.has(key)) duplicateMap.set(key, []);
      duplicateMap.get(key).push(row);
    }
  }
}

const duplicateConflicts = [];
for (const [rsid, rows] of duplicateMap.entries()) {
  if (rows.length < 2) continue;
  const fields = [
    'effect_allele',
    'variant_type',
    'effect_direction',
    'condition_or_trait',
    'clinical_semantics',
  ];
  const conflicts = fields.filter((field) => new Set(rows.map((row) => String(row[field]))).size > 1);
  if (conflicts.length > 0) {
    const contextFields = ['source_assertion', 'evidence_tier', 'clinical_confirmation_required'];
    duplicateConflicts.push({
      rsid,
      row_count: rows.length,
      fields: conflicts,
      field_values: Object.fromEntries(conflicts.map((field) => [
        field,
        [...new Set(rows.map((row) => String(row[field])))].sort(),
      ])),
      contextual_differences: contextFields.filter((field) =>
        new Set(rows.map((row) => String(row[field]))).size > 1,
      ),
      row_refs: rows.map((row) => ({
        file: row.file,
        index: row.index,
        variant_type: row.variant_type,
        effect_class: row.effect_class,
      })),
    });
  }
}

const matchableSnpRows = markerRows.filter((row) =>
  row.scoring_allowed && ['snp', 'pharmacogenomic_snp'].includes(row.variant_type.toLowerCase()),
);
const strandAmbiguousRows = matchableSnpRows.filter((row) => {
  const plusAlleles = row.expected_plus_alleles || '';
  return plusAlleles === 'A,T' || plusAlleles === 'C,G';
});
const orientationReviewQueue = strandAmbiguousRows
  .filter((row) => !row.has_orientation_metadata)
  .map(({ marker_id, pack, rsid, variant_type }) => ({ marker_id, pack, rsid, variant_type }));
const sourceBuildReviewQueue = matchableSnpRows
  .filter((row) => !row.has_source_build)
  .map(({ marker_id, pack, rsid, variant_type }) => ({ marker_id, pack, rsid, variant_type }));
const expectedAlleleReviewQueue = matchableSnpRows
  .filter((row) => !row.expected_plus_alleles)
  .map(({ marker_id, pack, rsid, variant_type }) => ({ marker_id, pack, rsid, variant_type }));

const summary = {
  generated_at: new Date().toISOString(),
  marker_count: markerRows.length,
  standard_rsid_rows: markerRows.filter((row) => /^rs\d+$/i.test(row.rsid)).length,
  effect_classes: Object.fromEntries([...new Set(markerRows.map((row) => row.effect_class))].sort().map((key) => [
    key,
    markerRows.filter((row) => row.effect_class === key).length,
  ])),
  variant_types: Object.fromEntries([...variantTypes.entries()].sort(([left], [right]) => left.localeCompare(right))),
  evidence_tiers: Object.fromEntries([...evidenceTiers.entries()].sort(([left], [right]) => left.localeCompare(right))),
  callability_policy_counts: Object.fromEntries(
    [...markerRows.reduce((counts, row) => {
      counts.set(row.scoring_policy, (counts.get(row.scoring_policy) || 0) + 1);
      return counts;
    }, new Map()).entries()].sort(([left], [right]) => left.localeCompare(right)),
  ),
  unregistered_variant_types: [...new Set(errors.map((error) => error.replace('unregistered variant type: ', '')))].sort(),
  non_matchable_snp_rows: markerRows.filter((row) =>
    /^rs\d+$/i.test(row.rsid)
      && ['snp', 'pharmacogenomic_snp'].includes(row.variant_type.toLowerCase())
      && !['literal_snp_allele'].includes(row.effect_class),
  ).map(({ file, index, rsid, variant_type, effect_class, effect_allele }) => ({
    file, index, rsid, variant_type, effect_class, effect_allele,
  })),
  marker_snapshot: markerRows.map((row) => ({
    marker_id: row.marker_id,
    pack: row.pack,
    rsid: row.rsid,
    variant_type: row.variant_type,
    evidence_tier: row.evidence_tier,
    effect_class: row.effect_class,
    callability_class: row.callability_class,
    scoring_allowed: row.scoring_allowed,
  })),
  duplicate_rsid_count: [...duplicateMap.values()].filter((rows) => rows.length > 1).length,
  duplicate_conflict_count: duplicateConflicts.length,
  duplicate_conflicts: duplicateConflicts,
  orientation_audit: {
    matchable_snp_rows: matchableSnpRows.length,
    strand_ambiguous_rows: strandAmbiguousRows.length,
    ambiguous_rows_missing_verification: orientationReviewQueue,
    matchable_rows_missing_source_build: sourceBuildReviewQueue,
    matchable_rows_missing_expected_plus_alleles: expectedAlleleReviewQueue,
  },
};

if (jsonOutput) {
  console.log(JSON.stringify(summary, null, 2));
} else {
  console.log(`Audited ${summary.marker_count} curated marker rows.`);
  console.log(`Standard rsID rows: ${summary.standard_rsid_rows}.`);
  console.log(`Non-matchable SNP-like rows: ${summary.non_matchable_snp_rows.length}.`);
  console.log(`Duplicated rsIDs: ${summary.duplicate_rsid_count}; conflicts: ${summary.duplicate_conflict_count}.`);
  console.log(`Orientation/build review: ${summary.orientation_audit.strand_ambiguous_rows} strand-ambiguous rows; ${summary.orientation_audit.ambiguous_rows_missing_verification.length} lack verification; ${summary.orientation_audit.matchable_rows_missing_source_build.length} matchable rows lack source-build metadata; ${summary.orientation_audit.matchable_rows_missing_expected_plus_alleles.length} lack expected plus-strand alleles.`);
  console.log('Effect classes:');
  for (const [key, count] of Object.entries(summary.effect_classes)) console.log(`  ${key}: ${count}`);
  console.log('Variant types:');
  for (const [key, count] of Object.entries(summary.variant_types)) console.log(`  ${key}: ${count}`);
  console.log('Callability policies:');
  for (const [key, count] of Object.entries(summary.callability_policy_counts)) console.log(`  ${key}: ${count}`);
  if (summary.non_matchable_snp_rows.length > 0) {
    console.log('Non-matchable SNP-like assertions are listed by marker ID only; no genotype values were read.');
  }
}

for (const error of errors) console.error(`ERROR: ${error}`);
if (errors.length > 0) process.exit(1);
