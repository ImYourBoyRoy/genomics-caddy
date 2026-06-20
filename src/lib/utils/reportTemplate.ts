// ./src/lib/utils/reportTemplate.ts
/**
 * Builds the merged marker-pack report template for Tauri report generation.
 */

import type { MarkerDefinition, ReportTemplate, SectionDefinition } from "../types/genomics";
import manifest from "../marker-packs/manifest.json";
import core from "../marker-packs/core.json";
import pgx from "../marker-packs/pgx.json";
import metabolic from "../marker-packs/metabolic.json";
import nutrients from "../marker-packs/nutrients.json";
import neuropsych from "../marker-packs/neuropsych.json";
import sleep from "../marker-packs/sleep.json";
import connectiveTissue from "../marker-packs/connective_tissue.json";
import thyroidAutoimmune from "../marker-packs/thyroid_autoimmune.json";
import cardiovascular from "../marker-packs/cardiovascular.json";
import cancerConfirmationOnly from "../marker-packs/cancer_confirmation_only.json";

type PackContent = { name: string; markers: MarkerDefinition[] };

const PACKS_MAP: Record<string, PackContent> = {
  core: core as PackContent,
  pgx: pgx as PackContent,
  metabolic: metabolic as PackContent,
  nutrients: nutrients as PackContent,
  neuropsych: neuropsych as PackContent,
  sleep: sleep as PackContent,
  connective_tissue: connectiveTissue as PackContent,
  thyroid_autoimmune: thyroidAutoimmune as PackContent,
  cardiovascular: cardiovascular as PackContent,
  cancer_confirmation_only: cancerConfirmationOnly as PackContent,
};

export function buildMergedReportTemplate(): ReportTemplate {
  const sections: SectionDefinition[] = [];
  for (const pack of manifest.packs) {
    const packContent = PACKS_MAP[pack.id];
    if (packContent?.markers) {
      sections.push({
        name: packContent.name || pack.label,
        markers: packContent.markers,
      });
    }
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
