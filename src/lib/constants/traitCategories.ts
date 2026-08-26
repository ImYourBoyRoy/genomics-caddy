// ./src/lib/constants/traitCategories.ts
import researchTaxonomy from "../marker-packs/research_taxonomy.json";

/** UI discovery categories derived from the shared research taxonomy resource. */

export interface TraitCategoryOption {
  id: string;
  label: string;
  queryHint: string;
}

export const TRAIT_CATEGORIES: TraitCategoryOption[] = researchTaxonomy.categories.map((category) => ({
  id: category.id,
  label: category.label,
  queryHint: category.query_hint,
}));

export type VariantNavTarget = "report" | "map" | "browser" | "evidence";
