import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import type {
  GenomeWideClinVarAssociation,
  GenomeWideConditionRelevance,
} from '../types/genomics';
import { cleanConditionName } from './plainConditionCopy';

interface TaxonomyRoute {
  id: string;
  label: string;
  keywords: string[];
}

const routes = (researchTaxonomy as { categories: TaxonomyRoute[] }).categories;

export type ConditionAssociationKind = 'condition' | 'risk_factor' | 'mixed';

export interface GenomeWideConditionGroup {
  /** One ClinVar condition matched to one exact variant. */
  id: string;
  condition: string;
  categoryId: string;
  categoryLabel: string;
  rsid: string;
  geneSymbol: string | null;
  variationId: string;
  assertions: GenomeWideClinVarAssociation[];
  /** All matched ClinVar records must agree before a copy count is summarized. */
  copyCount: number | null;
  /** True only if every included ClinVar submission calls this a risk factor. */
  associationKind: ConditionAssociationKind;
  /** ClinVar's variant-wide summary conflict; this is not condition-specific. */
  variantSummaryConflict: boolean;
  /** Unreadable or ID-only condition labels stay attached to their own lab record. */
  unnamed: boolean;
  /** Conflicting per-record relevance is kept in the cautious unclear group. */
  relevance: GenomeWideConditionRelevance;
  /** Inheritance is summarized only when every record gives the same modes. */
  inheritance: string[];
}

export interface GenomeWideRelevanceSection {
  relevance: GenomeWideConditionRelevance;
  title: string;
  description: string;
  groups: GenomeWideConditionGroup[];
  /** Distinct matched variants in this section. */
  variantCount: number;
}

export const RELEVANCE_ORDER: readonly GenomeWideConditionRelevance[] = [
  'may_be_relevant',
  'carrier',
  'unclear',
];

const RELEVANCE_COPY: Record<GenomeWideConditionRelevance, { title: string; description: string }> = {
  may_be_relevant: {
    title: 'May be relevant',
    description:
      'The copy count fits an inherited pattern reported for this condition. This is a database match, not a diagnosis. A clinical genetic test is needed to confirm a rare variant.',
  },
  carrier: {
    title: 'One-copy carrier pattern',
    description:
      'One copy matched a pattern usually linked to carrying a recessive condition. This does not show that you have the condition; a genetic counselor can explain when it matters for family planning.',
  },
  unclear: {
    title: 'Unclear or mixed evidence',
    description:
      'ClinVar records conflict or lack the inheritance details needed to interpret this link.',
  },
};

function normalize(value: string): string {
  return value.normalize('NFKC').trim().toLocaleLowerCase();
}

/** Taxonomy provides navigation only; each association still comes from ClinVar. */
export function classifyConditionTopic(condition: string): { id: string; label: string } {
  const normalized = normalize(condition);
  const match = routes.find((route) =>
    route.keywords.some((keyword) => normalized.includes(normalize(keyword)))
  );
  return match
    ? { id: match.id, label: match.label }
    : { id: 'other', label: 'Other conditions' };
}

export const UNNAMED_CONDITION_LABEL = 'Condition name not supplied by the lab';

function relevanceFor(assertions: readonly GenomeWideClinVarAssociation[]): GenomeWideConditionRelevance {
  const values = new Set(assertions.map((item) => item.relevance ?? 'unclear'));
  return values.size === 1 ? [...values][0] : 'unclear';
}

function inheritanceFor(assertions: readonly GenomeWideClinVarAssociation[]): string[] {
  const normalizedModes = assertions.map((item) =>
    [...new Set((item.inheritance ?? []).map(normalize))].sort().join('|')
  );
  if (new Set(normalizedModes).size !== 1 || !normalizedModes[0]) return [];
  return assertions[0]?.inheritance ?? [];
}

function associationKindFor(
  assertions: readonly GenomeWideClinVarAssociation[],
): ConditionAssociationKind {
  const kinds = new Set(assertions.map((item) =>
    item.association_is === 'variant_risk_factor_summary' ? 'risk_factor' : 'condition'
  ));
  return kinds.size > 1 ? 'mixed' : [...kinds][0] === 'risk_factor' ? 'risk_factor' : 'condition';
}

