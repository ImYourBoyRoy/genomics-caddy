import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker, GeneratedReport, GenomeSample } from '../types/genomics';
import { CYCLE_DIARY_SCHEMA } from './cycleDiary';
import { EMPTY_PROFILE_CONTEXT, type ProfileContext } from './profileContext';
import { buildAiReviewJson, buildReportBundleFiles } from './reportBundleExport';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'test:bundle-marker',
    rsid: 'rs-bundle',
    gene: 'BUNDLE1',
    variant_name: 'Bundle test marker',
    chromosome: '1',
    position: null,
    user_genotype: 'CALL_VALUE',
    normalized_genotype: 'CALL_VALUE',
    effect_allele: 'A',
    effect_count: 1,
    impact: 'Technical marker context',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'Research interpretation',
    effect_direction: 'context_dependent',
    severity_class: 'context_dependent',
    assertion_status: 'Verified',
    interpretation_allowed: true,
    sources: [{
      name: 'Bundle test source',
      url: 'https://example.test/bundle-source',
      accessed: '2026-08-29',
      evidence_type: 'Research context',
    }],
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

function report(): GeneratedReport {
  return {
    schema_version: '2.0.0',
    export_format: 'normalized_sparse',
    generated_at: '2026-08-29T00:00:00.000Z',
    title: 'Bundle test report',
    description: 'Bundle test report',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [{
      name: 'Bundle test area',
      markers: [marker()],
      section_signal_score: 0,
      summary: {} as GeneratedReport['sections'][number]['summary'],
    }],
  };
}

const sample: GenomeSample = {
  id: 42,
  name: 'Bundle Test Profile',
  genetic_sex: 'Male',
  imported_at: '2026-08-29T00:00:00.000Z',
};

const profileContext: ProfileContext = {
  ...EMPTY_PROFILE_CONTEXT,
  notes: { ...EMPTY_PROFILE_CONTEXT.notes, goals: 'Recovery planning' },
  safety: {
    ...EMPTY_PROFILE_CONTEXT.safety,
    medications: ['Medication A'],
    cycleDiary: [{
      id: 'entry-1',
      values: { entry_date: '2026-08-01', mood_behavior_score: '2' },
    }],
  },
};

