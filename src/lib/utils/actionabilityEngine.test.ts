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
});
