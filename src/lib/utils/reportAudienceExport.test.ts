import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport, GenomeSample } from '../types/genomics';
import {
  buildReportAudienceMarkdown,
  reportAudienceFilename,
  reportExportPrivacyWarning,
} from './reportAudienceExport';
import { reportReferenceId } from './reportReferences';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'test:marker',
    rsid: 'rs123',
    gene: 'TEST1',
    variant_name: 'Example pathway marker',
    chromosome: '1',
    position: null,
    user_genotype: 'SYNTHETIC_CALL',
    normalized_genotype: 'SYNTHETIC_CALL',
    effect_allele: 'SYNTHETIC_ALLELE',
    effect_count: 2,
    impact: 'Technical impact text',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'Clinical interpretation text that should be separated by audience.',
    effect_direction: 'context_dependent',
    severity_class: 'context_dependent',
    assertion_status: 'Verified',
    interpretation_allowed: true,
    sources: [
      {
        name: 'Example source',
        url: 'https://example.test/source',
        accessed: '2026-08-26',
        evidence_type: 'Research context',
      },
    ],
    db_enriched_sources: [
      {
        source_type: 'EvidenceLibrary',
        citation: 'Example citation',
        details: 'Catalog evidence',
        url: 'https://example.test/source',
      },
    ],
    clinvar_significance: null,
    clinvar_conditions: null,
    clinvar_review_status: null,
    gwas_top_trait: null,
    gwas_best_pvalue: null,
    gwas_association_count: null,
    population_af: null,
    population_rarity: null,
    confirm_with: [],
    do_not_claim: ['This is not a diagnosis.'],
    ...overrides,
  };
}

function report(markers: EvaluatedMarker[]): GeneratedReport {
  return {
    schema_version: '2.0.0',
    export_format: 'normalized_sparse',
    generated_at: '2026-08-26T00:00:00.000Z',
    title: 'Test report',
    description: 'Test report',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [{
      name: 'Test health area',
      markers,
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }],
  };
}

const sample: GenomeSample = {
  id: 1,
  name: 'Test Profile',
  genetic_sex: 'unknown',
  imported_at: '2026-08-26T00:00:00.000Z',
};

describe('audience-specific report exports', () => {
  it('keeps personal exports plain-language and free of raw or medical fallback text', () => {
    const femaleSample = { ...sample, genetic_sex: 'Female-like (XX chromosome pattern; no Y calls observed)' };
    const output = buildReportAudienceMarkdown({
      audience: 'personal',
      report: report([marker()]),
      sample: femaleSample,
    });

    expect(output).toContain('# Personal Simple Genomics Report');
    expect(output).toContain('- Sex: Female');
    expect(output).not.toContain('Female-like');
    expect(output).toContain('This DNA result is associated with a research finding');
    expect(output).not.toContain('SYNTHETIC_CALL');
    expect(output).not.toContain('Technical impact text');
    expect(output).not.toContain('Clinical interpretation text');
    expect(output).toContain(reportExportPrivacyWarning);
  });

  it('includes structured technical data and deduplicates shared references for clinician handoff', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: report([marker(), marker({ link_id: 'test:marker-2', rsid: 'rs456' })]),
      sample,
      includeRawGenotypes: true,
    });

    expect(output).toContain('# Clinician Handoff');
    expect(output).toContain('TEST1 — rs123');
    expect(output).toContain('Raw genotype call: SYNTHETIC_CALL');
    expect(output).toContain('Technical interpretation: Clinical interpretation text');
    const referenceId = reportReferenceId(marker().sources[0]);
    expect(output.match(new RegExp(`### ${referenceId} —`, 'g'))).toHaveLength(1);
    expect(output.match(/### REF-[A-F0-9]{8} —/g)).toHaveLength(1);
  });

  it('includes explicit anti-diagnosis instructions in AI Review exports', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'ai',
      report: report([marker()]),
      sample,
      includeRawGenotypes: true,
    });

    expect(output).toContain('# AI Review');
    expect(output).toContain('Do not diagnose');
    expect(output).toContain('Raw genotype call: SYNTHETIC_CALL');
    expect(output).toContain(`Reference IDs: ${reportReferenceId(marker().sources[0])}`);
  });

  it('creates safe deterministic filenames', () => {
    expect(reportAudienceFilename('Jen Neal / Profile', 'personal')).toBe('jen_neal_profile_personal_report.md');
  });
});
