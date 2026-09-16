// ./src/lib/utils/actionabilityEngine.ts
/*
Purpose: Derive dashboard diet / supplement / lab / activity / medication-safety guidance and top findings from a report.
Responsibilities:
- Rank top association findings for the dashboard.
- Apply pack-authored actionability_guidance.json rules (not hardcoded UI rules).
- Optionally surface pack `confirm_with` items as lab discussion prompts.
Key Inputs: GeneratedReport markers; actionability_guidance.json.
Key Outputs: ActionablePlan for DashboardSummaryPanel.
Operational Notes: Guidance is educational only. Expand rules in the JSON pack file.
*/

import type {
  EffectDirection,
  FindingClinicalState,
  FindingInheritanceModel,
  FindingInterpretationClass,
  GeneratedReport,
  EvaluatedMarker,
  SeverityClass,
} from '../types/genomics';
import guidanceDoc from '../marker-packs/actionability_guidance.json';
import activityGuardrails from '../marker-packs/activity_guardrails.json';
import allergySensitivityCatalog from '../marker-packs/allergy_sensitivity_catalog.json';
import cycleSupport from '../marker-packs/cycle_support_guidance.json';
import dietaryRequirements from '../marker-packs/dietary_requirements.json';
import pgxDiplotypeGuidance from '../marker-packs/pgx_diplotype_guidance.json';
import safetyGuardrails from '../marker-packs/safety_guardrails.json';
import supplementSafety from '../marker-packs/supplement_safety.json';
import {
  activeReproductiveContextIds,
  cycleSupportEvidenceLayersForContextIds,
  cycleSupportDomainsForContextIds,
  reproductiveDnaCoverageForReport,
  selectedReproductiveContextOption,
} from './reproductiveContext';
import { activityDomainMatches, activityPersonalContextText } from './activityContext';
import { reviewCycleDiary, type CycleDiaryReview } from './cycleDiaryReview';
import type { PersonalSafetyContext } from './personalSafetyContext';
import type { ReproductiveIntakeValues } from './reproductiveIntake';
import { cycleDiaryAppliesToContexts, type CycleDiaryEntry } from './cycleDiary';
import type { PersonalDietaryProfile } from './personalSafetyContext';
import {
  buildCanonicalFindingGroups,
  type CanonicalActionabilityLevel,
  type CanonicalCallState,
} from './findingIdentity';
import { buildConditionEvidenceSummaries, type ConditionEvidenceSummary } from './conditionEvidence';
import { isCallableGenotype } from './genotype';
import { getLaypersonTranslation, getSimpleFindingCopy } from './layperson';
import { isVerifiedAssertionStatus } from './reportStatuses';
import { derivePgxComponentCoverage, type PgxComponentCoverage } from './pgxCoverage';

export interface TopFinding {
  /** Stable signal identity shared by all source rows for this finding. */
  finding_id: string;
  /** Every source marker retained for Clinical/technical drill-down. */
  source_marker_ids: string[];
  rsid: string;
  gene: string;
  variant_name: string;
  severity_class: SeverityClass;
  interpretation: string;
  section_name: string;
  link_id: string;
  topic_id: string;
  topic_ids: string[];
  health_area_ids: string[];
  health_area_labels: string[];
  evidence_tiers: string[];
  effect_directions: EffectDirection[];
  condition_labels: string[];
  interpretation_classes: FindingInterpretationClass[];
  inheritance_models: FindingInheritanceModel[];
  clinical_states: FindingClinicalState[];
  actionability_level: CanonicalActionabilityLevel;
  clinical_confirmation_required: boolean;
  applicability_scopes: string[];
  call_state: CanonicalCallState;
  reference_ids: string[];
  /** The same direction-aware title shown by the Simple finding card. */
  plain_title: string;
  /** Concise direction label, such as lower-dose tendency or higher susceptibility. */
  direction_label: string;
  /** Plain-language reason this finding appears in the first-screen queue. */
  priority_reason: string;
  /** Ordered urgency bucket used for ranking and visual tone. */
  priority_urgency: PriorityUrgency;
  /** Visual tone; it never represents disease probability. */
  priority_tone: PriorityTone;
  /** Simple queue grouping for related clinical medication components. */
  priority_group?: string;
  /** Number of DNA findings represented by the queue item. */
  related_marker_count?: number;
}

export interface DietaryGuidance {
  favor: string[];
  avoid: string[];
  /** Canonical DNA-linked recommendation records; legacy strings remain for compatibility. */
  favorItems: RecommendationItem[];
  avoidItems: RecommendationItem[];
}

export type RecommendationCategory = 'food' | 'supplement' | 'activity';

export interface RecommendationItem {
  recommendation_id: string;
  name: string;
  category: RecommendationCategory;
  basis_rule_ids: string[];
  basis_topic_ids: string[];
  basis_marker_ids: string[];
  /** Stable report link IDs used to jump from a recommendation to its DNA rows. */
  basis_marker_link_ids: string[];
  basis_genes: string[];
  /** Distinct source markers supporting this recommendation after deduplication. */
  basis_marker_count: number;
  /** Number of authored rules that contributed to this recommendation. */
  basis_rule_count: number;
  /** Compact relationship label for simple users and exports. */
  dna_basis: string;
  /** Short explanation of why this item appears for the loaded report. */
  why_it_appears: string;
  /** Highest evidence tier among the matched source markers. */
  evidence_level: string;
  /** The context in which the item is most useful. */
  when_relevant: string;
  /** Profile conflicts that can suppress or qualify the item. */
  conflicts: string[];
}

export interface SupplementItem extends RecommendationItem {
  name: string;
  reason: string;
}

export interface SupplementAvoidItem extends RecommendationItem {
  name: string;
  reason?: string;
}

export interface LabTest {
  /** Stable resource-defined identity; display names and aliases may evolve. */
  canonical_id: string;
  name: string;
  /** Short purpose shown when a user expands a lab row. */
  purpose: string;
  reason: string;
  /** All distinct DNA/topic reasons that contributed to this canonical test. */
  reason_topics: string[];
  basis_rule_ids: string[];
  basis_marker_ids: string[];
  basis_genes: string[];
  /** Internal sort weight — display uses `tier` labels instead of shouting URGENT. */
  urgency: 'routine' | 'consider' | 'urgent';
  tier: 'counselor' | 'discuss' | 'optional';
  category: string;
  requires_counselor?: boolean;
}

export interface LabTestGroup {
  tier: LabTest['tier'];
  label: string;
  hint: string;
  tests: LabTest[];
}

export interface LabRequestGroup {
  category: string;
  tests: LabTest[];
}

export interface ActivityGuidance {
  principles: string[];
  simpleFramework: typeof activityGuardrails.simple_framework;
  stopAndEscalate: string[];
  relevantDomains: ActivityDomain[];
  /** DNA-linked activity prompts with the matched domain and marker basis. */
  recommendationItems: ActivityRecommendationItem[];
}

type ActivityGuardrailDomain = typeof activityGuardrails.domains[number];

export type ActivityRecommendationKind = 'build' | 'watch' | 'verify';

export interface ActivityRecommendationItem extends RecommendationItem {
  category: 'activity';
  kind: ActivityRecommendationKind;
}

export type ActivityDomain = ActivityGuardrailDomain & {
  matched_marker_ids: string[];
  matched_genes: string[];
};

export interface MedicationSafetyGuidance {
  rules: string[];
  askFor: string[];
}

export interface MedicationPathway {
  id: string;
  label: string;
  direction_label: string;
  genes: string[];
  detail: string;
  matchedMarkerCount: number;
  matchedMarkerLinkIds: string[];
}

export interface CycleSupportGuidance {
  principles: typeof cycleSupport.principles;
  relevantEvidenceLayers: typeof cycleSupport.evidence_layers;
  dnaCoverage: ReturnType<typeof reproductiveDnaCoverageForReport>;
  relevantDomains: typeof cycleSupport.domains;
  diaryReview: CycleDiaryReview | null;
}

export interface PgxInterpretationGuidance {
  policy: typeof pgxDiplotypeGuidance.policy;
  relevantGenes: typeof pgxDiplotypeGuidance.genes;
  componentCoverage: PgxComponentCoverage[];
  doNotDo: typeof pgxDiplotypeGuidance.do_not_do;
}

export interface SupplementSafetyGuidance {
  principles: typeof supplementSafety.principles;
  relevantRules: typeof supplementSafety.rules;
}

export interface FoodSafetyGuidance {
  priorityOrder: typeof dietaryRequirements.priority_order;
  explicitExclusions: string[];
  confirmedAllergies: string[];
  suspectedAllergies: string[];
  relevantRules: typeof dietaryRequirements.rules;
  notes: string[];
  conflictNotes: string[];
  suppressedSuggestions: string[];
}

export interface AllergyDnaContext {
  id: string;
  label: string;
  signal_label: string;
  summary: string;
  relevance: string;
  evidence_label: string;
  examples: string[];
  next_step: string;
  matched_marker_count: number;
  matched_marker_ids: string[];
  matched_marker_link_ids: string[];
  matched_genes: string[];
  sources: string[];
}

export interface AllergyMedicationSafetyItem {
  id: string;
  label: string;
  signal_label: string;
  medications: string[];
  summary: string;
  relevance: string;
  next_step: string;
  evidence_label: string;
  matched_marker_count: number;
  matched_marker_ids: string[];
  matched_marker_link_ids: string[];
  matched_genes: string[];
  sources: string[];
}

export interface AllergySensitivityGuidance {
  dnaContexts: AllergyDnaContext[];
  medicationSafety: AllergyMedicationSafetyItem[];
  exposureChecklists: typeof allergySensitivityCatalog.exposure_checklists;
  matchedMarkerCount: number;
  hasDnaSignal: boolean;
}

export interface PersonalContextGuidance {
  medications: string[];
  supplements: string[];
  allergies: string[];
  symptoms: string[];
  labObservations: string[];
  dietaryProfile?: PersonalDietaryProfile;
  reproductiveIntake?: ReproductiveIntakeValues;
  cycleDiary?: CycleDiaryEntry[];
  priorityNotes: string[];
}

