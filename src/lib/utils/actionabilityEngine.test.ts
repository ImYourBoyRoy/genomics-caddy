import { describe, expect, it } from 'vitest';
import { deriveActionablePlan } from './actionabilityEngine';
import type { GeneratedReport, EvaluatedMarker } from '../types/genomics';

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
    });
    expect(plan.activity.relevantDomains.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(plan.activity.stopAndEscalate.some((item) => item.includes('Chest pain'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('contraceptive'))).toBe(true);
    expect(plan.medication.askFor.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_phase_and_symptom_timing')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'heavy_bleeding_pelvic_pain')).toBe(true);
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
});
