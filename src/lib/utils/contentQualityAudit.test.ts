import { describe, expect, it } from 'vitest';
import { analyzeContentQuality } from '../../../scripts/content_quality_metrics.mjs';

describe('content-quality audit metrics', () => {
  it('measures duplicate markers, repeated copy, authored meaning, and action provenance', () => {
    const result = analyzeContentQuality({
      packDocs: [
        {
          id: 'alpha',
          markers: [
            {
              rsid: 'rs100',
              gene: 'GENE1',
              impact: 'Research context',
              interpretation: 'The same research context.',
              confirm_with: ['a relevant lab'],
            },
            {
              rsid: 'rs100',
              gene: 'GENE1',
              impact: 'Research context',
              interpretation: 'The same research context.',
              confirm_with: ['a relevant lab'],
            },
          ],
        },
        {
          id: 'beta',
          markers: [
            {
              rsid: 'rs200',
              gene: 'GENE2',
              impact: 'Another context',
              interpretation: 'A different association.',
              confirm_with: ['a relevant lab'],
            },
          ],
        },
      ],
      actionability: {
        rules: [{
          id: 'gene1-food',
          genes: ['GENE1'],
          dietary_favor: ['beans'],
          sources: ['source-one'],
        }],
      },
      laypersonTranslations: {
        translations: [{ rsid: 'rs100', simpleMeaning: 'A plain explanation.' }],
        fallback: { simpleMeaning: 'A safe fallback explanation.' },
      },
      sourceRegistry: { sources: { 'source-one': {} } },
    });

    expect(result.marker_rows).toBe(3);
    expect(result.unique_standard_rsids).toBe(2);
    expect(result.duplicate_standard_rsids).toBe(1);
    expect(result.duplicate_standard_rows).toBe(2);
    expect(result.duplicate_standard_excess_rows).toBe(1);
    expect(result.copy.repeated_visible_phrases.repeated_values).toBeGreaterThan(0);
    expect(result.copy.generic_interpretation_phrases.repeated_values).toBe(1);
    expect(result.copy.reuse_classes.repeated_marker_interpretation.repeated_values).toBe(1);
    expect(result.copy.reuse_classes.shared_follow_up.repeated_values).toBe(1);
    expect(result.copy.reuse_classes.marker_impact.repeated_values).toBe(1);
    expect(result.copy.reuse_classes.other_copy.repeated_values).toBe(0);
    expect(result.copy.reuse_classes.fallback_copy.candidate_entries).toBe(1);
    expect(result.warnings).not.toContain(expect.stringContaining('unclassified repeated copy'));
    expect(result.plain_meaning.authored).toBe(2);
    expect(result.plain_meaning.fallback).toBe(1);
    expect(result.plain_meaning.missing).toBe(0);
    expect(result.findings_without_action).toBe(0);
    expect(result.simple_copy_contract.required_fields).toEqual([
      'plainTitle',
      'signal',
      'whyItMatters',
      'reviewAction',
      'evidenceLabel',
    ]);
    expect(result.simple_copy_contract.fully_structured_entries).toBe(0);
    expect(result.simple_copy_contract.legacy_entries_needing_migration).toBe(1);
    expect(result.recommendations.total).toBe(1);
    expect(result.recommendations.rules_without_marker_basis).toBe(0);
    expect(result.errors).toEqual([]);
  });

  it('fails structural content gaps without revealing marker or genotype values', () => {
    const result = analyzeContentQuality({
      packDocs: [{
        id: 'synthetic',
        markers: [{
          rsid: 'rs300',
          gene: 'GENE3',
          impact: 'Context',
          interpretation: 'Context only.',
        }],
      }],
      supportResources: [{
        id: 'synthetic-support.json',
        value: {
          source_ids: ['missing-source'],
          matched_marker_count: 2,
          matched_marker_link_ids: ['one-link'],
          matched_marker_ids: ['marker-a', 'marker-a'],
        },
      }],
      actionability: {
        rules: [{
          id: 'unscoped-rule',
          favor: ['a recommendation'],
          sources: ['valid-source'],
        }],
      },
      laypersonTranslations: {},
      sourceRegistry: { sources: { 'valid-source': {} } },
    });

    expect(result.errors).toHaveLength(4);
    expect(result.references.invalid_ids).toBe(1);
    expect(result.plain_meaning.missing).toBe(1);
    expect(result.findings_without_action).toBe(1);
    expect(result.simple_copy_contract.legacy_entries_needing_migration).toBe(0);
    expect(result.multi_marker.link_count_mismatches).toBe(1);
    expect(result.multi_marker.duplicate_references).toBe(1);
    expect(result.recommendations.rules_without_marker_basis).toBe(1);
    expect(result.warnings).toContain('recommendation rules without an explicit marker/gene basis: 1');
    expect(result.copy.reuse_classes.fallback_copy.candidate_entries).toBe(0);

    const serialized = JSON.stringify(result);
    expect(serialized).not.toContain('genotype');
    expect(serialized).not.toContain('allele');
  });
});
