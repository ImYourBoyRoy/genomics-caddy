/**
 * Builds condition-level summaries from already evaluated DNA findings.
 *
 * This is an evidence aggregation layer, not a diagnostic engine. It keeps
 * three quantities separate:
 * - coded: curated indicators in the condition definition;
 * - callable: those indicators with a verified, usable call in this report;
 * - aligned: callable indicators with a non-zero reported effect count.
 *
 * Counts are never converted into disease probabilities. Cross-pack copies of
 * one marker are collapsed before counting, while the original source rows
 * remain available through their link IDs and the normal report sections.
 */

import conditionIntegrations from '../marker-packs/condition_integrations.json';
import type {
  EffectDirection,
  EvaluatedMarker,
  FindingClinicalState,
  FindingInheritanceModel,
  FindingInterpretationClass,
  GeneratedReport,
} from '../types/genomics';
import { normalizeFindingSemantics } from './findingSemantics';
import { callabilityStateForResult } from './callability';
import { isCallableGenotype } from './genotype';
import { getLaypersonTranslation, getSimpleFindingCopy } from './layperson';
import { isVerifiedAssertionStatus } from './reportStatuses';

export type ConditionSignalType =
  | 'susceptibility_context'
  | 'research_context'
  | 'carrier_possibility'
  | 'clinically_actionable_variant';

export type ConditionDiagnosticCapability =
  | 'clinical_evaluation_required'
  | 'clinical_variant_can_establish_when_confirmed'
  | 'not_diagnostic';

export type ConditionRelativeSignal = 'higher' | 'moderate' | 'limited' | 'insufficient';

export interface ConditionEvidenceSummary {
  id: string;
  label: string;
  category: string;
  signal_type: ConditionSignalType;
  diagnostic_capability: ConditionDiagnosticCapability;
  relative_signal: ConditionRelativeSignal;
  relative_signal_label: string;
  /** Curated denominator; this is not a probability denominator. */
  coded_indicator_count: number;
  /** Distinct curated indicators with a verified usable call. */
  callable_indicator_count: number;
  /** Distinct indicators with a verified call and a non-zero effect count. */
  matched_indicator_count: number;
  /** Curated indicators present in the generated report, regardless of call. */
  available_indicator_count: number;
  /** Called indicators that do not align with their authored effect allele. */
  non_aligned_indicator_count: number;
  plain_meaning: string;
  clinical_route: string;
  evidence_label: string;
  direction_summary: string;
  genes: string[];
  rsids: string[];
  matched_marker_ids: string[];
  matched_marker_link_ids: string[];
  reference_ids: string[];
  source_ids: string[];
  /** True when this summary came from an authored clinical condition label. */
  authored_condition: boolean;
  /** Present for authored clinical annotations; omitted for registry summaries. */
  interpretation_classes?: FindingInterpretationClass[];
  inheritance_models?: FindingInheritanceModel[];
  clinical_states?: FindingClinicalState[];
}

export type ConditionCoverageStatus =
  | 'not_observed'
  | 'unavailable'
  | 'partial'
  | 'callable_coverage';

/**
 * Coverage accounting for every registered condition route, including routes
 * with no matched markers. This is deliberately separate from
 * ConditionEvidenceSummary, which is the active matched-finding surface.
 */
export interface ConditionCoverageSummary {
  id: string;
  label: string;
  category: string;
  signal_type: ConditionSignalType;
  diagnostic_capability: ConditionDiagnosticCapability;
  coded_indicator_count: number;
  available_indicator_count: number;
  callable_indicator_count: number;
  matched_indicator_count: number;
  non_aligned_indicator_count: number;
  not_present_indicator_count: number;
  unknown_indicator_count: number;
  blocked_indicator_count: number;
  not_callable_indicator_count: number;
  missing_indicator_count: number;
  status: ConditionCoverageStatus;
  clinical_route: string;
  source_ids: string[];
}

export interface ConditionCoverageGap {
  id: string;
  label: string;
  status: string;
  display: string;
  clinical_next_step?: string;
  sources?: string[];
}

export type AssociationIs =
  | 'variant_condition_summary'
  | 'variant_risk_factor_summary'
  | 'variant_trait_statistical_association'
  | 'gene_disease_validity'
  | 'variant_drug_response';

export type AssociationScope = 'variant' | 'locus' | 'gene';
export type AssociationAlleleMatch = 'matched' | 'not_detected' | 'not_verifiable' | 'not_reported';

