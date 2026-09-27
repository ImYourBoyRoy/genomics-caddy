import { describe, expect, it } from 'vitest';
import type { GenomeWideClinVarAssociation } from '../types/genomics';
import {
  classifyConditionTopic,
  groupGenomeWideConditionAssociations,
  sectionConditionGroupsByRelevance,
  UNNAMED_CONDITION_LABEL,
} from './genomewideConditionDiscovery';

function association(
  condition: string,
  rsid: string,
  overrides: Partial<GenomeWideClinVarAssociation> = {},
): GenomeWideClinVarAssociation {
  return {
    association_is: 'variant_condition_summary',
    association_scope: 'variant',
    condition,
    rsid,
    variation_id: '12345',
    scv_accession: 'SCV000000001.1',
    clinical_significance: 'Pathogenic',
    variant_summary_clinical_significance: 'Pathogenic',
    review_status: 'criteria provided, multiple submitters, no conflicts',
    allele_match: 'matched',
    origin_status: 'Germline observation reported',
    variant_summary_conflict: false,
    source_url: 'https://www.ncbi.nlm.nih.gov/clinvar/?term=SCV000000001.1',
    alt_allele_copies: 1,
    relevance: 'unclear',
    ...overrides,
  };
}

describe('genome-wide condition discovery grouping', () => {
  it('routes broad topics through the shared taxonomy without creating associations', () => {
    expect(classifyConditionTopic('Postural orthostatic tachycardia syndrome').id)
      .toBe('autonomic_orthostatic');
    expect(classifyConditionTopic('Breast cancer').id).toBe('cancer_risk');
    expect(classifyConditionTopic('Unmapped condition').id).toBe('other');
  });

  it('keeps the same condition separate for different DNA markers', () => {
    const grouped = groupGenomeWideConditionAssociations([
      association('Coronary artery disease', 'rs1', { relevance: 'may_be_relevant', alt_allele_copies: 1 }),
      association('Coronary artery disease', 'rs2', { relevance: 'carrier', alt_allele_copies: 2 }),
    ]);

    expect(grouped).toHaveLength(2);
    expect(grouped.map((group) => group.rsid)).toEqual(['rs1', 'rs2']);
    expect(grouped.map((group) => group.copyCount)).toEqual([1, 2]);
    expect(grouped.map((group) => group.relevance)).toEqual(['may_be_relevant', 'carrier']);
  });

  it('groups duplicate submissions only for the same condition and exact variant', () => {
    const grouped = groupGenomeWideConditionAssociations([
      association('Cystic fibrosis', 'rs1', {
        relevance: 'may_be_relevant',
        inheritance: ['Autosomal recessive'],
      }),
      association('Cystic fibrosis', 'rs1', {
        scv_accession: 'SCV000000002.1',
        relevance: 'may_be_relevant',
        inheritance: ['autosomal recessive'],
      }),
      association('Cystic fibrosis', 'rs2', {
        variation_id: '67890',
        relevance: 'may_be_relevant',
        alt_allele_copies: 2,
      }),
    ]);

    expect(grouped).toHaveLength(2);
    expect(grouped[0].assertions).toHaveLength(2);
    expect(grouped[0].copyCount).toBe(1);
    expect(grouped[0].inheritance).toEqual(['Autosomal recessive']);
    expect(grouped[1].rsid).toBe('rs2');
  });

  it('keeps unnamed lab records separate, including records for an already named variant', () => {
    const grouped = groupGenomeWideConditionAssociations([
      association('Ectodermal dysplasia', 'rs1'),
      association('MedGen:C0796093', 'rs1', { scv_accession: 'SCV000000002.1' }),
      association('C3661900:not provided', 'rs2'),
      association('Human Phenotype Ontology:HP:0007607', 'rs3'),
    ]);

    expect(grouped.map((group) => group.condition)).toEqual([
      'Ectodermal dysplasia',
      UNNAMED_CONDITION_LABEL,
      UNNAMED_CONDITION_LABEL,
      UNNAMED_CONDITION_LABEL,
    ]);
    expect(grouped.filter((group) => group.unnamed).map((group) => group.assertions[0].rsid))
      .toEqual(['rs1', 'rs2', 'rs3']);
  });

  it('labels mixed condition and risk-factor submissions instead of hiding the difference', () => {
    const [group] = groupGenomeWideConditionAssociations([
      association('Alzheimer disease', 'rs1'),
      association('Alzheimer disease', 'rs1', {
        scv_accession: 'SCV000000002.1',
        association_is: 'variant_risk_factor_summary',
      }),
    ]);

    expect(group.associationKind).toBe('mixed');
  });

  it('uses the unclear group when lab records disagree on relevance or copy count', () => {
    const [group] = groupGenomeWideConditionAssociations([
      association('Cystic fibrosis', 'rs1', { relevance: 'may_be_relevant', alt_allele_copies: 1 }),
      association('Cystic fibrosis', 'rs1', {
        scv_accession: 'SCV000000002.1',
        relevance: 'carrier',
        alt_allele_copies: 2,
      }),
    ]);

    expect(group.relevance).toBe('unclear');
    expect(group.copyCount).toBeNull();
  });

  it('sections exact marker-condition matches in cautious relevance order', () => {
    const sections = sectionConditionGroupsByRelevance(groupGenomeWideConditionAssociations([
      association('Unclear condition', 'rs1'),
      association('Carrier condition', 'rs2', { relevance: 'carrier' }),
      association('Possible condition', 'rs3', { relevance: 'may_be_relevant' }),
    ]));

    expect(sections.map((section) => section.relevance))
      .toEqual(['may_be_relevant', 'carrier', 'unclear']);
    expect(sections[0].title).toBe('May be relevant');
    expect(sections[1].variantCount).toBe(1);
  });

  it('counts distinct markers when a section includes multiple conditions at one marker', () => {
    const [section] = sectionConditionGroupsByRelevance(groupGenomeWideConditionAssociations([
      association('Thrombophilia', 'rs1', { relevance: 'may_be_relevant' }),
      association('Factor V deficiency', 'rs1', {
        variation_id: '67890',
        relevance: 'may_be_relevant',
      }),
    ]));

    expect(section.groups).toHaveLength(2);
    expect(section.variantCount).toBe(1);
  });
});