export interface ActionablePlan {
  topFindings: TopFinding[];
  /** Named condition and health-pattern summaries derived from matched DNA signals. */
  conditionEvidence: ConditionEvidenceSummary[];
  diet: DietaryGuidance;
  /** Fully qualified mixed-domain rule guidance for Clinical/AI consumers. */
  advancedGuidance: string[];
  supplements: SupplementItem[];
  supplementAvoid: SupplementAvoidItem[];
  labTests: LabTest[];
  labGroups: LabTestGroup[];
  activity: ActivityGuidance;
  medication: MedicationSafetyGuidance;
  medicationPathways: MedicationPathway[];
  cycleSupport: CycleSupportGuidance;
  pgxGuidance: PgxInterpretationGuidance;
  supplementSafety: SupplementSafetyGuidance;
  foodSafety: FoodSafetyGuidance;
  allergy: AllergySensitivityGuidance;
  personalContext: PersonalContextGuidance;
  /** Guardrails shown with every generated actionability plan. */
  safetyNotes: string[];
}

export interface ActionabilityContext {
  /** User-supplied applicability context; never inferred from genotype or chromosome calls. */
  reproductiveContext?: string;
  /** Per-profile context used only to prioritize safety questions and follow-up. */
  personalSafetyContext?: PersonalSafetyContext;
}

export type ActionabilityClass =
  | 'general_wellness'
  | 'symptom_or_lab_conditioned'
  | 'clinical_confirmation';

export type PriorityUrgency = 'safety' | 'clinical' | 'discussion' | 'research' | 'favorable';
export type PriorityTone = 'danger' | 'warning' | 'info' | 'positive';

export interface PriorityMetadata {
  reason: string;
  urgency: PriorityUrgency;
  tone: PriorityTone;
  /** Internal ordering band; not a probability, confidence, or risk score. */
  band: number;
}

const TOP_FINDINGS_LIMIT = 9;

interface ActionableRule {
  id?: string;
  genes: string[];
  /** Optional exact curated marker IDs for drug/allele-specific routing. */
  marker_ids?: string[];
  actionability_class?: ActionabilityClass;
  severity_classes?: SeverityClass[];
  interpretation_contains?: string[];
  /** Resource-authored food and beverage ideas for the Dietary Alignment card. */
  dietary_favor?: string[];
  /** Resource-authored food and beverage limits for the Dietary Alignment card. */
  dietary_avoid?: string[];
  /** Resource-authored supplements worth considering for this matched DNA context. */
  supplement_favor?: {
    name: string;
    reason?: string;
  }[];
  /** Resource-authored supplements or concentrated products to avoid or confirm first. */
  supplement_avoid?: {
    name: string;
    reason?: string;
  }[];
  favor?: string[];
  avoid?: string[];
  supplements?: string[];
  lab_tests?: {
    name: string;
    urgency: 'routine' | 'consider' | 'urgent';
    requires_counselor?: boolean;
  }[];
  medication_context?: string[];
  notes?: string;
}

interface RecommendationAccumulator {
  name: string;
  category: RecommendationCategory;
  basis_rule_ids: Set<string>;
  basis_topic_ids: Set<string>;
  basis_marker_ids: Set<string>;
  basis_marker_link_ids: Set<string>;
  basis_genes: Set<string>;
  evidence_tiers: Set<string>;
  actionability_classes: Set<ActionabilityClass>;
  reasons: Set<string>;
}

function recommendationEvidenceLevel(evidenceTiers: Iterable<string>): string {
  const strength = Array.from(evidenceTiers).reduce((best, tier) => {
    const normalized = String(tier || '').trim().toUpperCase();
    const value = normalized.startsWith('A') ? 4
      : normalized.startsWith('B') ? 3
        : normalized.startsWith('C') ? 2
          : normalized.startsWith('D') ? 1
            : 0;
    return Math.max(best, value);
  }, 0);
  if (strength >= 4) return 'Higher evidence';
  if (strength === 3) return 'Moderate evidence';
  if (strength === 2) return 'Limited evidence';
  if (strength === 1) return 'Research context';
  return 'Evidence not graded';
}

function recommendationBasisLabel(
  genes: string[],
  markerCount: number,
  evidenceLevel: string,
): string {
  const pathway = genes.length > 0 ? genes.join(' · ') : 'curated pathway';
  const markerLabel = `${markerCount} ${markerCount === 1 ? 'marker' : 'markers'}`;
  return `${pathway} · ${markerLabel} · ${evidenceLevel}`;
}

function recommendationWhenRelevant(actionabilityClass: ActionabilityClass): string {
  if (actionabilityClass === 'clinical_confirmation') {
    return 'Most relevant when this pathway or related clinical testing is being reviewed.';
  }
  if (actionabilityClass === 'symptom_or_lab_conditioned') {
    return 'Most relevant when related symptoms, goals, or laboratory results are present.';
  }
  return 'Most relevant to your goals and usual diet or routine.';
}

function addRecommendation(
  map: Map<string, RecommendationAccumulator>,
  category: RecommendationCategory,
  name: string,
  rule: ActionableRule,
  matchingMarkers: { marker: EvaluatedMarker; sectionName: string }[],
  reason?: string,
): void {
  const cleanName = name.trim();
  if (!cleanName) return;
  const ruleId = String(rule.id || rule.genes.join('_')).trim();
  const key = `${category}:${normalizedDietaryTerm(cleanName)}`;
  const current = map.get(key) || {
    name: cleanName,
    category,
    basis_rule_ids: new Set<string>(),
    basis_topic_ids: new Set<string>(),
    basis_marker_ids: new Set<string>(),
    basis_marker_link_ids: new Set<string>(),
    basis_genes: new Set<string>(),
    evidence_tiers: new Set<string>(),
    actionability_classes: new Set<ActionabilityClass>(),
    reasons: new Set<string>(),
  } satisfies RecommendationAccumulator;
  current.basis_rule_ids.add(ruleId);
  // Actionability rule IDs are the stable topic/pathway IDs for this guidance
  // surface. They remain machine-readable while the UI uses genes and copy.
  current.basis_topic_ids.add(ruleId);
  matchingMarkers.forEach(({ marker }) => {
    if (marker.rsid) current.basis_marker_ids.add(marker.rsid);
    if (marker.link_id || marker.rsid) current.basis_marker_link_ids.add(marker.link_id || marker.rsid);
    normalizedGeneSymbols(marker.gene).forEach((gene) => current.basis_genes.add(gene));
    if (marker.evidence_tier) current.evidence_tiers.add(marker.evidence_tier);
  });
  current.actionability_classes.add(
    rule.actionability_class || ACTIONABILITY_POLICY.default_actionability || 'symptom_or_lab_conditioned',
  );
  if (reason?.trim()) current.reasons.add(reason.trim());
  map.set(key, current);
}

function recommendationItems(
  map: Map<string, RecommendationAccumulator>,
): RecommendationItem[] {
  return Array.from(map.values())
    .map((item) => {
      const genes = Array.from(item.basis_genes).sort();
      const ruleIds = Array.from(item.basis_rule_ids).sort();
      const reasons = Array.from(item.reasons).filter(Boolean).sort();
      const actionabilityClass = item.actionability_classes.has('clinical_confirmation')
        ? 'clinical_confirmation'
        : item.actionability_classes.has('symptom_or_lab_conditioned')
          ? 'symptom_or_lab_conditioned'
          : 'general_wellness';
      const evidenceLevel = recommendationEvidenceLevel(item.evidence_tiers);
      return {
        recommendation_id: `${item.category}:${normalizedDietaryTerm(item.name).replace(/[^a-z0-9]+/g, '-')}`,
        name: item.name,
        category: item.category,
        basis_rule_ids: ruleIds,
        basis_topic_ids: Array.from(item.basis_topic_ids).sort(),
        basis_marker_ids: Array.from(item.basis_marker_ids).sort(),
        basis_marker_link_ids: Array.from(item.basis_marker_link_ids).sort(),
        basis_genes: genes,
        basis_marker_count: item.basis_marker_ids.size,
        basis_rule_count: item.basis_rule_ids.size,
        dna_basis: recommendationBasisLabel(genes, item.basis_marker_ids.size, evidenceLevel),
        why_it_appears: reasons.length > 0
          ? reasons.join(' ')
          : `Included because your report matched the ${genes.join('/')} pathway.`,
        evidence_level: evidenceLevel,
        when_relevant: recommendationWhenRelevant(actionabilityClass),
        conflicts: [],
      };
    })
    .sort((left, right) => {
      const evidenceRank = (value: string) => value === 'Higher evidence' ? 4
        : value === 'Moderate evidence' ? 3
          : value === 'Limited evidence' ? 2
            : value === 'Research context' ? 1
              : 0;
      return evidenceRank(right.evidence_level) - evidenceRank(left.evidence_level)
        || right.basis_marker_count - left.basis_marker_count
        || right.basis_rule_count - left.basis_rule_count
        || left.name.localeCompare(right.name);
    });
}

function activityMarkerIsKnown(marker: EvaluatedMarker): boolean {
  return isVerifiedAssertionStatus(marker.assertion_status)
    && marker.interpretation_allowed !== false
    && isCallableGenotype(marker.user_genotype);
}

function activityDomainMatchesMarker(
  domain: ActivityGuardrailDomain,
  marker: EvaluatedMarker,
  sectionName: string,
): boolean {
  const haystack = `${sectionName} ${marker.gene} ${marker.variant_name || ''}`.toLowerCase();
  const sectionKeywords = Array.isArray(domain.section_keywords) ? domain.section_keywords : [];
  const relevantSignals = Array.isArray((domain as unknown as { relevant_pack_signals?: string[] }).relevant_pack_signals)
    ? (domain as unknown as { relevant_pack_signals: string[] }).relevant_pack_signals
    : [];
  return [...sectionKeywords, ...relevantSignals.map((signal) => signal.replace(/_/g, ' '))]
    .some((keyword) => haystack.includes(String(keyword).toLowerCase()));
}

function activityEvidenceLevel(markers: EvaluatedMarker[]): string {
  return recommendationEvidenceLevel(markers.map((marker) => marker.evidence_tier));
}