/** A source-specific catalog relationship, kept separate from curated health-pattern scores. */
export interface CatalogAssociationSummary {
  id: string;
  label: string;
  association_is: AssociationIs;
  association_scope: AssociationScope;
  source_type: 'ClinVar' | 'GWAS Catalog' | 'ClinGen' | 'ClinPGx';
  relationship_label: string;
  evidence_summary: string;
  marker_count: number;
  genes: string[];
  rsids: string[];
  matched_marker_link_ids: string[];
  reference_ids: string[];
  record_ids: string[];
  source_urls: string[];
  clinical_significance?: string;
  review_statuses?: string[];
  allele_match?: AssociationAlleleMatch;
  condition_specific_assertion_available?: boolean;
  rcv_accessions?: string[];
  best_p_value?: number;
  study_accessions?: string[];
  classification?: string;
  inheritance_models?: string[];
}

interface ConditionDefinition {
  id: string;
  label: string;
  category: string;
  signal_type: ConditionSignalType;
  diagnostic_capability: ConditionDiagnosticCapability;
  max_relative_signal: Exclude<ConditionRelativeSignal, 'insufficient'>;
  marker_ids: string[];
  plain_meaning: string;
  clinical_route: string;
  sources: string[];
}

interface SourceMarker {
  marker: EvaluatedMarker;
  sectionName: string;
}

const definitions = conditionIntegrations.conditions as ConditionDefinition[];
const coverageGaps = ((conditionIntegrations.policy as typeof conditionIntegrations.policy & {
  coverage_gaps?: ConditionCoverageGap[];
}).coverage_gaps || []) as ConditionCoverageGap[];
const RELATIVE_SIGNAL_LABELS: Record<ConditionRelativeSignal, string> = {
  higher: 'Higher relative signal',
  moderate: 'Moderate relative signal',
  limited: 'Limited relative signal',
  insufficient: 'Not enough callable data',
};

const EVIDENCE_LABELS: Record<string, string> = {
  A: 'Higher clinical relevance',
  B: 'Replicated or clinically relevant context',
  C: 'Candidate or mechanistic context',
  D: 'Research-level context',
  E: 'Evidence gap',
};

function normalize(value: unknown): string {
  return String(value || '').trim().toLowerCase();
}

function uniqueStrings(values: Iterable<string>): string[] {
  return Array.from(new Set(Array.from(values).map((value) => value.trim()).filter(Boolean)));
}

function knownCall(marker: EvaluatedMarker): boolean {
  return isVerifiedAssertionStatus(marker.assertion_status)
    && isCallableGenotype(marker.user_genotype);
}

function alignedCall(marker: EvaluatedMarker): boolean {
  if (!knownCall(marker)) return false;
  if (marker.effect_count === 0) return false;
  if (typeof marker.effect_count === 'number' && marker.effect_count > 0) return true;
  // A clinically validated panel can carry a verified call without a numeric
  // allele count. Ordinary consumer-array rows should have a numeric count.
  return marker.variant_type === 'gene_panel' || marker.variant_type === 'hla';
}

function coverageState(marker: EvaluatedMarker): ReturnType<typeof callabilityStateForResult> {
  return marker.callability_state
    || callabilityStateForResult(marker.variant_type, marker.assertion_status);
}

function indicatorId(marker: EvaluatedMarker): string {
  return normalize(marker.rsid) || normalize(marker.link_id);
}

function evidenceRank(marker: EvaluatedMarker): number {
  const grade = normalize(marker.evidence_tier).charAt(0).toUpperCase();
  return grade === 'A' ? 5 : grade === 'B' ? 4 : grade === 'C' ? 3 : grade === 'D' ? 2 : grade === 'E' ? 1 : 0;
}

function evidenceLabel(markers: readonly EvaluatedMarker[]): string {
  const best = markers.reduce((winner, marker) => evidenceRank(marker) > winner ? evidenceRank(marker) : winner, 0);
  return EVIDENCE_LABELS[['', 'E', 'D', 'C', 'B', 'A'][best] || ''] || 'Evidence not classified';
}

