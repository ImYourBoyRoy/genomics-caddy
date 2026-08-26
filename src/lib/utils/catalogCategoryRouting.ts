/**
 * Dynamic routing for the research discovery catalog.
 *
 * The catalog is the source of truth for available categories. Labels and the
 * default scan set are authored in research_taxonomy.json. The category list
 * itself is still derived from the catalog so new resource domains cannot
 * become silently unreachable.
 */

import researchTaxonomy from '../marker-packs/research_taxonomy.json';

export interface DiscoveryCatalogMarkerLike {
  category?: string | null;
  categories?: readonly string[] | null;
}

export interface CatalogCategoryOption {
  id: string;
  label: string;
  count: number;
}

interface DiscoveryCategoryConfig {
  id: string;
  label: string;
  order: number;
  default_selected: boolean;
}

const DISCOVERY_CATEGORY_CONFIG: DiscoveryCategoryConfig[] = Array.isArray(
  (researchTaxonomy as { discovery_categories?: DiscoveryCategoryConfig[] }).discovery_categories
)
  ? ((researchTaxonomy as { discovery_categories: DiscoveryCategoryConfig[] }).discovery_categories)
      .filter((category) => category && typeof category.id === 'string' && typeof category.label === 'string')
      .sort((left, right) => left.order - right.order)
  : [];

const DISCOVERY_CATEGORY_BY_ID = new Map(
  DISCOVERY_CATEGORY_CONFIG.map((category) => [category.id, category])
);

/**
 * Categories enabled in the normal scan. The full catalog remains available
 * through the UI, but scanning every research entry by default would create a
 * large and avoidable external-query workload.
 */
export const DEFAULT_SELECTED_CATALOG_CATEGORY_IDS = DISCOVERY_CATEGORY_CONFIG
  .filter((category) => category.default_selected)
  .map((category) => category.id);

function humanizeCategory(category: string): string {
  return category
    .split(/[_-]+/)
    .filter(Boolean)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function categorySortKey(category: string): [number, string] {
  return [DISCOVERY_CATEGORY_BY_ID.get(category)?.order ?? Number.MAX_SAFE_INTEGER, category];
}

export function catalogMarkerCategories(marker: DiscoveryCatalogMarkerLike): string[] {
  const categories = Array.isArray(marker.categories) ? marker.categories : [];
  return Array.from(new Set([
    ...categories,
    marker.category,
  ].map((category) => String(category || '').trim()).filter(Boolean)));
}

export function buildCatalogCategories(
  markers: readonly DiscoveryCatalogMarkerLike[]
): CatalogCategoryOption[] {
  const counts = new Map<string, number>();

  for (const marker of markers) {
    for (const category of catalogMarkerCategories(marker)) {
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
  }

  return [...counts.entries()]
    .sort(([left], [right]) => {
      const [leftRank, leftName] = categorySortKey(left);
      const [rightRank, rightName] = categorySortKey(right);
      return leftRank - rightRank || leftName.localeCompare(rightName);
    })
    .map(([id, count]) => ({
      id,
      label: DISCOVERY_CATEGORY_BY_ID.get(id)?.label ?? `🔬 ${humanizeCategory(id)}`,
      count
    }));
}

export function defaultCatalogCategorySelection(
  categories: readonly CatalogCategoryOption[]
): Record<string, boolean> {
  const defaults = new Set<string>(DEFAULT_SELECTED_CATALOG_CATEGORY_IDS);
  return Object.fromEntries(categories.map(category => [category.id, defaults.has(category.id)]));
}

export function selectedCatalogCategoryIds(
  categories: readonly CatalogCategoryOption[],
  selected: Readonly<Record<string, boolean>>
): Set<string> {
  return new Set(categories.filter(category => selected[category.id]).map(category => category.id));
}
