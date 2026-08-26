import { describe, expect, it } from 'vitest';
import { deriveActionablePlan } from './actionabilityEngine';
import type { GeneratedReport, EvaluatedMarker } from '../types/genomics';
import type { PersonalSafetyContext } from './personalSafetyContext';

function marker(overrides: Partial<EvaluatedMarker>): EvaluatedMarker {
  return {
    link_id: 'test:marker',
    rsid: 'rs1801133',
    gene: 'MTHFR',
    variant_name: 'test marker',
    chromosome: '1',
    position: null,
    user_genotype: 'TT',
    normalized_genotype: null,
    effect_allele: 'T',
    effect_count: 2,
    impact: 'test impact',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'test interpretation',
    effect_direction: 'risk',
    severity_class: 'moderate_risk',
    assertion_status: 'Verified',
    interpretation_allowed: true,
    sources: [],
    db_enriched_sources: [],
    clinvar_significance: null,
    clinvar_conditions: null,
    clinvar_review_status: null,
    gwas_top_trait: null,
    gwas_best_pvalue: null,
    gwas_association_count: null,
    population_af: null,
    population_rarity: null,
    confirm_with: [],
    do_not_claim: [],
    ...overrides,
  };
}

function report(markers: EvaluatedMarker[]): GeneratedReport {
  return {
    schema_version: '2.0.0',
    export_format: 'normalized_sparse',
    generated_at: new Date(0).toISOString(),
    title: 'test',
    description: 'test',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [
      {
        name: 'Nutrients',
        markers,
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ],
  };
}

describe('actionability engine safety policy', () => {
  it('qualifies diet guidance and never revives the MTHFR folic-acid avoidance myth', () => {
    const plan = deriveActionablePlan(report([marker({})]));
    expect(plan.diet.favor.some((item) => item.startsWith('Consider only if symptoms'))).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('Folic acid fortified foods');
    expect(plan.supplements).toHaveLength(0);
    expect(plan.safetyNotes.some((note) => note.includes('More markers increase coverage'))).toBe(true);
  });

  it('qualifies iron avoidance as clinical-confirmation guidance', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs1800562',
        gene: 'HFE',
        evidence_tier: 'A_clinically_relevant_rare_variant',
        severity_class: 'high_risk',
      }),
    ]));
    expect(plan.diet.avoid.some((item) => item.startsWith('Do not avoid solely from raw DNA'))).toBe(true);
    expect(plan.supplements.some((item) => item.reason.includes('Discuss with a clinician or pharmacist'))).toBe(true);
  });

  it('surfaces cycle-aware activity and medication guardrails for reproductive context', () => {
    const plan = deriveActionablePlan({
      ...report([marker({
        rsid: 'PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY',
        gene: 'ESR1/ESR2/PGR',
        sex_scope: 'menstrual_cycle_context',
      })]),
      sections: [{
        ...report([]).sections[0],
        name: 'Menstrual Cycle, Hormones & Reproductive Context',
        markers: [marker({
          rsid: 'PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY',
          gene: 'ESR1/ESR2/PGR',
          sex_scope: 'menstrual_cycle_context',
        })],
      }],
    }, { reproductiveContext: 'menstrual_cycle' });
    expect(plan.activity.relevantDomains.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(plan.activity.stopAndEscalate.some((item) => item.includes('Chest pain'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('contraceptive'))).toBe(true);
    expect(plan.medication.askFor.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_phase_and_symptom_timing')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'heavy_bleeding_pelvic_pain')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_linked_pain_headache_context')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_nutrition_activity_context')).toBe(true);
  });

  it('surfaces context-relevant supplement safety rules without inventing a supplement need', () => {
    const cyclePlan = deriveActionablePlan(report([]), { reproductiveContext: 'menstrual_cycle' });
    const cycleRuleIds = cyclePlan.supplementSafety.relevantRules.map((rule) => rule.id);
    expect(cycleRuleIds).toEqual(expect.arrayContaining(['iron', 'magnesium', 'calcium', 'vitamin_b6']));
    expect(cyclePlan.supplements).toHaveLength(0);

    const neutralPlan = deriveActionablePlan(report([]));
    expect(neutralPlan.supplementSafety.relevantRules.map((rule) => rule.id)).not.toContain('iron');
  });

  it('routes an explicitly named B6 product to the neuropathy guardrail', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: ['pyridoxine 50 mg'],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const b6Rule = plan.supplementSafety.relevantRules.find((rule) => rule.id === 'vitamin_b6');
    expect(b6Rule?.avoid.join(' ')).toContain('neurologic symptoms');
    expect(b6Rule?.sources).toContain('nih_ods_vitamin_b6');
  });

  it('keeps PGx medication guidance at the review boundary', () => {
    const plan = deriveActionablePlan({
      ...report([]),
      sections: [{
        ...report([]).sections[0],
        name: 'Pharmacogenomics (PGx)',
        markers: [marker({ gene: 'CYP2C19', rsid: 'rs4244285' })],
      }],
    });
    expect(plan.medication.rules.some((item) => item.includes('raw DNA alone'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('HLA'))).toBe(true);
  });

  it('surfaces named medication pathways without turning raw SNPs into prescriptions', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C19', rsid: 'rs4244285' }),
      marker({ gene: 'CYP2D6', rsid: 'rs3892097' }),
      marker({ gene: 'SLCO1B1', rsid: 'rs4149056' }),
      marker({ gene: 'VKORC1', rsid: 'rs9923231' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('clopidogrel');
    expect(medicationText).toContain('warfarin');
    expect(medicationText).toContain('statin');
    expect(medicationText).toContain('codeine or tramadol');
    expect(medicationText).toContain('tamoxifen');
    expect(medicationText).toContain('antidepressant');
    expect(medicationText).toContain('raw DNA');
    expect(medicationText).not.toContain('Start clopidogrel');
    expect(medicationText).not.toContain('Change warfarin dose');
  });

  it('routes CYP2C9 to NSAID exposure and menstrual-pain safety review', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C9', rsid: 'rs1057910', variant_name: 'CYP2C9*3' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP2C9') && test.name.includes('NSAID'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Kidney function/eGFR'))).toBe(true);
    expect(medicationText).toContain('menstrual or other pain');
    expect(medicationText).toContain('raw-array data');
    expect(medicationText).not.toContain('dose this NSAID');
  });

  it('routes CYP3A5 to transplant-only tacrolimus monitoring', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP3A5', rsid: 'rs776746', variant_name: 'CYP3A5*3' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP3A5') && test.name.includes('tacrolimus'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('whole-blood trough'))).toBe(true);
    expect(medicationText).toContain('transplant care');
    expect(medicationText).toContain('cannot determine a tacrolimus dose');
  });

  it('routes additional high-impact PGx markers to bounded clinical safety review', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'BCHE', rsid: 'rs1803274', variant_name: 'BCHE anesthesia context' }),
      marker({ gene: 'UGT1A1', rsid: 'rs887829', variant_name: 'UGT1A1*28 tag' }),
      marker({ gene: 'NAT2', rsid: 'rs1801280', variant_name: 'NAT2*5 component' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('pseudocholinesterase'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('UGT1A1') && test.name.includes('irinotecan'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('NAT2') && test.name.includes('hydralazine'))).toBe(true);
    expect(medicationText).toContain('succinylcholine');
    expect(medicationText).toContain('irinotecan');
    expect(medicationText).toContain('hydralazine');
    expect(medicationText).not.toContain('Change hydralazine dose to');
  });

  it('routes nutrient supplement safety from related pathway markers', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'DIO2', rsid: 'rs225014' }),
      marker({ gene: 'VDR', rsid: 'rs2228570' }),
      marker({ gene: 'SLC30A8', rsid: 'rs13266634' }),
      marker({ gene: 'SLC12A3', rsid: 'rs13333226' }),
      marker({ gene: 'VKORC1', rsid: 'rs9923231' }),
    ]));
    const ruleIds = plan.supplementSafety.relevantRules.map((rule) => rule.id);

    expect(ruleIds).toEqual(expect.arrayContaining(['iodine', 'calcium', 'zinc', 'potassium', 'vitamin_k']));
  });

  it('applies per-profile medication, supplement, allergy, symptom, and lab context', () => {
    const personalSafetyContext: PersonalSafetyContext = {
      medications: ['Norethindrone progestin-only birth control', 'warfarin'],
      supplements: ['zinc'],
      allergies: ['fish allergy'],
      symptoms: ['mood changes in the late luteal phase'],
      labObservations: ['ferritin 18 ng/mL (2026-05-10)'],
    };
    const plan = deriveActionablePlan(report([]), { personalSafetyContext });
    const medicationText = plan.medication.rules.join(' ');
    const supplementRuleIds = plan.supplementSafety.relevantRules.map((rule) => rule.id);

    expect(medicationText).toContain('contraceptive product');
    expect(supplementRuleIds).toContain('zinc');
    expect(supplementRuleIds).toContain('omega3');
    expect(supplementRuleIds).toContain('vitamin_k');
    expect(plan.personalContext.medications).toEqual(personalSafetyContext.medications);
    expect(plan.personalContext.symptoms).toEqual(personalSafetyContext.symptoms);
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('measured phenotype outranks');
    expect(plan.cycleSupport.relevantDomains).toHaveLength(0);
  });

  it('recognizes an exact progestin ingredient without inferring that it is contraception', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: ['norethindrone'],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).toContain('active ingredients');
    expect(medicationText).not.toContain('contraceptive product');
  });

  it('uses structured ingredient context for label review without inferring contraception', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        reproductiveIntake: {
          active_ingredients: 'norethindrone 0.35 mg',
          question_or_belief_to_verify: 'I think this is progesterone-only',
        },
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).toContain('active ingredients');
    expect(medicationText).not.toContain('contraceptive product');
    expect(plan.personalContext.reproductiveIntake?.active_ingredients).toBe('norethindrone 0.35 mg');
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('Structured cycle and hormone details');
  });

  it('keeps diary observations separate from genotype guidance', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        cycleDiary: [{ id: 'day-1', values: { entry_date: '2026-08-01', mood_behavior_score: '3' } }],
      },
    });

    expect(plan.personalContext.cycleDiary?.[0].values.mood_behavior_score).toBe('3');
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('Missing entries are not symptom-free days');
    expect(plan.supplements).toHaveLength(0);
  });

  it('separates general hormone therapy from contraceptive composition warnings', () => {
    const plan = deriveActionablePlan(report([]), {
      reproductiveContext: 'hormone_therapy_context',
      personalSafetyContext: {
        medications: ['testosterone cypionate'],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).not.toContain('contraceptive product');
    expect(plan.cycleSupport.relevantDomains.map((domain) => domain.id)).toEqual([
      'exogenous_hormone_medication_context',
      'general_symptom_day_support',
    ]);
  });

  it('keeps preconception medication review separate from fertility prediction', () => {
    const plan = deriveActionablePlan(report([]), { reproductiveContext: 'preconception_fertility' });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('planning pregnancy or pursuing fertility care');
    expect(plan.cycleSupport.relevantDomains.map((domain) => domain.id)).toContain('preconception_fertility_context');
    expect(plan.cycleSupport.relevantDomains.find((domain) => domain.id === 'preconception_fertility_context')?.context)
      .toContain('cannot measure ovarian reserve');
  });

  it('keeps cycle-linked pain and migraine support at the clinical-context boundary', () => {
    const plan = deriveActionablePlan(report([]), { reproductiveContext: 'cycle_linked_pain_headache' });
    const domain = plan.cycleSupport.relevantDomains.find((item) => item.id === 'cycle_linked_pain_headache_context');

    expect(domain?.context).toContain('cannot diagnose');
    expect(domain?.support_options.join(' ')).toContain('DNA result');
    expect(plan.medication.rules.join(' ')).toContain('contraceptive product');
  });

  it('connects confirmed thrombophilia context to contraceptive review without prescribing a change', () => {
    const plan = deriveActionablePlan({
      ...report([]),
      sections: [{
        ...report([]).sections[0],
        name: 'Cardiovascular Health',
        markers: [marker({
          gene: 'F5',
          rsid: 'rs6025',
          severity_class: 'confirmation_required',
          clinical_confirmation_required: true,
        })],
      }],
    });
    expect(plan.medication.rules.some((item) => item.includes('whether it contains estrogen'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Factor V Leiden'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('do not change medication'))).toBe(true);
  });

  it('turns metabolic markers into conditional glucose and activity follow-up', () => {
    const base = report([marker({
      rsid: 'rs7903146',
      gene: 'TCF7L2',
      interpretation: 'Common TCF7L2 glucose and insulin-secretion association',
    })]);
    const plan = deriveActionablePlan({
      ...base,
      sections: [{ ...base.sections[0], name: 'Metabolic Health' }],
    });

    expect(plan.diet.favor.some((item) => item.includes('fiber-rich foods'))).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'HbA1c')).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Fasting plasma glucose')).toBe(true);
    expect(plan.activity.relevantDomains.some((domain) => domain.id === 'metabolic')).toBe(true);
    expect(plan.diet.avoid.some((item) => item.includes('diabetes diagnosis'))).toBe(true);
  });

  it('uses LPA context to request phenotype testing without prescribing therapy', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs10455872',
      gene: 'LPA',
      interpretation: 'LPA lipoprotein(a) association; measured Lp(a) remains actionable',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Lipoprotein(a)') && test.category === 'Heart & lipids')).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('ApoB'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('statin'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('raw DNA'))).toBe(true);
  });

  it('routes thyroid pathway markers to measured thyroid follow-up', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs225014',
      gene: 'DIO2',
      interpretation: 'Thyroid conversion context marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name === 'TSH')).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Free T4')).toBe(true);
    expect(plan.diet.avoid.some((item) => item.includes('High-dose iodine'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('levothyroxine'))).toBe(true);
  });

  it('routes sleep markers to symptom tracking and sleep-apnea evaluation', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs1801260',
      gene: 'CLOCK',
      interpretation: 'Circadian timing association marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Sleep study'))).toBe(true);
    expect(plan.diet.favor.some((item) => item.includes('consistent sleep opportunity'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('chronotype'))).toBe(true);
  });

  it('keeps food-allergy actionability exposure-led rather than genotype-led', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs10156191',
      gene: 'AOC1',
      interpretation: 'Histamine-processing pathway context marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Allergist-directed'))).toBe(true);
    expect(plan.diet.avoid.some((item) => item.includes('Permanent food elimination'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('AOC1'))).toBe(true);
  });

  it('connects high-impact PGx findings to drug-specific clinical review', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs3918290',
        gene: 'DPYD',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'DPYD*2A context marker; confirm clinically before fluoropyrimidine therapy.',
      }),
      marker({
        rsid: 'rs116855232',
        gene: 'NUDT15',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'NUDT15 thiopurine context marker; confirm clinically.',
      }),
      marker({
        rsid: 'HLA-B*57:01',
        gene: 'HLA-B',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'HLA drug-hypersensitivity context marker; confirm with clinical HLA typing.',
      }),
      marker({
        rsid: 'PANEL_G6PD_DEFICIENCY',
        gene: 'G6PD',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'G6PD deficiency context panel; confirm clinically.',
      }),
      marker({
        rsid: 'PANEL_RYR1_CACNA1S_MALIGNANT_HYPERTHERMIA',
        gene: 'RYR1/CACNA1S',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'Malignant-hyperthermia susceptibility context panel; confirm clinically.',
      }),
    ]));

    expect(plan.labTests.some((test) => test.name.includes('DPYD'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('TPMT/NUDT15'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HLA'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('G6PD'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('RYR1/CACNA1S'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('fluorouracil'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('thiopurine'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('abacavir'))).toBe(true);
  });

  it('routes exact HLA alleles to the matching drug pathway without cross-triggering allele rules', () => {
    const abacavirPlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-B*57:01',
      gene: 'HLA-B',
      variant_name: 'HLA-B*57:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-B*57:01 context; confirm with clinical HLA typing.',
    })]));
    const allopurinolPlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-B*58:01',
      gene: 'HLA-B',
      variant_name: 'HLA-B*58:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-B*58:01 context; confirm with clinical HLA typing.',
    })]));
    const carbamazepinePlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-A*31:01',
      gene: 'HLA-A',
      variant_name: 'HLA-A*31:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-A*31:01 context; confirm with clinical HLA typing.',
    })]));

    expect(abacavirPlan.medication.rules.some((item) => item.includes('validated clinical HLA-B*57:01 result'))).toBe(true);
    expect(abacavirPlan.labTests.some((test) => test.name.includes('HLA-B*57:01') && test.name.includes('abacavir'))).toBe(true);
    expect(allopurinolPlan.medication.rules.some((item) => item.includes('validated HLA-B*58:01 status'))).toBe(true);
    expect(allopurinolPlan.labTests.some((test) => test.name.includes('HLA-B*58:01') && test.name.includes('allopurinol'))).toBe(true);
    expect(allopurinolPlan.medication.rules.some((item) => item.includes('validated clinical HLA-B*57:01 result'))).toBe(false);
    expect(carbamazepinePlan.medication.rules.some((item) => item.includes('carbamazepine or oxcarbazepine'))).toBe(true);
    expect(carbamazepinePlan.labTests.some((test) => test.name.includes('HLA-B*15:02') && test.name.includes('HLA-A*31:01'))).toBe(true);
  });

  it('routes additional clinical PGx pathways without turning them into raw-array prescriptions', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C19', rsid: 'rs4244285' }),
      marker({ gene: 'CYP2B6', rsid: 'rs3745274' }),
      marker({ gene: 'CYP2C9', rsid: 'rs1057910' }),
      marker({ gene: 'HLA-B', rsid: 'HLA-B*15:02' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP2C19') && test.name.includes('omeprazole'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('CYP2B6') && test.name.includes('efavirenz'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('CYP2C9') && test.name.includes('phenytoin'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HLA-B*15:02') && test.name.includes('phenytoin'))).toBe(true);
    expect(medicationText).toContain('proton-pump inhibitor');
    expect(medicationText).toContain('efavirenz');
    expect(medicationText).toContain('phenytoin');
    expect(medicationText).not.toContain('Change phenytoin dose to');
  });

  it('does not infer reproductive applicability from hormone marker text', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs2234693',
      gene: 'ESR1',
      variant_name: 'estrogen receptor alpha marker',
      sex_scope: 'menstrual_cycle_context',
      interpretation: 'Hormone context marker; not diagnostic.',
      effect_direction: 'context_dependent',
    })]));

    expect(plan.cycleSupport.relevantDomains).toHaveLength(0);
    expect(plan.medication.rules.some((item) => item.includes('Raw DNA cannot identify a person'))).toBe(false);
  });

  it('supports an explicitly selected androgen context without showing menstrual guidance', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'PANEL_AR_CAG_REPEAT_CALLOUT',
      gene: 'AR',
      sex_scope: 'androgen_reproductive_context',
      effect_direction: 'context_dependent',
    })]), { reproductiveContext: 'androgen_reproductive' });

    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'androgen_reproductive_context')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_phase_and_symptom_timing')).toBe(false);
  });
});