function buildActivityRecommendations(
  domains: ActivityGuardrailDomain[],
  allMarkers: { marker: EvaluatedMarker; sectionName: string }[],
): { domains: ActivityDomain[]; recommendations: ActivityRecommendationItem[] } {
  const itemMaps = new Map<string, RecommendationAccumulator & { kind: ActivityRecommendationKind }>();
  const enrichedDomains: ActivityDomain[] = domains.map((domain) => {
    const matched = allMarkers
      .filter(({ marker, sectionName }) => activityMarkerIsKnown(marker) && activityDomainMatchesMarker(domain, marker, sectionName));
    const matchedMarkerIds = Array.from(new Set(matched.map(({ marker }) => marker.link_id || marker.rsid).filter(Boolean))).sort();
    const matchedGenes = Array.from(new Set(matched.flatMap(({ marker }) => normalizedGeneSymbols(marker.gene)))).sort();
    const add = (kind: ActivityRecommendationKind, value: string) => {
      const name = value.trim();
      if (!name || matched.length === 0) return;
      const key = `${kind}:${normalizedDietaryTerm(name)}`;
      const current = itemMaps.get(key) || {
        name,
        category: 'activity',
        kind,
        basis_rule_ids: new Set<string>(),
        basis_topic_ids: new Set<string>(),
        basis_marker_ids: new Set<string>(),
        basis_marker_link_ids: new Set<string>(),
        basis_genes: new Set<string>(),
        evidence_tiers: new Set<string>(),
        actionability_classes: new Set<ActionabilityClass>(['general_wellness']),
        reasons: new Set<string>(),
      };
      current.basis_rule_ids.add(domain.id);
      current.basis_topic_ids.add(domain.id);
      matched.forEach(({ marker }) => {
        current.basis_marker_ids.add(marker.link_id || marker.rsid);
        current.basis_marker_link_ids.add(marker.link_id || marker.rsid);
        normalizedGeneSymbols(marker.gene).forEach((gene) => current.basis_genes.add(gene));
        current.evidence_tiers.add(marker.evidence_tier);
      });
      current.reasons.add(`Matched ${domain.label} DNA context.`);
      itemMaps.set(key, current);
    };
    domain.simple_favor?.forEach((value) => add('build', value));
    domain.simple_watch?.forEach((value) => add('watch', value));
    domain.simple_verify?.forEach((value) => add('verify', value));
    return { ...domain, matched_marker_ids: matchedMarkerIds, matched_genes: matchedGenes };
  });

  const recommendations = Array.from(itemMaps.values())
    .map((item) => ({
      recommendation_id: `activity:${item.kind}:${normalizedDietaryTerm(item.name).replace(/[^a-z0-9]+/g, '-')}`,
      name: item.name,
      category: 'activity' as const,
      kind: item.kind,
      basis_rule_ids: Array.from(item.basis_rule_ids).sort(),
        basis_topic_ids: Array.from(item.basis_topic_ids).sort(),
        basis_marker_ids: Array.from(item.basis_marker_ids).sort(),
        basis_marker_link_ids: Array.from(item.basis_marker_ids).sort(),
      basis_genes: Array.from(item.basis_genes).sort(),
      basis_marker_count: item.basis_marker_ids.size,
      basis_rule_count: item.basis_rule_ids.size,
      dna_basis: recommendationBasisLabel(Array.from(item.basis_genes).sort(), item.basis_marker_ids.size, activityEvidenceLevel(allMarkers
        .filter(({ marker }) => item.basis_marker_ids.has(marker.link_id || marker.rsid))
        .map(({ marker }) => marker))),
      why_it_appears: Array.from(item.reasons).sort().join(' ') || 'Included because the report matched a related DNA pathway.',
      evidence_level: activityEvidenceLevel(allMarkers
        .filter(({ marker }) => item.basis_marker_ids.has(marker.link_id || marker.rsid))
        .map(({ marker }) => marker)),
      when_relevant: 'Most relevant when this pathway matches your training goals, symptoms, or recovery history.',
      conflicts: [],
    }))
    .sort((left, right) => left.kind.localeCompare(right.kind)
      || right.basis_marker_count - left.basis_marker_count
      || left.name.localeCompare(right.name));
  return { domains: enrichedDomains, recommendations };
}

interface LabCategory {
  id: string;
  label: string;
  order: number;
  keywords: string[];
  fallback?: boolean;
}

interface LabCatalogEntry {
  id: string;
  name: string;
  category: string;
  aliases: string[];
  purpose: string;
}

interface LabTier {
  id: LabTest['tier'];
  label: string;
  hint: string;
  order: number;
}

interface LabConfirmationFilter {
  allowed_keywords: string[];
  blocked_keywords: string[];
}

function normalizedGeneSymbols(value: string): string[] {
  return String(value || '')
    .split(/[\s/]+/)
    .map((gene) => gene.trim().toUpperCase())
    .filter(Boolean);
}

function actionabilityRuleMatchesMarker(rule: ActionableRule, marker: EvaluatedMarker): boolean {
  const ruleGenes = new Set(rule.genes.map((gene) => gene.trim().toUpperCase()));
  const geneMatches = normalizedGeneSymbols(marker.gene).some((gene) => ruleGenes.has(gene));
  if (!geneMatches) return false;
  if (!rule.marker_ids?.length) return true;
  const markerIds = new Set(rule.marker_ids.map((id) => id.trim().toLowerCase()));
  return markerIds.has(String(marker.rsid || '').trim().toLowerCase());
}

const MEDICATION_PATHWAY_LABELS: Record<string, string> = {
  cyp2c19_clopidogrel: 'Clopidogrel',
  cyp2c9_vkorc1_warfarin: 'Warfarin',
  slco1b1_statin_safety: 'Statins',
  cyp2d6_opioid_context: 'Codeine & tramadol',
  cyp2d6_tamoxifen_context: 'Tamoxifen',
  cyp2d6_cyp2c19_ssri_context: 'SSRIs & related antidepressants',
  dpyd_fluoropyrimidine_safety: 'Fluorouracil & capecitabine',
  tpmt_nudt15_thiopurine_safety: 'Thiopurines',
  hla_drug_hypersensitivity_safety: 'HLA drug-sensitivity pathways',
  hla_b5701_abacavir_safety: 'Abacavir',
  hla_carbamazepine_oxcarbazepine_safety: 'Carbamazepine & oxcarbazepine',
  hla_b5801_allopurinol_safety: 'Allopurinol',
  g6pd_oxidative_medication_safety: 'Oxidative-stress medicines',
  ryr1_anesthesia_safety: 'Anesthesia & malignant-hyperthermia pathway',
  cyp2c9_nsaid_context: 'NSAIDs',
  cyp3a5_tacrolimus_context: 'Tacrolimus',
  bche_succinylcholine_anesthesia: 'Succinylcholine & mivacurium',
  ugt1a1_irinotecan_safety: 'Irinotecan',
  nat2_hydralazine_context: 'Hydralazine',
  cyp2c19_proton_pump_inhibitor_context: 'Proton-pump inhibitors',
  cyp2b6_efavirenz_context: 'Efavirenz',
  cyp2c9_phenytoin_safety: 'Phenytoin',
  hla_b1502_phenytoin_safety: 'HLA phenytoin pathway',
  b12_status_context: 'B12 status & medicines',
  thrombophilia_estrogen_context: 'Estrogen-containing medicines',
  atherogenic_lipid_context: 'Lipid-lowering medicines',
  thyroid_function_context: 'Thyroid medicines',
  sleep_airway_context: 'Sleep & alertness medicines',
  allergy_food_reaction_context: 'Allergy & airway medicines',
  bone_mineral_density_context: 'Bone-health medicines',
  digestive_inflammatory_context: 'GI & immune medicines',
  celiac_hla_context: 'Celiac-related medication context',
  bone_rare_disorder_panel_context: 'Rare bone-disorder medicines',
  hypophosphatasia_alpl_panel_context: 'Hypophosphatasia medication context',
  hereditary_fructose_intolerance_panel_context: 'Hereditary fructose intolerance medication context',
  ttr_val142ile_amyloid_confirmation: 'Amyloidosis medicines',
  arrhythmogenic_cardiomyopathy_panel_context: 'Cardiomyopathy medicines',
  chek2_cancer_confirmation: 'Cancer-treatment context',
  mutyh_cancer_confirmation: 'Polyposis-treatment context',
  hoxb13_prostate_cancer_context: 'Prostate-cancer treatment context',
  pmdd_ovarian_steroid_sensitivity_context: 'PMDD & ovarian-steroid medicines',
  pcos_reproductive_metabolic_context: 'PCOS & metabolic medicines',
  endometriosis_symptom_context: 'Endometriosis pain & hormone medicines',
  adenomyosis_research_gap_context: 'Adenomyosis symptom-treatment context',
  mast_cell_mediator_clinical_context: 'Mast-cell medicines',
  hereditary_angioedema_red_flag_context: 'Angioedema medicines',
  alpha1_antitrypsin_lung_liver_confirmation: 'Lung & liver medicines',
  apol1_kidney_risk_context: 'Kidney medicines',
};