function relativeSignal(
  matched: number,
  callable: number,
  coded: number,
  markers: readonly EvaluatedMarker[],
  cap: Exclude<ConditionRelativeSignal, 'insufficient'>,
): ConditionRelativeSignal {
  if (matched === 0 || callable === 0 || coded === 0) return 'insufficient';
  const ratio = matched / coded;
  const bestEvidence = markers.reduce((winner, marker) => Math.max(winner, evidenceRank(marker)), 0);
  let value: Exclude<ConditionRelativeSignal, 'insufficient'>;
  if (bestEvidence >= 5) {
    value = ratio >= 0.75 ? 'higher' : ratio >= 0.5 ? 'moderate' : 'limited';
  } else if (bestEvidence >= 4) {
    value = ratio >= 0.75 ? 'higher' : ratio >= 0.5 ? 'moderate' : 'limited';
  } else if (bestEvidence >= 3) {
    value = ratio >= 0.75 ? 'moderate' : 'limited';
  } else {
    value = ratio >= 0.5 ? 'moderate' : 'limited';
  }

  const capRank = { limited: 1, moderate: 2, higher: 3 };
  return capRank[value] > capRank[cap] ? cap : value;
}

function directionSummary(markers: readonly EvaluatedMarker[]): string {
  const counts: Record<EffectDirection, number> = {
    risk: 0,
    protective: 0,
    context_dependent: 0,
    trait: 0,
    unknown: 0,
    not_applicable: 0,
    no_claim: 0,
  };
  for (const marker of markers) counts[marker.effect_direction] += 1;
  const parts = [
    counts.risk > 0 ? `${counts.risk} risk-leaning` : '',
    counts.context_dependent > 0 ? `${counts.context_dependent} context-dependent` : '',
    counts.protective > 0 ? `${counts.protective} protective` : '',
    counts.trait > 0 ? `${counts.trait} trait` : '',
  ].filter(Boolean);
  return parts.join(' · ') || 'Direction not established';
}

function chooseMarker(markers: readonly SourceMarker[]): SourceMarker | undefined {
  return [...markers].sort((left, right) => {
    const leftKnown = knownCall(left.marker) ? 0 : 1;
    const rightKnown = knownCall(right.marker) ? 0 : 1;
    return leftKnown - rightKnown
      || evidenceRank(right.marker) - evidenceRank(left.marker)
      || left.marker.link_id.localeCompare(right.marker.link_id);
  })[0];
}

function dedupeSources(markers: readonly SourceMarker[]): SourceMarker[] {
  const selected = new Map<string, SourceMarker>();
  for (const source of markers) {
    const key = indicatorId(source.marker);
    const previous = selected.get(key);
    if (!previous || (knownCall(source.marker) && !knownCall(previous.marker)) || evidenceRank(source.marker) > evidenceRank(previous.marker)) {
      selected.set(key, source);
    }
  }
  return Array.from(selected.values());
}

interface MutableCatalogAssociation {
  id: string;
  label: string;
  association_is: AssociationIs;
  association_scope: AssociationScope;
  source_type: CatalogAssociationSummary['source_type'];
  relationship_label: string;
  evidence_summary: string;
  clinical_significance?: string;
  reviewStatuses: Set<string>;
  allele_match?: AssociationAlleleMatch;
  condition_specific_assertion_available?: boolean;
  classification?: string;
  markerIds: Set<string>;
  genes: Set<string>;
  rsids: Set<string>;
  links: Set<string>;
  referenceIds: Set<string>;
  recordIds: Set<string>;
  urls: Set<string>;
  studyAccessions: Set<string>;
  rcvAccessions: Set<string>;
  inheritanceModels: Set<string>;
  pValues: number[];
}

function hasObservedCall(marker: EvaluatedMarker): boolean {
  const state = coverageState(marker);
  return isCallableGenotype(marker.normalized_genotype || marker.user_genotype)
    && state !== 'not_present'
    && state !== 'not_callable'
    && state !== 'unknown';
}

function associationSlug(value: string): string {
  return normalize(value).replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 72) || 'unnamed';
}

function clinvarAssociationClass(significance: string): 'pathogenic' | 'risk' | null {
  const value = normalize(significance);
  if (!value || /conflict|uncertain|benign|drug response|not provided/.test(value)) return null;
  if (/likely\s+pathogenic|pathogenic/.test(value)) return 'pathogenic';
  if (/risk factor|risk allele|association/.test(value)) return 'risk';
  return null;
}

function parsedSnpCall(marker: EvaluatedMarker): string[] | null {
  const genotype = String(marker.normalized_genotype || marker.user_genotype || '')
    .trim()
    .toUpperCase()
    .replace(/[|/]/g, '');
  return /^[ACGT]{2}$/.test(genotype) ? genotype.split('') : null;
}

