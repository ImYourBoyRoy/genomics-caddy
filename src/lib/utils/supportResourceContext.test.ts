import { describe, expect, it } from 'vitest';
import foodRequirementPrompts from '../marker-packs/food_requirement_prompts.json';
import manifest from '../marker-packs/manifest.json';
import phenotypePrompts from '../marker-packs/phenotype_prompts.json';
import { buildSupportResourceContext } from './supportResourceContext';

describe('support resource context', () => {
  it('selects reproductive phenotype and lab resources without inventing a diagnosis', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'menstrual_cycle',
    });

    expect(context.phenotype_prompts.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(context.evidence_policy.display.tiers.A.label).toContain('Tier A');
    expect(context.evidence_policy.display.scope_labels.menstrual_cycle_context).toContain('Menstrual');
    expect(context.lab_overlays.some((overlay) => overlay.domain === 'adenomyosis_heavy_bleeding_pelvic_pain')).toBe(true);
    expect(context.safety_guardrails.some((rule) => rule.id === 'ADENOMYOSIS_REQUIRES_GYNECOLOGIC_WORKUP')).toBe(true);
    expect(context.personal_context_notes.reproductive_intake).toContain('exact label');
    expect(context.medication_context.ask_for.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'selenium')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'vitamin_a')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'vitamin_k')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'potassium')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'vitamin_b6')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'b12_status')).toBe(true);
    expect(context.supplement_safety.rules.find((rule) => rule.id === 'b12_status')?.sources).toContain('nih_ods_b12');
    expect(context.supplement_safety.rules.find((rule) => rule.id === 'vitamin_b6')?.sources).toContain('nih_ods_vitamin_b6');
    expect(context.food_safety.profile_minimum_required.length).toBeGreaterThan(0);
    expect(context.food_safety.profile_routes.allergies_confirmed.fish).toContain('RULE_ALLERGY_MAJOR_FOOD_STRICT_AVOIDANCE');
    expect(context.food_safety.context_routes.menstrual_cycle).toContain('RULE_LOW_IRON_OR_ANEMIA_CONTEXT');
    expect(context.food_safety.profile_notes.confirmed_allergies).toContain('not a genetic finding');
    expect(context.food_safety.recommendation_conflicts.some((item) => item.id === 'FISH_EXCLUSION')).toBe(true);
    expect(context.food_safety.source_registry.nih_ods_selenium).toBeDefined();
    expect(context.food_safety.conditional_questions.pmdd_cycle).toBeDefined();
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'pmdd_like_mood_symptoms')).toBe(true);
    expect(context.cycle_support.source_registry.acog_premenstrual_disorders).toBeDefined();
    expect(context.cycle_support.source_registry.cdc_usmec_2024).toBeDefined();
    expect(context.cycle_support.source_registry.adenomyosis_2026_gwas).toBeDefined();
    expect(context.cycle_support.context_options.some((option) => option.id === 'menstrual_cycle')).toBe(true);
    expect(context.cycle_support.context_options.some((option) => option.id === 'androgen_reproductive')).toBe(true);
    expect(context.cycle_support.selected_context_id).toBe('menstrual_cycle');
    expect(context.cycle_support.intake_schema.fields.some((field) => field.id === 'active_ingredients')).toBe(true);
    expect(context.cycle_support.intake_schema.fields.some((field) => field.id === 'hormone_lab_timing_context')).toBe(true);
    expect(context.cycle_support.intake_schema.do_not_infer.join(' ')).toContain('adenomyosis');
    expect(context.cycle_support.intake_schema.do_not_infer.join(' ')).toContain('fixed character trait');
    expect(context.cycle_support.diary_schema.fields.some((field) => field.id === 'mood_behavior_score')).toBe(true);
    expect(context.cycle_support.diary_schema.retention_limit).toBe(180);
    expect(context.cycle_support.review_schema.metrics.some((metric) => metric.field_id === 'mood_behavior_score')).toBe(true);
    expect(context.cycle_support.diary_review).toBeNull();
    expect(context.cycle_support.marker_contexts.menstrual_cycle).toContain('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY');
    expect(context.cycle_support.marker_contexts.shared_reproductive).toContain('rs2234693');
    expect(context.cycle_support.marker_context_packs.menstrual_cycle.cardiovascular).toContain('rs6025');
    expect(context.cycle_support.relevant_evidence_layers.map((layer) => layer.id)).toEqual([
      'current_hormone_state',
      'natural_cycle_timing',
      'genetic_pathway_context',
      'hormone_product_label',
      'thrombophilia_contraception_context',
      'adenomyosis_structural_workup',
    ]);
    expect(context.cycle_support.relevant_evidence_layers.find((layer) => layer.id === 'genetic_pathway_context')?.marker_pack_ids)
      .toContain('hormones_reproductive');
    const pmddDomain = context.cycle_support.relevant_domains.find((domain) => domain.id === 'pmdd_like_mood_symptoms');
    expect(pmddDomain?.support_options.join(' ')).toContain('SSRI');
    expect(pmddDomain?.support_options.join(' ')).toContain('not on a DNA marker');
    const contraceptiveDomain = context.cycle_support.relevant_domains.find((domain) => domain.id === 'contraceptive_product_context');
    expect(contraceptiveDomain?.support_options.join(' ')).toContain('no new adverse effects');
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'cycle_linked_pain_headache_context')).toBe(true);
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'cycle_nutrition_activity_context')).toBe(true);
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'measured_hormone_context')).toBe(true);
    const moodDomain = context.cycle_support.relevant_domains.find((domain) => domain.id === 'pmdd_like_mood_symptoms');
    expect(moodDomain?.support_options.join(' ')).toContain('request space');
    expect(moodDomain?.support_options.join(' ')).toContain('not a diagnosis');
  });

  it('routes cycle-linked pain and migraine to diary, aura, and safety resources', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive', 'pain_migraine_sensory'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'cycle_linked_pain_headache',
    });
    const domain = context.cycle_support.relevant_domains.find((item) => item.id === 'cycle_linked_pain_headache_context');

    expect(domain).toBeDefined();
    expect(domain?.questions.join(' ')).toContain('aura');
    expect(domain?.support_options.join(' ')).toContain('medication-use');
    expect(domain?.support_options.join(' ')).toContain('combined hormonal contraception');
    expect(context.cycle_support.source_registry.nice_menstrual_related_migraine).toBeDefined();
    expect(context.cycle_support.source_registry.cdc_migraine_contraception_safety).toBeDefined();
    expect(context.safety_guardrails.some((rule) => rule.id === 'CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA')).toBe(true);
  });

  it('routes cyclic mood and avoidance symptoms to timing and communication support', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'cyclic_mood_symptoms',
    });

    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual([
      'cycle_phase_and_symptom_timing',
      'measured_hormone_context',
      'pmdd_like_mood_symptoms',
      'contraceptive_product_context',
      'contraception_thrombophilia_context',
      'general_symptom_day_support',
    ]);
    expect(context.cycle_support.relevant_domains.find((domain) => domain.id === 'pmdd_like_mood_symptoms')?.questions.join(' '))
      .toContain('avoidance');
    expect(context.cycle_support.relevant_domains.find((domain) => domain.id === 'pmdd_like_mood_symptoms')?.support_options.join(' '))
      .toContain('communication plan');
  });

  it('routes suspected adenomyosis to the imaging and bleeding workup overlay', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'suspected_adenomyosis',
    });
    const domain = context.cycle_support.relevant_domains.find((item) => item.id === 'heavy_bleeding_pelvic_pain');
    const overlay = context.lab_overlays.find((item) => item.domain === 'adenomyosis_heavy_bleeding_pelvic_pain');

    expect(domain?.context).toContain('not diagnosed');
    expect(domain?.confirm_with.join(' ')).toContain('transvaginal ultrasound');
    expect(overlay?.labs).toContain('transvaginal ultrasound');
    expect(overlay).not.toHaveProperty('labs_tests');
  });

  it('routes cycle-linked nutrition and activity support through food and safety guardrails', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive', 'nutrients'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'menstrual_cycle',
    });
    const domain = context.cycle_support.relevant_domains.find((item) => item.id === 'cycle_nutrition_activity_context');

    expect(domain?.support_options.join(' ')).toContain('CBC and ferritin');
    expect(domain?.support_options.join(' ')).toContain('megadose');
    expect(domain?.questions.join(' ')).toContain('exact supplement');
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'iron')).toBe(true);
    expect(context.supplement_safety.rules.some((rule) => rule.id === 'magnesium')).toBe(true);
  });

  it('routes exogenous hormone therapy context with product and monitoring sources', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'hormone_therapy_context',
    });

    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual([
      'exogenous_hormone_medication_context',
      'measured_hormone_context',
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

  it('routes preconception and fertility context across body and partner factors', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'preconception_fertility',
    });

    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual([
      'preconception_fertility_context',
      'measured_hormone_context',
      'general_symptom_day_support',
    ]);
    expect(context.cycle_support.source_registry.acog_preconception_counseling).toBeDefined();
    expect(context.cycle_support.source_registry.asrm_fertility_evaluation_women_2021).toBeDefined();
    expect(context.cycle_support.source_registry.aua_asrm_male_infertility_2024).toBeDefined();
    expect(context.safety_guardrails.some((rule) => rule.id === 'PRECONCEPTION_MEDICATION_REVIEW_NOT_IN_DNA')).toBe(true);
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

  it('routes self-reported reproductive intake when the selector is blank', () => {
    const context = buildSupportResourceContext({
      packIds: [],
      consultationMode: 'general',
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        reproductiveIntake: {
          active_ingredients: 'norethindrone 0.35 mg',
          hormone_lab_timing_context: 'cycle day 24',
        },
      },
    });

    expect(context.phenotype_prompts.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(context.cycle_support.selected_context_id).toBeNull();
    expect(context.cycle_support.context_activation).toBe('self_reported_context');
    expect(context.cycle_support.active_context_ids).toEqual(expect.arrayContaining([
      'menstrual_cycle',
      'hormone_therapy_context',
    ]));
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'measured_hormone_context')).toBe(true);
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'contraceptive_product_context')).toBe(true);
  });

  it('passes an observation-only diary review into the AI resource payload', () => {
    const context = buildSupportResourceContext({
      packIds: ['hormones_reproductive'],
      consultationMode: 'hormones_reproductive',
      reproductiveContext: 'cyclic_mood_symptoms',
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        cycleDiary: [
          { id: 'day-1', values: { entry_date: '2026-08-01', bleeding_level: '3', mood_behavior_score: '2' } },
        ],
      },
    });

    expect(context.cycle_support.diary_review?.entry_count).toBe(1);
    expect(context.cycle_support.diary_review?.co_occurrence.mood_behavior_with_bleeding_days).toBe(1);
    expect(JSON.stringify(context.cycle_support.diary_review)).not.toContain('genotype');
  });

  it('routes explicit user goals through taxonomy without inferring body or identity', () => {
    const context = buildSupportResourceContext({
      packIds: [],
      consultationMode: 'general',
      profileContext: 'I want to understand prostate screening and testosterone treatment questions.',
    });

    expect(context.phenotype_prompts.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(context.cycle_support.relevant_domains.map((domain) => domain.id)).toEqual(['androgen_reproductive_context']);
    expect(context.cycle_support.context_activation).toBe('profile_context');
    expect(context.cycle_support.active_context_ids).toEqual(['androgen_reproductive']);
    expect(context.context_routing.profile_category_ids).toContain('hormones_reproductive');
    expect(context.context_routing.profile_pack_ids).toContain('hormones_reproductive');
    expect(context.context_routing.profile_context_ids).toEqual(['androgen_reproductive']);
  });

  it('lets an explicit context override a conflicting profile-text route', () => {
    const context = buildSupportResourceContext({
      packIds: [],
      reproductiveContext: 'menstrual_cycle',
      profileContext: 'prostate screening and testosterone questions',
    });

    expect(context.cycle_support.selected_context_id).toBe('menstrual_cycle');
    expect(context.cycle_support.active_context_ids).toEqual(['menstrual_cycle']);
    expect(context.cycle_support.relevant_domains.some((domain) => domain.id === 'androgen_reproductive_context')).toBe(false);
    expect(context.context_routing.profile_context_ids).toEqual(['androgen_reproductive']);
  });

  it('routes explicit reproductive and personal safety context into the AI activity resources', () => {
    const context = buildSupportResourceContext({
      packIds: [],
      reproductiveContext: 'menstrual_cycle',
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: ['wheezing during exercise', 'daytime sleepiness'],
        labObservations: ['eGFR 45 mL/min/1.73 m²'],
      },
    });
    const activityDomainIds = context.activity_safety.relevant_domains.map((domain) => domain.id);

    expect(context.cycle_support.selected_context_id).toBe('menstrual_cycle');
    expect(context.cycle_support.relevant_domains.length).toBeGreaterThan(0);
    expect(activityDomainIds).toEqual(expect.arrayContaining([
      'hormones_reproductive',
      'respiratory_airway',
      'sleep_recovery',
      'kidney_fluid_electrolytes',
    ]));
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
    expect(context.pgx_diplotype.relevant_genes.some((gene) => gene.id === 'CYP2D6')).toBe(true);
    expect(context.pgx_diplotype.relevant_genes.some((gene) => gene.id === 'TPMT_NUDT15')).toBe(true);
    expect(context.pgx_diplotype.source_registry.cpic_clopidogrel_2022).toBeDefined();
    expect(context.pgx_diplotype.policy.display_rule).toContain('never assign');
  });

  it('keeps PRS and common-marker evidence probabilistic', () => {
    const context = buildSupportResourceContext({ packIds: ['cardiovascular'] });

    expect(context.evidence_policy.claim_policy.never_claim).toContain('diagnosis from raw DNA');
    expect(context.prs_policy.principle).toContain('Do not compute');
    expect(context.actionability_policy.safety_notes?.[0]).toContain('More markers increase coverage');
    expect(context.actionability_policy.confirm_with_cap).toBe(12);
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
    expect(context.actionability_rules.find((rule) => rule.id === 'glucose_insulin_context')?.genes).toContain('KCNQ1');
    expect(context.actionability_rules.find((rule) => rule.id === 'triglyceride_liver_context')?.genes).toContain('LPL');
    expect(context.activity_safety.relevant_domains.some((domain) => domain.id === 'metabolic')).toBe(true);
    expect(context.food_safety.source_registry.cdc_prediabetes_lifestyle).toBeDefined();
    expect(context.food_safety.source_registry.aha_dyslipidemia_2026).toBeDefined();
  });

  it('routes bone and digestive consultation context to clinical follow-up resources', () => {
    const context = buildSupportResourceContext({
      packIds: ['bone_growth_mineral_density', 'digestive_gut_microbiome'],
    });
    const actionabilityIds = context.actionability_rules.map((rule) => rule.id);

    expect(actionabilityIds).toEqual(expect.arrayContaining([
      'bone_mineral_density_context',
      'bone_rare_disorder_panel_context',
      'hypophosphatasia_alpl_panel_context',
      'digestive_inflammatory_context',
      'celiac_hla_context',
      'hereditary_fructose_intolerance_panel_context',
    ]));
    expect(context.actionability_rules.find((rule) => rule.id === 'bone_mineral_density_context')?.genes)
      .toContain('WNT16');
    expect(context.actionability_rules.find((rule) => rule.id === 'digestive_inflammatory_context')?.genes)
      .toContain('NOD2');
    expect(context.food_safety.source_registry.niams_bone_density_tests).toBeDefined();
    expect(context.food_safety.source_registry.niddk_crohns_diagnosis).toBeDefined();
    expect(context.food_safety.source_registry.niddk_ulcerative_colitis_diagnosis).toBeDefined();
    expect(context.food_safety.source_registry.niddk_celiac_tests).toBeDefined();
  });

  it('routes every phenotype domain and its source records from resource-authored pack IDs', () => {
    for (const domain of phenotypePrompts.domains) {
      const context = buildSupportResourceContext({ packIds: [domain.id] });

      expect(context.phenotype_prompts.some((item) => item.id === domain.id)).toBe(true);
      for (const sourceId of domain.sources) {
        expect(context.food_safety.source_registry[sourceId]).toBeDefined();
      }
    }

    const phenotypeIds = new Set(phenotypePrompts.domains.map((domain) => domain.id));
    for (const pack of manifest.packs) {
      if (pack.id === 'research_found') continue;
      expect(phenotypeIds.has(pack.id)).toBe(true);
    }
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
