import { describe, expect, it } from 'vitest';
import type { GenomeWideClinVarAssociation } from '../types/genomics';
import {
  classifyConditionTopic,
  groupGenomeWideConditionAssociations,
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
    ...overrides,
  };
}

describe('genome-wide condition discovery grouping', () => {
  it('routes broad topics through shared taxonomy without creating associations', () => {
    expect(classifyConditionTopic('Postural orthostatic tachycardia syndrome').id)
      .toBe('autonomic_orthostatic');
    expect(classifyConditionTopic('Breast cancer').id).toBe('cancer_risk');
    expect(classifyConditionTopic('Unmapped condition').id).toBe('other');
  });

  it('groups repeated condition assertions while preserving variant records and conflicts', () => {
    const grouped = groupGenomeWideConditionAssociations([
      association('Coronary artery disease', 'rs1'),
      association('Coronary artery disease', 'rs2', { variant_summary_conflict: true }),
      association('Other condition', 'rs3'),
    ]);

    expect(grouped).toHaveLength(2);
    expect(grouped[0].condition).toBe('Coronary artery disease');
    expect(grouped[0].variantCount).toBe(2);
    expect(grouped[0].assertions).toHaveLength(2);
    expect(grouped[0].conflicts).toBe(true);
  });
});
