// ./src/lib/utils/supportResourceContext.ts
/**
 * Selects the evidence, phenotype, laboratory, food-safety, and callability
 * resources that belong in an AI consultation context.
 *
 * The source JSON files are intentionally kept separate from marker packs:
 * marker packs describe genotype-linked observations, while these resources
 * describe what must be asked, checked, confirmed, or avoided before acting.
 * This module is the runtime bridge between those resources and the prompt.
 */

import actionabilityGuidance from '../marker-packs/actionability_guidance.json';
import activityGuardrails from '../marker-packs/activity_guardrails.json';
import callabilityRules from '../marker-packs/callability_rules.json';
import cycleSupport from '../marker-packs/cycle_support_guidance.json';
import consultationModes from '../marker-packs/consultation_modes.json';
import dietPatternProfiles from '../marker-packs/diet_pattern_profiles.json';
import dietaryRequirements from '../marker-packs/dietary_requirements.json';
import evidencePolicy from '../marker-packs/evidence_policy.json';
import foodNutrientMatrix from '../marker-packs/food_nutrient_matrix.json';
import foodRequirementPrompts from '../marker-packs/food_requirement_prompts.json';
import labOverlays from '../marker-packs/lab_overlays.json';
import mealPlanningRules from '../marker-packs/meal_planning_rules.json';
import phenotypePrompts from '../marker-packs/phenotype_prompts.json';
import pgxDiplotypeGuidance from '../marker-packs/pgx_diplotype_guidance.json';
import prsRegistry from '../marker-packs/prs_registry.json';
import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import safetyGuardrails from '../marker-packs/safety_guardrails.json';
import supplementSafety from '../marker-packs/supplement_safety.json';
import sourceRegistry from '../marker-packs/source_registry.json';
import userDietProfileSchema from '../marker-packs/user_diet_profile_schema.json';
import {
  activeReproductiveContextIds,
  cycleSupportDomainsForContextIds,
  cycleSupportEvidenceLayersForContextIds,
  hasReproductivePersonalContext,
  reproductiveContextIdsForProfileText,
  selectedReproductiveContextOption,
} from './reproductiveContext';
import { activityDomainMatchesPersonalContext, activityPersonalContextText } from './activityContext';
import { reviewCycleDiary, type CycleDiaryReview } from './cycleDiaryReview';
import { cycleDiaryAppliesToContexts } from './cycleDiary';
import type { PersonalSafetyContext } from './personalSafetyContext';

export type SupportConsultationMode = typeof consultationModes.modes[number]['id'];

