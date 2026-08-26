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
import prsRegistry from '../marker-packs/prs_registry.json';
import safetyGuardrails from '../marker-packs/safety_guardrails.json';
import supplementSafety from '../marker-packs/supplement_safety.json';
import sourceRegistry from '../marker-packs/source_registry.json';
import userDietProfileSchema from '../marker-packs/user_diet_profile_schema.json';
import { cycleSupportDomainsForContext, selectedReproductiveContextOption } from './reproductiveContext';

export type SupportConsultationMode = typeof consultationModes.modes[number]['id'];

export interface SupportResourceContext {
  evidence_policy: {
    tiers: typeof evidencePolicy.tiers;
    claim_policy: typeof evidencePolicy.claim_policy;
  };
  safety_guardrails: typeof safetyGuardrails.rules;
  medication_context: typeof safetyGuardrails.medication_context;
  supplement_safety: {
    principles: typeof supplementSafety.principles;
    rules: typeof supplementSafety.rules;
    do_not_do: typeof supplementSafety.do_not_do;
  };
  callability_rules: typeof callabilityRules.rules;
  phenotype_prompts: typeof phenotypePrompts.domains;
  lab_overlays: Array<Record<string, unknown>>;
  actionability_policy: {
    default_actionability?: string;
    safety_notes?: string[];
  };
  actionability_rules: Array<Record<string, unknown>>;
  food_safety: {
    priority_order: typeof dietaryRequirements.priority_order;
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
    context_options: typeof cycleSupport.context_options;
    marker_contexts: typeof cycleSupport.marker_contexts;
    selected_context_id: string | null;
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

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)));
}

function relevantPackIds(packIds: string[], consultationMode: SupportConsultationMode): Set<string> {
  return new Set(uniqueStrings([...packIds, CONSULTATION_PACK[consultationMode] || '']));
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

function selectActivityDomains(packIds: Set<string>): typeof activityGuardrails.domains {
  return activityGuardrails.domains.filter((domain) => {
    const signals = Array.isArray(domain.relevant_pack_signals)
      ? domain.relevant_pack_signals
      : [domain.id];
    return signals.some((signal) => packIds.has(signal));
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
}: {
  packIds: string[];
  consultationMode?: SupportConsultationMode;
  reproductiveContext?: string;
}): SupportResourceContext {
  const selectedPackIds = relevantPackIds(packIds, consultationMode);
  const phenotype = selectPhenotypeDomains(selectedPackIds);
  const overlays = selectLabOverlays(selectedPackIds);
  const relevantCycleDomains = cycleSupportDomainsForContext(reproductiveContext).filter((domain) => {
    const signals = Array.isArray(domain.relevant_pack_signals) ? domain.relevant_pack_signals : [];
    return signals.length === 0 || signals.some((signal) => selectedPackIds.has(signal));
  });
  const selectedSourceRecords = selectSourceRecords(supportSourceIds(selectedPackIds, relevantCycleDomains));

  return {
    evidence_policy: {
      tiers: evidencePolicy.tiers,
      claim_policy: evidencePolicy.claim_policy,
    },
    safety_guardrails: safetyGuardrails.rules,
    medication_context: safetyGuardrails.medication_context,
    supplement_safety: {
      principles: supplementSafety.principles,
      rules: supplementSafety.rules,
      do_not_do: supplementSafety.do_not_do,
    },
    callability_rules: callabilityRules.rules,
    phenotype_prompts: phenotype,
    lab_overlays: overlays,
    actionability_policy: {
      default_actionability: actionabilityGuidance.policy?.default_actionability,
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
      relevant_domains: selectActivityDomains(selectedPackIds),
      sources: activityGuardrails.sources,
    },
    cycle_support: {
      principles: cycleSupport.principles,
      context_options: cycleSupport.context_options,
      marker_contexts: cycleSupport.marker_contexts,
      selected_context_id: selectedReproductiveContextOption(reproductiveContext)?.id || null,
      domains: cycleSupport.domains,
      relevant_domains: relevantCycleDomains,
      do_not_do: cycleSupport.do_not_do,
      sources: relevantCycleDomains.flatMap((domain) => domain.sources || []),
      source_registry: selectedSourceRecords,
    },
  };
}
