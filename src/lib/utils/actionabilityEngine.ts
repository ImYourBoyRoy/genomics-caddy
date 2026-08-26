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

import type { GeneratedReport, EvaluatedMarker, SeverityClass } from '../types/genomics';
import guidanceDoc from '../marker-packs/actionability_guidance.json';
import activityGuardrails from '../marker-packs/activity_guardrails.json';
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

export interface TopFinding {
  rsid: string;
  gene: string;
  variant_name: string;
  severity_class: SeverityClass;
  interpretation: string;
  section_name: string;
  link_id: string;
}

export interface DietaryGuidance {
  favor: string[];
  avoid: string[];
  notes?: string;
}

export interface SupplementItem {
  name: string;
  reason: string;
}

export interface LabTest {
  name: string;
  reason: string;
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

export interface ActivityGuidance {
  principles: string[];
  stopAndEscalate: string[];
  relevantDomains: typeof activityGuardrails.domains;
}

export interface MedicationSafetyGuidance {
  rules: string[];
  askFor: string[];
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
  diet: DietaryGuidance;
  supplements: SupplementItem[];
  labTests: LabTest[];
  labGroups: LabTestGroup[];
  activity: ActivityGuidance;
  medication: MedicationSafetyGuidance;
  cycleSupport: CycleSupportGuidance;
  pgxGuidance: PgxInterpretationGuidance;
  supplementSafety: SupplementSafetyGuidance;
  foodSafety: FoodSafetyGuidance;
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

interface ActionableRule {
  id?: string;
  genes: string[];
  /** Optional exact curated marker IDs for drug/allele-specific routing. */
  marker_ids?: string[];
  actionability_class?: ActionabilityClass;
  severity_classes?: SeverityClass[];
  interpretation_contains?: string[];
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

interface LabCategory {
  id: string;
  label: string;
  order: number;
  keywords: string[];
  fallback?: boolean;
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
        ? `Do not avoid solely from raw DNA; confirm the finding clinically first: ${clean}`
        : `Only consider after clinical confirmation and individualized advice: ${clean}`;
    case 'symptom_or_lab_conditioned':
      return kind === 'avoid'
        ? `Consider limiting only if symptoms, labs, or clinician guidance support it: ${clean}`
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

function inferLabCategory(name: string): string {
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

function upsertLab(
  map: Map<
    string,
    {
      reason: string;
      urgency: 'routine' | 'consider' | 'urgent';
      requires_counselor: boolean;
    }
  >,
  name: string,
  reason: string,
  urgency: 'routine' | 'consider' | 'urgent',
  requiresCounselor = false
) {
  const existing = map.get(name);
  if (existing) {
    if (URGENCY_RANK[urgency] > URGENCY_RANK[existing.urgency]) {
      existing.urgency = urgency;
    }
    existing.requires_counselor = existing.requires_counselor || requiresCounselor;
    if (!existing.reason.includes(reason)) {
      existing.reason = `${existing.reason}; ${reason}`;
    }
  } else {
    map.set(name, {
      reason,
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
  return {
    policy: pgxDiplotypeGuidance.policy,
    relevantGenes: pgxDiplotypeGuidance.genes.filter((gene) =>
      markers.some((marker) => pgxGeneMatchesMarker(gene, marker))
    ),
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
  const favorSet = new Set<string>();
  const avoidSet = new Set<string>();
  const safetyNotes = new Set<string>(ACTIONABILITY_POLICY.safety_notes || []);
  const supplementsMap = new Map<string, string[]>();
  const medicationContext = new Set<string>();
  const labTestsMap = new Map<
    string,
    {
      reason: string;
      urgency: 'routine' | 'consider' | 'urgent';
      requires_counselor: boolean;
    }
  >();
  let overallNotes = '';

  const allMarkers: { marker: EvaluatedMarker; sectionName: string }[] = [];
  for (const section of report.sections || []) {
    for (const marker of section.markers || []) {
      allMarkers.push({ marker, sectionName: section.name });
    }
  }

  const scoredFindings = allMarkers
    .filter(({ marker }) => marker.interpretation_allowed)
    .map(({ marker, sectionName }) => {
      let score = 0;
      if (marker.severity_class === 'high_risk') score = 100;
      else if (marker.severity_class === 'confirmation_required') score = 90;
      else if (marker.severity_class === 'moderate_risk') score = 50;
      else if (marker.severity_class === 'low_risk') score = 20;

      return {
        score,
        finding: {
          rsid: marker.rsid,
          gene: marker.gene,
          variant_name: marker.variant_name || marker.rsid,
          severity_class: marker.severity_class,
          interpretation: marker.interpretation,
          section_name: sectionName,
          link_id: marker.link_id,
        },
      };
    })
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score);

  const uniqueFindings: typeof scoredFindings = [];
  const seenKeys = new Set<string>();
  for (const item of scoredFindings) {
    const key = `${item.finding.gene}-${item.finding.rsid}`;
    if (!seenKeys.has(key)) {
      seenKeys.add(key);
      uniqueFindings.push(item);
      if (uniqueFindings.length >= 10) break;
    }
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

    const reason = `Based on your ${rule.genes.join('/')} variant (${matchingMarkers
      .map((m) => m.marker.rsid)
      .join(', ')})`;
    const actionabilityClass =
      rule.actionability_class ||
      ACTIONABILITY_POLICY.default_actionability ||
      'symptom_or_lab_conditioned';

    rule.favor?.forEach((f) => favorSet.add(qualifyGuidance(f, actionabilityClass, 'favor')));
    rule.avoid?.forEach((a) => avoidSet.add(qualifyGuidance(a, actionabilityClass, 'avoid')));
    rule.supplements?.forEach((s) => {
      const list = supplementsMap.get(s) || [];
      list.push(`${reason}; ${qualifyGuidance(s, actionabilityClass, 'supplement')}`);
      supplementsMap.set(s, list);
    });
    rule.medication_context?.forEach((item) => medicationContext.add(item));
    rule.lab_tests?.forEach((lt) => {
      upsertLab(labTestsMap, lt.name, reason, lt.urgency, !!lt.requires_counselor);
    });
    if (rule.notes) {
      overallNotes +=
        (overallNotes ? '\n' : '') +
        `• ${rule.genes.join('/')}: ${qualifyGuidance(rule.notes, actionabilityClass, 'favor')}`;
    }
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
        `Pack marker ${marker.gene} (${marker.rsid})`,
        requiresCounselor ? 'urgent' : 'consider',
        requiresCounselor
      );
      confirmWithAdded += 1;
    }
  }

  const supplements: SupplementItem[] = Array.from(supplementsMap.entries()).map(
    ([name, reasons]) => ({
      name,
      reason: reasons.join('; '),
    })
  );

  const labTests: LabTest[] = Array.from(labTestsMap.entries())
    .map(([name, val]) => {
      const tier = deriveLabTier(val.urgency, val.requires_counselor);
      return {
        name,
        reason: val.reason,
        urgency: val.urgency,
        tier,
        category: inferLabCategory(name),
        requires_counselor: val.requires_counselor,
      };
    })
    .sort((a, b) => {
      const tierOrder = LAB_TIER_META[a.tier].order - LAB_TIER_META[b.tier].order;
      if (tierOrder !== 0) return tierOrder;
      return a.name.localeCompare(b.name);
    });

  const labGroups = buildLabGroups(labTests);
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
  const candidateDietFavor = Array.from(favorSet);
  const foodSafety = deriveFoodSafetyGuidance(
    personalSafetyContext?.dietaryProfile,
    activeContextIds,
    candidateDietFavor,
  );

  return {
    topFindings,
    diet: {
      favor: candidateDietFavor.filter((item) => !foodSafety.suppressedSuggestions.includes(item)),
      avoid: Array.from(avoidSet),
      notes: overallNotes || undefined,
    },
    supplements,
    labTests,
    labGroups,
    activity: {
      principles: activityGuardrails.principles,
      stopAndEscalate: activityGuardrails.stop_and_escalate,
      relevantDomains: relevantActivityDomains,
    },
    medication: {
      rules: Array.from(new Set([...medicationSafety.rules, ...medicationContext])),
      askFor: medicationSafety.askFor,
    },
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
    personalContext,
    safetyNotes: Array.from(safetyNotes),
  };
}
