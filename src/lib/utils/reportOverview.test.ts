import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import { computeReportOverviewStats } from './reportOverview';

function marker(overrides: Partial<EvaluatedMarker>): EvaluatedMarker {
  return {
    link_id: 'test:overview:marker',
    rsid: 'rs-overview-marker',
    gene: 'OVERVIEW1',
    variant_name: 'overview test signal',
    chromosome: '1',
    position: null,
    user_genotype: 'called',
    normalized_genotype: null,
    effect_allele: 'effect',
    effect_count: 1,
    impact: 'overview test impact',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'overview test interpretation',
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
    title: 'overview test',
    description: 'overview test',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [{
      name: 'Overview test area',
      markers,
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }],
  };
}

describe('report overview statistics', () => {
  it('counts a reused locus once while retaining the strongest queue classification', () => {
    const stats = computeReportOverviewStats(report([
      marker({ link_id: 'test:overview:source-a' }),
      marker({
        link_id: 'test:overview:source-b',
        severity_class: 'high_risk',
      }),
    ]));

    expect(stats).toEqual({
      priority: 1,
      higherConcern: 1,
      context: 0,
      protective: 0,
      unassessed: 0,
    });
  });

  it('keeps protective and unknown signals out of the active review queue', () => {
    const stats = computeReportOverviewStats(report([
      marker({
        rsid: 'rs-overview-protective',
        link_id: 'test:overview:protective',
        severity_class: 'protective',
        effect_direction: 'protective',
      }),
      marker({
        rsid: 'rs-overview-unknown',
        link_id: 'test:overview:unknown',
        user_genotype: '',
        assertion_status: 'NoData',
      }),
    ]));

    expect(stats).toEqual({
      priority: 0,
      higherConcern: 0,
      context: 0,
      protective: 1,
      unassessed: 1,
    });
  });

  it('keeps called context signals visible in the context bucket', () => {
    const stats = computeReportOverviewStats(report([marker({
      rsid: 'rs-overview-context',
      link_id: 'test:overview:context',
      severity_class: 'context_dependent',
    })]));

    expect(stats.context).toBe(1);
    expect(stats.priority).toBe(0);
    expect(stats.unassessed).toBe(0);
  });

  it('does not relabel benign technical rows as context findings', () => {
    const stats = computeReportOverviewStats(report([marker({
      rsid: 'rs-overview-benign',
      link_id: 'test:overview:benign',
      severity_class: 'benign',
      effect_direction: 'no_claim',
    })]));

    expect(stats.context).toBe(0);
    expect(stats.priority).toBe(0);
    expect(stats.protective).toBe(0);
    expect(stats.unassessed).toBe(0);
  });
});
