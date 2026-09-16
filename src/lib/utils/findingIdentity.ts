/**
 * Canonical finding identity and topic grouping.
 *
 * A report may contain the same locus in several curated packs. This module
 * gives those source rows one stable signal identity for Simple presentation
 * while retaining every source marker, section, evidence relationship, and
 * reference for Clinical/technical views.
 *
 * This is a display/grouping model only. It never changes calls, severity,
 * evidence, clinical interpretation, or the meaning of an uncalled marker.
 */

import manifest from '../marker-packs/manifest.json';
import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import type {
  EffectDirection,
  EvaluatedMarker,
  FindingClinicalState,
  FindingInheritanceModel,
  FindingInterpretationClass,
  GeneratedReport,
  SeverityClass,
} from '../types/genomics';
import { normalizeFindingSemantics } from './findingSemantics';
import { isCallableGenotype } from './genotype';
import { isVerifiedAssertionStatus } from './reportStatuses';

export type CanonicalCallState = 'called' | 'unknown' | 'mixed';
export type CanonicalActionabilityLevel =
  | 'clinical_confirmation'
  | 'symptom_or_lab_conditioned'
  | 'research_context';

export interface CanonicalFindingActionability {
  level: CanonicalActionabilityLevel;
  clinicalConfirmationRequired: boolean;
  hasFollowUp: boolean;
}

export interface CanonicalFindingApplicability {
  scopes: string[];
  contextRequired: boolean;
}

export interface CanonicalFindingSource {
  marker: EvaluatedMarker;
  sectionName: string;
  topicId: string;
  sourceIndex: number;
}

export interface CanonicalFinding {
  /** Stable signal identity, independent of a user's genotype call. */
  findingId: string;
  /** Exact report link IDs for every contributing source row. */
  sourceMarkerIds: string[];
  /** Standard or source-provided marker identifiers represented by the group. */
  rsids: string[];
  genes: string[];
  variantNames: string[];
  /** Primary topic used for Simple labels. */
  topicId: string;
  topicLabel: string;
  /** All topics/areas represented by the grouped source rows. */
  topicIds: string[];
  topicLabels: string[];
  healthAreaIds: string[];
  healthAreaLabels: string[];
  evidenceTiers: string[];
  effectDirections: EffectDirection[];
  severityClasses: SeverityClass[];
  conditionLabels: string[];
  interpretationClasses: FindingInterpretationClass[];
  inheritanceModels: FindingInheritanceModel[];
  clinicalStates: FindingClinicalState[];
  actionability: CanonicalFindingActionability;
  applicability: CanonicalFindingApplicability;
  referenceIds: string[];
  callState: CanonicalCallState;
}

export interface CanonicalFindingGroup extends CanonicalFinding {
  /** Source rows stay available for representative selection and drill-down. */
  sourceMarkers: CanonicalFindingSource[];
  representativeSource: CanonicalFindingSource;
}

interface TopicDescriptor {
  id: string;
  label: string;
}

interface ReportSectionLike {
  name: string;
  markers: EvaluatedMarker[];
}

const SEVERITY_ORDER: SeverityClass[] = [
  'high_risk',
  'confirmation_required',
  'moderate_risk',
  'low_risk',
  'protective',
  'trait',
  'context_dependent',
  'not_evaluated',
  'benign',
  'no_data',
];

function normalize(value: unknown): string {
  return String(value || '')
    .trim()
    .toLowerCase()
    .replace(/\s+/g, ' ');
}

function stablePart(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'unknown';
}

function uniqueStrings(values: readonly string[]): string[] {
  return Array.from(new Set(values.map((value) => value.trim()).filter(Boolean)));
}

function uniqueLowercaseStrings(values: readonly string[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of values) {
    const trimmed = value.trim();
    const key = trimmed.toLowerCase();
    if (!key || seen.has(key)) continue;
    seen.add(key);
    result.push(trimmed);
  }
  return result;
}

function taxonomyTopicForPack(packId: string): TopicDescriptor | undefined {
  const category = researchTaxonomy.categories.find((item) =>
    Array.isArray(item.packs) && item.packs.includes(packId),
  );
  if (category) return { id: category.id, label: category.label };

  const discoveryCategory = researchTaxonomy.discovery_categories.find((item) => item.id === packId);
  if (discoveryCategory) return { id: discoveryCategory.id, label: discoveryCategory.label };
  return undefined;
}

