/**
 * Dynamic routing for the research discovery catalog.
 *
 * The catalog is the source of truth for available categories. Labels and the
 * default scan set are UI policy, while the category list itself is derived
 * from the catalog so new resource domains cannot become silently unreachable.
 */

export interface DiscoveryCatalogMarkerLike {
  category?: string | null;
}

export interface CatalogCategoryOption {
  id: string;
  label: string;
  count: number;
}

const CATEGORY_LABELS: Record<string, string> = {
  core: "🧬 Core Traits",
  pgx: "💊 Drug Metabolism",
  metabolic: "🥗 Metabolic",
  nutrients: "🍽️ Nutrients",
  neuropsych: "🧠 Neurotype & Mood",
  cardiovascular: "🫀 Cardiovascular",
  cancer_confirmation_only: "🎗️ Cancer Risk",
  hormones_reproductive: "🌙 Hormones & Reproductive",
  allergy_atopy_mast_cell: "🌿 Allergy & Mast Cell",
  bone_growth_mineral_density: "🦴 Bone & Mineral Density",
  connective_tissue: "🧵 Connective Tissue",
  dental_oral_health: "🦷 Dental & Oral Health",
  digestive_gut_microbiome: "🫃 Digestive & Gut",
  immune_autoimmune_general: "🛡️ Immune & Autoimmune",
  kidney_fluid_electrolytes: "💧 Kidney & Electrolytes",
  longevity_aging_resilience: "⌛ Longevity & Resilience",
  muscle_performance_recovery: "💪 Muscle & Recovery",
  pain_migraine_sensory: "🩹 Pain & Sensory",
  respiratory_airway: "🫁 Respiratory & Airway",
  skin_hair_dermatology: "🧴 Skin, Hair & Dermatology",
  sleep: "🌙 Sleep",
  thyroid_autoimmune: "🦋 Thyroid & Autoimmune"
};

const CATEGORY_ORDER = [
  "core",
  "pgx",
  "metabolic",
  "nutrients",
  "neuropsych",
  "cardiovascular",
  "cancer_confirmation_only",
  "hormones_reproductive"
];

/**
 * Categories enabled in the normal scan. The full catalog remains available
 * through the UI, but scanning every research entry by default would create a
 * large and avoidable external-query workload.
 */
export const DEFAULT_SELECTED_CATALOG_CATEGORY_IDS = [
  "core",
  "pgx",
  "metabolic",
  "nutrients",
  "neuropsych",
  "cardiovascular",
  "cancer_confirmation_only",
  "hormones_reproductive"
] as const;

function humanizeCategory(category: string): string {
  return category
    .split(/[_-]+/)
    .filter(Boolean)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function categorySortKey(category: string): [number, string] {
  const knownIndex = CATEGORY_ORDER.indexOf(category);
  return [knownIndex === -1 ? CATEGORY_ORDER.length : knownIndex, category];
}

export function buildCatalogCategories(
  markers: readonly DiscoveryCatalogMarkerLike[]
): CatalogCategoryOption[] {
  const counts = new Map<string, number>();

  for (const marker of markers) {
    const category = marker.category?.trim();
    if (!category) continue;
    counts.set(category, (counts.get(category) ?? 0) + 1);
  }

  return [...counts.entries()]
    .sort(([left], [right]) => {
      const [leftRank, leftName] = categorySortKey(left);
      const [rightRank, rightName] = categorySortKey(right);
      return leftRank - rightRank || leftName.localeCompare(rightName);
    })
    .map(([id, count]) => ({
      id,
      label: CATEGORY_LABELS[id] ?? `🔬 ${humanizeCategory(id)}`,
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
