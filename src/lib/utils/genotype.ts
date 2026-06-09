// ./src/lib/utils/genotype.ts
/*
Module Docstring:
Purpose: Genotype extraction and normalization utility functions.
Responsibilities:
- Normalize genotype raw call strings from DNA text profiles.
- Extract effect counts, alleles, and call status.
Key Inputs: Evaluated marker records.
Key Outputs: Normalized genotypes, counts, and boolean call status flags.
Operational Notes: Uses only the current "effect_allele" / "effect_count" schema.
*/

import type { EvaluatedMarker, SeverityClass } from "../types/genomics";

export function getEffectCount(marker: EvaluatedMarker): number {
  return marker.effect_count ?? 0;
}

export function getEffectAllele(marker: EvaluatedMarker): string {
  return marker.effect_allele || "";
}

export function getEffectDirection(marker: EvaluatedMarker): string {
  return marker.effect_direction || "unknown";
}

export function getSeverityClass(marker: EvaluatedMarker): SeverityClass {
  return marker.severity_class || "benign";
}

export function normalizeGenotype(genotype: string): string {
  if (
    !genotype ||
    genotype === "--" ||
    genotype.includes("-") ||
    genotype.includes("0") ||
    genotype.includes("?")
  ) {
    return "--";
  }
  return genotype.toUpperCase().trim();
}

export function isNoCall(genotype: string): boolean {
  return normalizeGenotype(genotype) === "--";
}

export function isVariantDetected(marker: EvaluatedMarker): boolean {
  const genotype = normalizeGenotype(marker.user_genotype);
  if (genotype === "--") return false;
  const effectAllele = getEffectAllele(marker);
  if (!effectAllele) return false;

  const effectChar = effectAllele.charAt(0);
  return genotype.includes(effectChar);
}