describe('report bundle export', () => {
  it('creates the stable six-file bundle contract and includes context in every audience', () => {
    const expectedFiles = ['report.md', 'dna_analysis.json', 'context.json', 'diary.csv', 'manifest.json', 'PRIVACY.txt'];

    for (const audience of ['personal', 'clinician', 'ai'] as const) {
      const files = buildReportBundleFiles({
        audience,
        report: report(),
        sample,
        profileContext,
        includeRawGenotypes: audience !== 'personal',
      });

      expect(files.map((file) => file.filename)).toEqual(expectedFiles);
      expect(files.find((file) => file.filename === 'context.json')?.content).toContain('Recovery planning');
      expect(files.find((file) => file.filename === 'report.md')?.content).toContain('Recovery planning');
      expect(files.find((file) => file.filename === 'PRIVACY.txt')?.content).toContain('generated locally');
    }
  });

  it('includes source-scoped catalog associations without adding genotype values to that metadata', () => {
    const sourceReport: GeneratedReport = {
      ...report(),
      sections: [{
        ...report().sections[0],
        markers: [marker({
          rsid: 'rs-bundle-gwas',
          gene: 'SYNTHETIC1',
          user_genotype: 'AG',
          normalized_genotype: 'AG',
          gwas_associations: [{
            association_is: 'variant_trait_statistical_association',
            trait_name: 'Synthetic trait',
            pvalue: 1e-9,
            study_accession: 'GCST000001',
          }],
        })],
      }],
    };
    const files = buildReportBundleFiles({
      audience: 'personal',
      report: sourceReport,
      sample,
      profileContext,
    });
    const dna = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);

    expect(dna.catalog_associations).toContainEqual(expect.objectContaining({
      label: 'Synthetic trait',
      association_is: 'variant_trait_statistical_association',
      association_scope: 'locus',
      study_accessions: ['GCST000001'],
    }));
    expect(JSON.stringify(dna.catalog_associations)).not.toContain('AG');
  });

  it('exports full-genome ClinVar condition submissions without serializing raw alleles', () => {
    const sourceReport: GeneratedReport = {
      ...report(),
      genomewide_clinvar: {
        local_index_ready: true,
        genotypes_scanned: 650_000,
        exact_variant_count: 1,
        association_count: 1,
        omitted_association_count: 0,
        allele_orientation: 'Forward strand',
        source_asset_ids: ['clinvar_variant_summary', 'clinvar_submission_summary'],
        associations: [{
          association_is: 'variant_condition_summary',
          association_scope: 'variant',
          condition: 'Synthetic inherited condition',
          rsid: 'rs-synthetic-condition',
          gene_symbol: 'SYNTHETIC1',
          variation_id: '12345',
          scv_accession: 'SCV000000001.1',
          clinical_significance: 'Pathogenic',
          variant_summary_clinical_significance: 'Pathogenic',
          review_status: 'criteria provided, multiple submitters, no conflicts',
          allele_match: 'matched',
          origin_status: 'Germline observation reported',
          variant_summary_conflict: false,
          source_url: 'https://www.ncbi.nlm.nih.gov/clinvar/?term=SCV000000001.1',
        }],
      },
    };
    const files = buildReportBundleFiles({
      audience: 'personal',
      report: sourceReport,
      sample,
      profileContext,
    });
    const markdown = files.find((file) => file.filename === 'report.md')!.content;
    const dna = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);

    expect(markdown).toContain('Potential disease associations from full-genome ClinVar scan');
    expect(markdown).toContain('Synthetic inherited condition');
    expect(dna.genomewide_clinvar_discovery.associations[0].scv_accession).toBe('SCV000000001.1');
    expect(JSON.stringify(dna.genomewide_clinvar_discovery)).not.toContain('A/G');
    expect(JSON.stringify(dna.genomewide_clinvar_discovery)).not.toContain('allele1');
  });

  it('keeps personal DNA JSON free of raw genotype fields while retaining them for explicit technical audiences', () => {
    const personalFiles = buildReportBundleFiles({
      audience: 'personal',
      report: report(),
      sample,
      profileContext,
      includeRawGenotypes: false,
    });
    const clinicianFiles = buildReportBundleFiles({
      audience: 'clinician',
      report: report(),
      sample,
      profileContext,
      includeRawGenotypes: false,
    });

    const personalMarker = JSON.parse(personalFiles.find((file) => file.filename === 'dna_analysis.json')!.content).sections[0].markers[0];
    const clinicianMarker = JSON.parse(clinicianFiles.find((file) => file.filename === 'dna_analysis.json')!.content).sections[0].markers[0];
    expect(personalMarker).not.toHaveProperty('user_genotype');
    expect(personalMarker).not.toHaveProperty('normalized_genotype');
    expect(clinicianMarker).toHaveProperty('user_genotype');
    expect(clinicianMarker).toHaveProperty('normalized_genotype');
    expect(personalMarker.clinical_semantics).toEqual({
      condition_label: null,
      interpretation_class: 'research_context',
      inheritance_model: 'unknown',
      clinical_state: 'unknown',
      evidence_level: 'B_replicated_common_marker',
      clinical_confirmation_required: false,
      applicability_scopes: [],
    });
    expect(personalMarker.callability_policy.scoring_policy).toBe('snp_allele_count');
    expect(personalMarker.callability_state).toBe('callable');
    expect(personalMarker.orientation_state).toBe('not_required');
    expect(JSON.parse(personalMarker.assertion_key).version).toBe(1);
    expect(personalMarker.assertion_key).not.toContain('CALL_VALUE');
    expect(JSON.parse(personalFiles.find((file) => file.filename === 'dna_analysis.json')!.content).condition_evidence).toEqual([]);
    expect(JSON.parse(personalFiles.find((file) => file.filename === 'dna_analysis.json')!.content).condition_coverage)
      .toEqual(expect.arrayContaining([
        expect.objectContaining({ id: 'pmdd_steroid_sensitivity', status: 'not_observed' }),
      ]));
  });

  it('exports the resolved callability policy beside non-SNP technical context', () => {
    const files = buildReportBundleFiles({
      audience: 'ai',
      report: {
        ...report(),
        sections: [{
          ...report().sections[0],
          markers: [marker({ variant_type: 'hla_tag', effect_allele: 'A' })],
        }],
      },
      sample,
      profileContext,
    });
    const dna = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);
    expect(dna.sections[0].markers[0]).toMatchObject({
      variant_type: 'hla_tag',
      callability_policy: {
        scoring_policy: 'not_evaluated',
        display_policy: 'context_only',
      },
      callability_state: 'not_callable',
    });
  });

  it('exports PGx component coverage without creating a phenotype or dose', () => {
    const base = report();
    const files = buildReportBundleFiles({
      audience: 'ai',
      report: {
        ...base,
        sections: [{
          ...base.sections[0],
          name: 'Pharmacogenomics (PGx)',
          markers: [marker({
            rsid: 'rs4244285',
            gene: 'CYP2C19',
            variant_name: 'CYP2C19*2 component',
          })],
        }],
      },
      sample,
      profileContext,
    });
    const dna = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);
    const pathway = dna.pgx.component_coverage.find((item: { id: string }) => item.id === 'CYP2C19');
    expect(pathway).toMatchObject({
      status: 'incomplete',
      phenotype_or_dose_allowed: false,
      required_inputs: expect.arrayContaining(['Complete CYP2C19 diplotype or phenotype']),
    });
    expect(dna.prs.modules[0]).toMatchObject({
      status: 'unscored',
      score: null,
      missing_inputs: expect.any(Array),
    });
    expect(JSON.stringify(pathway)).not.toContain('CALL_VALUE');
  });

  it('carries typed food, supplement, and activity provenance into DNA JSON', () => {
    const base = report();
    const files = buildReportBundleFiles({
      audience: 'clinician',
      report: {
        ...base,
        sections: [{
          ...base.sections[0],
          name: 'Cardiovascular & Lipid Transport',
          markers: [
            marker({ link_id: 'test:apoe', rsid: 'rs429358', gene: 'APOE', interpretation: 'APOE4 lipid context.' }),
            marker({ link_id: 'test:fads', rsid: 'rs174547', gene: 'FADS1', interpretation: 'FADS1 fatty-acid conversion context.' }),
          ],
        }],
      },
      sample,
      profileContext,
      includeRawGenotypes: false,
    });
    const dna = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);
    const food = dna.recommendations.food.favor.find((item: { name: string }) => item.name.includes('Fatty fish'));
    const activity = dna.recommendations.activity;

    expect(food).toMatchObject({
      category: 'food',
      basis_topic_ids: expect.arrayContaining(['apoe_lipid']),
      basis_marker_ids: expect.arrayContaining(['rs429358']),
      basis_genes: expect.arrayContaining(['APOE']),
    });
    expect(activity).toEqual(expect.any(Array));
    expect(JSON.stringify(dna.recommendations)).not.toContain('CALL_VALUE');
  });

  it('carries canonical lab follow-ups and the grouped request list into DNA JSON', () => {
    const files = buildReportBundleFiles({
      audience: 'clinician',
      report: report(),
      sample,
      profileContext,
      includeRawGenotypes: false,
    });
    const base = JSON.parse(files.find((file) => file.filename === 'dna_analysis.json')!.content);
    const withLab = buildReportBundleFiles({
      audience: 'clinician',
      report: {
        ...report(),
        sections: [{
          ...report().sections[0],
          markers: [marker({
            rsid: 'rs602662',
            gene: 'FUT2',
            interpretation: 'FUT2 B12-status context marker; not diagnostic.',
          })],
        }],
      },
      sample,
      profileContext,
      includeRawGenotypes: false,
    });
    const dna = JSON.parse(withLab.find((file) => file.filename === 'dna_analysis.json')!.content);

    expect(base.recommendations.labs).toEqual([]);
    expect(dna.recommendations.labs).toEqual(expect.arrayContaining([
      expect.objectContaining({
        canonical_id: 'serum_b12',
        name: 'Serum or plasma vitamin B12',
        category: 'Nutrients & methylation',
        basis_genes: expect.arrayContaining(['FUT2']),
      }),
    ]));
    expect(dna.recommendations.clinician_request_list).toContain(
      'Serum or plasma vitamin B12 — DNA-linked FUT2/TCN2/CUBN pathway',
    );
    expect(dna.recommendations.clinician_request_list).not.toContain('Duplicate');
  });

  it('uses the resource-defined diary schema for CSV headers and rows', () => {
    const diary = buildReportBundleFiles({
      audience: 'personal',
      report: report(),
      sample,
      profileContext,
      includeRawGenotypes: false,
    }).find((file) => file.filename === 'diary.csv')!.content;
    const [header, row] = diary.trimEnd().split('\n');

    expect(header.split(',')).toEqual(CYCLE_DIARY_SCHEMA.fields.map((field) => field.id));
    expect(row.split(',')[0]).toBe('2026-08-01');
  });

  it('creates a single AI-ready JSON handoff with DNA, context, diary, and instructions', () => {
    const aiJson = JSON.parse(buildAiReviewJson({
      audience: 'ai',
      report: report(),
      sample,
      profileContext,
      includeRawGenotypes: false,
    }));

    expect(aiJson).toMatchObject({
      schema_version: 1,
      export_kind: 'ai_review_json',
      audience: 'ai',
      raw_genotypes_included: true,
      profile: {
        sample_id: 42,
        sample_name: 'Bundle Test Profile',
        chromosome_call_context: 'Male-like',
      },
    });
    expect(aiJson.review_instructions).toEqual(expect.any(Array));
    expect(aiJson.dna_analysis).toHaveProperty('sections');
    expect(aiJson.context.notes.goals).toBe('Recovery planning');
    expect(aiJson.diary.fields).toEqual(expect.any(Array));
    expect(aiJson.diary.entries).toHaveLength(1);
    expect(aiJson.dna_analysis.sections[0].markers[0]).toHaveProperty('user_genotype');
  });
});