function medicationMarkerText(marker: EvaluatedMarker): string {
  const annotations = [
    ...(marker.pharmgkb_annotations || []),
    ...(marker.pharmgkb ? [marker.pharmgkb] : []),
  ];
  return [
    marker.variant_name,
    marker.impact,
    marker.interpretation,
    ...annotations.flatMap((annotation) => [annotation.drug, annotation.phenotype]),
  ]
    .filter(Boolean)
    .join(' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function medicationDirectionLabel(markers: EvaluatedMarker[]): string {
  const labels = markers.map((marker) => {
    const copy = getSimpleFindingCopy(marker, getLaypersonTranslation(marker));
    return copy.direction_label;
  });
  const unique = [...new Set(labels)];
  if (unique.length === 1 && unique[0]) return unique[0];
  if (unique.some((label) => label.includes('Lower-dose')) && unique.some((label) => label.includes('Higher-dose'))) {
    return 'Mixed lower-/higher-dose components';
  }
  if (unique.some((label) => label.includes('Reduced-function')) && unique.some((label) => label.includes('Increased-function'))) {
    return 'Mixed reduced-/increased-function components';
  }
  if (unique.some((label) => label.includes('Possible sensitivity'))) return 'Possible sensitivity signal';
  return 'Direction incomplete from these markers';
}

function medicationPathwayLabel(rule: ActionableRule, markers: EvaluatedMarker[] = []): string {
  const base = rule.id && MEDICATION_PATHWAY_LABELS[rule.id]
    ? MEDICATION_PATHWAY_LABELS[rule.id]
    : 'Medication processing';
  const direction = medicationDirectionLabel(markers);
  return `${base} — ${direction.charAt(0).toLowerCase()}${direction.slice(1)}`;
}

function compactMedicationPathwayDetail(text: string): string {
  const compact = String(text || '')
    .replace(/\s*;\s*(?:but|and raw DNA|do not|never|a consumer|the prescribing|a genotype|raw DNA cannot).*$/i, '')
    .replace(/\s+(?:and\s+)?raw DNA.*$/i, '')
    .replace(/\s+Do not .*$/i, '')
    .replace(/\s+Never .*$/i, '')
    .trim();
  return compact || 'Review this medication pathway when the named medication or treatment question is relevant.';
}

interface ActionabilityPolicy {
  default_actionability?: ActionabilityClass;
  confirm_with_cap?: number;
  safety_notes?: string[];
}

const ACTIONABILITY_POLICY: ActionabilityPolicy =
  (guidanceDoc as { policy?: ActionabilityPolicy }).policy || {};

const ACTIONABLE_RULES: ActionableRule[] = Array.isArray(
  (guidanceDoc as { rules?: ActionableRule[] }).rules
)
  ? ((guidanceDoc as { rules: ActionableRule[] }).rules)
  : [];

function rankingRuleMatchesMarker(rule: ActionableRule, marker: EvaluatedMarker): boolean {
  if (!actionabilityRuleMatchesMarker(rule, marker)) return false;

  if (rule.severity_classes && !rule.severity_classes.includes(marker.severity_class)) {
    return false;
  }

  if (rule.interpretation_contains) {
    const interpretation = (marker.interpretation || '').toLowerCase();
    if (!rule.interpretation_contains.some((pattern) => interpretation.includes(pattern.toLowerCase()))) {
      return false;
    }
  }

  if (!rule.severity_classes && !rule.interpretation_contains) {
    if (marker.effect_count != null && marker.effect_count === 0) return false;
    if (marker.severity_class === 'benign' || marker.severity_class === 'protective') return false;
  }

  return true;
}

/**
 * Rank discussion prompts by useful follow-up value, not just association
 * severity. A common marker with a named lab, medication, or confirmation
 * route should rise above a generic research association. This remains a
 * prioritization aid; it never changes the finding's evidence or meaning.
 */
function actionabilityRankingBoost(marker: EvaluatedMarker): number {
  let boost = 0;
  if (marker.clinical_confirmation_required) boost += 28;
  if (marker.confirm_with.length > 0) boost += 14;

  for (const rule of ACTIONABLE_RULES) {
    if (!rankingRuleMatchesMarker(rule, marker)) continue;
    if (rule.lab_tests?.length) boost += 18;
    if (rule.medication_context?.length) boost += 16;
    if (rule.supplement_favor?.length || rule.supplement_avoid?.length) boost += 8;
    if (rule.dietary_favor?.length || rule.dietary_avoid?.length) boost += 5;
    if (rule.actionability_class === 'clinical_confirmation') boost += 12;
    else if (rule.actionability_class === 'symptom_or_lab_conditioned') boost += 6;
  }

  return Math.min(boost, 60);
}

function prioritySeverityScore(marker: EvaluatedMarker): number {
  if (marker.severity_class === 'high_risk') return 4;
  if (marker.severity_class === 'confirmation_required') return 3;
  if (marker.severity_class === 'moderate_risk') return 2;
  if (marker.severity_class === 'low_risk') return 1;
  return 0;
}

function evidenceStrengthScore(marker: EvaluatedMarker): number {
  const tier = String(marker.evidence_tier || '').trim().charAt(0).toUpperCase();
  if (tier === 'A') return 3;
  if (tier === 'B') return 2;
  if (tier === 'C') return 1;
  return 0;
}

function authoredMarkerText(marker: EvaluatedMarker): string {
  return [marker.variant_name, marker.impact, marker.interpretation]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
}

function isMedicationSafetyMarker(marker: EvaluatedMarker): boolean {
  const text = authoredMarkerText(marker);
  return Boolean(priorityGroupForMarker(marker))
    || /\b(?:allerg(?:y|ic|ies)|hypersensitivity|HLA[-*]|drug[- ]specific|medication safety|malignant hyperthermia|anesthesia|oxidative[- ]stress medicine|before exposure)\b/.test(text);
}

/**
 * Give the queue a useful reason for ranking instead of presenting a generic
 * severity label. Actionability comes first; authored severity and evidence
 * refine the order within an actionability band. This deliberately does not
 * produce a disease probability or certainty claim.
 */
export function priorityMetadataForMarker(marker: EvaluatedMarker): PriorityMetadata {
  if (marker.effect_direction === 'protective') {
    return {
      reason: 'Potentially favorable context',
      urgency: 'favorable',
      tone: 'positive',
      band: 1,
    };
  }

  if (marker.clinical_confirmation_required && isMedicationSafetyMarker(marker)) {
    return {
      reason: 'Medication safety check',
      urgency: 'safety',
      tone: 'danger',
      band: 6,
    };
  }

  if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
    return {
      reason: 'Clinical review needed',
      urgency: 'clinical',
      tone: 'danger',
      band: 5,
    };
  }

  if (marker.confirm_with.length > 0 || marker.severity_class === 'high_risk' || marker.severity_class === 'moderate_risk') {
    return {
      reason: 'Review this signal',
      urgency: 'discussion',
      tone: 'warning',
      band: evidenceStrengthScore(marker) > 0 ? 4 : 3,
    };
  }

  if (marker.effect_direction === 'risk' || marker.severity_class === 'low_risk') {
    return {
      reason: 'Research signal',
      urgency: 'research',
      tone: 'info',
      band: evidenceStrengthScore(marker) > 0 ? 2 : 1,
    };
  }

  return {
    reason: 'Research signal',
    urgency: 'research',
    tone: 'info',
    band: 1,
  };
}

function hasKnownMarkerCall(marker: EvaluatedMarker): boolean {
  return isVerifiedAssertionStatus(marker.assertion_status)
    && isCallableGenotype(marker.user_genotype);
}

/**
 * Keep the first-screen queue focused on findings with meaningful review
 * value. Benign/no-effect markers and authored guardrail language remain in
 * the complete report, but should not consume a concern slot.
 */
function isExcludedFromPriorityQueue(marker: EvaluatedMarker): boolean {
  if (!hasKnownMarkerCall(marker)) return true;
  if (marker.effect_count === 0) return true;
  if (marker.severity_class === 'benign' || marker.severity_class === 'protective' || marker.severity_class === 'no_data') {
    return true;
  }

  // A high-looking label without a concrete follow-up is not useful as a
  // first-screen concern. Keep the complete finding in its health-area
  // section, but reserve the queue for evidence or an authored route.
  const hasConcreteRoute = marker.confirm_with.some((item) =>
    !/^(?:clinical|medical|doctor|clinician|healthcare|specialist)\s+(?:review|confirmation|testing|evaluation)$/i.test(item.trim()),
  );
  if ((marker.severity_class === 'high_risk' || marker.severity_class === 'moderate_risk')
      && evidenceStrengthScore(marker) === 0
      && !hasConcreteRoute
      && actionabilityRankingBoost(marker) === 0) {
    return true;
  }

  const authoredText = authoredMarkerText(marker);
  return /\b(?:harmless|benign)\b/.test(authoredText)
    && /\b(?:cancer|tumou?r|predisposition|dna repair)\b/.test(authoredText);
}

/**
 * Several clinical PGx SNPs describe one medication-processing pathway. Keep
 * them together in the Simple queue so users see one useful topic rather
 * than a list of allele components. Clinical and technical sections still
 * show every individual marker.
 */
function priorityGroupForMarker(marker: EvaluatedMarker): string | undefined {
  if (!marker.clinical_confirmation_required) return undefined;

  const authoredText = medicationMarkerText(marker).toLowerCase();
  const isMedicationMarker = /\b(?:clinical pgx|warfarin|medication|drug|dose|haplotype|allele|star)\b/.test(authoredText);
  if (!isMedicationMarker) return undefined;

  const medicationTopics: Array<[string, RegExp]> = [
    ['warfarin', /warfarin/],
    ['clopidogrel', /clopidogrel/],
    ['statin', /statin|SLCO1B1/],
    ['thiopurine', /thiopurine|TPMT|NUDT15/],
    ['fluoropyrimidine', /fluorouracil|capecitabine|fluoropyrimidine|DPYD/],
    ['tacrolimus', /tacrolimus|CYP3A5/],
    ['HLA-drug-safety', /hypersensitivity|severe skin|HLA[-*]|drug-specific/],
  ];
  const medicationTopic = medicationTopics.find(([, pattern]) => pattern.test(authoredText))?.[0];
  if (medicationTopic) return `clinical-medication:${medicationTopic}`;

  const genes = normalizedGeneSymbols(marker.gene).sort().join('/');
  return genes ? `clinical-medication:${genes}` : undefined;
}

const URGENCY_RANK = { routine: 0, consider: 1, urgent: 2 } as const;

function qualifyGuidance(
  text: string,
  actionabilityClass: ActionabilityClass,
  kind: 'favor' | 'avoid' | 'supplement'
): string {
  const clean = text.trim();
  if (!clean) return clean;

  if (kind === 'supplement') {
    return `Discuss with a clinician or pharmacist before starting: ${clean}`;
  }

  switch (actionabilityClass) {
    case 'clinical_confirmation':
      return kind === 'avoid'
        ? `Do not make this change from raw DNA; confirm the finding clinically first: ${clean}`
        : `Only consider after clinical confirmation and individualized advice: ${clean}`;
    case 'symptom_or_lab_conditioned':
      return kind === 'avoid'
        ? `Do not make this change unless symptoms, labs, or clinician guidance support it: ${clean}`
        : `Consider only if symptoms, labs, or personal goals support it: ${clean}`;
    case 'general_wellness':
    default:
      return kind === 'avoid'
        ? `General health consideration, not a genotype-specific restriction: ${clean}`
        : `General low-risk option, not a genotype prescription: ${clean}`;
  }
}

const LAB_TIER_META = Object.fromEntries(
  (((guidanceDoc as { lab_tiers?: LabTier[] }).lab_tiers || [])
    .filter((tier) => tier && typeof tier.id === 'string' && typeof tier.label === 'string')
    .map((tier) => [tier.id, tier]))
) as Record<LabTest['tier'], LabTier>;

const LAB_CONFIRMATION_FILTER: LabConfirmationFilter =
  (guidanceDoc as { lab_confirmation_filter?: LabConfirmationFilter }).lab_confirmation_filter || {
    allowed_keywords: [],
    blocked_keywords: [],
  };

const LAB_CATEGORIES: LabCategory[] = Array.isArray(
  (guidanceDoc as { lab_categories?: LabCategory[] }).lab_categories
)
  ? ((guidanceDoc as { lab_categories: LabCategory[] }).lab_categories)
      .filter((category) => category && typeof category.label === 'string' && Array.isArray(category.keywords))
      .sort((left, right) => left.order - right.order)
  : [];

