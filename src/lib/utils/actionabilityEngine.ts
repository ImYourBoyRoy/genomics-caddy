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
import safetyGuardrails from '../marker-packs/safety_guardrails.json';
import supplementSafety from '../marker-packs/supplement_safety.json';
import { cycleSupportDomainsForContext, selectedReproductiveContextOption } from './reproductiveContext';
import type { PersonalSafetyContext } from './personalSafetyContext';

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
  relevantDomains: typeof cycleSupport.domains;
}

export interface SupplementSafetyGuidance {
  principles: typeof supplementSafety.principles;
  relevantRules: typeof supplementSafety.rules;
}

export interface PersonalContextGuidance {
  medications: string[];
  supplements: string[];
  allergies: string[];
  symptoms: string[];
  labObservations: string[];
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
  supplementSafety: SupplementSafetyGuidance;
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

const LAB_TIER_META: Record<LabTest['tier'], { label: string; hint: string; order: number }> = {
  counselor: {
    label: 'Clinical confirmation',
    hint: 'High-stakes — discuss with a genetic counselor or specialist before acting.',
    order: 0,
  },
  discuss: {
    label: 'Worth discussing',
    hint: 'Reasonable follow-up labs to review with your clinician at a routine visit.',
    order: 1,
  },
  optional: {
    label: 'Optional / if symptomatic',
    hint: 'Lower priority or symptom-driven — not an emergency workup.',
    order: 2,
  },
};

function inferLabCategory(name: string): string {
  const lower = name.toLowerCase();
  if (/brca|lynch|ngs|confirmation|counselor|mammograph|colonoscop/i.test(lower)) {
    return 'Clinical confirmation';
  }
  if (/apob|lipo\(a\)|lipid|lipoprotein|crp|cholesterol|cardiovascular|blood pressure|omega-3/i.test(lower)) {
    return 'Heart & lipids';
  }
  if (/homocysteine|folate|b12|vitamin d|25\(oh\)|selenium|methyl/i.test(lower)) {
    return 'Nutrients & methylation';
  }
  if (/ferritin|iron|tibc|transferrin/i.test(lower)) {
    return 'Iron studies';
  }
  if (/uric acid|glucose|insulin|metabolic/i.test(lower)) {
    return 'Metabolic';
  }
  if (/liver|alt|ast|hepatic/i.test(lower)) {
    return 'Liver';
  }
  if (/kidney|egfr|creatinine|urinalysis|electrolyte/i.test(lower)) {
    return 'Kidney & fluids';
  }
  if (/thyroid|tsh|autoantibod/i.test(lower)) {
    return 'Thyroid & immune';
  }
  if (/spirometry|sleep|oxygen/i.test(lower)) {
    return 'Respiratory & sleep';
  }
  if (/dxa|bone|calcium|pth/i.test(lower)) {
    return 'Bone & minerals';
  }
  if (/pgx|pharmacogen/i.test(lower)) {
    return 'Pharmacogenomics';
  }
  return 'Other follow-up';
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

  const blocked = [
    'family history',
    'personal/family',
    'symptom diary',
    'diary',
    'clinician review',
    'clinical review',
    'genetic counseling',
    'trigger pattern',
    'visible phenotype',
    'hygiene',
    'training log',
    'body composition',
    'medication response',
    'exposure history',
    'cycle/symptom',
    'specialist review',
    'dermatology review',
    'dental exam',
    'screening plan',
    'diet/',
    'logs',
    'tracking',
  ];

  const allowed = [
    'panel',
    'sequencing',
    'genotyp',
    'mri',
    'mammograph',
    'colonoscop',
    'dxa',
    'spirometry',
    'apob',
    'lipid',
    'homocysteine',
    'ferritin',
    'tibc',
    'transferrin',
    'uric acid',
    'vitamin d',
    '25(oh)',
    'pth',
    'calcium',
    'crp',
    'esr',
    'autoantibod',
    'breath test',
    'pgx',
    'egfr',
    'creatinine',
    'urinalysis',
    'omega-3',
    'selenium',
    'folate',
    'b12',
    'liver',
    'alt',
    'ast',
    'blood pressure',
    'nmr',
    'lipoprotein',
    'lpa',
    'inflammation',
    'glucose',
    'electrolyte',
    'hormone',
    'cortisol',
    'thyroid',
    'tsh',
    'sleep study',
    'polysomn',
    'allerg',
  ];

  if (allowed.some((a) => lower.includes(a))) return true;
  if (blocked.some((b) => lower.includes(b))) return false;
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

function activityContextMatches(
  domain: typeof activityGuardrails.domains[number],
  markers: EvaluatedMarker[],
  sectionNames: string[]
): boolean {
  const context = [
    ...sectionNames,
    ...markers.map((marker) => `${marker.gene} ${marker.variant_name || ''} ${marker.sex_scope || ''}`),
  ].join(' ').toLowerCase();
  const keywords = Array.isArray(domain.section_keywords) ? domain.section_keywords : [];
  return keywords.some((keyword) => context.includes(String(keyword).toLowerCase()));
}

function deriveMedicationSafety(
  markers: EvaluatedMarker[],
  sectionNames: string[],
  reproductiveContext?: string,
  personalSafetyContext?: PersonalSafetyContext,
): MedicationSafetyGuidance {
  const context = [
    ...sectionNames,
    ...markers.map((marker) => `${marker.gene} ${marker.variant_name || ''} ${marker.sex_scope || ''}`),
    ...(personalSafetyContext?.medications || []),
  ].join(' ').toLowerCase();
  const pgxContext = safetyGuardrails.medication_context.pgx_context_keywords.some((keyword) =>
    context.includes(String(keyword).toLowerCase())
  );
  const relevantRuleIds = new Set(['PGX_NO_MED_CHANGE', 'LABS_AND_PHENOTYPE_FIRST']);
  if (pgxContext) {
    relevantRuleIds.add('HLA_TAGS_NOT_TYPING');
    relevantRuleIds.add('CNV_STR_VNTR_NOT_ARRAY_SAFE');
  }

  // A user-supplied hormonal medication name is an explicit medication
  // context signal. It does not establish anatomy, cycle status, or hormone
  // levels; it only makes the composition/label guardrail relevant.
  const hasHormonalMedication = (personalSafetyContext?.medications || []).some((name) =>
    /contracept|birth control|estrogen|estradiol|progesterone|progestin|hormone therapy|hormonal/i.test(name)
  );
  if (hasHormonalMedication) relevantRuleIds.add('CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA');
  for (const ruleId of selectedReproductiveContextOption(reproductiveContext)?.medication_rule_ids || []) {
    relevantRuleIds.add(ruleId);
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

function selectSupplementSafetyRules(
  markers: EvaluatedMarker[],
  supplementNames: string[],
  medicationNames: string[],
  allergyTerms: string[],
): typeof supplementSafety.rules {
  const markerGenes = new Set(markers.flatMap((marker) => marker.gene.split(/[\s/]+/).map((gene) => gene.toLowerCase())));
  const supplementContext = supplementNames.join(' ').toLowerCase();
  const medicationContext = medicationNames.join(' ').toLowerCase();
  const allergyContext = allergyTerms.join(' ').toLowerCase();
  return supplementSafety.rules.filter((rule) => {
    const geneMatch = rule.signal_genes.some((gene) => markerGenes.has(gene.toLowerCase()));
    const termMatch = rule.match_terms.some((term) => supplementContext.includes(term.toLowerCase()));
    const medicationMatch = (rule as typeof rule & { medication_terms?: string[] }).medication_terms?.some((term) =>
      medicationContext.includes(term.toLowerCase())
    ) || false;
    const allergyMatch = (rule as typeof rule & { allergy_terms?: string[] }).allergy_terms?.some((term) =>
      allergyContext.includes(term.toLowerCase())
    ) || false;
    return geneMatch || termMatch || medicationMatch || allergyMatch;
  });
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
    priorityNotes.push(
      'Reported allergies or intolerances take priority over genotype-based food and supplement prompts; never use raw DNA to clear an exposure.'
    );
  }
  if (context.medications.length > 0) {
    priorityNotes.push(
      'Current medication names are safety context only. Confirm the exact product, active ingredients, dose, timing, indication, and prescriber instructions before acting.'
    );
  }
  if (context.supplements.length > 0) {
    priorityNotes.push(
      'Current supplements are included for interaction and duplicate-nutrient review; a genotype match is not proof that another supplement is needed.'
    );
  }
  if (context.symptoms.length > 0) {
    priorityNotes.push(
      'Symptoms and timing are phenotype evidence to track and discuss; do not convert them into a DNA diagnosis or a hormone-level claim.'
    );
  }
  if (context.labObservations.length > 0) {
    priorityNotes.push(
      'User-recorded labs and clinician findings should be interpreted with dates, units, reference ranges, and clinical context; measured phenotype outranks a generic SNP prompt.'
    );
  }

  return {
    medications: [...context.medications],
    supplements: [...context.supplements],
    allergies: [...context.allergies],
    symptoms: [...context.symptoms],
    labObservations: [...context.labObservations],
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
  const CONFIRM_WITH_CAP = 12;
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
      if (confirmWithAdded >= CONFIRM_WITH_CAP) break;
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
  const relevantActivityDomains = activityGuardrails.domains.filter((domain) =>
    activityContextMatches(domain, markerValues, sectionNames)
  );
  const relevantCycleDomains = cycleSupportDomainsForContext(actionabilityContext.reproductiveContext);
  const personalSafetyContext = actionabilityContext.personalSafetyContext;
  const supplementSafetyRules = selectSupplementSafetyRules(
    markerValues,
    [
      ...supplements.map((item) => item.name),
      ...(personalSafetyContext?.supplements || []),
    ],
    personalSafetyContext?.medications || [],
    [
      ...(personalSafetyContext?.allergies || []),
    ]
  );
  const medicationSafety = deriveMedicationSafety(
    markerValues,
    sectionNames,
    actionabilityContext.reproductiveContext,
    personalSafetyContext
  );
  const personalContext = derivePersonalContextGuidance(personalSafetyContext);

  return {
    topFindings,
    diet: {
      favor: Array.from(favorSet),
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
      relevantDomains: relevantCycleDomains,
    },
    supplementSafety: {
      principles: supplementSafety.principles,
      relevantRules: supplementSafetyRules,
    },
    personalContext,
    safetyNotes: Array.from(safetyNotes),
  };
}
