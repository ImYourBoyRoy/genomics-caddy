import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import { buildCanonicalFindingGroups, canonicalFindingKey } from './findingIdentity';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'test:marker',
    rsid: 'rs100',
    gene: 'TEST1',
    variant_name: 'Test marker',
    chromosome: '1',
    position: null,
    user_genotype: 'AA',
    normalized_genotype: null,
    effect_allele: 'A',
    effect_count: 1,
    impact: 'Test impact',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'Test interpretation',
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
    reference_ids: [],
    ...overrides,
  };
}

function report(sections: GeneratedReport['sections']): GeneratedReport {
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
    sections,
  };
}

describe('canonical finding identity', () => {
  it('groups a repeated locus while retaining every contributing topic and source row', () => {
    const groups = buildCanonicalFindingGroups(report([
      {
        name: 'Cardiovascular',
        markers: [marker({ link_id: 'test:cardio', reference_ids: ['ref:cardio'] })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Nutrients',
        markers: [marker({ link_id: 'test:nutrients', reference_ids: ['ref:nutrients'] })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Sleep',
        markers: [marker({ rsid: 'rs200', link_id: 'test:sleep' })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ]));

    expect(groups).toHaveLength(2);
    const repeated = groups.find((group) => group.rsids.includes('rs100'));
    expect(repeated?.findingId).toBe('finding-rsid-rs100');
    expect(repeated?.sourceMarkerIds).toEqual(['test:cardio', 'test:nutrients']);
    expect(repeated?.topicIds).toEqual(['cardiovascular', 'nutrients']);
    expect(repeated?.healthAreaLabels).toEqual(['Cardiovascular', 'Nutrients']);
    expect(repeated?.referenceIds).toEqual(['ref:cardio', 'ref:nutrients']);
    expect(repeated?.sourceMarkers).toHaveLength(2);
  });

  it('keeps unknown calls unknown and marks a known plus unknown group as mixed', () => {
    const groups = buildCanonicalFindingGroups(report([
      {
        name: 'Nutrients',
        markers: [marker({
          link_id: 'test:known',
          rsid: 'rs300',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Metabolic',
        markers: [marker({
          link_id: 'test:unknown',
          rsid: 'rs300',
          user_genotype: '--',
          assertion_status: 'NoData',
          interpretation_allowed: false,
          severity_class: 'no_data',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Sleep',
        markers: [marker({
          link_id: 'test:only-unknown',
          rsid: 'rs400',
          user_genotype: '--',
          assertion_status: 'NotInRawFile',
          interpretation_allowed: false,
          severity_class: 'no_data',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ]));

    expect(groups.find((group) => group.rsids.includes('rs300'))?.callState).toBe('mixed');
    expect(groups.find((group) => group.rsids.includes('rs400'))?.callState).toBe('unknown');
    expect(groups.find((group) => group.rsids.includes('rs400'))?.severityClasses).toEqual(['no_data']);
  });

  it('keeps same-rsid assertions separate when their biomedical semantics conflict', () => {
    const groups = buildCanonicalFindingGroups(report([
      {
        name: 'Cardiovascular',
        markers: [marker({
          link_id: 'test:rs100-risk',
          effect_allele: 'A',
          effect_direction: 'risk',
          variant_name: 'Lipid context',
          clinical_semantics: {
            condition_label: 'Atherogenic lipid context',
            interpretation_class: 'susceptibility_context',
            inheritance_model: 'unknown',
            clinical_state: 'unknown',
          },
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Nutrients',
        markers: [marker({
          link_id: 'test:rs100-trait',
          effect_allele: 'G',
          effect_direction: 'trait',
          variant_name: 'Nutrient transport context',
          clinical_semantics: {
            condition_label: 'Nutrient transport context',
            interpretation_class: 'trait_context',
            inheritance_model: 'unknown',
            clinical_state: 'not_applicable',
          },
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ]));

    expect(groups).toHaveLength(2);
    expect(groups.map((group) => group.sourceMarkerIds)).toEqual([
      ['test:rs100-risk'],
      ['test:rs100-trait'],
    ]);
  });

  it('retains explicit disease and inheritance semantics across grouped source rows', () => {
    const groups = buildCanonicalFindingGroups(report([{
      name: 'Inherited conditions',
      markers: [marker({
        clinical_semantics: {
          condition_label: 'Synthetic condition',
          interpretation_class: 'carrier_possibility',
          inheritance_model: 'autosomal_recessive',
          clinical_state: 'carrier_possibility',
        },
      })],
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }]));

    expect(groups[0]?.conditionLabels).toEqual(['Synthetic condition']);
    expect(groups[0]?.interpretationClasses).toEqual(['carrier_possibility']);
    expect(groups[0]?.inheritanceModels).toEqual(['autosomal_recessive']);
    expect(groups[0]?.clinicalStates).toEqual(['carrier_possibility']);
  });

  it('chooses an interpretable, effect-bearing source as the representative', () => {
    const groups = buildCanonicalFindingGroups(report([
      {
        name: 'Cardiovascular',
        markers: [marker({
          link_id: 'test:unusable',
          rsid: 'rs500',
          interpretation_allowed: false,
          effect_count: 2,
          severity_class: 'high_risk',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Nutrients',
        markers: [marker({
          link_id: 'test:usable',
          rsid: 'rs500',
          interpretation_allowed: true,
          effect_count: 1,
          severity_class: 'moderate_risk',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ]));

    expect(groups[0]?.representativeSource.marker.link_id).toBe('test:usable');
  });

  it('exposes a stable key without depending on a genotype call', () => {
    expect(canonicalFindingKey(marker({ rsid: 'rs600', user_genotype: 'AA' }))).toBe('rsid:rs600');
    expect(canonicalFindingKey(marker({ rsid: '', gene: 'GENE2', variant_name: 'Variant 2' }))).toBe('locus:gene2:variant 2');
  });
});
