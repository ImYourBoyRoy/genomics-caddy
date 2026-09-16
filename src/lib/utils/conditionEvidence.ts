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
