import { describe, expect, it } from 'vitest';
import foodRequirementPrompts from '../marker-packs/food_requirement_prompts.json';
import { buildSupportResourceContext } from './supportResourceContext';

describe('support resource context', () => {
  it('selects reproductive phenotype and lab resources without inventing a diagnosis', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'menstrual_cycle',
    });

    expect(context.phenotype_prompts.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(context.lab_overlays.some((overlay) => overlay.domain === 'adenomyosis_heavy_bleeding_pelvic_pain')).toBe(true);
    expect(context.safety_guardrails.some((rule) => rule.id === 'ADENOMYOSIS_REQUIRES_GYNECOLOGIC_WORKUP')).toBe(true);
    expect(context.medication_context.ask_for.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'selenium')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'vitamin_a')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'vitamin_k')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'potassium')).toBe(true);
    expect(context.food_safety.profile_minimum_required.length).toBeGreaterThan(0);
    expect(context.food_safety.source_registry.nih_ods_selenium).toBeDefined();
    expect(context.food_safety.conditional_questions.pmdd_cycle).toBeDefined();
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'pmdd_like_mood_symptoms')).toBe(true);
    expect(context.cycle_support.source_registry.acog_premenstrual_disorders).toBeDefined();
    expect(context.cycle_support.source_registry.cdc_usmec_2024).toBeDefined();
    expect(context.cycle_support.context_options.some((option) => option.id === 'menstrual_cycle')).toBe(true);
    expect(context.cycle_support.context_options.some((option) => option.id === 'androgen_reproductive')).toBe(true);
    expect(context.cycle_support.selected_context_id).toBe('menstrual_cycle');
    expect(context.cycle_support.marker_contexts.menstrual_cycle).toContain('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY');
    expect(context.cycle_support.marker_contexts.shared_reproductive).toContain('rs2234693');
    const pmddDomain = context.cycle_support.relevant_domains.find((domain) => domain.id === 'pmdd_like_mood_symptoms');
    expect(pmddDomain?.support_options.join(' ')).toContain('SSRI');
    expect(pmddDomain?.support_options.join(' ')).toContain('not on a DNA marker');
    const contraceptiveDomain = context.cycle_support.relevant_domains.find((domain) => domain.id === 'contraceptive_product_context');
    expect(contraceptiveDomain?.support_options.join(' ')).toContain('no new adverse effects');
  });

  it('routes exogenous hormone therapy context with product and monitoring sources', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'hormone_therapy_context',
    });

    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual([
      'exogenous_hormone_medication_context',
      'general_symptom_day_support',
    ]);
    expect(context.cycle_support.source_registry.endocrine_gender_affirming_hormone_therapy).toBeDefined();
    expect(context.cycle_support.source_registry.acog_transgender_gender_diverse_care).toBeDefined();
    expect(context.safety_guardrails.some((rule) => rule.id === 'HORMONE_THERAPY_COMPOSITION_NOT_IN_DNA')).toBe(true);
  });

  it('routes explicit pregnancy, postpartum, and lactation context to safety resources', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'pregnancy_postpartum',
    });

    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual([
      'pregnancy_postpartum_lactation_context',
      'general_symptom_day_support',
    ]);
    expect(context.cycle_support.source_registry.cdc_medicine_pregnancy).toBeDefined();
    expect(context.cycle_support.source_registry.ncbi_lactmed).toBeDefined();
    expect(context.safety_guardrails.some((rule) => rule.id === 'PREGNANCY_LACTATION_MEDICATION_REVIEW_NOT_IN_DNA')).toBe(true);
  });

  it('keeps reproductive domains hidden until an explicit context is selected', () => {
    const hidden = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
    });
    const androgen = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'androgen_reproductive',
    });

    expect(hidden.cycle_support.relevant_domains).toHaveLength(0);
    expect(hidden.cycle_support.selected_context_id).toBeNull();
    expect(androgen.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual(['androgen_reproductive_context']);
    expect(androgen.cycle_support.selected_context_id).toBe('androgen_reproductive');
  });

  it('selects PGx confirmation resources and retains medication safety priorities', () => {
    const context = buildSupportResourceContext({ packIds: ['pgx'], consultationMode: 'pgx' });

    expect(context.lab_overlays.some((overlay) => overlay.domain === 'pgx')).toBe(true);
    expect(context.callability_rules.some((rule) => rule.id === 'STAR_ALLELE_DIPLOTYPE_REQUIRED')).toBe(true);
    expect(context.safety_guardrails.some((rule) => rule.id === 'PGX_NO_MED_CHANGE')).toBe(true);
    const hlaPhenytoinRule = context.actionability_rules.find((rule) => rule.id === 'hla_b1502_phenytoin_safety');
    expect(hlaPhenytoinRule?.marker_ids).toContain('HLA-B*15:02');
    expect(context.food_safety.priority_order[1]).toContain('medication');
    expect(context.supplement_safety.do_not_do.some((item) => item.includes('common SNP'))).toBe(true);
    expect(context.food_safety.nutrient_matrix.length).toBeGreaterThan(0);
  });

  it('keeps PRS and common-marker evidence probabilistic', () => {
    const context = buildSupportResourceContext({ packIds: ['cardiovascular'] });

    expect(context.evidence_policy.claim_policy.never_claim).toContain('diagnosis from raw DNA');
    expect(context.prs_policy.principle).toContain('Do not compute');
    expect(context.actionability_policy.safety_notes?.[0]).toContain('More markers increase coverage');
    const apoeRule = context.actionability_rules.find((rule) => rule.id === 'apoe_lipid');
    expect(apoeRule?.interpretation_contains).toContain('APOE4');
    expect(apoeRule?.pack_hints).toContain('cardiovascular');
    expect(context.activity_safety.relevant_domains.some((domain) => domain.id === 'cardiovascular')).toBe(true);
    expect(context.activity_safety.stop_and_escalate.some((item) => item.includes('Chest pain'))).toBe(true);
  });

  it('routes metabolic consultation context to glucose, lipid, and activity resources', () => {
    const context = buildSupportResourceContext({ packIds: ['metabolic'], consultationMode: 'metabolic' });

    expect(context.actionability_rules.some((rule) => rule.id === 'glucose_insulin_context')).toBe(true);
    expect(context.actionability_rules.some((rule) => rule.id === 'triglyceride_liver_context')).toBe(true);
    expect(context.actionability_rules.some((rule) => rule.id === 'atherogenic_lipid_context')).toBe(true);
    expect(context.activity_safety.relevant_domains.some((domain) => domain.id === 'metabolic')).toBe(true);
    expect(context.food_safety.source_registry.cdc_prediabetes_lifestyle).toBeDefined();
    expect(context.food_safety.source_registry.aha_dyslipidemia_2026).toBeDefined();
  });

  it('routes cross-domain activity safety from resource-authored pack signals', () => {
    const context = buildSupportResourceContext({
      packIds: ['bone_growth_mineral_density', 'sleep', 'kidney_fluid_electrolytes', 'pain_migraine_sensory'],
    });
    const domainIds = context.activity_safety.relevant_domains.map((domain) => domain.id);

    expect(domainIds).toContain('connective_tissue');
    expect(domainIds).toContain('respiratory_airway');
    expect(domainIds).toContain('sleep_recovery');
    expect(domainIds).toContain('kidney_fluid_electrolytes');
    expect(domainIds).toContain('pain_migraine_sensory');
  });

  it('routes every resource-authored conditional food prompt without a TypeScript map', () => {
    const signals = foodRequirementPrompts.conditional_prompt_signals as Record<string, string[]>;
    for (const [promptId, linkedPacks] of Object.entries(signals)) {
      const context = buildSupportResourceContext({ packIds: [linkedPacks[0]] });
      expect(context.food_safety.conditional_questions[promptId]).toEqual(
        foodRequirementPrompts.conditional_prompts[promptId as keyof typeof foodRequirementPrompts.conditional_prompts],
      );
    }
  });
});