const LAB_CATALOG: LabCatalogEntry[] = Array.isArray(
  (guidanceDoc as { lab_catalog?: LabCatalogEntry[] }).lab_catalog,
)
  ? (guidanceDoc as { lab_catalog: LabCatalogEntry[] }).lab_catalog
      .filter((entry) => entry
        && typeof entry.id === 'string'
        && typeof entry.name === 'string'
        && typeof entry.category === 'string'
        && Array.isArray(entry.aliases)
        && typeof entry.purpose === 'string')
  : [];

function normalizeLabName(name: string): string {
  return String(name || '')
    .toLowerCase()
    .replace(/\[[^\]]*\]/g, (value) => ` ${value.replace(/[\[\]]/g, ' ')} `)
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
    .replace(/\s+/g, ' ');
}

const LAB_CATALOG_BY_ALIAS = new Map<string, LabCatalogEntry>();
for (const entry of LAB_CATALOG) {
  for (const alias of [entry.name, ...entry.aliases]) {
    const key = normalizeLabName(alias);
    if (key && !LAB_CATALOG_BY_ALIAS.has(key)) LAB_CATALOG_BY_ALIAS.set(key, entry);
  }
}

function labCatalogEntry(name: string): LabCatalogEntry | undefined {
  return LAB_CATALOG_BY_ALIAS.get(normalizeLabName(name));
}

function customLabId(name: string): string {
  const slug = normalizeLabName(name).replace(/ /g, '_');
  return `custom:${slug || 'unnamed'}`;
}

/** Resolve a display name to the resource-backed identity used for deduplication. */
export function canonicalizeLabName(name: string): Pick<LabTest, 'canonical_id' | 'name' | 'purpose' | 'category'> {
  const entry = labCatalogEntry(name);
  if (entry) {
    return {
      canonical_id: entry.id,
      name: entry.name,
      purpose: entry.purpose,
      category: entry.category,
    };
  }
  return {
    canonical_id: customLabId(name),
    name: String(name || '').trim() || 'Unspecified follow-up',
    purpose: 'Purpose depends on the matched DNA pathway and clinical question.',
    category: inferLabCategory(name),
  };
}

function inferLabCategory(name: string): string {
  const catalog = labCatalogEntry(name);
  if (catalog) return catalog.category;
  const lower = name.toLowerCase();
  const match = LAB_CATEGORIES.find((category) =>
    !category.fallback && category.keywords.some((keyword) => lower.includes(keyword.toLowerCase()))
  );
  return match?.label || LAB_CATEGORIES.find((category) => category.fallback)?.label || 'Other follow-up';
}

function deriveLabTier(
  urgency: LabTest['urgency'],
  requiresCounselor: boolean
): LabTest['tier'] {
  if (requiresCounselor) return 'counselor';
  if (urgency === 'routine') return 'optional';
  return 'discuss';
}

/** Only promote pack `confirm_with` strings that look like named labs or imaging — not history/symptoms. */
function isLabLikeConfirmItem(text: string): boolean {
  const lower = text.toLowerCase().trim();
  if (!lower || lower.length < 4) return false;

  if (LAB_CONFIRMATION_FILTER.blocked_keywords.some((keyword) => lower.includes(keyword.toLowerCase()))) return false;
  if (LAB_CONFIRMATION_FILTER.allowed_keywords.some((keyword) => lower.includes(keyword.toLowerCase()))) return true;
  return false;
}

function buildLabGroups(tests: LabTest[]): LabTestGroup[] {
  const byTier = new Map<LabTest['tier'], LabTest[]>();
  for (const test of tests) {
    const list = byTier.get(test.tier) || [];
    list.push(test);
    byTier.set(test.tier, list);
  }

  return (['counselor', 'discuss', 'optional'] as const)
    .filter((tier) => (byTier.get(tier)?.length ?? 0) > 0)
    .map((tier) => {
      const meta = LAB_TIER_META[tier];
      const grouped = byTier.get(tier) || [];
      grouped.sort((a, b) => a.category.localeCompare(b.category) || a.name.localeCompare(b.name));
      return {
        tier,
        label: meta.label,
        hint: meta.hint,
        tests: grouped,
      };
    });
}

/** Group canonical tests into copyable clinician-request sections. */
export function buildLabRequestList(tests: readonly LabTest[]): LabRequestGroup[] {
  const byCategory = new Map<string, LabTest[]>();
  const seenIds = new Set<string>();
  for (const test of tests) {
    if (seenIds.has(test.canonical_id)) continue;
    seenIds.add(test.canonical_id);
    const list = byCategory.get(test.category) || [];
    list.push(test);
    byCategory.set(test.category, list);
  }
  return Array.from(byCategory.entries())
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([category, categoryTests]) => ({
      category,
      tests: [...categoryTests].sort((left, right) => left.name.localeCompare(right.name)),
    }));
}

/** Plain text request list for clinician handoff and clipboard-friendly UI. */
export function buildLabRequestListText(tests: readonly LabTest[]): string {
  const groups = buildLabRequestList(tests);
  if (groups.length === 0) return 'No DNA-linked lab follow-ups were generated.';
  return groups.map((group) => [
    group.category,
    ...group.tests.map((test) => `- ${test.name} — ${test.reason}`),
  ].join('\n')).join('\n\n');
}

interface LabAccumulator {
  canonical_id: string;
  name: string;
  purpose: string;
  category: string;
  reasons: Set<string>;
  basis_rule_ids: Set<string>;
  basis_marker_ids: Set<string>;
  basis_genes: Set<string>;
  urgency: 'routine' | 'consider' | 'urgent';
  requires_counselor: boolean;
}

function upsertLab(
  map: Map<string, LabAccumulator>,
  name: string,
  reason: string,
  urgency: 'routine' | 'consider' | 'urgent',
  requiresCounselor = false,
  provenance: {
    ruleId?: string;
    markerIds?: string[];
    genes?: string[];
  } = {},
) {
  const identity = canonicalizeLabName(name);
  const existing = map.get(identity.canonical_id);
  if (existing) {
    if (URGENCY_RANK[urgency] > URGENCY_RANK[existing.urgency]) {
      existing.urgency = urgency;
    }
    existing.requires_counselor = existing.requires_counselor || requiresCounselor;
    if (reason.trim()) existing.reasons.add(reason.trim());
    if (provenance.ruleId) existing.basis_rule_ids.add(provenance.ruleId);
    provenance.markerIds?.filter(Boolean).forEach((id) => existing.basis_marker_ids.add(id));
    provenance.genes?.filter(Boolean).forEach((gene) => existing.basis_genes.add(gene));
  } else {
    map.set(identity.canonical_id, {
      ...identity,
      reasons: new Set(reason.trim() ? [reason.trim()] : []),
      basis_rule_ids: new Set(provenance.ruleId ? [provenance.ruleId] : []),
      basis_marker_ids: new Set(provenance.markerIds?.filter(Boolean) || []),
      basis_genes: new Set(provenance.genes?.filter(Boolean) || []),
      urgency,
      requires_counselor: requiresCounselor,
    });
  }
}

function deriveMedicationSafety(
  markers: EvaluatedMarker[],
  sectionNames: string[],
  reproductiveContext?: string,
  personalSafetyContext?: PersonalSafetyContext,
  activeContextIds: readonly string[] = [],
): MedicationSafetyGuidance {
  const structuredMedicationNames = Object.entries(personalSafetyContext?.reproductiveIntake || {})
    .filter(([fieldId]) => safetyGuardrails.medication_context.reproductive_intake_field_ids.includes(fieldId))
    .map(([, value]) => value);
  const context = [
    ...sectionNames,
    ...markers.map((marker) => `${marker.gene} ${marker.variant_name || ''} ${marker.sex_scope || ''}`),
    ...(personalSafetyContext?.medications || []),
    ...structuredMedicationNames,
  ].join(' ').toLowerCase();
  const pgxContext = safetyGuardrails.medication_context.pgx_context_keywords.some((keyword) =>
    context.includes(String(keyword).toLowerCase())
  );
  const relevantRuleIds = new Set(safetyGuardrails.medication_context.base_rule_ids);
  if (pgxContext) {
    for (const ruleId of safetyGuardrails.medication_context.pgx_rule_ids) relevantRuleIds.add(ruleId);
  }

  // A user-supplied hormonal medication name is an explicit medication
  // context signal. It does not establish anatomy, cycle status, or hormone
  // levels; it only makes the composition/label guardrail relevant.
  const medicationNames = [
    ...(personalSafetyContext?.medications || []),
    ...structuredMedicationNames,
  ];
  const medicationMatchesKeywords = (keywords: readonly string[]) => medicationNames.some((name) => {
    const lowerName = String(name).toLowerCase();
    return keywords.some((keyword) => lowerName.includes(String(keyword).toLowerCase()));
  });
  const hasHormonalMedication = medicationMatchesKeywords(
    safetyGuardrails.medication_context.hormone_medication_keywords
  );
  const hasContraceptiveMedication = medicationMatchesKeywords(
    safetyGuardrails.medication_context.contraceptive_medication_keywords
  );
  if (hasContraceptiveMedication) relevantRuleIds.add('CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA');
  if (hasHormonalMedication) relevantRuleIds.add('HORMONE_THERAPY_COMPOSITION_NOT_IN_DNA');
  const relevantContextIds = activeContextIds.length > 0
    ? activeContextIds
    : selectedReproductiveContextOption(reproductiveContext)?.id
      ? [selectedReproductiveContextOption(reproductiveContext)!.id]
      : [];
  for (const option of cycleSupport.context_options) {
    if (!relevantContextIds.includes(option.id)) continue;
    for (const ruleId of option.medication_rule_ids || []) relevantRuleIds.add(ruleId);
  }

  const rules = [
    ...safetyGuardrails.medication_context.do_not_do,
    ...safetyGuardrails.rules
      .filter((rule) => relevantRuleIds.has(rule.id))
      .map((rule) => rule.text),
  ];

  return {
    rules: Array.from(new Set(rules)),
    askFor: safetyGuardrails.medication_context.ask_for,
  };
}

function pgxGeneMatchesMarker(
  gene: typeof pgxDiplotypeGuidance.genes[number],
  marker: EvaluatedMarker,
): boolean {
  const markerGeneSymbols = marker.gene
    .toUpperCase()
    .split(/[\s/]+/)
    .filter(Boolean);
  return gene.marker_ids.includes(marker.rsid)
    || gene.gene_symbols.some((symbol) => markerGeneSymbols.includes(symbol.toUpperCase()));
}

