import { describe, expect, it } from 'vitest';
import type { EvaluatedMarker } from '../types/genomics';
import cardiovascularPack from '../marker-packs/cardiovascular.json';
import cancerPack from '../marker-packs/cancer_confirmation_only.json';
import {
  clinicalStateLabel,
  inheritanceModelLabel,
  interpretationClassLabel,
  normalizeFindingSemantics,
} from './findingSemantics';

function marker(overrides: Partial<EvaluatedMarker> = {}): EvaluatedMarker {
  return {
    link_id: 'semantic:test',
    rsid: 'rs-semantic-test',
    gene: 'SEMANTIC1',
    variant_name: 'Synthetic semantic marker',
    chromosome: '1',
    position: null,
    user_genotype: 'synthetic-call',
    normalized_genotype: null,
    effect_allele: 'synthetic-allele',
    effect_count: 1,
    impact: 'Synthetic marker context',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'Synthetic interpretation',
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

describe('finding semantics', () => {
  it('keeps evidence-backed clinical labels attached to the intended resource records', () => {
    const cardiovascularIds = ['rs6025', 'rs1799963', 'rs76992529', 'rs1800562'];
    const cancerIds = ['rs386833395', 'rs80357906', 'rs80359550', 'rs28897696', 'rs555607708', 'rs34612342', 'rs36053993'];
    const cardiovascularMarkers = cardiovascularPack.markers.filter((item) => cardiovascularIds.includes(item.rsid));
    const cancerMarkers = cancerPack.markers.filter((item) => cancerIds.includes(item.rsid));

    expect(cardiovascularMarkers).toHaveLength(cardiovascularIds.length);
    expect(cancerMarkers).toHaveLength(cancerIds.length);
    expect(cardiovascularMarkers.every((item) => item.clinical_semantics?.clinical_state === 'unknown')).toBe(true);
    expect(cancerMarkers.every((item) => item.clinical_semantics?.clinical_state === 'unknown')).toBe(true);
    expect(cardiovascularMarkers.find((item) => item.rsid === 'rs76992529')?.clinical_semantics).toMatchObject({
      condition_label: 'Hereditary transthyretin amyloidosis',
      inheritance_model: 'autosomal_dominant',
    });
    expect(cancerMarkers.filter((item) => item.gene === 'MUTYH').every((item) => item.clinical_semantics?.inheritance_model === 'autosomal_recessive')).toBe(true);
  });

  it('classifies a common risk association as susceptibility context', () => {
    const semantics = normalizeFindingSemantics(marker());

    expect(semantics.interpretation_class).toBe('susceptibility_context');
    expect(semantics.inheritance_model).toBe('unknown');
    expect(semantics.clinical_state).toBe('unknown');
    expect(semantics.condition_label).toBeNull();
  });

  it('keeps lower-evidence risk markers in research context', () => {
    const semantics = normalizeFindingSemantics(marker({ evidence_tier: 'D_research_only' }));

    expect(semantics.interpretation_class).toBe('research_context');
    expect(semantics.clinical_state).toBe('unknown');
  });

  it('preserves explicitly authored carrier and inheritance semantics', () => {
    const semantics = normalizeFindingSemantics(marker({
      clinical_semantics: {
        condition_label: 'Synthetic recessive condition',
        interpretation_class: 'carrier_possibility',
        inheritance_model: 'autosomal_recessive',
        clinical_state: 'carrier_possibility',
      },
    }));

    expect(semantics.condition_label).toBe('Synthetic recessive condition');
    expect(interpretationClassLabel(semantics.interpretation_class)).toBe('Carrier possibility');
    expect(inheritanceModelLabel(semantics.inheritance_model)).toBe('Autosomal recessive');
    expect(clinicalStateLabel(semantics.clinical_state)).toBe('Carrier possibility');
  });

  it('supports established dominant and X-linked inheritance without inferring it', () => {
    const dominant = normalizeFindingSemantics(marker({
      clinical_semantics: {
        condition_label: 'Synthetic dominant condition',
        interpretation_class: 'clinically_actionable_variant',
        inheritance_model: 'autosomal_dominant',
        clinical_state: 'unknown',
      },
    }));
    const xLinked = normalizeFindingSemantics(marker({
      clinical_semantics: {
        condition_label: 'Synthetic X-linked condition',
        interpretation_class: 'clinically_actionable_variant',
        inheritance_model: 'x_linked',
        clinical_state: 'unknown',
      },
    }));

    expect(inheritanceModelLabel(dominant.inheritance_model)).toBe('Autosomal dominant');
    expect(inheritanceModelLabel(xLinked.inheritance_model)).toBe('X-linked');
  });

  it('preserves biological applicability as a scope hint only', () => {
    const semantics = normalizeFindingSemantics(marker({ sex_scope: 'xx_reproductive' }));

    expect(semantics.applicability_scopes).toEqual(['xx_reproductive']);
    expect(semantics.clinical_state).toBe('unknown');
  });

  it('never infers active disease from a clinical-confirmation marker', () => {
    const semantics = normalizeFindingSemantics(marker({
      evidence_tier: 'A_clinically_actionable_gene_panel',
      impact: 'Pathogenic variant panel',
      clinical_confirmation_required: true,
      severity_class: 'confirmation_required',
    }));

    expect(semantics.interpretation_class).toBe('clinically_actionable_variant');
    expect(semantics.clinical_state).toBe('unknown');
    expect(semantics).not.toHaveProperty('active_disease');
  });

  it('labels protective context without converting it into a guarantee', () => {
    const semantics = normalizeFindingSemantics(marker({ effect_direction: 'protective', severity_class: 'protective' }));

    expect(semantics.interpretation_class).toBe('protective_context');
    expect(semantics.clinical_state).toBe('unknown');
  });

  it('keeps unsupported or non-callable findings unknown', () => {
    const semantics = normalizeFindingSemantics(marker({
      effect_direction: 'unknown',
      severity_class: 'no_data',
      assertion_status: 'NoData',
      interpretation_allowed: false,
      user_genotype: '--',
    }));

    expect(semantics.interpretation_class).toBe('unknown');
    expect(semantics.inheritance_model).toBe('unknown');
    expect(semantics.clinical_state).toBe('unknown');
  });
});