function topicForSectionName(sectionName: string): TopicDescriptor {
  const normalizedSection = normalize(sectionName);
  const directCategory = researchTaxonomy.categories.find((item) => normalize(item.label) === normalizedSection);
  if (directCategory) return { id: directCategory.id, label: directCategory.label };

  const manifestPack = manifest.packs.find((pack) => normalize(pack.label) === normalizedSection);
  if (manifestPack) {
    return taxonomyTopicForPack(manifestPack.id) || { id: manifestPack.id, label: manifestPack.label };
  }

  const discoveryCategory = researchTaxonomy.discovery_categories.find((item) => normalize(item.label) === normalizedSection);
  if (discoveryCategory) return { id: discoveryCategory.id, label: discoveryCategory.label };

  return { id: stablePart(sectionName), label: sectionName.trim() || 'Uncategorized' };
}

function baseSignalKey(marker: EvaluatedMarker): string {
  const markerId = String(marker.rsid || '').trim();
  if (markerId) return `rsid:${normalize(markerId)}`;

  const gene = normalize(marker.gene);
  const variant = normalize(marker.variant_name);
  if (gene || variant) return `locus:${gene}:${variant}`;

  return `link:${normalize(marker.link_id)}`;
}

/**
 * Return only source-authored biomedical fields that can distinguish two
 * assertions at the same locus. Derived callability/clinical defaults are
 * intentionally excluded so a known row and its no-data mirror still form a
 * mixed group instead of being split apart.
 */
function semanticPartitionKey(marker: EvaluatedMarker): string {
  const authored = marker.clinical_semantics || {};
  const parts = [
    normalize(marker.gene),
    normalize(marker.variant_type),
    normalize(marker.source_build),
    normalize(marker.hgvs),
    normalize(marker.effect_allele),
    [...(marker.expected_plus_alleles || [])].map(normalize).sort().join(','),
    normalize(marker.effect_direction),
    normalize(authored.condition_label),
    normalize(authored.interpretation_class),
    normalize(authored.inheritance_model),
    normalize(authored.clinical_state),
  ];
  return parts.join(':');
}

function findingIdForKey(key: string): string {
  return `finding-${stablePart(key)}`;
}

function severityRank(marker: EvaluatedMarker): number {
  const index = SEVERITY_ORDER.indexOf(marker.severity_class);
  return index === -1 ? SEVERITY_ORDER.length : index;
}

function isKnownCall(marker: EvaluatedMarker): boolean {
  return isVerifiedAssertionStatus(marker.assertion_status)
    && isCallableGenotype(marker.user_genotype);
}

function sourcePriority(source: CanonicalFindingSource): [number, number, number, number] {
  const marker = source.marker;
  return [
    isKnownCall(marker) ? 0 : 1,
    marker.interpretation_allowed ? 0 : 1,
    marker.effect_count === 0 ? 1 : 0,
    severityRank(marker),
  ];
}

function compareSourcePriority(left: CanonicalFindingSource, right: CanonicalFindingSource): number {
  const leftRank = sourcePriority(left);
  const rightRank = sourcePriority(right);
  for (let index = 0; index < leftRank.length; index += 1) {
    if (leftRank[index] !== rightRank[index]) return leftRank[index] - rightRank[index];
  }
  return left.marker.link_id.localeCompare(right.marker.link_id);
}

function actionabilityForSources(sources: readonly CanonicalFindingSource[]): CanonicalFindingActionability {
  const clinicalConfirmationRequired = sources.some((source) =>
    source.marker.clinical_confirmation_required === true
      || source.marker.severity_class === 'confirmation_required',
  );
  const hasFollowUp = sources.some((source) => source.marker.confirm_with.length > 0);
  return {
    level: clinicalConfirmationRequired
      ? 'clinical_confirmation'
      : hasFollowUp
        ? 'symptom_or_lab_conditioned'
        : 'research_context',
    clinicalConfirmationRequired,
    hasFollowUp,
  };
}

function callStateForSources(sources: readonly CanonicalFindingSource[]): CanonicalCallState {
  const calledCount = sources.filter((source) => isKnownCall(source.marker)).length;
  if (calledCount === 0) return 'unknown';
  if (calledCount === sources.length) return 'called';
  return 'mixed';
}