function derivePgxInterpretationGuidance(markers: EvaluatedMarker[]): PgxInterpretationGuidance {
  const relevantGenes = pgxDiplotypeGuidance.genes.filter((gene) =>
    markers.some((marker) => pgxGeneMatchesMarker(gene, marker)),
  );
  return {
    policy: pgxDiplotypeGuidance.policy,
    relevantGenes,
    componentCoverage: derivePgxComponentCoverage(markers),
    doNotDo: pgxDiplotypeGuidance.do_not_do,
  };
}

function selectSupplementSafetyRules(
  markers: EvaluatedMarker[],
  supplementNames: string[],
  medicationNames: string[],
  allergyTerms: string[],
  reproductiveContext?: string,
  activeContextIds: readonly string[] = [],
): typeof supplementSafety.rules {
  const markerGenes = new Set(markers.flatMap((marker) => marker.gene.split(/[\s/]+/).map((gene) => gene.toLowerCase())));
  const supplementContext = supplementNames.join(' ').toLowerCase();
  const medicationContext = medicationNames.join(' ').toLowerCase();
  const allergyContext = allergyTerms.join(' ').toLowerCase();
  const contextIds = activeContextIds.length > 0
    ? activeContextIds
    : selectedReproductiveContextOption(reproductiveContext)?.id
      ? [selectedReproductiveContextOption(reproductiveContext)!.id]
      : [];
  return supplementSafety.rules.filter((rule) => {
    const geneMatch = rule.signal_genes.some((gene) => markerGenes.has(gene.toLowerCase()));
    const termMatch = rule.match_terms.some((term) => supplementContext.includes(term.toLowerCase()));
    const medicationMatch = (rule as typeof rule & { medication_terms?: string[] }).medication_terms?.some((term) =>
      medicationContext.includes(term.toLowerCase())
    ) || false;
    const allergyMatch = (rule as typeof rule & { allergy_terms?: string[] }).allergy_terms?.some((term) =>
      allergyContext.includes(term.toLowerCase())
    ) || false;
    const contextMatch = (rule as typeof rule & { context_ids?: string[] }).context_ids?.some((contextId) => contextIds.includes(contextId)) || false;
    return geneMatch || termMatch || medicationMatch || allergyMatch || contextMatch;
  });
}

type DietaryProfileRoutes = Record<string, Record<string, string[]>>;

const DIETARY_PROFILE_ROUTES = dietaryRequirements.profile_routes as DietaryProfileRoutes;
const DIETARY_CONTEXT_ROUTES = dietaryRequirements.context_routes as Record<string, string[]>;
type DietaryRecommendationConflict = typeof dietaryRequirements.recommendation_conflicts[number];
const DIETARY_RECOMMENDATION_CONFLICTS = dietaryRequirements.recommendation_conflicts as DietaryRecommendationConflict[];

function normalizedDietaryTerm(value: unknown): string {
  return String(value || '')
    .toLocaleLowerCase()
    .replace(/[_-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function recommendationConflictsForProfile(
  dietaryProfile: PersonalDietaryProfile | undefined,
): DietaryRecommendationConflict[] {
  if (!dietaryProfile) return [];
  return DIETARY_RECOMMENDATION_CONFLICTS.filter((conflict) =>
    conflict.profile_fields.some((field) => {
      const values = dietaryProfile[field as keyof PersonalDietaryProfile];
      if (!Array.isArray(values)) return false;
      return values.some((value) => {
        const normalizedValue = normalizedDietaryTerm(value);
        return normalizedValue && conflict.profile_terms.some((term) => {
          const normalizedTerm = normalizedDietaryTerm(term);
          return normalizedTerm && (normalizedValue === normalizedTerm || normalizedValue.includes(normalizedTerm));
        });
      });
    }),
  );
}

function suggestionMatchesConflict(
  suggestion: string,
  conflict: DietaryRecommendationConflict,
): boolean {
  const normalizedSuggestion = normalizedDietaryTerm(suggestion);
  return conflict.blocked_suggestion_terms.some((term) => {
    const normalizedTerm = normalizedDietaryTerm(term);
    return normalizedTerm && normalizedSuggestion.includes(normalizedTerm);
  });
}

/**
 * Select only food rules activated by explicit profile/context values. The
 * rules themselves remain resource-authored; this function only performs
 * conservative routing and never treats a missing field as a negative.
 */
function deriveFoodSafetyGuidance(
  dietaryProfile: PersonalDietaryProfile | undefined,
  activeContextIds: readonly string[],
  candidateSuggestions: string[] = [],
): FoodSafetyGuidance {
  const ruleIds = new Set<string>();
  for (const [field, values] of Object.entries(dietaryProfile || {})) {
    const routes = DIETARY_PROFILE_ROUTES[field] || {};
    for (const value of Array.isArray(values) ? values : []) {
      const normalizedValue = normalizedDietaryTerm(value);
      if (!normalizedValue) continue;
      for (const [term, routedRuleIds] of Object.entries(routes)) {
        const normalizedTerm = normalizedDietaryTerm(term);
        if (normalizedTerm && (normalizedValue === normalizedTerm || normalizedValue.includes(normalizedTerm))) {
          routedRuleIds.forEach((ruleId) => ruleIds.add(ruleId));
        }
      }
    }
  }
  for (const contextId of activeContextIds) {
    for (const ruleId of DIETARY_CONTEXT_ROUTES[contextId] || []) ruleIds.add(ruleId);
  }

  const rulesById = new Map(dietaryRequirements.rules.map((rule) => [rule.id, rule]));
  const relevantRules = Array.from(ruleIds)
    .flatMap((ruleId) => {
      const rule = rulesById.get(ruleId);
      return rule ? [rule] : [];
    })
    .sort((left, right) => right.priority - left.priority || left.label.localeCompare(right.label));

  const explicitExclusions = [...(dietaryProfile?.hard_exclusions || [])];
  const confirmedAllergies = [...(dietaryProfile?.allergies_confirmed || [])];
  const suspectedAllergies = [...(dietaryProfile?.allergies_suspected || [])];
  const recommendationConflicts = recommendationConflictsForProfile(dietaryProfile);
  const notes: string[] = [];
  const conflictNotes = recommendationConflicts.map((conflict) => conflict.reason);
  if (explicitExclusions.length > 0) {
    notes.push(dietaryRequirements.profile_notes.hard_exclusions);
  }
  if (confirmedAllergies.length > 0) {
    notes.push(dietaryRequirements.profile_notes.confirmed_allergies);
  }
  if (suspectedAllergies.length > 0) {
    notes.push(dietaryRequirements.profile_notes.suspected_allergies);
  }

  return {
    priorityOrder: dietaryRequirements.priority_order,
    explicitExclusions,
    confirmedAllergies,
    suspectedAllergies,
    relevantRules,
    notes: Array.from(new Set(notes)),
    conflictNotes: Array.from(new Set(conflictNotes)),
    suppressedSuggestions: candidateSuggestions.filter((suggestion) =>
      recommendationConflicts.some((conflict) => suggestionMatchesConflict(suggestion, conflict))
    ),
  };
}

function allergyMarkerMatches(marker: EvaluatedMarker, markerIds: readonly string[]): boolean {
  if (!marker.interpretation_allowed) return false;
  if (marker.severity_class === 'benign' || marker.severity_class === 'protective') return false;
  if (marker.effect_count != null && marker.effect_count === 0) return false;
  const ids = new Set(markerIds.map((id) => id.trim().toLowerCase()));
  return ids.has(String(marker.rsid || '').trim().toLowerCase());
}

function matchedAllergyGenes(markers: EvaluatedMarker[]): string[] {
  return Array.from(new Set(
    markers
      .flatMap((marker) => normalizedGeneSymbols(marker.gene))
      .filter((gene) => gene !== 'UNKNOWN'),
  )).sort();
}

function uniqueMatchedMarkers(markers: EvaluatedMarker[]): EvaluatedMarker[] {
  const seen = new Set<string>();
  return markers.filter((marker) => {
    const key = String(marker.link_id || marker.rsid || '').trim().toLowerCase();
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

/**
 * Build a compact allergy/sensitivity map from the curated marker IDs. The
 * catalog deliberately separates DNA-linked pathway context from exposure
 * categories that require the person's history or clinical testing.
 */
export function deriveAllergySensitivityGuidance(report: GeneratedReport): AllergySensitivityGuidance {
  const allMarkers = (report.sections || []).flatMap((section) => section.markers || []);
  const dnaContexts: AllergyDnaContext[] = allergySensitivityCatalog.dna_contexts.flatMap((context) => {
    const matched = uniqueMatchedMarkers(allMarkers.filter((marker) => allergyMarkerMatches(marker, context.matched_marker_ids)));
    if (matched.length === 0) return [];
    return [{
      id: context.id,
      label: context.label,
      signal_label: context.signal_label,
      summary: context.summary,
      relevance: context.relevance,
      evidence_label: context.evidence_label,
      examples: [...context.examples],
      next_step: context.next_step,
      matched_marker_count: matched.length,
      matched_marker_ids: matched.map((marker) => marker.rsid),
      matched_marker_link_ids: matched.map((marker) => marker.link_id),
      matched_genes: matchedAllergyGenes(matched),
      sources: [...context.sources],
    }];
  });

  const medicationSafety: AllergyMedicationSafetyItem[] = allergySensitivityCatalog.clinical_safety_routes.flatMap((route) => {
    const actionabilityRule = ACTIONABLE_RULES.find((rule) => rule.id === route.actionability_rule_id);
    if (!actionabilityRule) return [];
    const matched = uniqueMatchedMarkers(allMarkers.filter((marker) => rankingRuleMatchesMarker(actionabilityRule, marker)));
    if (matched.length === 0) return [];
    return [{
      id: route.id,
      label: route.label,
      signal_label: route.signal_label,
      medications: [...route.medications],
      summary: route.summary,
      relevance: route.relevance,
      next_step: route.next_step,
      evidence_label: route.evidence_label,
      matched_marker_count: matched.length,
      matched_marker_ids: matched.map((marker) => marker.rsid),
      matched_marker_link_ids: matched.map((marker) => marker.link_id),
      matched_genes: matchedAllergyGenes(matched),
      sources: [...route.sources],
    }];
  });

  const matchedMarkerIds = new Set([
    ...dnaContexts.flatMap((context) => context.matched_marker_ids),
    ...medicationSafety.flatMap((item) => item.matched_marker_ids),
  ]);

  return {
    dnaContexts,
    medicationSafety,
    exposureChecklists: allergySensitivityCatalog.exposure_checklists,
    matchedMarkerCount: matchedMarkerIds.size,
    hasDnaSignal: dnaContexts.length > 0 || medicationSafety.length > 0,
  };
}

function derivePersonalContextGuidance(
  personalSafetyContext?: PersonalSafetyContext,
): PersonalContextGuidance {
  const context = personalSafetyContext || {
    medications: [],
    supplements: [],
    allergies: [],
    symptoms: [],
    labObservations: [],
  };
  const priorityNotes: string[] = [];
  if (context.allergies.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.allergies);
  }
  if (context.medications.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.medications);
  }
  if (context.supplements.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.supplements);
  }
  if (context.symptoms.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.symptoms);
  }
  if (context.labObservations.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.labs);
  }
  if (context.dietaryProfile && Object.values(context.dietaryProfile).some((items) => items.length > 0)) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.dietary_profile);
  }
  if (context.reproductiveIntake && Object.keys(context.reproductiveIntake).length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.reproductive_intake);
  }
  if (context.cycleDiary && context.cycleDiary.length > 0) {
    priorityNotes.push(safetyGuardrails.personal_context_notes.cycle_diary);
  }

  return {
    medications: [...context.medications],
    supplements: [...context.supplements],
    allergies: [...context.allergies],
    symptoms: [...context.symptoms],
    labObservations: [...context.labObservations],
    ...(context.dietaryProfile ? {
      dietaryProfile: Object.fromEntries(
        Object.entries(context.dietaryProfile).map(([field, values]) => [field, [...values]]),
      ) as unknown as PersonalDietaryProfile,
    } : {}),
    ...(context.reproductiveIntake ? { reproductiveIntake: { ...context.reproductiveIntake } } : {}),
    ...(context.cycleDiary ? { cycleDiary: context.cycleDiary.map((entry) => ({ id: entry.id, values: { ...entry.values } })) } : {}),
    priorityNotes,
  };
}