export interface SupportResourceContext {
  context_routing: {
    profile_category_ids: string[];
    profile_pack_ids: string[];
    profile_context_ids: string[];
  };
  evidence_policy: {
    tiers: typeof evidencePolicy.tiers;
    claim_policy: typeof evidencePolicy.claim_policy;
    display: typeof evidencePolicy.display;
  };
  safety_guardrails: typeof safetyGuardrails.rules;
  personal_context_notes: typeof safetyGuardrails.personal_context_notes;
  medication_context: typeof safetyGuardrails.medication_context;
  supplement_safety: {
    principles: typeof supplementSafety.principles;
    rules: typeof supplementSafety.rules;
    do_not_do: typeof supplementSafety.do_not_do;
  };
  callability_rules: typeof callabilityRules.rules;
  pgx_diplotype: {
    policy: typeof pgxDiplotypeGuidance.policy;
    genes: typeof pgxDiplotypeGuidance.genes;
    relevant_genes: typeof pgxDiplotypeGuidance.genes;
    do_not_do: typeof pgxDiplotypeGuidance.do_not_do;
    source_registry: Record<string, unknown>;
  };
  phenotype_prompts: typeof phenotypePrompts.domains;
  lab_overlays: Array<Record<string, unknown>>;
  actionability_policy: {
    default_actionability?: string;
    confirm_with_cap?: number;
    safety_notes?: string[];
  };
  actionability_rules: Array<Record<string, unknown>>;
  food_safety: {
    priority_order: typeof dietaryRequirements.priority_order;
    profile_routes: typeof dietaryRequirements.profile_routes;
    context_routes: typeof dietaryRequirements.context_routes;
    profile_notes: typeof dietaryRequirements.profile_notes;
    recommendation_conflicts: typeof dietaryRequirements.recommendation_conflicts;
    meal_decision_pipeline: typeof mealPlanningRules.decision_pipeline;
    priority_weights: typeof mealPlanningRules.priority_weights;
    relevant_rules: Array<Record<string, unknown>>;
    nutrient_matrix: typeof foodNutrientMatrix.food_groups;
    diet_pattern_profiles: typeof dietPatternProfiles.profiles;
    profile_schema: typeof userDietProfileSchema.schema;
    profile_minimum_required: typeof userDietProfileSchema.minimum_required_for_food_advice;
    profile_do_not_infer: typeof userDietProfileSchema.do_not_infer;
    source_registry: Record<string, unknown>;
    do_not_do: typeof mealPlanningRules.do_not_do;
    intake_questions: typeof foodRequirementPrompts.global_first_run_questions;
    conditional_questions: Record<string, string[]>;
  };
  prs_policy: {
    principle: string;
    modules: typeof prsRegistry.prs_modules;
  };
  activity_safety: {
    principles: typeof activityGuardrails.principles;
    stop_and_escalate: typeof activityGuardrails.stop_and_escalate;
    relevant_domains: typeof activityGuardrails.domains;
    sources: typeof activityGuardrails.sources;
  };
  cycle_support: {
    principles: typeof cycleSupport.principles;
    evidence_layers: typeof cycleSupport.evidence_layers;
    relevant_evidence_layers: typeof cycleSupport.evidence_layers;
    context_options: typeof cycleSupport.context_options;
    marker_contexts: typeof cycleSupport.marker_contexts;
    marker_context_packs: typeof cycleSupport.marker_context_packs;
    intake_schema: typeof cycleSupport.intake_schema;
    diary_schema: typeof cycleSupport.diary_schema;
    review_schema: typeof cycleSupport.review_schema;
    diary_review: CycleDiaryReview | null;
    selected_context_id: string | null;
    active_context_ids: string[];
    context_activation: 'explicit_selection' | 'self_reported_context' | 'profile_context' | 'none';
    domains: typeof cycleSupport.domains;
    relevant_domains: typeof cycleSupport.domains;
    do_not_do: typeof cycleSupport.do_not_do;
    sources: string[];
    source_registry: Record<string, unknown>;
  };
}

const CONSULTATION_PACK = Object.fromEntries(
  consultationModes.modes
    .filter((mode) => typeof mode.pack_id === 'string' && mode.pack_id.trim() !== '')
    .map((mode) => [mode.id, mode.pack_id]),
) as Partial<Record<SupportConsultationMode, string>>;

interface ProfileContextRouting {
  category_ids: string[];
  pack_ids: string[];
  context_ids: string[];
}

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)));
}

/** Route explicitly supplied profile goals/context through the shared taxonomy. */
function routeProfileContext(profileContext?: string): ProfileContextRouting {
  const text = String(profileContext || '').trim().toLowerCase();
  if (!text) return { category_ids: [], pack_ids: [], context_ids: [] };

  const matched = researchTaxonomy.categories.filter((category) =>
    category.keywords.some((keyword) => text.includes(String(keyword).toLowerCase()))
  );
  return {
    category_ids: matched.map((category) => category.id),
    pack_ids: uniqueStrings(matched.flatMap((category) => category.packs)),
    context_ids: reproductiveContextIdsForProfileText(text),
  };
}

