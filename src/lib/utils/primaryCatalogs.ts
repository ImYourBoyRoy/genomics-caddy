// ./src/lib/utils/primaryCatalogs.ts
/*
Purpose: Shared definition of the four primary reference catalogs shown in the UI.
Used by EmptyState / sidebar messaging so "missing" counts match what users see.
*/

/** Sidebar primary download rows (bundles count as one catalog each). */
export const PRIMARY_CATALOG_IDS = [
  'gwas_catalog',
  'clinvar_variant_summary',
  'pharmgkb_clinical_variants',
  'dbsnp_merged_json',
] as const;

export type PrimaryCatalogId = (typeof PRIMARY_CATALOG_IDS)[number];

export function isPrimaryCatalogId(id: string): id is PrimaryCatalogId {
  return (PRIMARY_CATALOG_IDS as readonly string[]).includes(id);
}
