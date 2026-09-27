import { describe, expect, it } from 'vitest';
import {
  catalogKindLabel,
  catalogLabelIsReadable,
  catalogPlainSummary,
  cleanConditionName,
  inheritancePlain,
  plainConditionSummary,
  yourCopiesLabel,
} from './plainConditionCopy';

describe('plain condition copy', () => {
  it('drops database IDs and placeholders but keeps readable names', () => {
    expect(cleanConditionName('MedGen:C0796093')).toBeNull();
    expect(cleanConditionName('MedGen:C0796093;MedGen:C1835492')).toBeNull();
    expect(cleanConditionName('C3661900:not provided')).toBeNull();
    expect(cleanConditionName('Human Phenotype Ontology:HP:0007607')).toBeNull();
    expect(cleanConditionName('6 conditions')).toBeNull();
    expect(cleanConditionName('not provided')).toBeNull();
    expect(cleanConditionName('F5-related disorders')).toBe('F5-related disorders');
    expect(cleanConditionName('Hemochromatosis type 1|MedGen:C3469186|Hemochromatosis type 1'))
      .toBe('Hemochromatosis type 1');
  });

  it('explains inheritance and keeps copy counts tied to a specific marker', () => {
    expect(inheritancePlain(['Autosomal recessive'])).toContain('usually needs two copies to cause symptoms');
    expect(inheritancePlain(['Autosomal dominant'])).toContain('sometimes be linked to one copy');
    expect(inheritancePlain(['Autosomal semidominant'])).toContain('stronger with two copies than with one');
    expect(inheritancePlain(['X-linked recessive'])).toContain('X-linked inheritance pattern');
    expect(inheritancePlain([])).toBeNull();
    expect(yourCopiesLabel(2)).toBe('2 copies at this marker');
    expect(yourCopiesLabel(1)).toBe('1 copy at this marker');
    expect(yourCopiesLabel(null)).toBe('Copy counts differ across lab records');
  });

  it('does not misattribute variant-wide conflicts to one condition', () => {
    const summary = plainConditionSummary({
      inheritance: ['Autosomal recessive'],
      associationKind: 'condition',
      variantSummaryConflict: true,
    });

    expect(summary).toContain('usually needs two copies');
    expect(summary).toContain('broader variant records conflict');
    expect(summary).toContain('may refer to a different condition');
    expect(summary).not.toContain('Labs disagree on whether this variant causes it');
  });

  it('distinguishes a risk factor from a disease link and exposes mixed record types', () => {
    expect(plainConditionSummary({
      inheritance: [],
      associationKind: 'risk_factor',
      variantSummaryConflict: false,
    })).toContain('risk factor for the condition, not as a confirmed cause');
    expect(plainConditionSummary({
      inheritance: [],
      associationKind: 'mixed',
      variantSummaryConflict: false,
    })).toContain('in some records and calls it a risk factor in others');
    expect(plainConditionSummary({
      inheritance: [],
      associationKind: 'condition',
      variantSummaryConflict: false,
    })).toBe('ClinVar lab reports link this marker to the condition.');
  });

  it('labels broader catalog links without calling them disease variants', () => {
    expect(catalogKindLabel('variant_condition_summary')).toBe('Variant-level condition link');
    expect(catalogKindLabel('variant_risk_factor_summary')).toBe('Variant-level risk association');
    expect(catalogKindLabel('gene_disease_validity')).toBe('Gene–disease link');
    expect(catalogPlainSummary({ association_is: 'variant_condition_summary', allele_match: 'matched', genes: [] }))
      .toContain('overall variant summary');
    expect(catalogPlainSummary({ association_is: 'variant_condition_summary', allele_match: 'not_verifiable', genes: ['HFE'] }))
      .toContain('cannot confirm that you carry it');
    expect(catalogPlainSummary({ association_is: 'gene_disease_validity', genes: ['HFE'] }))
      .toContain('HFE gene');
    expect(catalogLabelIsReadable({ association_is: 'variant_condition_summary', label: '6 conditions' })).toBe(false);
    expect(catalogLabelIsReadable({ association_is: 'variant_trait_statistical_association', label: 'Height' })).toBe(true);
  });
});
