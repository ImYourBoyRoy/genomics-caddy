import researchTaxonomy from '../marker-packs/research_taxonomy.json';
import type { GenomeWideClinVarAssociation } from '../types/genomics';

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
}

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
    };
    group.assertions.push(association);
    group.conflicts ||= association.variant_summary_conflict;
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