function relevantPackIds(
  packIds: string[],
  consultationMode: SupportConsultationMode,
  reproductiveContext?: string,
  personalSafetyContext?: PersonalSafetyContext,
  profileContext?: string,
): { selected: Set<string>; profileRouting: ProfileContextRouting; activeContextIds: string[] } {
  const profileRouting = routeProfileContext(profileContext);
  const selectedContext = selectedReproductiveContextOption(reproductiveContext);
  const activeContextIds = selectedContext
    ? [selectedContext.id]
    : uniqueStrings([
        ...activeReproductiveContextIds('', personalSafetyContext),
        ...profileRouting.context_ids,
      ]);
  const selected = new Set(uniqueStrings([...packIds, CONSULTATION_PACK[consultationMode] || '']));
  for (const packId of profileRouting.pack_ids) selected.add(packId);
  if (activeContextIds.length > 0 || hasReproductivePersonalContext(personalSafetyContext)) {
    selected.add('hormones_reproductive');
  }
  return { selected, profileRouting, activeContextIds };
}

function overlayPackIds(overlay: Record<string, unknown>): string[] {
  const values = overlay.markers_or_packs;
  return Array.isArray(values) ? values.filter((value): value is string => typeof value === 'string') : [];
}

function selectPhenotypeDomains(packIds: Set<string>): typeof phenotypePrompts.domains {
  return phenotypePrompts.domains.filter((domain) => packIds.has(domain.id));
}

function selectLabOverlays(packIds: Set<string>): Array<Record<string, unknown>> {
  return labOverlays.overlays.filter((overlay) => {
    const linkedPacks = overlayPackIds(overlay as Record<string, unknown>);
    return linkedPacks.some((packId) => packIds.has(packId));
  }) as Array<Record<string, unknown>>;
}

function selectDietaryRules(packIds: Set<string>): Array<Record<string, unknown>> {
  return dietaryRequirements.rules
    .filter((rule) => {
      const signals = Array.isArray(rule.relevant_pack_signals) ? rule.relevant_pack_signals : [];
      return signals.length === 0 || signals.some((signal) => packIds.has(signal));
    })
    .map((rule) => ({
      id: rule.id,
      label: rule.label,
      rule_type: rule.rule_type,
      priority: rule.priority,
      recommendation: rule.recommendation,
      substitutions: rule.substitutions,
      confirm_with: rule.confirm_with,
      conflict_resolution: rule.conflict_resolution,
      do_not_claim: rule.do_not_claim,
      sources: rule.sources,
    }))
    .slice(0, 18) as Array<Record<string, unknown>>;
}

function selectSourceRecords(sourceIds: string[]): Record<string, unknown> {
  const registry = sourceRegistry.sources as Record<string, unknown>;
  const selected: Record<string, unknown> = {};
  for (const sourceId of uniqueStrings(sourceIds)) {
    if (registry[sourceId]) selected[sourceId] = registry[sourceId];
  }
  return selected;
}

function supportSourceIds(
  selectedPackIds: Set<string>,
  relevantCycleDomains: typeof cycleSupport.domains,
): string[] {
  const ids = new Set<string>([
    ...foodNutrientMatrix.sources,
    ...activityGuardrails.sources,
    ...actionabilityGuidance.rules.flatMap((rule) => rule.sources || []),
    ...supplementSafety.rules.flatMap((rule) => rule.sources),
    ...relevantCycleDomains.flatMap((domain) => domain.sources || []),
    ...selectPhenotypeDomains(selectedPackIds).flatMap((domain) => domain.sources || []),
  ]);
  for (const rule of selectDietaryRules(selectedPackIds)) {
    if (Array.isArray(rule.sources)) {
      for (const sourceId of rule.sources) {
        if (typeof sourceId === 'string') ids.add(sourceId);
      }
    }
  }
  return Array.from(ids);
}

function selectConditionalQuestions(packIds: Set<string>): Record<string, string[]> {
  const result: Record<string, string[]> = {};
  const conditionalPrompts = foodRequirementPrompts.conditional_prompts as Record<string, string[]>;
  const promptSignals = foodRequirementPrompts.conditional_prompt_signals as Record<string, string[]>;
  for (const [questionId, questions] of Object.entries(conditionalPrompts)) {
    const linkedPacks = Array.isArray(promptSignals[questionId]) ? promptSignals[questionId] : [];
    if (linkedPacks.some((packId) => packIds.has(packId))) {
      result[questionId] = questions;
    }
  }
  return result;
}

