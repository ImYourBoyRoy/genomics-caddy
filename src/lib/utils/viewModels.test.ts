import { describe, expect, it } from 'vitest';
import type { NormalizedReport } from '../types/genomics';
import { getLaypersonTranslation } from './layperson';
import { normalizeReportStatuses } from './reportStatuses';
import { denormalizeReport, deriveDisplayMarkers } from './viewModels';

describe('report display view models', () => {
  it('preserves the canonical variant type for identifier-aware Simple copy', () => {
    const report: NormalizedReport = {
      schema_version: 'test',
      export_format: 'normalized',
      generated_at: '2026-08-30T00:00:00Z',
      title: 'Test report',
      description: 'Test report',
      overall_signal_score: 0,
      variants: {
        PANEL_EXAMPLE: {
          rsid: 'PANEL_EXAMPLE',
          gene: 'EXAMPLE',
          variant_name: 'Example clinical panel',
          variant_type: 'gene_panel',
        },
      },
      user_calls: {
        PANEL_EXAMPLE: {
          user_genotype: '--',
          normalized_genotype: null,
          call_status: 'NotInRawFile',
          source_build: null,
        },
      },
      category_links: {
        'test:panel': {
          link_id: 'test:panel',
          rsid: 'PANEL_EXAMPLE',
          category_id: 'panel',
          category_label: 'Panel',
          impact: 'Clinical panel context',
          evidence_tier: 'Tier B',
          interpretation: 'A focused clinical panel route.',
          effect_direction: 'context_dependent',
          effect_allele: 'A',
          severity_class: 'context',
          assertion_status: 'NotInRawFile',
          requires_orientation_verification: false,
          interpretation_allowed: false,
          do_not_claim: [],
          confirm_with: [],
          sources: [],
          reference_ids: [],
        },
      },
      enrichment: {
        PANEL_EXAMPLE: {
          clinvar_annotations: [{
            clinical_significance: 'Pathogenic',
            conditions: 'Synthetic linked condition',
            variation_id: '12345',
          }],
          gwas_hits: [],
          gwas_associations: [{
            association_is: 'variant_trait_statistical_association',
            trait_name: 'Synthetic linked trait',
            pvalue: 1e-9,
            study_accession: 'GCST000001',
          }],
          db_enriched_sources: [],
        },
      },
      sections: [{
        section_id: 'panel',
        name: 'Panel',
        link_ids: ['test:panel'],
        section_signal_score: 0,
        summary: {
          risk_effect_count: 0,
          risk_possible: 0,
          protective_effect_count: 0,
          protective_possible: 0,
          trait_count: 0,
          context_dependent_count: 1,
          no_data_count: 1,
          confirmation_required_count: 0,
          total_markers: 1,
          show_percent_score: false,
          all_require_confirmation: true,
          active_marker_count: 0,
          active_risk_marker_count: 0,
          active_protective_marker_count: 0,
          active_trait_marker_count: 0,
          active_context_marker_count: 0,
          blocked_unverified_count: 0,
          benign_modifier_count: 0,
        },
      }],
    };

    const marker = deriveDisplayMarkers(report, 'panel')[0];
    expect(marker?.variant_type).toBe('gene_panel');
    expect(marker?.callability_state).toBe('not_callable');
    expect(marker?.assertion_key).toContain('"version":1');
    expect(marker?.assertion_key).not.toContain('NotInRawFile');
    expect(marker?.clinvar_annotations?.[0]?.conditions).toBe('Synthetic linked condition');
    expect(marker?.gwas_associations?.[0]?.study_accession).toBe('GCST000001');
    expect(marker ? getLaypersonTranslation(marker).isFallback : undefined).not.toBe(true);
    expect(marker ? getLaypersonTranslation(marker).signal : '').toContain('Example clinical panel');
  });

  it('normalizes Rust snake_case statuses before report selectors consume them', () => {
    const wireReport = {
      user_calls: {
        rs1: {
          user_genotype: 'AA',
          normalized_genotype: 'AA',
          call_status: 'found',
          source_build: 'GRCh38',
        },
      },
      category_links: {
        'test:rs1': {
          assertion_status: 'verified',
        },
      },
      sections: [],
    } as unknown as NormalizedReport;

    const normalized = normalizeReportStatuses(wireReport);
    expect(normalized.user_calls.rs1.call_status).toBe('Found');
    expect(normalized.category_links['test:rs1'].assertion_status).toBe('Verified');
    expect(denormalizeReport(wireReport).sections).toEqual([]);
  });
});