function copyCountFor(assertions: readonly GenomeWideClinVarAssociation[]): number | null {
  const counts = new Set(assertions.map((item) => item.alt_allele_copies));
  return counts.size === 1 ? [...counts][0] : null;
}

/**
 * Groups duplicate lab submissions only when they name the same condition and
 * identify the same exact ClinVar variant. Different markers are never folded
 * together, and unnamed disease labels remain separate records.
 */
export function groupGenomeWideConditionAssociations(
  associations: readonly GenomeWideClinVarAssociation[],
): GenomeWideConditionGroup[] {
  const groups = new Map<string, GenomeWideConditionGroup>();

  for (const association of associations) {
    const condition = cleanConditionName(association.condition);
    const unnamed = condition === null;
    const displayCondition = condition ?? UNNAMED_CONDITION_LABEL;
    const topic = unnamed
      ? { id: 'unnamed', label: '' }
      : classifyConditionTopic(displayCondition);
    const markerKey = `${association.rsid}:${association.variation_id}`;
    // Without a condition name, do not claim that separate lab records refer
    // to the same disease. Preserve each assertion as its own item.
    const id = unnamed
      ? `unnamed:${markerKey}:${association.scv_accession}:${association.condition}`
      : `${topic.id}:${normalize(displayCondition)}:${markerKey}`;

    const existing = groups.get(id);
    if (existing) {
      existing.assertions.push(association);
      existing.copyCount = copyCountFor(existing.assertions);
      existing.associationKind = associationKindFor(existing.assertions);
      existing.variantSummaryConflict ||= association.variant_summary_conflict;
      existing.relevance = relevanceFor(existing.assertions);
      existing.inheritance = inheritanceFor(existing.assertions);
      continue;
    }

    groups.set(id, {
      id,
      condition: displayCondition,
      categoryId: topic.id,
      categoryLabel: topic.label,
      rsid: association.rsid,
      geneSymbol: association.gene_symbol ?? null,
      variationId: association.variation_id,
      assertions: [association],
      copyCount: association.alt_allele_copies,
      associationKind: associationKindFor([association]),
      variantSummaryConflict: association.variant_summary_conflict,
      unnamed,
      relevance: association.relevance ?? 'unclear',
      inheritance: association.inheritance ?? [],
    });
  }

  const categoryOrder = new Map(routes.map((route, index) => [route.id, index]));
  return [...groups.values()].sort((left, right) =>
    Number(left.unnamed) - Number(right.unnamed)
    || relevanceOrder(left.relevance) - relevanceOrder(right.relevance)
    || (categoryOrder.get(left.categoryId) ?? Number.MAX_SAFE_INTEGER)
      - (categoryOrder.get(right.categoryId) ?? Number.MAX_SAFE_INTEGER)
    || left.condition.localeCompare(right.condition)
    || left.rsid.localeCompare(right.rsid)
  );
}

function relevanceOrder(value: GenomeWideConditionRelevance): number {
  const index = RELEVANCE_ORDER.indexOf(value);
  return index === -1 ? RELEVANCE_ORDER.length : index;
}

/** Splits exact condition-and-variant entries by cautious relevance. */
export function sectionConditionGroupsByRelevance(
  groups: readonly GenomeWideConditionGroup[],
): GenomeWideRelevanceSection[] {
  return RELEVANCE_ORDER.map((relevance) => {
    const sectionGroups = groups.filter((group) => group.relevance === relevance);
    const variants = new Set(sectionGroups.map((group) => group.rsid));
    return {
      relevance,
      ...RELEVANCE_COPY[relevance],
      groups: sectionGroups,
      variantCount: variants.size,
    };
  }).filter((section) => section.groups.length > 0);
}

export function copyCountLabel(copies: number): string {
  if (copies >= 2) return 'Two copies';
  if (copies === 1) return 'One copy';
  return 'Copy count unavailable';
}