function selectPgxGenes(packIds: Set<string>): typeof pgxDiplotypeGuidance.genes {
  return packIds.has('pgx') ? pgxDiplotypeGuidance.genes : [];
}

function selectActivityDomains(
  packIds: Set<string>,
  reproductiveContext?: string,
  personalSafetyContext?: PersonalSafetyContext,
  activeContextIds: readonly string[] = [],
): typeof activityGuardrails.domains {
  const personalContextText = activityPersonalContextText(personalSafetyContext);
  return activityGuardrails.domains.filter((domain) => {
    const signals = Array.isArray(domain.relevant_pack_signals)
      ? domain.relevant_pack_signals
      : [domain.id];
    return signals.some((signal) => packIds.has(signal))
      || activityDomainMatchesPersonalContext(domain, personalContextText, reproductiveContext, activeContextIds);
  });
}

function selectCycleDomains(packIds: Set<string>): typeof cycleSupport.domains {
  return cycleSupport.domains.filter((domain) => {
    const signals = Array.isArray(domain.relevant_pack_signals) ? domain.relevant_pack_signals : [];
    return signals.length === 0 || signals.some((signal) => packIds.has(signal));
  });
}

/**
 * Build a bounded, pack-aware resource payload. The payload contains rules and
 * questions, not raw DNA, and intentionally keeps the full safety/callability
 * policies present even when a user has few active findings.
 */
