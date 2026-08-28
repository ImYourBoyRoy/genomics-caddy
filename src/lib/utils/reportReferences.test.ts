import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport } from '../types/genomics';
import {
  buildReferenceEntries,
  buildReportReferenceRegistry,
  reportReferenceId,
} from './reportReferences';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'test:marker',
    rsid: 'rs123',
    gene: 'TEST1',
    variant_name: 'Example marker',
    chromosome: '1',
    position: null,
    user_genotype: 'TEST_CALL',
    normalized_genotype: 'TEST_CALL',
    effect_allele: 'A',
    effect_count: 1,
    impact: 'Context',
    evidence_tier: 'B',
    interpretation: 'Context only',
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
    generated_at: '2026-08-27T00:00:00.000Z',
    title: 'Reference test report',
    description: 'Synthetic report for reference registry tests.',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [{
      name: 'Test area',
      markers,
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }],
  };
}

const sharedSource = {
  name: 'Shared evidence source',
  url: 'https://example.test/evidence',
  accessed: '2026-08-27',
  evidence_type: 'Research context',
};

describe('report reference registry', () => {
  it('deduplicates shared URLs while keeping a stable ID', () => {
    const first = reportReferenceId(sharedSource);
    const second = reportReferenceId({ ...sharedSource, name: 'Renamed display source' });

    expect(first).toBe(second);
    expect(buildReferenceEntries([sharedSource], [{
      source_type: 'EvidenceLibrary',
      citation: 'Same URL catalog record',
      url: 'https://example.test/evidence',
    }])).toHaveLength(1);
  });

  it('assigns the same reference ID to findings that share a source', () => {
    const registry = buildReportReferenceRegistry(report([
      marker({ sources: [sharedSource] }),
      marker({ link_id: 'test:marker-2', sources: [sharedSource] }),
    ]));
    const firstIds = registry.idsByMarker.get('Test area:test:marker');
    const secondIds = registry.idsByMarker.get('Test area:test:marker-2');

    expect(registry.references).toHaveLength(1);
    expect(firstIds).toEqual(secondIds);
    expect(firstIds?.[0]).toBe(registry.references[0].id);
  });

  it('uses persisted report references and per-finding IDs when available', () => {
    const persistedId = 'REF-1234ABCD';
    const payload = report([marker({
      reference_ids: [persistedId],
      sources: [],
      db_enriched_sources: [],
    })]);
    payload.references = [{
      id: persistedId,
      title: 'Persisted evidence record',
      organization: 'Local reference registry',
      date: '2026-08-27',
      evidence_role: 'Report-level source metadata',
      url: 'https://example.test/persisted',
    }];

    const registry = buildReportReferenceRegistry(payload);

    expect(registry.references).toHaveLength(1);
    expect(registry.references[0].id).toBe(persistedId);
    expect(registry.references[0].evidenceRole).toBe('Report-level source metadata');
    expect(registry.idsByMarker.get('Test area:test:marker')).toEqual([persistedId]);
  });
});
