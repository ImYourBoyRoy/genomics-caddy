// ./src/lib/utils/researchScopeHelpers.ts
/**
 * Pure helpers for research sweep scope configuration and enrichment source presets.
 */

import type { ResearchScopeConfig } from "../types/research";
import { DEFAULT_ENRICHMENT_SOURCES, FULL_ENRICHMENT_SOURCES } from "../types/research";

export function ensureEnrichmentSources(scope: ResearchScopeConfig): void {
  if (!scope.enrichment_sources) {
    scope.enrichment_sources = scope.sweep_fast
      ? { ...DEFAULT_ENRICHMENT_SOURCES }
      : { ...FULL_ENRICHMENT_SOURCES };
  }
}

export function syncSweepFastFromSources(scope: ResearchScopeConfig): void {
  ensureEnrichmentSources(scope);
  const s = scope.enrichment_sources!;
  scope.sweep_fast =
    !s.gnomad &&
    !s.clinvar_live &&
    !s.pubmed &&
    !s.gtex &&
    !s.vep_dbsnp &&
    !s.secondary;
}

export function detectActivePreset(
  scope: ResearchScopeConfig
): "fast" | "clinical" | "full" | null {
  ensureEnrichmentSources(scope);
  const s = scope.enrichment_sources!;
  const isFull =
    s.gnomad &&
    s.clinvar_live &&
    s.pubmed &&
    s.gtex &&
    s.vep_dbsnp &&
    s.secondary;
  const isClinical =
    !s.gnomad &&
    s.clinvar_live &&
    s.pubmed &&
    s.gtex &&
    s.vep_dbsnp &&
    !s.secondary;
  const isFast =
    !s.gnomad &&
    !s.clinvar_live &&
    !s.pubmed &&
    !s.gtex &&
    !s.vep_dbsnp &&
    !s.secondary;
  if (isFull) return "full";
  if (isClinical) return "clinical";
  if (isFast) return "fast";
  return null;
}

export function applySourcePreset(
  scope: ResearchScopeConfig,
  preset: "fast" | "clinical" | "full"
): string {
  ensureEnrichmentSources(scope);
  const keepSupplement = scope.enrichment_sources!.supplement_missing ?? false;
  if (preset === "fast") {
    scope.enrichment_sources = { ...DEFAULT_ENRICHMENT_SOURCES, supplement_missing: keepSupplement };
  } else if (preset === "clinical") {
    scope.enrichment_sources = {
      gnomad: false,
      clinvar_live: true,
      pubmed: true,
      gtex: true,
      vep_dbsnp: true,
      secondary: false,
      supplement_missing: keepSupplement,
    };
  } else {
    scope.enrichment_sources = { ...FULL_ENRICHMENT_SOURCES, supplement_missing: keepSupplement };
  }
  syncSweepFastFromSources(scope);
  if (preset === "fast") return "Source preset: Fast index (local GWAS + embed only).";
  if (preset === "clinical") return "Source preset: Clinical depth (no gnomAD / secondary adapters).";
  return "Source preset: Full depth (all live sources).";
}
