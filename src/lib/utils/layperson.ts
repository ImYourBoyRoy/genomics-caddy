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
  /** True when the safe generic fallback is being used instead of authored copy. */
  isFallback?: boolean;
}

export const LAYPERSON_MAP: Record<string, LaypersonTranslation> = Object.fromEntries(
  laypersonTranslations.translations.map(({ rsid, simpleImpact, simpleMeaning }) => [
    rsid,
    { simpleImpact, simpleMeaning },
  ]),
) as Record<string, LaypersonTranslation>;

export const DEFAULT_LAYPERSON_TRANSLATION: LaypersonTranslation = laypersonTranslations.fallback;

const SAFE_FALLBACK: LaypersonTranslation = {
  simpleImpact: "A biological pathway studied in genetic research.",
  simpleMeaning:
    "This DNA result is associated with a research finding. It does not predict whether you have a condition, measure your current health, or tell you what treatment to use.",
  isFallback: true,
};

/**
 * Keep untranslated markers useful without leaking technical interpretation
 * text into Simple mode. A missing translation is a content-quality gap, not
 * permission to expose the clinical resource wording to casual users.
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

  if (marker.clinical_confirmation_required) {
    return {
      ...SAFE_FALLBACK,
      simpleMeaning: `${SAFE_FALLBACK.simpleMeaning} A clinical test may be needed before medical decisions.`,
    };
  }

  return { ...SAFE_FALLBACK };
}