export function buildSupportResourceContext({
  packIds,
  consultationMode = 'general',
  reproductiveContext,
  personalSafetyContext,
  profileContext,
}: {
  packIds: string[];
  consultationMode?: SupportConsultationMode;
  reproductiveContext?: string;
  personalSafetyContext?: PersonalSafetyContext;
  /** User-supplied goals/context used only for taxonomy-based resource routing. */
  profileContext?: string;
}): SupportResourceContext {
  const routing = relevantPackIds(
    packIds,
    consultationMode,
    reproductiveContext,
    personalSafetyContext,
    profileContext,
  );
  const selectedPackIds = routing.selected;
  const phenotype = selectPhenotypeDomains(selectedPackIds);
  const overlays = selectLabOverlays(selectedPackIds);
  const relevantCycleDomains = cycleSupportDomainsForContextIds(routing.activeContextIds).filter((domain) => {
    const signals = Array.isArray(domain.relevant_pack_signals) ? domain.relevant_pack_signals : [];
    return signals.length === 0 || signals.some((signal) => selectedPackIds.has(signal));
  });
  const relevantCycleEvidenceLayers = cycleSupportEvidenceLayersForContextIds(routing.activeContextIds);
  const diaryReview = cycleDiaryAppliesToContexts(routing.activeContextIds)
    ? reviewCycleDiary(personalSafetyContext?.cycleDiary)
    : null;
  const relevantPgxGenes = selectPgxGenes(selectedPackIds);
  const selectedSourceRecords = selectSourceRecords([
    ...supportSourceIds(selectedPackIds, relevantCycleDomains),
    ...relevantCycleEvidenceLayers.flatMap((layer) => layer.sources || []),
    ...relevantPgxGenes.flatMap((gene) => gene.sources || []),
  ]);

  return {
    context_routing: {
      profile_category_ids: routing.profileRouting.category_ids,
      profile_pack_ids: routing.profileRouting.pack_ids,
      profile_context_ids: routing.profileRouting.context_ids,
    },
    evidence_policy: {
      tiers: evidencePolicy.tiers,
      claim_policy: evidencePolicy.claim_policy,
      display: evidencePolicy.display,
    },
    safety_guardrails: safetyGuardrails.rules,
    personal_context_notes: safetyGuardrails.personal_context_notes,
    medication_context: safetyGuardrails.medication_context,
    supplement_safety: {
      principles: supplementSafety.principles,
      rules: supplementSafety.rules,
      do_not_do: supplementSafety.do_not_do,
    },
    callability_rules: callabilityRules.rules,
    pgx_diplotype: {
      policy: pgxDiplotypeGuidance.policy,
      genes: pgxDiplotypeGuidance.genes,
      relevant_genes: relevantPgxGenes,
      do_not_do: pgxDiplotypeGuidance.do_not_do,
      source_registry: selectedSourceRecords,
    },
    phenotype_prompts: phenotype,
    lab_overlays: overlays,
    actionability_policy: {
      default_actionability: actionabilityGuidance.policy?.default_actionability,
      confirm_with_cap: actionabilityGuidance.policy?.confirm_with_cap,
      safety_notes: actionabilityGuidance.policy?.safety_notes,
    },
    actionability_rules: actionabilityGuidance.rules.map((rule) => ({
      id: rule.id,
      genes: rule.genes,
      marker_ids: rule.marker_ids,
      actionability_class: rule.actionability_class,
      severity_classes: rule.severity_classes,
      interpretation_contains: rule.interpretation_contains,
      pack_hints: rule.pack_hints,
      favor: rule.favor,
      avoid: rule.avoid,
      supplements: rule.supplements,
      lab_tests: rule.lab_tests,
      medication_context: rule.medication_context,
      notes: rule.notes,
      sources: rule.sources,
    })),
    food_safety: {
      priority_order: dietaryRequirements.priority_order,
      profile_routes: dietaryRequirements.profile_routes,
      context_routes: dietaryRequirements.context_routes,
      profile_notes: dietaryRequirements.profile_notes,
      recommendation_conflicts: dietaryRequirements.recommendation_conflicts,
      meal_decision_pipeline: mealPlanningRules.decision_pipeline,
      priority_weights: mealPlanningRules.priority_weights,
      relevant_rules: selectDietaryRules(selectedPackIds),
      nutrient_matrix: foodNutrientMatrix.food_groups,
      diet_pattern_profiles: dietPatternProfiles.profiles,
      profile_schema: userDietProfileSchema.schema,
      profile_minimum_required: userDietProfileSchema.minimum_required_for_food_advice,
      profile_do_not_infer: userDietProfileSchema.do_not_infer,
      source_registry: selectedSourceRecords,
      do_not_do: mealPlanningRules.do_not_do,
      intake_questions: foodRequirementPrompts.global_first_run_questions,
      conditional_questions: selectConditionalQuestions(selectedPackIds),
    },
    prs_policy: {
      principle: prsRegistry.principle,
      modules: prsRegistry.prs_modules,
    },
    activity_safety: {
      principles: activityGuardrails.principles,
      stop_and_escalate: activityGuardrails.stop_and_escalate,
      relevant_domains: selectActivityDomains(
        selectedPackIds,
        reproductiveContext,
        personalSafetyContext,
        routing.activeContextIds,
      ),
      sources: activityGuardrails.sources,
    },
    cycle_support: {
      principles: cycleSupport.principles,
      evidence_layers: cycleSupport.evidence_layers,
      relevant_evidence_layers: relevantCycleEvidenceLayers,
      context_options: cycleSupport.context_options,
      marker_contexts: cycleSupport.marker_contexts,
      marker_context_packs: cycleSupport.marker_context_packs,
      intake_schema: cycleSupport.intake_schema,
      diary_schema: cycleSupport.diary_schema,
      review_schema: cycleSupport.review_schema,
      diary_review: diaryReview,
      selected_context_id: selectedReproductiveContextOption(reproductiveContext)?.id || null,
      active_context_ids: routing.activeContextIds,
      context_activation: selectedReproductiveContextOption(reproductiveContext)
        ? 'explicit_selection'
        : activeReproductiveContextIds('', personalSafetyContext).length > 0
          ? 'self_reported_context'
          : routing.profileRouting.context_ids.length > 0
            ? 'profile_context'
          : 'none',
      domains: cycleSupport.domains,
      relevant_domains: relevantCycleDomains,
      do_not_do: cycleSupport.do_not_do,
      sources: relevantCycleDomains.flatMap((domain) => domain.sources || []),
      source_registry: selectedSourceRecords,
    },
  };
}
