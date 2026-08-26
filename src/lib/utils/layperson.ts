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
