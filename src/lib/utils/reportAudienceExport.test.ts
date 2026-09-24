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
    expect(output).toContain('What this might mean: A biological pathway signal is present.');
    expect(output).toContain('- Direction: Context-dependent');
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
      includeRawGenotypes: false,
    });

    expect(output).toContain('# Clinician Handoff');
    expect(output).toContain('TEST1 — rs123');
    expect(output).toContain('Raw genotype call: SYNTHETIC_CALL');
    expect(output).toContain('Technical interpretation: Clinical interpretation text');
    expect(output).toContain('Interpretation class: Research context');
    expect(output).toContain('Inheritance model: Not established');
    expect(output).toContain('Clinical state: Not determined from this DNA result');
    expect(output).toContain('Orientation: Orientation check not required');
    const referenceId = reportReferenceId(marker().sources[0]);
    expect(output.match(new RegExp(`### ${referenceId} —`, 'g'))).toHaveLength(1);
    expect(output.match(/### REF-[A-F0-9]{8} —/g)).toHaveLength(1);
  });

  it('adds source-scoped full-genome disease associations to a handoff without allele strings', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: {
        ...report([]),
        genomewide_clinvar: {
          local_index_ready: true,
          genotypes_scanned: 600_000,
          exact_variant_count: 1,
          association_count: 1,
          omitted_association_count: 0,
          allele_orientation: 'Forward strand',
          source_asset_ids: ['clinvar_variant_summary', 'clinvar_submission_summary'],
          associations: [{
            association_is: 'variant_condition_summary',
            association_scope: 'variant',
            condition: 'Synthetic condition',
            rsid: 'rs-condition',
            variation_id: '123',
            scv_accession: 'SCV000000001.1',
            clinical_significance: 'Likely pathogenic',
            variant_summary_clinical_significance: 'Likely pathogenic',
            review_status: 'criteria provided, single submitter',
            allele_match: 'matched',
            origin_status: 'Origin not specified',
            variant_summary_conflict: false,
            source_url: 'https://www.ncbi.nlm.nih.gov/clinvar/?term=SCV000000001.1',
          }],
        },
      },
      sample,
      includeRawGenotypes: false,
    });

    expect(output).toContain('Potential disease associations from full-genome ClinVar scan');
    expect(output).toContain('Synthetic condition');
    expect(output).toContain('Origin not specified');
    expect(output).not.toContain('A/G');
  });

  it('enumerates incomplete PGx components in technical handoffs', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: report([marker({
        rsid: 'rs4244285',
        gene: 'CYP2C19',
        variant_name: 'CYP2C19*2 component',
      })]),
      sample,
    });

    expect(output).toContain('## PGx component coverage');
    expect(output).toContain('Coverage status: incomplete');
    expect(output).toContain('Phenotype or dose from this export: not allowed');
    expect(output).toContain('Complete CYP2C19 diplotype or phenotype');
    expect(output).toContain('## PRS readiness');
    expect(output).toContain('PRS entries remain unscored model specifications');
  });

  it('does not present a called but non-evaluated assertion as benign', () => {
    const nonEvaluated = marker({
      effect_count: null,
      severity_class: 'not_evaluated',
      assertion_status: 'NotEvaluated',
      interpretation_allowed: false,
    });
    const personal = buildReportAudienceMarkdown({
      audience: 'personal',
      report: report([nonEvaluated]),
      sample,
    });
    const clinician = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: report([nonEvaluated]),
      sample,
    });

    expect(personal).toContain('This assertion was not scored');
    expect(personal).not.toContain('Normal / Benign');
    expect(clinician).toContain('NotEvaluated');
    expect(clinician).toContain('Callability: Not callable from this DNA representation');
    expect(clinician).toContain('Assertion key: {');
    expect(clinician).toContain('The raw call was not converted into a finding');
  });

  it('includes explicit evidence-boundary instructions in AI Review exports', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'ai',
      report: report([marker()]),
      sample,
      includeRawGenotypes: false,
    });

    expect(output).toContain('# AI Review');
    expect(output).toContain('Do not assign disease probabilities or convert marker counts into a diagnosis');
    expect(output).toContain('a validated clinical result may establish a finding');
    expect(output).toContain('Raw genotype call: SYNTHETIC_CALL');
    expect(output).toContain(`Reference IDs: ${reportReferenceId(marker().sources[0])}`);
  });

  it('includes DNA-linked allergy context without a generic exposure checklist', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'personal',
      report: report([marker({ rsid: 'rs20541', gene: 'IL13' })]),
      sample,
    });

    expect(output).toContain('## Allergy & sensitivity map');
    expect(output).toContain('Atopy & IgE tendency');
    expect(output).toContain('Examples: seasonal or year-round rhinitis');
    expect(output).toContain('What it points toward: Matched variants point to immune-response pathways');
    expect(output).toContain('Relevant when: Most useful when seasonal allergies');
    expect(output).toContain('Matched genes: IL13');
    expect(output).toContain('### Focused follow-up');
    expect(output).not.toContain('Exposure history checklist');
    expect(output).not.toContain('Peanut: peanut, peanut butter, sauces');
    expect(output).not.toContain('Cold: cold air, water, or cold objects');
    expect(output).not.toContain('confirmed allergy');
  });

  it('includes named condition summaries with counts and clinical capability', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'personal',
      report: report([marker({
        rsid: 'rs2234693',
        gene: 'ESR1',
        link_id: 'test:pmdd',
        effect_direction: 'risk',
      })]),
      sample,
    });

    expect(output).toContain('## Potential conditions & health patterns');
    expect(output).toContain('PMDD-related steroid sensitivity');
    expect(output).toContain('Indicators: 1 of 7 aligned; 1 callable');
    expect(output).toContain('Relative signal: Limited relative signal');
    expect(output).toContain('Clinical capability: Clinical evaluation is required');
  });

  it('renders catalog relationships with source scope and allele-match status', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'personal',
      report: report([marker({
        rsid: 'rs-clinvar-export',
        gene: 'SYNTHETIC1',
        link_id: 'test:clinvar-export',
        user_genotype: 'AG',
        normalized_genotype: 'AG',
        expected_plus_alleles: ['A', 'G'],
        orientation_state: 'verified',
        clinvar_annotations: [{
          clinical_significance: 'Pathogenic',
          conditions: 'Synthetic catalog condition',
          variation_id: '45678',
          rcv_accession: 'RCV000000003',
          reference_allele: 'A',
          alternate_allele: 'G',
        }],
      })]),
      sample,
    });

    expect(output).toContain('## Catalog-linked conditions, traits & medication responses');
    expect(output).toContain('Synthetic catalog condition');
    expect(output).toContain('variant_condition_summary (variant-level)');
    expect(output).toContain('- Allele match: matched');
    expect(output).toContain('condition-specific RCV assertion');
    expect(output).not.toContain('AG');
  });

  it('keeps zero and partial condition coverage in technical handoffs', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: report([marker({ rsid: 'rs2234693', gene: 'ESR1' })]),
      sample,
    });

    expect(output).toContain('## Condition coverage');
    expect(output).toContain('PMDD-related steroid sensitivity');
    expect(output).toContain('Indicators: 1 available of 7; 1 callable; 1 aligned');
    expect(output).toContain('absent from report: 6');
  });

  it('includes a concise grouped clinician request list for DNA-linked labs', () => {
    const output = buildReportAudienceMarkdown({
      audience: 'clinician',
      report: report([marker({
        rsid: 'rs602662',
        gene: 'FUT2',
        interpretation: 'FUT2 B12-status context marker; not diagnostic.',
      })]),
      sample,
    });

    expect(output).toContain('### Clinician request list');
    expect(output).toContain('Nutrients & methylation');
    expect(output).toContain('Serum or plasma vitamin B12 — DNA-linked FUT2/TCN2/CUBN pathway');
    expect(output).not.toContain('A1c');
  });

  it('creates safe deterministic filenames', () => {
    expect(reportAudienceFilename('Jen Neal / Profile', 'personal')).toBe('jen_neal_profile_personal_report.md');
  });
});
