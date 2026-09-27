import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import type {
  GenomeWideClinVarAssociation,
  GenomeWideConditionRelevance,
} from '../types/genomics';

interface TaxonomyRoute {
  id: string;
  label: string;
  keywords: string[];
}

const routes = (researchTaxonomy as { categories: TaxonomyRoute[] }).categories;

export interface GenomeWideConditionGroup {
  id: string;
  condition: string;
  categoryId: string;
  categoryLabel: string;
  assertions: GenomeWideClinVarAssociation[];
  variantCount: number;
  conflicts: boolean;
  relevance: GenomeWideConditionRelevance;
  inheritance: string[];
  maxCopies: number;
}

export interface GenomeWideRelevanceSection {
  relevance: GenomeWideConditionRelevance;
  title: string;
  description: string;
  groups: GenomeWideConditionGroup[];
  /** Distinct matched variants behind the section's condition labels. */
  variantCount: number;
}

export const RELEVANCE_ORDER: readonly GenomeWideConditionRelevance[] = [
  'may_be_relevant',
  'carrier',
  'unclear',
];

const RELEVANCE_COPY: Record<GenomeWideConditionRelevance, { title: string; description: string }> = {
  may_be_relevant: {
    title: 'May be relevant to you',
    description:
      'The matched copy count fits how this condition is inherited. Rare disease-causing variants read from consumer DNA chips are often false positives, so confirm with a clinical genetic test before acting.',
  },
  carrier: {
    title: 'Carrier (one copy)',
    description:
      'One copy of a variant for a recessive or X-linked condition. Carriers usually do not have the condition, but it can matter for family planning and relatives.',
  },
  unclear: {
    title: 'Disputed or unclear',
    description:
      'Labs disagree about these variants, the gene’s inheritance is not curated, or the record is a risk factor rather than a cause. Variants ClinVar considers benign overall are not listed.',
  },
};

function normalize(value: string): string {
  return value.normalize('NFKC').trim().toLocaleLowerCase();
}

/** Taxonomy provides navigation only; the association always comes from ClinVar. */
export function classifyConditionTopic(condition: string): { id: string; label: string } {
  const normalized = normalize(condition);
  const match = routes.find((route) =>
    route.keywords.some((keyword) => normalized.includes(normalize(keyword)))
  );
  return match
    ? { id: match.id, label: match.label }
    : { id: 'other', label: 'Other conditions' };
}

function relevanceRank(value: GenomeWideConditionRelevance | undefined): number {
  const index = RELEVANCE_ORDER.indexOf(value ?? 'unclear');
  return index === -1 ? RELEVANCE_ORDER.length : index;
}

export function groupGenomeWideConditionAssociations(
  associations: readonly GenomeWideClinVarAssociation[],
): GenomeWideConditionGroup[] {
  const groups = new Map<string, GenomeWideConditionGroup>();

  for (const association of associations) {
    const condition = association.condition.trim();
    if (!condition) continue;
    const topic = classifyConditionTopic(condition);
    const id = `${topic.id}:${normalize(condition)}`;
    const group = groups.get(id) ?? {
      id,
      condition,
      categoryId: topic.id,
      categoryLabel: topic.label,
      assertions: [],
      variantCount: 0,
      conflicts: false,
      relevance: 'unclear' as GenomeWideConditionRelevance,
      inheritance: [],
      maxCopies: 0,
    };
    group.assertions.push(association);
    group.conflicts ||= association.variant_summary_conflict;
    if (group.assertions.length === 1 || relevanceRank(association.relevance) < relevanceRank(group.relevance)) {
      group.relevance = association.relevance ?? 'unclear';
    }
    group.maxCopies = Math.max(group.maxCopies, association.alt_allele_copies ?? 0);
    for (const mode of association.inheritance ?? []) {
      if (!group.inheritance.includes(mode)) group.inheritance.push(mode);
    }
    groups.set(id, group);
  }

  for (const group of groups.values()) {
    group.variantCount = new Set(group.assertions.map((item) => item.rsid)).size;
  }

  const categoryOrder = new Map(routes.map((route, index) => [route.id, index]));
  return [...groups.values()].sort((left, right) =>
    (categoryOrder.get(left.categoryId) ?? Number.MAX_SAFE_INTEGER)
      - (categoryOrder.get(right.categoryId) ?? Number.MAX_SAFE_INTEGER)
    || left.condition.localeCompare(right.condition)
  );
}

/** Splits condition groups into may-be-relevant, carrier, and unclear sections, dropping empty ones. */
export function sectionConditionGroupsByRelevance(
  groups: readonly GenomeWideConditionGroup[],
): GenomeWideRelevanceSection[] {
  return RELEVANCE_ORDER.map((relevance) => {
    const sectionGroups = groups.filter((group) => group.relevance === relevance);
    const variants = new Set(sectionGroups.flatMap((group) => group.assertions.map((item) => item.rsid)));
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
