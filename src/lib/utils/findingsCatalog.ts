// ./src/lib/utils/findingsCatalog.ts
/**
 * Non-AI findings catalog: presets, labels, and sort options for individuals vs clinicians.
 */

export type AudienceMode = "individual" | "clinical";
export type CatalogPreset = "actionable" | "clinical" | "gwas" | "unknown" | "all";
export type CatalogSort = "wellness" | "clinical" | "dq" | "gene" | "rsid";

export const PAGE_SIZE = 40;

export interface CatalogPresetOption {
  id: CatalogPreset;
  individualLabel: string;
  clinicalLabel: string;
  description: string;
}

export const CATALOG_PRESETS: CatalogPresetOption[] = [
  {
    id: "actionable",
    individualLabel: "Most relevant to you",
    clinicalLabel: "High wellness actionability",
    description: "Variants ranked by lifestyle and wellness relevance with known direction when available.",
  },
  {
    id: "clinical",
    individualLabel: "Clinical literature signals",
    clinicalLabel: "Clinical actionability",
    description: "Variants with stronger clinical annotation or pharmacogenomic context — review, not diagnosis.",
  },
  {
    id: "gwas",
    individualLabel: "Research associations (GWAS)",
    clinicalLabel: "GWAS catalog hits",
    description: "Population-level trait associations from GWAS catalog overlap with your genotype.",
  },
  {
    id: "unknown",
    individualLabel: "Needs direction review",
    clinicalLabel: "Unknown effect direction",
    description: "Associations present but personal allele direction is not yet resolved.",
  },
  {
    id: "all",
    individualLabel: "Full indexed library",
    clinicalLabel: "All indexed variants",
    description: "Every variant with association facts in your local index.",
  },
];

export const SORT_OPTIONS: { id: CatalogSort; label: string }[] = [
  { id: "wellness", label: "Wellness relevance" },
  { id: "clinical", label: "Clinical score" },
  { id: "dq", label: "Data quality" },
  { id: "gene", label: "Gene name" },
  { id: "rsid", label: "rsID" },
];

export function presetLabel(preset: CatalogPresetOption, audience: AudienceMode): string {
  return audience === "clinical" ? preset.clinicalLabel : preset.individualLabel;
}

export function defaultPreset(audience: AudienceMode): CatalogPreset {
  return audience === "clinical" ? "clinical" : "actionable";
}

export function defaultSort(audience: AudienceMode): CatalogSort {
  return audience === "clinical" ? "clinical" : "wellness";
}

export function formatDirection(raw: string): string {
  return raw.replace(/_/g, " ");
}

export function directionBadgeClass(direction: string): string {
  const d = direction.toLowerCase();
  if (d.includes("increased") || d.includes("risk")) return "dir-risk";
  if (d.includes("decreased") || d.includes("protect")) return "dir-protect";
  if (d.includes("unknown")) return "dir-unknown";
  return "dir-neutral";
}

export function traitLabel(id: string): string {
  return id.replace(/_/g, " ");
}