function clinvarAlleleMatch(
  marker: EvaluatedMarker,
  annotation: NonNullable<EvaluatedMarker['clinvar_annotations']>[number],
): AssociationAlleleMatch {
  const alleles = parsedSnpCall(marker);
  const reference = String(annotation.reference_allele || '').trim().toUpperCase();
  const alternate = String(annotation.alternate_allele || '').trim().toUpperCase();
  const expected = (marker.expected_plus_alleles || []).map((allele) => String(allele).toUpperCase());
  const orientationUsable = marker.orientation_state === 'verified' || marker.orientation_state === 'not_required';
  if (!hasObservedCall(marker) || !knownCall(marker) || !marker.interpretation_allowed
      || !orientationUsable || !alleles || !/^[ACGT]$/.test(reference)
      || !/^[ACGT]$/.test(alternate) || !expected.includes(reference) || !expected.includes(alternate)) {
    return 'not_verifiable';
  }
  return alleles.includes(alternate) ? 'matched' : 'not_detected';
}

function numericPvalue(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string' && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

/**
 * Collect database-backed links from called markers. Variant summaries,
 * locus-level GWAS traits, and gene-level validity remain distinct; none is a
 * disease diagnosis or a personal risk estimate.
 */
export function buildCatalogAssociationSummaries(
  report: Pick<GeneratedReport, 'sections'>,
): CatalogAssociationSummary[] {
  const groups = new Map<string, MutableCatalogAssociation>();
  const add = (input: {
    marker: EvaluatedMarker;
    label: string;
    association_is: AssociationIs;
    association_scope: AssociationScope;
    source_type: MutableCatalogAssociation['source_type'];
    relationship_label: string;
    evidence_summary: string;
    clinical_significance?: string;
    review_status?: string;
    allele_match?: AssociationAlleleMatch;
    condition_specific_assertion_available?: boolean;
    rcv_accessions?: string[];
    classification?: string;
    record_id?: string | null;
    url?: string | null;
    study_accession?: string | null;
    p_value?: number | null;
    inheritance_model?: string | null;
  }) => {
    const label = input.label.trim();
    if (!label) return;
    const groupKey = [input.association_is, normalize(label), normalize(input.clinical_significance),
      normalize(input.allele_match), normalize(input.classification)].join('|');
    const current = groups.get(groupKey) || {
      id: `catalog-${input.association_is}-${associationSlug(label)}-${associationSlug(input.allele_match || input.classification || 'linked')}`,
      label,
      association_is: input.association_is,
      association_scope: input.association_scope,
      source_type: input.source_type,
      relationship_label: input.relationship_label,
      evidence_summary: input.evidence_summary,
      clinical_significance: input.clinical_significance,
      reviewStatuses: new Set<string>(),
      allele_match: input.allele_match,
      condition_specific_assertion_available: input.condition_specific_assertion_available,
      classification: input.classification,
      markerIds: new Set<string>(),
      genes: new Set<string>(),
      rsids: new Set<string>(),
      links: new Set<string>(),
      referenceIds: new Set<string>(),
      recordIds: new Set<string>(),
      urls: new Set<string>(),
      studyAccessions: new Set<string>(),
      rcvAccessions: new Set<string>(),
      inheritanceModels: new Set<string>(),
      pValues: [],
    };
    const markerId = indicatorId(input.marker);
    if (markerId) current.markerIds.add(markerId);
    if (input.marker.gene) current.genes.add(input.marker.gene);
    if (input.marker.rsid) current.rsids.add(input.marker.rsid);
    if (input.marker.link_id) current.links.add(input.marker.link_id);
    if (input.review_status) current.reviewStatuses.add(input.review_status);
    (input.marker.reference_ids || []).forEach((id) => current.referenceIds.add(id));
    if (input.record_id) current.recordIds.add(input.record_id);
    if (input.url) current.urls.add(input.url);
    if (input.study_accession) current.studyAccessions.add(input.study_accession);
    (input.rcv_accessions || []).forEach((accession) => current.rcvAccessions.add(accession));
    if (input.inheritance_model) current.inheritanceModels.add(input.inheritance_model);
    if (typeof input.p_value === 'number' && Number.isFinite(input.p_value)) current.pValues.push(input.p_value);
    groups.set(groupKey, current);
  };

  for (const section of report.sections || []) {
    for (const marker of section.markers || []) {
      if (!hasObservedCall(marker)) continue;

      const clinvar = marker.clinvar_annotations?.length
        ? marker.clinvar_annotations
        : marker.clinvar_significance
          ? [{
              clinical_significance: marker.clinvar_significance,
              conditions: marker.clinvar_conditions,
              review_status: marker.clinvar_review_status,
            }]
          : [];
      for (const annotation of clinvar) {
        const kind = clinvarAssociationClass(annotation.clinical_significance || '');
        const conditionLabels = String(annotation.conditions || '')
          .split('|')
          .map((label) => label.trim())
          .filter((label) => label && !/^not provided$/i.test(label));
        if (!kind || conditionLabels.length === 0) continue;
        const alleleMatch = clinvarAlleleMatch(marker, annotation);
        if (alleleMatch === 'not_detected') continue;
        const pathogenic = kind === 'pathogenic';
        const id = annotation.variation_id || annotation.allele_id || marker.rsid;
        for (const condition of conditionLabels) {
          add({
            marker,
            label: condition,
            association_is: pathogenic ? 'variant_condition_summary' : 'variant_risk_factor_summary',
            association_scope: 'variant',
            source_type: 'ClinVar',
            relationship_label: pathogenic ? 'ClinVar variant-summary condition label' : 'ClinVar variant-summary risk label',
            evidence_summary: alleleMatch === 'matched'
              ? `ClinVar's variant-level summary lists this condition and ${annotation.clinical_significance}. The local index does not preserve which condition-specific RCV assertion supplied that classification.`
              : `ClinVar's variant-level summary lists this condition and ${annotation.clinical_significance}, but this report could not verify the exact allele match or a condition-specific RCV assertion.`,
            clinical_significance: annotation.clinical_significance,
            review_status: annotation.review_status || undefined,
            allele_match: alleleMatch,
            condition_specific_assertion_available: false,
            rcv_accessions: (annotation.rcv_accession || '').split('|').map((accession) => accession.trim()).filter(Boolean),
            record_id: id,
            url: annotation.variation_id
              ? `https://www.ncbi.nlm.nih.gov/clinvar/?term=${encodeURIComponent(annotation.variation_id)}%5BVariant+ID%5D`
              : `https://www.ncbi.nlm.nih.gov/clinvar/?term=${encodeURIComponent(marker.rsid)}`,
          });
        }
      }

      const gwasRecords = marker.gwas_associations?.length
        ? marker.gwas_associations
        : marker.gwas_top_trait && marker.gwas_best_pvalue != null
          ? [{ trait_name: marker.gwas_top_trait, pvalue: marker.gwas_best_pvalue }]
          : [];
      for (const association of gwasRecords) {
        // A lightweight internal routing seed is stored in the same table as
        // downloaded GWAS records, but its placeholder p-value is not evidence.
        if (normalize(association.source) === 'discovery_catalog_fallback') continue;
        const trait = association.trait_name || association.trait?.trait || '';
        const pValue = numericPvalue(association.pvalue);
        if (!trait || pValue == null || pValue > 1e-5) continue;
        const study = association.study_accession || undefined;
        add({
          marker,
          label: trait,
          association_is: 'variant_trait_statistical_association',
          association_scope: 'locus',
          source_type: 'GWAS Catalog',
          relationship_label: 'GWAS locus–trait association',
          evidence_summary: 'A published statistical association is reported at this locus. The local index keeps up to 12 strongest study records per marker and does not establish effect-allele alignment or personal disease risk.',
          record_id: study || marker.rsid,
          study_accession: study,
          p_value: pValue,
          url: study
            ? `https://www.ebi.ac.uk/gwas/studies/${encodeURIComponent(study)}`
            : `https://www.ebi.ac.uk/gwas/variants/${encodeURIComponent(marker.rsid)}`,
        });
      }

      const geneRecords = marker.clingen_annotations?.length
        ? marker.clingen_annotations
        : marker.clingen
          ? [marker.clingen]
          : [];
      for (const annotation of geneRecords) {
        if (!annotation.disease_label) continue;
        add({
          marker,
          label: annotation.disease_label,
          association_is: 'gene_disease_validity',
          association_scope: 'gene',
          source_type: 'ClinGen',
          relationship_label: 'ClinGen gene–disease validity',
          evidence_summary: `ClinGen curates this relationship for ${annotation.gene_symbol || marker.gene}; it is gene-level context, not evidence that this marker causes the condition.`,
          classification: annotation.classification || undefined,
          record_id: annotation.hgnc_id || annotation.gene_symbol || marker.gene,
          inheritance_model: annotation.mode_of_inheritance || undefined,
          url: annotation.report_url || (annotation.gene_symbol
            ? `https://search.clinicalgenome.org/kb/genes/${encodeURIComponent(annotation.gene_symbol)}`
            : undefined),
        });
      }

      const pgxRecords = marker.pharmgkb_annotations?.length
        ? marker.pharmgkb_annotations
        : marker.pharmgkb
          ? [marker.pharmgkb]
          : [];
      for (const annotation of pgxRecords) {
        if (!annotation.drug && !annotation.phenotype) continue;
        const label = [annotation.drug, annotation.phenotype].filter(Boolean).join(' — ');
        add({
          marker,
          label,
          association_is: 'variant_drug_response',
          association_scope: 'variant',
          source_type: 'ClinPGx',
          relationship_label: 'Pharmacogenomic drug–response annotation',
          evidence_summary: 'This is a medication-response annotation linked to the marker; it is not a diagnosis, dosing instruction, or reason to change a medicine.',
          classification: annotation.evidence_level || undefined,
          record_id: annotation.drug || marker.rsid,
          url: `https://api.clinpgx.org/v1/variant?name=${encodeURIComponent(marker.rsid)}`,
        });
      }
    }
  }

  const relationOrder: Record<AssociationIs, number> = {
    variant_condition_summary: 0,
    variant_risk_factor_summary: 1,
    variant_trait_statistical_association: 2,
    gene_disease_validity: 3,
    variant_drug_response: 4,
  };
  return Array.from(groups.values()).map((group): CatalogAssociationSummary => ({
    id: group.id,
    label: group.label,
    association_is: group.association_is,
    association_scope: group.association_scope,
    source_type: group.source_type,
    relationship_label: group.relationship_label,
    evidence_summary: group.evidence_summary,
    marker_count: group.markerIds.size,
    genes: [...group.genes].sort(),
    rsids: [...group.rsids].sort(),
    matched_marker_link_ids: [...group.links].sort(),
    reference_ids: [...group.referenceIds].sort(),
    record_ids: [...group.recordIds].sort(),
    source_urls: [...group.urls].sort(),
    ...(group.clinical_significance ? { clinical_significance: group.clinical_significance } : {}),
    ...(group.reviewStatuses.size ? { review_statuses: [...group.reviewStatuses].sort() } : {}),
    ...(group.allele_match ? { allele_match: group.allele_match } : {}),
    ...(group.condition_specific_assertion_available != null
      ? { condition_specific_assertion_available: group.condition_specific_assertion_available }
      : {}),
    ...(group.rcvAccessions.size ? { rcv_accessions: [...group.rcvAccessions].sort() } : {}),
    ...(group.pValues.length ? { best_p_value: Math.min(...group.pValues) } : {}),
    ...(group.studyAccessions.size ? { study_accessions: [...group.studyAccessions].sort() } : {}),
    ...(group.classification ? { classification: group.classification } : {}),
    ...(group.inheritanceModels.size ? { inheritance_models: [...group.inheritanceModels].sort() } : {}),
  })).sort((left, right) => relationOrder[left.association_is] - relationOrder[right.association_is]
    || right.marker_count - left.marker_count
    || (left.best_p_value ?? Number.POSITIVE_INFINITY) - (right.best_p_value ?? Number.POSITIVE_INFINITY)
    || left.label.localeCompare(right.label));
}

function summaryFromDefinition(
  definition: ConditionDefinition,
  sources: readonly SourceMarker[],
): ConditionEvidenceSummary | null {
  const distinctSources = dedupeSources(sources);
  const byId = new Map(distinctSources.map((source) => [indicatorId(source.marker), source]));
  const available = definition.marker_ids.filter((id) => byId.has(normalize(id)));
  const called = distinctSources.filter((source) => knownCall(source.marker));
  const aligned = called.filter((source) => alignedCall(source.marker));
  const nonAligned = called.filter((source) => !alignedCall(source.marker));
  if (aligned.length === 0) return null;

  const markerValues = distinctSources.map((source) => source.marker);
  const alignedMarkers = aligned.map((source) => source.marker);
  const evidenceMarkers = called.length > 0 ? called.map((source) => source.marker) : markerValues;
  const matchedIds = aligned.map((source) => indicatorId(source.marker));
  const genes = uniqueStrings(aligned.map((source) => source.marker.gene));
  const rsids = uniqueStrings(aligned.map((source) => source.marker.rsid));
  const links = uniqueStrings(aligned.map((source) => source.marker.link_id));
  const refs = uniqueStrings(aligned.flatMap((source) => source.marker.reference_ids || []));
  const signal = relativeSignal(aligned.length, called.length, definition.marker_ids.length, evidenceMarkers, definition.max_relative_signal);

  return {
    id: definition.id,
    label: definition.label,
    category: definition.category,
    signal_type: definition.signal_type,
    diagnostic_capability: definition.diagnostic_capability,
    relative_signal: signal,
    relative_signal_label: RELATIVE_SIGNAL_LABELS[signal],
    coded_indicator_count: definition.marker_ids.length,
    callable_indicator_count: called.length,
    matched_indicator_count: aligned.length,
    available_indicator_count: available.length,
    non_aligned_indicator_count: nonAligned.length,
    plain_meaning: definition.plain_meaning,
    clinical_route: definition.clinical_route,
    evidence_label: evidenceLabel(evidenceMarkers),
    direction_summary: directionSummary(alignedMarkers),
    genes,
    rsids,
    matched_marker_ids: matchedIds,
    matched_marker_link_ids: links,
    reference_ids: refs,
    source_ids: [...definition.sources],
    authored_condition: false,
  };
}

function authoredConditionSummaries(sources: readonly SourceMarker[]): ConditionEvidenceSummary[] {
  const groups = new Map<string, SourceMarker[]>();
  for (const source of dedupeSources(sources)) {
    const semantics = normalizeFindingSemantics(source.marker);
    if (!semantics.condition_label) continue;
    const members = groups.get(normalize(semantics.condition_label)) || [];
    members.push(source);
    groups.set(normalize(semantics.condition_label), members);
  }

  return Array.from(groups.values()).flatMap((members) => {
    const representative = chooseMarker(members);
    if (!representative) return [];
    const representativeSemantics = normalizeFindingSemantics(representative.marker);
    const labels = members.map((source) => normalizeFindingSemantics(source.marker));
    const markerValues = members.map((source) => source.marker);
    const called = members.filter((source) => knownCall(source.marker));
    const aligned = called.filter((source) => alignedCall(source.marker));
    if (aligned.length === 0) return [];
    const evidenceMarkers = called.length > 0 ? called.map((source) => source.marker) : markerValues;
    const hasClinicalVariant = labels.some((semantics) =>
      semantics.interpretation_class === 'clinically_actionable_variant'
        || semantics.clinical_confirmation_required,
    );
    const interpretationClasses = uniqueStrings(labels.map((semantics) => semantics.interpretation_class)) as FindingInterpretationClass[];
    const inheritanceModels = uniqueStrings(labels.map((semantics) => semantics.inheritance_model)) as FindingInheritanceModel[];
    const clinicalStates = uniqueStrings(labels.map((semantics) => semantics.clinical_state)) as FindingClinicalState[];
    const simple = getSimpleFindingCopy(representative.marker, getLaypersonTranslation(representative.marker));
    const label = representativeSemantics.condition_label || 'Clinically relevant condition';
    const coded = members.length;
    const matched = aligned.length;
    const refs = uniqueStrings(members.flatMap((source) => source.marker.reference_ids || []));
    const signal = relativeSignal(matched, called.length, coded, evidenceMarkers, hasClinicalVariant ? 'higher' : 'moderate');
    return [{
      id: `authored-${normalize(label).replace(/[^a-z0-9]+/g, '-')}`,
      label,
      category: 'clinically_annotated',
      signal_type: hasClinicalVariant ? 'clinically_actionable_variant' : 'susceptibility_context',
      diagnostic_capability: hasClinicalVariant ? 'clinical_variant_can_establish_when_confirmed' : 'clinical_evaluation_required',
      relative_signal: signal,
      relative_signal_label: RELATIVE_SIGNAL_LABELS[signal],
      coded_indicator_count: coded,
      callable_indicator_count: called.length,
      matched_indicator_count: matched,
      available_indicator_count: coded,
      non_aligned_indicator_count: called.length - matched,
      plain_meaning: simple.signal,
      clinical_route: representative.marker.confirm_with.join('; ') || simple.review_action,
      evidence_label: evidenceLabel(evidenceMarkers),
      direction_summary: directionSummary(aligned.map((source) => source.marker)),
      genes: uniqueStrings(members.map((source) => source.marker.gene)),
      rsids: uniqueStrings(members.map((source) => source.marker.rsid)),
      matched_marker_ids: uniqueStrings(members.map((source) => indicatorId(source.marker))),
      matched_marker_link_ids: uniqueStrings(members.map((source) => source.marker.link_id)),
      reference_ids: refs,
      source_ids: [],
      authored_condition: true,
      interpretation_classes: interpretationClasses,
      inheritance_models: inheritanceModels,
      clinical_states: clinicalStates,
    }];
  });
}

/** Build named potential-condition and health-pattern summaries for a report. */
export function buildConditionEvidenceSummaries(
  report: Pick<GeneratedReport, 'sections'>,
): ConditionEvidenceSummary[] {
  const sources: SourceMarker[] = [];
  for (const section of report.sections || []) {
    for (const marker of section.markers || []) sources.push({ marker, sectionName: section.name });
  }

  const summaries = definitions.flatMap((definition) => {
    const targetIds = new Set(definition.marker_ids.map(normalize));
    return summaryFromDefinition(
      definition,
      sources.filter((source) => targetIds.has(indicatorId(source.marker))),
    ) || [];
  });
  const authored = authoredConditionSummaries(sources);
  const seen = new Set(summaries.map((summary) => summary.label.toLowerCase()));
  return [...summaries, ...authored.filter((summary) => !seen.has(summary.label.toLowerCase()))]
    .sort((left, right) => {
      const signalOrder = { higher: 0, moderate: 1, limited: 2, insufficient: 3 };
      return signalOrder[left.relative_signal] - signalOrder[right.relative_signal]
        || left.label.localeCompare(right.label);
    });
}

/**
 * Return coverage for every curated condition definition. Counts describe the
 * current report's representation and evaluator capability only; they are not
 * disease probabilities and never imply that an absent component is negative.
 */
export function buildConditionCoverageSummaries(
  report: Pick<GeneratedReport, 'sections'>,
): ConditionCoverageSummary[] {
  const sources: SourceMarker[] = [];
  for (const section of report.sections || []) {
    for (const marker of section.markers || []) sources.push({ marker, sectionName: section.name });
  }

  return definitions.map((definition) => {
    const targetIds = new Set(definition.marker_ids.map(normalize));
    const byIndicator = new Map<string, SourceMarker[]>();
    for (const source of sources) {
      const id = indicatorId(source.marker);
      if (!targetIds.has(id)) continue;
      const members = byIndicator.get(id) || [];
      members.push(source);
      byIndicator.set(id, members);
    }

    let callable = 0;
    let matched = 0;
    let nonAligned = 0;
    let notPresent = 0;
    let unknown = 0;
    let blocked = 0;
    let notCallable = 0;

    for (const members of byIndicator.values()) {
      const hasCallable = members.some((source) =>
        coverageState(source.marker) === 'callable' && knownCall(source.marker),
      );
      if (hasCallable) {
        callable += 1;
        if (members.some((source) => alignedCall(source.marker))) matched += 1;
        else nonAligned += 1;
        continue;
      }

      const states = members.map((source) => coverageState(source.marker));
      if (states.includes('blocked')) blocked += 1;
      else if (states.includes('not_callable')) notCallable += 1;
      else if (states.includes('not_present')) notPresent += 1;
      else unknown += 1;
    }

    const available = byIndicator.size;
    const missing = Math.max(definition.marker_ids.length - available, 0);
    const status: ConditionCoverageStatus = available === 0
      ? 'not_observed'
      : callable === 0
        ? 'unavailable'
        : missing > 0 || notPresent > 0 || unknown > 0 || blocked > 0 || notCallable > 0
          ? 'partial'
          : 'callable_coverage';

    return {
      id: definition.id,
      label: definition.label,
      category: definition.category,
      signal_type: definition.signal_type,
      diagnostic_capability: definition.diagnostic_capability,
      coded_indicator_count: definition.marker_ids.length,
      available_indicator_count: available,
      callable_indicator_count: callable,
      matched_indicator_count: matched,
      non_aligned_indicator_count: nonAligned,
      not_present_indicator_count: notPresent,
      unknown_indicator_count: unknown,
      blocked_indicator_count: blocked,
      not_callable_indicator_count: notCallable,
      missing_indicator_count: missing,
      status,
      clinical_route: definition.clinical_route,
      source_ids: [...definition.sources],
    };
  });
}

/** Explain named conditions that are intentionally not scored by this tool. */
export function getConditionCoverageGaps(): ConditionCoverageGap[] {
  return coverageGaps.map((gap) => ({ ...gap }));
}

export function conditionRelativeSignalLabel(signal: ConditionRelativeSignal): string {
  return RELATIVE_SIGNAL_LABELS[signal];
}

export function conditionDiagnosticCapabilityLabel(
  capability: ConditionDiagnosticCapability,
): string {
  switch (capability) {
    case 'clinical_variant_can_establish_when_confirmed':
      return 'A validated clinical variant result may establish this finding';
    case 'clinical_evaluation_required':
      return 'Clinical evaluation is required';
    case 'not_diagnostic':
      return 'DNA signal only';
  }
}
