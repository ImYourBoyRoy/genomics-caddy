// ./src/lib/utils/plainConditionCopy.ts
/**
 * Plain-language copy for ClinVar condition discovery and catalog links.
 * Purpose: turn database labels (MedGen/HPO IDs, inheritance modes, review
 * statuses, association types) into short sentences a non-medical reader can
 * follow. Pure functions; no genotype bases are read or emitted.
 */
import type { AssociationIs, CatalogAssociationSummary } from './conditionEvidence';

const PLACEHOLDER_NAMES = /^(not provided|not specified|see cases|none provided|unspecified|\d+\s+conditions?)$/i;
const DATABASE_ID = /^(?:medgen|mesh|omim|orphanet|mondo|human phenotype ontology|hpo|hp|efo|snomed(?:\s*ct)?|umls)\s*:|^(?:C|CN)\d{4,}\b|^HP:\d+$/i;

/** Returns a readable condition name, or null when the label is only an ID or placeholder. */
export function cleanConditionName(raw: string): string | null {
  const names = raw
    .split(/[;|]/)
    .map((part) => part.trim())
    .filter((part) => part && !DATABASE_ID.test(part) && !PLACEHOLDER_NAMES.test(part))
    .map((part) => part.replace(/\s+/g, ' '));
  const unique = [...new Set(names.map((name) => name.toLocaleLowerCase()))]
    .map((key) => names.find((name) => name.toLocaleLowerCase() === key) as string);
  return unique.length ? unique.join(' / ') : null;
}

/** One short clause describing how a condition is passed on, or null when unknown. */
export function inheritancePlain(modes: readonly string[]): string | null {
  const text = modes.join(' ').toLocaleLowerCase();
  if (!text) return null;
  if (text.includes('x-linked')) return 'ClinVar lists an X-linked inheritance pattern';
  if (text.includes('mitochondrial')) return 'ClinVar lists a pattern passed on through the mother';
  if (text.includes('semidominant')) return 'Effects may be stronger with two copies than with one';
  const recessive = text.includes('recessive');
  const dominant = text.includes('dominant');
  if (recessive && dominant) return 'ClinVar reports different one-copy and two-copy patterns for different variants';
  if (recessive) return 'This condition usually needs two copies to cause symptoms';
  if (dominant) return 'This condition can sometimes be linked to one copy';
  return null;
}

export function yourCopiesLabel(copies: number | null): string {
  if (copies === null) return 'Copy counts differ across lab records';
  if (copies >= 2) return '2 copies at this marker';
  if (copies === 1) return '1 copy at this marker';
  return '0 copies at this marker';
}

export interface PlainConditionInput {
  inheritance: readonly string[];
  associationKind: 'condition' | 'risk_factor' | 'mixed';
  variantSummaryConflict: boolean;
}

/** A one- or two-sentence explanation of what a matched condition link means. */
export function plainConditionSummary(input: PlainConditionInput): string {
  const parts: string[] = [];
  const inheritance = inheritancePlain(input.inheritance);
  if (inheritance) parts.push(`${inheritance}.`);
  if (input.associationKind === 'risk_factor') {
    parts.push('This marker is listed as a risk factor for the condition, not as a confirmed cause.');
  } else if (input.associationKind === 'mixed') {
    parts.push('ClinVar links this marker to the condition in some records and calls it a risk factor in others.');
  } else if (!inheritance) {
    parts.push('ClinVar lab reports link this marker to the condition.');
  }
  if (input.variantSummaryConflict) {
    parts.push('ClinVar’s broader variant records conflict. Those reports may refer to a different condition.');
  }
  return parts.join(' ');
}

const KIND_LABELS: Record<AssociationIs, string> = {
  variant_condition_summary: 'Variant-level condition link',
  variant_risk_factor_summary: 'Variant-level risk association',
  variant_trait_statistical_association: 'Population study',
  gene_disease_validity: 'Gene–disease link',
  variant_drug_response: 'Medicine response',
};

export function catalogKindLabel(kind: AssociationIs): string {
  return KIND_LABELS[kind] ?? 'Database link';
}

/** Plain explanation of a catalog link for the Simple report surface. */
export function catalogPlainSummary(
  association: Pick<CatalogAssociationSummary, 'association_is' | 'allele_match' | 'genes'>,
): string {
  const gene = association.genes[0];
  switch (association.association_is) {
    case 'variant_condition_summary':
    case 'variant_risk_factor_summary': {
      const what = association.association_is === 'variant_risk_factor_summary'
        ? 'as a risk factor for this condition'
        : 'with this condition in its overall variant summary';
      return association.allele_match === 'matched'
        ? `Your file matches the variant that ClinVar lists ${what}. This does not confirm a diagnosis.`
        : `ClinVar lists a variant at this DNA position ${what}, but your file cannot confirm that you carry it.`;
    }
    case 'variant_trait_statistical_association':
      return 'Large population studies found this DNA position is statistically linked to this trait.';
    case 'gene_disease_validity':
      return gene
        ? `Changes in the ${gene} gene are known to cause this condition. This is about the gene in general, not your specific marker.`
        : 'Changes in this gene are known to cause this condition. This is about the gene in general, not your specific marker.';
    case 'variant_drug_response':
      return 'This DNA position can affect how your body responds to this medicine.';
    default:
      return '';
  }
}

/** Whether a catalog row has a readable label worth showing to a lay reader. */
export function catalogLabelIsReadable(association: Pick<CatalogAssociationSummary, 'association_is' | 'label'>): boolean {
  if (association.association_is !== 'variant_condition_summary'
      && association.association_is !== 'variant_risk_factor_summary') return true;
  return cleanConditionName(association.label) !== null;
}
