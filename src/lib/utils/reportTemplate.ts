// ./src/lib/utils/reportTemplate.ts
/**
 * Builds the merged marker-pack report template for Tauri report generation.
 *
 * Responsibilities:
 * - Build the merged report template using dynamically loaded packs from `markerPacksStore`.
 * - Expose functions to retrieve the merged report structure as an object or serialized JSON.
 *
 * Key Inputs: markerPacksStore state.
 * Key Outputs: ReportTemplate data structures.
 * Operational Notes: Completely dynamic; avoids static marker pack JSON imports.
 */

import type { ReportTemplate, SectionDefinition } from "../types/genomics";
import { markerPacksStore } from "./markerPacksState.svelte";

export function buildMergedReportTemplate(): ReportTemplate {
  const sections: SectionDefinition[] = [];
  const manifest = markerPacksStore.manifest;
  const packs = markerPacksStore.packs;

  for (const pack of manifest.packs) {
    const packContent = packs[pack.id];
    // Skip empty packs.
    if (!packContent?.markers?.length) continue;
    // research_found is opt-in via manifest.default_enabled (Review panel toggle).
    if (pack.id === "research_found" && pack.default_enabled === false) continue;
    sections.push({
      name: packContent.name || pack.label,
      markers: packContent.markers,
    });
  }
  return {
    title: "DNA Analysis & Biohacker Profile Report",
    description:
      "Personal genomic profile matching candidate markers across multiple health systems.",
    sections,
  };
}

export function mergedReportTemplateJson(): string {
  return JSON.stringify(buildMergedReportTemplate());
}