function finalizeGroup(
  key: string,
  sources: CanonicalFindingSource[],
): CanonicalFindingGroup {
  const representativeSource = [...sources].sort(compareSourcePriority)[0] || sources[0];
  const topicIds = uniqueStrings(sources.map((source) => source.topicId));
  const topicLabels = uniqueLowercaseStrings(sources.map((source) => source.sectionName));
  const healthAreaLabels = [...topicLabels];
  const taxonomyTopic = topicForSectionName(representativeSource.sectionName);

  return {
    findingId: findingIdForKey(key),
    sourceMarkerIds: uniqueStrings(sources.map((source) => source.marker.link_id)),
    rsids: uniqueLowercaseStrings(sources.map((source) => source.marker.rsid)),
    genes: uniqueLowercaseStrings(sources.map((source) => source.marker.gene)),
    variantNames: uniqueLowercaseStrings(sources.map((source) => source.marker.variant_name)),
    topicId: representativeSource.topicId || taxonomyTopic.id,
    topicLabel: taxonomyTopic.label || representativeSource.sectionName,
    topicIds,
    topicLabels,
    healthAreaIds: topicIds,
    healthAreaLabels,
    evidenceTiers: uniqueStrings(sources.map((source) => source.marker.evidence_tier)),
    effectDirections: uniqueStrings(sources.map((source) => source.marker.effect_direction)) as EffectDirection[],
    severityClasses: uniqueStrings(sources.map((source) => source.marker.severity_class)) as SeverityClass[],
    conditionLabels: uniqueStrings(sources.map((source) => normalizeFindingSemantics(source.marker).condition_label || '')),
    interpretationClasses: uniqueStrings(
      sources.map((source) => normalizeFindingSemantics(source.marker).interpretation_class),
    ) as FindingInterpretationClass[],
    inheritanceModels: uniqueStrings(
      sources.map((source) => normalizeFindingSemantics(source.marker).inheritance_model),
    ) as FindingInheritanceModel[],
    clinicalStates: uniqueStrings(
      sources.map((source) => normalizeFindingSemantics(source.marker).clinical_state),
    ) as FindingClinicalState[],
    actionability: actionabilityForSources(sources),
    applicability: {
      scopes: uniqueStrings(
        sources
          .map((source) => source.marker.sex_scope || '')
          .filter((scope) => scope && scope !== 'all'),
      ),
      contextRequired: sources.some((source) =>
        Boolean(source.marker.sex_scope && source.marker.sex_scope !== 'all'),
      ),
    },
    referenceIds: uniqueStrings(sources.flatMap((source) => source.marker.reference_ids || [])),
    callState: callStateForSources(sources),
    sourceMarkers: sources,
    representativeSource,
  };
}

/** Build canonical findings from report sections while preserving source rows. */
export function buildCanonicalFindingGroups(
  report: Pick<GeneratedReport, 'sections'> | { sections: readonly ReportSectionLike[] },
): CanonicalFindingGroup[] {
  const baseGroups = new Map<string, CanonicalFindingSource[]>();
  let sourceIndex = 0;

  for (const section of report.sections || []) {
    const topic = topicForSectionName(section.name);
    for (const marker of section.markers || []) {
      const source: CanonicalFindingSource = {
        marker,
        sectionName: section.name,
        topicId: topic.id,
        sourceIndex,
      };
      sourceIndex += 1;
      const key = baseSignalKey(marker);
      const sources = baseGroups.get(key) || [];
      sources.push(source);
      baseGroups.set(key, sources);
    }
  }

  const groups: Array<[string, CanonicalFindingSource[]]> = [];
  for (const [baseKey, sources] of baseGroups.entries()) {
    const partitions = new Map<string, CanonicalFindingSource[]>();
    for (const source of sources) {
      const partition = semanticPartitionKey(source.marker);
      const partitionSources = partitions.get(partition) || [];
      partitionSources.push(source);
      partitions.set(partition, partitionSources);
    }

    if (partitions.size === 1) {
      groups.push([baseKey, sources]);
      continue;
    }

    for (const [partition, partitionSources] of partitions.entries()) {
      groups.push([`${baseKey}:assertion:${partition}`, partitionSources]);
    }
  }

  return groups.map(([key, sources]) => finalizeGroup(key, sources));
}

/** Return the canonical identity key for diagnostics and future report views. */
export function canonicalFindingKey(marker: EvaluatedMarker): string {
  return baseSignalKey(marker);
}