export function deriveActionablePlan(
  report: GeneratedReport,
  actionabilityContext: ActionabilityContext = {}
): ActionablePlan {
  const topFindings: TopFinding[] = [];
  const dietaryFavorMap = new Map<string, RecommendationAccumulator>();
  const dietaryAvoidMap = new Map<string, RecommendationAccumulator>();
  const advancedGuidanceSet = new Set<string>();
  const safetyNotes = new Set<string>(ACTIONABILITY_POLICY.safety_notes || []);
  const supplementsMap = new Map<string, RecommendationAccumulator>();
  const supplementAvoidMap = new Map<string, RecommendationAccumulator>();
  const medicationContext = new Set<string>();
  const medicationPathwaysMap = new Map<string, MedicationPathway>();
  const labTestsMap = new Map<string, LabAccumulator>();

  const allMarkers: { marker: EvaluatedMarker; sectionName: string }[] = [];
  for (const section of report.sections || []) {
    for (const marker of section.markers || []) {
      allMarkers.push({ marker, sectionName: section.name });
    }
  }

  const canonicalFindingGroups = buildCanonicalFindingGroups(report);
  const conditionEvidence = buildConditionEvidenceSummaries(report);
  const priorityGroupMembers = new Map<string, EvaluatedMarker[]>();
  for (const { marker } of allMarkers) {
    if (!marker.interpretation_allowed || isExcludedFromPriorityQueue(marker)) continue;
    const group = priorityGroupForMarker(marker);
    if (!group) continue;
    const members = priorityGroupMembers.get(group) || [];
    members.push(marker);
    priorityGroupMembers.set(group, members);
  }
  const scoredFindings = canonicalFindingGroups
    .map((group) => {
      const eligibleSources = group.sourceMarkers.filter(({ marker }) =>
        marker.interpretation_allowed && !isExcludedFromPriorityQueue(marker),
      );
      if (eligibleSources.length === 0) return null;

      const primarySource = [...eligibleSources].sort((left, right) => {
        const leftPriority = priorityMetadataForMarker(left.marker);
        const rightPriority = priorityMetadataForMarker(right.marker);
        const leftScore = leftPriority.band * 100000
          + prioritySeverityScore(left.marker) * 1000
          + actionabilityRankingBoost(left.marker);
        const rightScore = rightPriority.band * 100000
          + prioritySeverityScore(right.marker) * 1000
          + actionabilityRankingBoost(right.marker);
        return rightScore - leftScore
          || left.sectionName.localeCompare(right.sectionName)
          || left.marker.gene.localeCompare(right.marker.gene)
          || left.marker.rsid.localeCompare(right.marker.rsid)
          || left.marker.link_id.localeCompare(right.marker.link_id);
      })[0];
      const marker = primarySource.marker;
      const priority = priorityMetadataForMarker(marker);
      const simpleCopy = getSimpleFindingCopy(marker, getLaypersonTranslation(marker));
      const priorityGroup = priorityGroupForMarker(marker);
      const groupedMarkers = priorityGroup
        ? priorityGroupMembers.get(priorityGroup) || eligibleSources.map(({ marker: sourceMarker }) => sourceMarker)
        : eligibleSources.map(({ marker: sourceMarker }) => sourceMarker);
      const groupedMedicationDirection = priorityGroup && /warfarin/i.test(
        groupedMarkers.map(medicationMarkerText).join(' '),
      )
        ? medicationDirectionLabel(groupedMarkers)
        : simpleCopy.direction_label;
      const plainTitle = priorityGroup && /warfarin/i.test(
        groupedMarkers.map(medicationMarkerText).join(' '),
      )
        ? `Warfarin dosing — ${groupedMedicationDirection.charAt(0).toLowerCase()}${groupedMedicationDirection.slice(1)}`
        : simpleCopy.plain_title;
      // Actionability determines the queue band. Severity, evidence, and
      // authored follow-up value only refine ties within that band.
      const score = priority.band * 100000
        + prioritySeverityScore(marker) * 1000
        + actionabilityRankingBoost(marker);

      return {
        score,
        priorityGroup,
        finding: {
          finding_id: group.findingId,
          source_marker_ids: [...group.sourceMarkerIds],
          rsid: marker.rsid,
          gene: marker.gene,
          variant_name: marker.variant_name || marker.rsid,
          severity_class: marker.severity_class,
          interpretation: marker.interpretation,
          section_name: primarySource.sectionName,
          link_id: marker.link_id,
          topic_id: group.topicId,
          topic_ids: [...group.topicIds],
          health_area_ids: [...group.healthAreaIds],
          health_area_labels: [...group.healthAreaLabels],
          evidence_tiers: [...group.evidenceTiers],
          effect_directions: [...group.effectDirections],
          condition_labels: [...group.conditionLabels],
          interpretation_classes: [...group.interpretationClasses],
          inheritance_models: [...group.inheritanceModels],
          clinical_states: [...group.clinicalStates],
          actionability_level: group.actionability.level,
          clinical_confirmation_required: group.actionability.clinicalConfirmationRequired,
          applicability_scopes: [...group.applicability.scopes],
          call_state: group.callState,
          reference_ids: [...group.referenceIds],
          plain_title: plainTitle,
          direction_label: groupedMedicationDirection,
          priority_reason: priority.reason,
          priority_urgency: priority.urgency,
          priority_tone: priority.tone,
        },
      };
    })
    .filter((item): item is NonNullable<typeof item> => item !== null)
    .filter((item) => item.score > 0)
    .sort((a, b) =>
      b.score - a.score ||
      a.finding.section_name.localeCompare(b.finding.section_name) ||
      a.finding.gene.localeCompare(b.finding.gene) ||
      a.finding.rsid.localeCompare(b.finding.rsid)
    );

  const uniqueFindings: typeof scoredFindings = [];
  const seenKeys = new Set<string>();
  const seenPriorityGroups = new Set<string>();
  const priorityGroupCounts = new Map<string, number>();
  for (const item of scoredFindings) {
    if (!item.priorityGroup) continue;
    priorityGroupCounts.set(
      item.priorityGroup,
      (priorityGroupCounts.get(item.priorityGroup) || 0) + item.finding.source_marker_ids.length,
    );
  }

  // Keep the queue ordered by concern and follow-up value. Do not hard-cap by
  // section: nine genuinely high-concern findings in one area should remain
  // visible. Deterministic sorting above keeps ties stable.
  for (const item of scoredFindings) {
    const key = item.finding.finding_id;
    if (seenKeys.has(key)) continue;
    if (item.priorityGroup && seenPriorityGroups.has(item.priorityGroup)) continue;
    seenKeys.add(key);
    if (item.priorityGroup) seenPriorityGroups.add(item.priorityGroup);
    uniqueFindings.push({
      ...item,
      finding: {
        ...item.finding,
        ...(item.priorityGroup ? {
          priority_group: item.priorityGroup,
          related_marker_count: priorityGroupCounts.get(item.priorityGroup) || 1,
        } : item.finding.source_marker_ids.length > 1 ? {
          related_marker_count: item.finding.source_marker_ids.length,
        } : {}),
      },
    });
    if (uniqueFindings.length >= TOP_FINDINGS_LIMIT) break;
  }
  topFindings.push(...uniqueFindings.map((item) => item.finding));

  // Pack-authored guidance rules (actionability_guidance.json)
  for (const rule of ACTIONABLE_RULES) {
    const matchingMarkers = allMarkers.filter(({ marker }) => {
      if (!actionabilityRuleMatchesMarker(rule, marker)) return false;
      if (!marker.interpretation_allowed) return false;

      if (rule.severity_classes && !rule.severity_classes.includes(marker.severity_class)) {
        return false;
      }

      if (rule.interpretation_contains) {
        const text = (marker.interpretation || '').toLowerCase();
        const matchesText = rule.interpretation_contains.some((pattern) =>
          text.includes(pattern.toLowerCase())
        );
        if (!matchesText) return false;
      }

      if (!rule.severity_classes && !rule.interpretation_contains) {
        if (marker.effect_count != null && marker.effect_count === 0) {
          return false;
        }
        if (marker.severity_class === 'benign' || marker.severity_class === 'protective') {
          return false;
        }
      }

      return true;
    });

    if (matchingMarkers.length === 0) continue;

    if (rule.medication_context?.length) {
      const pathwayId = rule.id || rule.genes.join('_').toLowerCase();
      medicationPathwaysMap.set(pathwayId, {
        id: pathwayId,
        label: medicationPathwayLabel(rule, matchingMarkers.map(({ marker }) => marker)),
        direction_label: medicationDirectionLabel(matchingMarkers.map(({ marker }) => marker)),
        genes: [...rule.genes],
        detail: compactMedicationPathwayDetail(rule.medication_context[0]),
        matchedMarkerCount: matchingMarkers.length,
        matchedMarkerLinkIds: matchingMarkers.map(({ marker }) => marker.link_id),
      });
    }

    const reason = `Based on your ${rule.genes.join('/')} variant (${matchingMarkers
      .map((m) => m.marker.rsid)
      .join(', ')})`;
    const actionabilityClass =
      rule.actionability_class ||
      ACTIONABILITY_POLICY.default_actionability ||
      'symptom_or_lab_conditioned';

    // Keep the original mixed-domain guidance available to advanced consumers.
    rule.favor?.forEach((item) => advancedGuidanceSet.add(qualifyGuidance(item, actionabilityClass, 'favor')));
    rule.avoid?.forEach((item) => advancedGuidanceSet.add(qualifyGuidance(item, actionabilityClass, 'avoid')));

    // `favor` and `avoid` contain mixed-domain guardrails. Only explicitly
    // authored dietary fields belong in the Dietary Alignment card; medical,
    // medication, sleep, activity, and supplement guardrails must not appear
    // there as if they were foods to eat or avoid.
    rule.dietary_favor?.forEach((item) => {
      const clean = item.trim();
      addRecommendation(dietaryFavorMap, 'food', clean, rule, matchingMarkers, reason);
    });
    rule.dietary_avoid?.forEach((item) => {
      const clean = item.trim();
      addRecommendation(dietaryAvoidMap, 'food', clean, rule, matchingMarkers, reason);
    });
    rule.supplement_favor?.forEach((item) => {
      const name = item.name.trim();
      if (!name) return;
      addRecommendation(
        supplementsMap,
        'supplement',
        name,
        rule,
        matchingMarkers,
        item.reason?.trim() || `${rule.genes.join('/')} nutrient-pathway context`,
      );
    });
    rule.supplement_avoid?.forEach((item) => {
      const name = item.name.trim();
      if (!name) return;
      addRecommendation(
        supplementAvoidMap,
        'supplement',
        name,
        rule,
        matchingMarkers,
        item.reason?.trim(),
      );
    });
    rule.supplements?.forEach((s) => {
      // Legacy `supplements` entries include some guardrails that are useful
      // to Clinical/AI consumers but are not actual products to consider in
      // the Simple supplement list. Curated item-level fields above are the
      // source for concise intake/avoidance rows.
      if (/^(?:avoid|do not|don't|tell the clinician)\b/i.test(s.trim())) {
        advancedGuidanceSet.add(qualifyGuidance(s, actionabilityClass, 'supplement'));
        return;
      }
      addRecommendation(
        supplementsMap,
        'supplement',
        s,
        rule,
        matchingMarkers,
        `${reason}; ${qualifyGuidance(s, actionabilityClass, 'supplement')}`,
      );
    });
    rule.medication_context?.forEach((item) => medicationContext.add(item));
    rule.lab_tests?.forEach((lt) => {
      upsertLab(
        labTestsMap,
        lt.name,
        `DNA-linked ${rule.genes.join('/')} pathway`,
        lt.urgency,
        !!lt.requires_counselor,
        {
          ruleId: String(rule.id || rule.genes.join('_')).trim(),
          markerIds: matchingMarkers.map(({ marker }) => marker.rsid),
          genes: rule.genes.flatMap((gene) => normalizedGeneSymbols(gene)),
        },
      );
    });
  }

  // Pack marker confirm_with → lab-like prompts only (strict filter; never auto-urgent)
  let confirmWithAdded = 0;
  const confirmWithCap = ACTIONABILITY_POLICY.confirm_with_cap || 12;
  for (const { marker } of allMarkers) {
    if (!marker.interpretation_allowed) continue;
    if (
      marker.severity_class !== 'high_risk' &&
      marker.severity_class !== 'moderate_risk' &&
      marker.severity_class !== 'confirmation_required'
    ) {
      continue;
    }
    for (const item of marker.confirm_with || []) {
      if (confirmWithAdded >= confirmWithCap) break;
      const trimmed = String(item || '').trim();
      if (!trimmed || !isLabLikeConfirmItem(trimmed)) continue;
      if (labTestsMap.has(trimmed)) continue;

      const requiresCounselor =
        !!marker.clinical_confirmation_required &&
        marker.severity_class === 'confirmation_required';
      upsertLab(
        labTestsMap,
        trimmed,
        `DNA-linked ${marker.gene} pathway`,
        requiresCounselor ? 'urgent' : 'consider',
        requiresCounselor,
        {
          markerIds: [marker.rsid],
          genes: normalizedGeneSymbols(marker.gene),
        },
      );
      confirmWithAdded += 1;
    }
  }

  const supplements: SupplementItem[] = recommendationItems(supplementsMap).map((item) => ({
    ...item,
    reason: item.why_it_appears,
  }));
  const supplementAvoid: SupplementAvoidItem[] = recommendationItems(supplementAvoidMap).map((item) => ({
    ...item,
    reason: item.why_it_appears,
  }));

  const labTests: LabTest[] = Array.from(labTestsMap.values())
    .map((val) => {
      const tier = deriveLabTier(val.urgency, val.requires_counselor);
      return {
        canonical_id: val.canonical_id,
        name: val.name,
        purpose: val.purpose,
        reason: Array.from(val.reasons)[0] || 'DNA-linked follow-up for the matched pathway.',
        reason_topics: Array.from(val.reasons),
        basis_rule_ids: Array.from(val.basis_rule_ids).sort(),
        basis_marker_ids: Array.from(val.basis_marker_ids).sort(),
        basis_genes: Array.from(val.basis_genes).sort(),
        urgency: val.urgency,
        tier,
        category: val.category,
        requires_counselor: val.requires_counselor,
      };
    })
    .sort((a, b) => {
      const tierOrder = LAB_TIER_META[a.tier].order - LAB_TIER_META[b.tier].order;
      if (tierOrder !== 0) return tierOrder;
      return a.name.localeCompare(b.name);
    });

  const labGroups = buildLabGroups(labTests);
  const medicationPathways = Array.from(medicationPathwaysMap.values())
    .sort((a, b) => a.label.localeCompare(b.label));
  const markerValues = allMarkers.map(({ marker }) => marker);
  const sectionNames = allMarkers.map(({ sectionName }) => sectionName);
  const personalSafetyContext = actionabilityContext.personalSafetyContext;
  const activeContextIds = activeReproductiveContextIds(
    actionabilityContext.reproductiveContext,
    personalSafetyContext,
  );
  const personalActivityContext = activityPersonalContextText(personalSafetyContext);
  const relevantActivityDomains = activityGuardrails.domains.filter((domain) =>
    activityDomainMatches(
      domain,
      markerValues,
      sectionNames,
      personalActivityContext,
      actionabilityContext.reproductiveContext,
      activeContextIds,
    )
  );
  const activityRecommendations = buildActivityRecommendations(relevantActivityDomains, allMarkers);
  const relevantCycleDomains = cycleSupportDomainsForContextIds(activeContextIds);
  const relevantCycleEvidenceLayers = cycleSupportEvidenceLayersForContextIds(activeContextIds);
  const reproductiveDnaCoverage = reproductiveDnaCoverageForReport(report, activeContextIds);
  const diaryReview = cycleDiaryAppliesToContexts(activeContextIds)
    ? reviewCycleDiary(personalSafetyContext?.cycleDiary)
    : null;
  const supplementSafetyRules = selectSupplementSafetyRules(
    markerValues,
    [
      ...supplements.map((item) => item.name),
      ...(personalSafetyContext?.supplements || []),
    ],
    personalSafetyContext?.medications || [],
    [
      ...(personalSafetyContext?.allergies || []),
      ...(personalSafetyContext?.dietaryProfile?.hard_exclusions || []),
      ...(personalSafetyContext?.dietaryProfile?.allergies_confirmed || []),
      ...(personalSafetyContext?.dietaryProfile?.allergies_suspected || []),
    ],
    actionabilityContext.reproductiveContext,
    activeContextIds,
  );
  const medicationSafety = deriveMedicationSafety(
    markerValues,
    sectionNames,
    actionabilityContext.reproductiveContext,
    personalSafetyContext,
    activeContextIds,
  );
  const personalContext = derivePersonalContextGuidance(personalSafetyContext);
  const pgxGuidance = derivePgxInterpretationGuidance(markerValues);
  const dietaryFavorItems = recommendationItems(dietaryFavorMap);
  const dietaryAvoidItems = recommendationItems(dietaryAvoidMap);
  const candidateDietFavor = dietaryFavorItems.map((item) => item.name);
  const foodSafety = deriveFoodSafetyGuidance(
    personalSafetyContext?.dietaryProfile,
    activeContextIds,
    candidateDietFavor,
  );
  const allergy = deriveAllergySensitivityGuidance(report);

  return {
    topFindings,
    conditionEvidence,
    diet: {
      favor: candidateDietFavor.filter((item) => !foodSafety.suppressedSuggestions.includes(item)),
      avoid: dietaryAvoidItems.map((item) => item.name),
      favorItems: dietaryFavorItems.filter((item) => !foodSafety.suppressedSuggestions.includes(item.name)),
      avoidItems: dietaryAvoidItems,
    },
    advancedGuidance: Array.from(advancedGuidanceSet),
    supplements,
    supplementAvoid,
    labTests,
    labGroups,
    activity: {
      principles: activityGuardrails.principles,
      simpleFramework: activityGuardrails.simple_framework,
      stopAndEscalate: activityGuardrails.stop_and_escalate,
      relevantDomains: activityRecommendations.domains,
      recommendationItems: activityRecommendations.recommendations,
    },
    medication: {
      rules: Array.from(new Set([...medicationSafety.rules, ...medicationContext])),
      askFor: medicationSafety.askFor,
    },
    medicationPathways,
    cycleSupport: {
      principles: cycleSupport.principles,
      relevantEvidenceLayers: relevantCycleEvidenceLayers,
      dnaCoverage: reproductiveDnaCoverage,
      relevantDomains: relevantCycleDomains,
      diaryReview,
    },
    pgxGuidance,
    supplementSafety: {
      principles: supplementSafety.principles,
      relevantRules: supplementSafetyRules,
    },
    foodSafety,
    allergy,
    personalContext,
    safetyNotes: Array.from(safetyNotes),
  };
}
