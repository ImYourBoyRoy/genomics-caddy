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

interface LaypersonTranslationGroup {
  id: string;
  rsids: string[];
  simpleImpact: string;
  simpleMeaning: string;
}

function buildLaypersonMap(): Record<string, LaypersonTranslation> {
  const map: Record<string, LaypersonTranslation> = {};
  for (const translation of laypersonTranslations.translations) {
    map[translation.rsid.trim().toLowerCase()] = {
      simpleImpact: translation.simpleImpact,
      simpleMeaning: translation.simpleMeaning,
    };
  }

  // Pack-scoped copy is an authored plain-language layer for markers that do
  // not yet have marker-specific wording. Explicit marker translations above
  // always win, so adding a group can never replace reviewed copy.
  const groups = (laypersonTranslations.translation_groups ?? []) as LaypersonTranslationGroup[];
  for (const group of groups) {
    for (const rsid of group.rsids) {
      const key = rsid.trim().toLowerCase();
      if (!map[key]) {
        map[key] = {
          simpleImpact: group.simpleImpact,
          simpleMeaning: group.simpleMeaning,
        };
      }
    }
  }
  return map;
}

export const LAYPERSON_MAP = buildLaypersonMap();

export const DEFAULT_LAYPERSON_TRANSLATION: LaypersonTranslation = laypersonTranslations.fallback;

const GENERIC_GUARDRAIL_PATTERNS = [
  /\bdoes not (?:diagnose|prove)\b/i,
  /\bdoes not predict whether you have (?:a )?condition\b/i,
  /\bnot (?:a|an) (?:diagnosis|treatment|prescription)\b/i,
  /\b(?:requires|needs?) (?:a|an|the)?\s*(?:doctor|clinician|clinical|medical-grade|healthcare)\b.*\bbefore\b/i,
  /\bbefore making (?:health|medical) decisions\b/i,
];

function isGenericGuardrail(text: string): boolean {
  return GENERIC_GUARDRAIL_PATTERNS.some((pattern) => pattern.test(text));
}

/**
 * Keep the visible Simple card focused on the finding itself. Broad claim
 * boundaries remain available in the card's disclosure and in clinical/AI
 * outputs; this display helper removes only generic warning sentences or
 * semicolon clauses that would otherwise repeat the report-level notice.
 */
export function getCompactSimpleMeaning(translation: LaypersonTranslation): string {
  const sentences = translation.simpleMeaning
    .trim()
    .split(/(?<=[.!?])\s+/)
    .map((sentence) => sentence.trim())
    .filter(Boolean);
  const compact = sentences
    .map((sentence) => sentence
      .split(/;\s+/)
      .filter((clause) => !isGenericGuardrail(clause))
      .join('; ')
      .trim())
    .filter((sentence) => sentence && !isGenericGuardrail(sentence));

  return compact.join(' ') || translation.simpleImpact;
}

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
  const explicit = translations[marker.rsid] || translations[marker.rsid.trim().toLowerCase()];
  if (explicit) return explicit;

  if (marker.clinical_confirmation_required) {
    return {
      ...SAFE_FALLBACK,
      simpleMeaning: `${SAFE_FALLBACK.simpleMeaning} A clinical test may be needed before medical decisions.`,
    };
  }

  return { ...SAFE_FALLBACK };
}
