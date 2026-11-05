// ./src/lib/utils/primaryCatalogs.ts
/*
Purpose: Shared catalog ID lists for compact attention messaging and full sync status.
Used by the welcome and sidebar UI so "missing" counts match what users see.
*/

/** The smaller set surfaced in the sidebar's catalog attention summary. */
export const PRIMARY_CATALOG_IDS = [
  'gwas_catalog',
  'clinvar_variant_summary',
  'pharmgkb_clinical_variants',
  'dbsnp_merged_json',
] as const;

/** Public reference files available from Sync All Missing (liftover is separate). */
export const REFERENCE_CATALOG_IDS = [
  'gwas_catalog',
  'clinvar_variant_summary',
  'clinvar_submission_summary',
  'pharmgkb_clinical_variants',
  'pharmgkb_genes',
  'clingen_gene_validity',
  'mane_select_summary',
  'dbsnp_merged_json',
  'dbsnp_withdrawn_json',
] as const;

export type PrimaryCatalogId = (typeof PRIMARY_CATALOG_IDS)[number];

export function isPrimaryCatalogId(id: string): id is PrimaryCatalogId {
  return (PRIMARY_CATALOG_IDS as readonly string[]).includes(id);
}
