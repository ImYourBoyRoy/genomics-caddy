import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import { buildBpdSymptomContext } from './bpdSymptomContext';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'neuropsych:rs4680',
    rsid: 'rs4680',
    gene: 'COMT',
    variant_name: 'Synthetic marker',
    chromosome: '22',
    position: null,
    user_genotype: 'AG',
    normalized_genotype: 'AG',
    effect_allele: 'A',
    effect_count: 1,
    impact: 'Synthetic context',
    evidence_tier: 'D_research_only',
    interpretation: 'Synthetic research context.',
    effect_direction: 'context_dependent',
    severity_class: 'context_dependent',
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
    generated_at: '2026-10-29T00:00:00.000Z',
    title: 'Synthetic report',
    description: 'Synthetic report',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [{
      name: 'Neurotransmitter & Mood Resiliency',
      markers,
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }],
  };
}

describe('BPD-adjacent symptom context', () => {
  it('maps callable neuropsych pathways to symptom domains without exposing genotypes', () => {
    const context = buildBpdSymptomContext(report([
      marker(),
      marker({
        link_id: 'neuropsych:rs2143288',
        rsid: 'rs2143288',
        gene: 'LINC00970/ATP1B1',
      }),
      marker({
        link_id: 'neuropsych:rs6311',
        rsid: 'rs6311',
        gene: 'HTR2A',
        assertion_status: 'NotInRawFile',
        interpretation_allowed: false,
      }),
    ]));

    expect(context.hasPathwayContext).toBe(true);
    expect(context.hasBpdLocusContext).toBe(true);
    expect(context.display.summary).toContain('may be relevant to experiencing specific patterns');
    expect(context.domains.map((domain) => domain.id)).toEqual(expect.arrayContaining([
      'emotion_regulation',
      'interpersonal_anger_recovery',
      'impulsivity_reward',
      'sexual_behavior_context',
      'recent_bpd_gwas_loci',
    ]));
    expect(context.matchedMarkerLinkIds).toEqual(expect.arrayContaining([
      'neuropsych:rs4680',
      'neuropsych:rs2143288',
    ]));
    expect(context.domains.find((domain) => domain.id === 'sexual_behavior_context')?.matchedMarkerLinkIds).toEqual([]);
    expect(context.domains.find((domain) => domain.id === 'impulsivity_reward')?.summary)
      .toContain('may be relevant to experiencing reward urgency');
    expect(context.domains.find((domain) => domain.id === 'interpersonal_anger_recovery')?.summary)
      .toContain('rapid anger escalation');
    expect(JSON.stringify(context)).not.toContain('AG');
  });

  it('stays empty when the neuropsych section has no callable active findings', () => {
    const context = buildBpdSymptomContext(report([marker({
      effect_count: 0,
      user_genotype: '--',
    })]));

    expect(context.hasPathwayContext).toBe(false);
    expect(context.hasBpdLocusContext).toBe(false);
    expect(context.domains).toEqual([]);
    expect(context.matchedMarkerCount).toBe(0);
  });

  it('keeps broad neuropsych pathway context separate from a BPD-locus match', () => {
    const context = buildBpdSymptomContext(report([marker()]));

    expect(context.hasPathwayContext).toBe(true);
    expect(context.hasBpdLocusContext).toBe(false);
    expect(context.domains.some((domain) => domain.id === 'recent_bpd_gwas_loci')).toBe(false);
  });
});
