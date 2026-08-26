// ./src/lib/utils/layperson.ts
/**
 * Purpose: Provide clear, plain-English translations for genetic marker impacts and meanings.
 * Responsibilities:
 * - Define the LaypersonTranslation interface.
 * - Load resource-authored translations for curated marker IDs.
 * Key Inputs: layperson_translations.json.
 * Key Outputs: LAYPERSON_MAP dictionary.
 * Operational Notes: Translations make findings easier to understand without changing evidence,
 * probability, callability, or clinical claim boundaries.
 */

import type { EvaluatedMarker } from "../types/genomics";
import laypersonTranslations from "../marker-packs/layperson_translations.json";

export interface LaypersonTranslation {
  simpleImpact: string;
  simpleMeaning: string;
}

export const LAYPERSON_MAP: Record<string, LaypersonTranslation> = Object.fromEntries(
  laypersonTranslations.translations.map(({ rsid, simpleImpact, simpleMeaning }) => [
    rsid,
    { simpleImpact, simpleMeaning },
  ]),
) as Record<string, LaypersonTranslation>;

export const DEFAULT_LAYPERSON_TRANSLATION: LaypersonTranslation = laypersonTranslations.fallback;

/**
 * Keep untranslated markers useful by reusing their curated resource text.
 * This is a presentation fallback only: it does not upgrade the marker's
 * evidence tier, claim boundaries, callability, or clinical actionability.
 */
export function getLaypersonTranslation(
  marker: Pick<
    EvaluatedMarker,
    "rsid" | "gene" | "impact" | "interpretation" | "raw_dna_limitation" | "clinical_confirmation_required"
  >,
  translations: Record<string, LaypersonTranslation> = LAYPERSON_MAP,
): LaypersonTranslation {
  const explicit = translations[marker.rsid];
  if (explicit) return explicit;

  const gene = marker.gene.trim();
  const impact = marker.impact.trim();
  const interpretation = marker.interpretation.trim();
  const limitation = marker.raw_dna_limitation?.trim();
  const simpleImpact = impact || (gene ? `${gene} genetic context` : DEFAULT_LAYPERSON_TRANSLATION.simpleImpact);
  const safetyBoundary = limitation
    || "This is probabilistic genetic context; it does not by itself diagnose a condition, measure current health, or predict an individual outcome.";
  const confirmationNote = marker.clinical_confirmation_required
    ? " Clinical confirmation is important before medical decisions."
    : "";

  return {
    simpleImpact,
    simpleMeaning: `${interpretation || DEFAULT_LAYPERSON_TRANSLATION.simpleMeaning} ${safetyBoundary}${confirmationNote}`.trim(),
  };
}
