import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import {
  buildConditionCoverageSummaries,
  buildConditionEvidenceSummaries,
  getConditionCoverageGaps,
} from './conditionEvidence';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'test:condition-marker',
    rsid: 'rs2234693',
    gene: 'ESR1',
    variant_name: 'Synthetic condition marker',
    chromosome: '6',
    position: null,
    user_genotype: 'SYNTHETIC_CALL',
    normalized_genotype: 'SYNTHETIC_CALL',
    effect_allele: 'A',
    effect_count: 1,
    impact: 'Synthetic research context',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'Synthetic condition-linked context',
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

function report(markers: EvaluatedMarker[], secondSection = false): GeneratedReport {
  const sections = [{
    name: 'Synthetic health area',
    markers,
    section_signal_score: 0,
    summary: {} as GeneratedReport['sections'][number]['summary'],
  }];
  if (secondSection) {
    sections.push({
      name: 'Synthetic duplicate area',
      markers: [markers[0]],
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    });
  }
  return {
    schema_version: '2.0.0',
    export_format: 'normalized_sparse',
    generated_at: '2026-08-31T00:00:00.000Z',
    title: 'Synthetic condition report',
    description: 'Synthetic condition report',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections,
  };
}

describe('condition-level evidence aggregation', () => {
  it('reports PMDD-related marker coverage without turning coverage into probability', () => {
    const ids = [
      'rs2234693', 'rs9340799', 'rs4986938', 'rs1256049', 'rs1042838', 'rs10895068',
    ];
    const summary = buildConditionEvidenceSummaries(report(ids.map((rsid) => marker({ rsid })) ))
      .find((item) => item.id === 'pmdd_steroid_sensitivity');

    expect(summary).toMatchObject({
      label: 'PMDD-related steroid sensitivity',
      coded_indicator_count: 7,
      callable_indicator_count: 6,
      matched_indicator_count: 6,
      available_indicator_count: 6,
      relative_signal: 'moderate',
    });
    expect(summary?.plain_meaning).toContain('normal ovarian-steroid changes');
    expect(summary?.plain_meaning).not.toContain('probability');
  });

  it('collapses duplicate pack rows before counting indicators', () => {
    const summary = buildConditionEvidenceSummaries(report([
      marker({ rsid: 'rs2234693', link_id: 'test:pmdd:one' }),
      marker({ rsid: 'rs2234693', link_id: 'test:pmdd:two' }),
    ], true)).find((item) => item.id === 'pmdd_steroid_sensitivity');

    expect(summary?.matched_indicator_count).toBe(1);
    expect(summary?.callable_indicator_count).toBe(1);
    expect(summary?.matched_marker_link_ids).toEqual(['test:pmdd:one']);
  });

  it('keeps common connective-tissue markers as a limited EDS-spectrum signal', () => {
    const summary = buildConditionEvidenceSummaries(report([
      marker({ rsid: 'rs1800012', gene: 'COL1A1' }),
    ])).find((item) => item.id === 'eds_spectrum_connective_tissue');

    expect(summary).toMatchObject({
      label: 'EDS-spectrum connective-tissue pattern',
      coded_indicator_count: 8,
      matched_indicator_count: 1,
      relative_signal: 'limited',
      diagnostic_capability: 'clinical_evaluation_required',
    });
    expect(summary?.plain_meaning).toContain('do not identify hEDS');
  });

  it('surfaces a clinical-variant condition label without claiming the array call is confirmed', () => {
    const summary = buildConditionEvidenceSummaries(report([marker({
      rsid: 'rs-clinical',
      gene: 'CLINICAL1',
      link_id: 'test:clinical',
      evidence_tier: 'A_clinical_variant',
      clinical_confirmation_required: true,
      clinical_semantics: {
        condition_label: 'Synthetic inherited condition',
        interpretation_class: 'clinically_actionable_variant',
        inheritance_model: 'autosomal_dominant',
        clinical_state: 'unknown',
      },
    })])).find((item) => item.label === 'Synthetic inherited condition');

    expect(summary).toMatchObject({
      diagnostic_capability: 'clinical_variant_can_establish_when_confirmed',
      relative_signal: 'higher',
      clinical_states: ['unknown'],
      inheritance_models: ['autosomal_dominant'],
    });
    expect(summary?.plain_meaning).toBe('A biological pathway signal is present.');
  });

  it('keeps uncalled authored condition members in the denominator', () => {
    const summary = buildConditionEvidenceSummaries(report([
      marker({
        rsid: 'rs-clinical-called',
        link_id: 'test:clinical:called',
        clinical_confirmation_required: true,
        clinical_semantics: {
          condition_label: 'Synthetic inherited condition',
          interpretation_class: 'clinically_actionable_variant',
          inheritance_model: 'autosomal_dominant',
          clinical_state: 'unknown',
        },
      }),
      marker({
        rsid: 'rs-clinical-uncalled',
        link_id: 'test:clinical:uncalled',
        user_genotype: '--',
        assertion_status: 'NoData',
        effect_count: null,
        clinical_confirmation_required: true,
        clinical_semantics: {
          condition_label: 'Synthetic inherited condition',
          interpretation_class: 'clinically_actionable_variant',
          inheritance_model: 'autosomal_dominant',
          clinical_state: 'unknown',
        },
      }),
    ])).find((item) => item.label === 'Synthetic inherited condition');

    expect(summary).toMatchObject({
      coded_indicator_count: 2,
      available_indicator_count: 2,
      callable_indicator_count: 1,
      matched_indicator_count: 1,
      relative_signal: 'moderate',
    });
  });

  it('does not invent a POTS result when no POTS model is defined', () => {
    expect(buildConditionEvidenceSummaries(report([marker({
      rsid: 'rs-pots-research',
      gene: 'POTS_RESEARCH',
    })]))).toEqual([]);
    expect(getConditionCoverageGaps()).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: 'pots', status: 'emerging_research' }),
      expect.objectContaining({ id: 'hypermobile_eds', status: 'clinical' }),
    ]));
  });

  it('does not count a called marker with no aligned effect allele as a matched indicator', () => {
    expect(buildConditionEvidenceSummaries(report([marker({
      rsid: 'rs2234693',
      effect_count: 0,
    })])).find((item) => item.id === 'pmdd_steroid_sensitivity')).toBeUndefined();
  });

  it('preserves zero and partial condition coverage for downstream review', () => {
    const coverage = buildConditionCoverageSummaries(report([marker({
      rsid: 'rs2234693',
    })]));
    const pmdd = coverage.find((item) => item.id === 'pmdd_steroid_sensitivity');
    const thyroid = coverage.find((item) => item.id === 'autoimmune_thyroid_context');

    expect(pmdd).toMatchObject({
      coded_indicator_count: 7,
      available_indicator_count: 1,
      callable_indicator_count: 1,
      matched_indicator_count: 1,
      missing_indicator_count: 6,
      status: 'partial',
    });
    expect(thyroid).toMatchObject({
      available_indicator_count: 0,
      callable_indicator_count: 0,
      missing_indicator_count: 3,
      status: 'not_observed',
    });
    expect(JSON.stringify(coverage)).not.toContain('SYNTHETIC_CALL');
  });

  it('distinguishes unavailable condition components from absent report components', () => {
    const coverage = buildConditionCoverageSummaries(report([marker({
      rsid: 'rs2234693',
      variant_type: 'hla_tag',
      assertion_status: 'NotEvaluated',
      callability_state: 'not_callable',
      interpretation_allowed: false,
      effect_count: null,
    })]));
    expect(coverage.find((item) => item.id === 'pmdd_steroid_sensitivity')).toMatchObject({
      available_indicator_count: 1,
      callable_indicator_count: 0,
      not_callable_indicator_count: 1,
      missing_indicator_count: 6,
      status: 'unavailable',
    });
  });

  it('does not relabel an unknown legacy callability state as not present', () => {
    const coverage = buildConditionCoverageSummaries(report([marker({
      rsid: 'rs2234693',
      assertion_status: 'Verified',
      callability_state: undefined,
      user_genotype: '--',
      normalized_genotype: '--',
      effect_count: null,
    })]));
    expect(coverage.find((item) => item.id === 'pmdd_steroid_sensitivity')).toMatchObject({
      available_indicator_count: 1,
      not_present_indicator_count: 0,
      unknown_indicator_count: 1,
      missing_indicator_count: 6,
      status: 'unavailable',
    });
  });
});
