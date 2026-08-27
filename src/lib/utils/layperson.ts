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

const GENERIC_TITLE_PREFIXES = new Set(['DNA', 'RNA', 'SNP']);

/**
 * Keep gene symbols and locus identifiers in Technical data for Simple mode.
 * The authored wording remains unchanged in the resource and technical
 * fields; this helper only changes personal-facing summary labels.
 */
export function getSimpleFindingTitle(simpleImpact: string): string {
  const title = simpleImpact.trim();
  if (!title) return 'Genetic context marker';

  const withoutGeneSuffix = title
    .replace(/\s+\([^()]*\bgene(?:\s+region)?\b[^()]*\)\s*$/i, '')
    .trim();
  const leadingTechnical = withoutGeneSuffix.match(
    /^((?:[A-Z]{2,}[A-Z0-9]*(?:\*[A-Z0-9]+)?(?:\/(?:[A-Z]{2,}[A-Z0-9]*(?:\*[A-Z0-9]+)?)|\/(?=[a-z]))?|[0-9]+p[0-9.]+))\s*(.+)$/,
  );

  if (!leadingTechnical || GENERIC_TITLE_PREFIXES.has(leadingTechnical[1])) {
    return withoutGeneSuffix;
  }

  return leadingTechnical[2].trim() || withoutGeneSuffix;
}

const GENERIC_GUARDRAIL_PATTERNS = [
  /\bdoes not (?:diagnose|prove)\b/i,
  /\bdoes not predict whether you have (?:a )?condition\b/i,
  /\bnot (?:a|an) (?:diagnosis|treatment|prescription)\b/i,
  /\b(?:requires|needs?) (?:a|an|the)?\s*(?:doctor|clinician|clinical|medical-grade|healthcare)\b.*\bbefore\b/i,
  /\bbefore making (?:health|medical) decisions\b/i,
];

const COMPACT_GUIDANCE_PREFIX = /(?:Do not make this change from raw DNA; confirm the finding clinically first:|Only consider after clinical confirmation and individualized advice:|Do not make this change unless symptoms, labs, or clinician guidance support it:|Consider only if symptoms, labs, or personal goals support it:|General health consideration, not a genotype-specific restriction:|General low-risk option, not a genotype prescription:)\s*/gi;

/**
 * Remove repeated actionability framing from secondary Simple guidance lists.
 * The section heading provides the shared context; authored plan data remains
 * unchanged for clinical and AI consumers.
 */
export function getCompactGuidanceText(text: string): string {
  return text.replace(COMPACT_GUIDANCE_PREFIX, '').trim();
}

/** Keep Simple supplement rows readable while preserving the authored plan. */
export function getCompactSupplementName(name: string): string {
  const normalized = name.replace(/\s+/g, ' ').trim();
  if (/bone-health product/i.test(normalized)) return 'Bone-health supplement review';
  if (/\bIBD marker\b/i.test(normalized)) return 'IBD supplement review';

  const label = normalized
    .replace(/^(?:a|an|the)\s+/i, '')
    .replace(/^(?:review|consider|use|favor|avoid|do not use)\s+/i, '')
    .split(/\s+(?:only after|after reviewing|after|before|without|against|with)\b/i)[0]
    .replace(/[,;:.]+$/, '')
    .trim();

  return label && label.length <= 72 ? label : 'Targeted supplement review';
}

/** Remove repeated attribution and review framing from Simple supplement rows. */
export function getCompactSupplementReason(reason: string): string {
  return reason
    .replace(/Based on your [^;]+;\s*/gi, '')
    .replace(/Discuss with a clinician or pharmacist before starting:\s*/gi, '')
    .replace(/,?\s*(?:and\s+)?clinician or pharmacist guidance\b/gi, '')
    .replace(/\s+with a clinician or pharmacist\b/gi, '')
    .replace(/\s{2,}/g, ' ')
    .replace(/\s+([,.;])/g, '$1')
    .trim();
}

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

  return compact.join(' ') || getSimpleFindingTitle(translation.simpleImpact);
}

/** Keep the primary Simple action consistent between cards and the dashboard queue. */
export function getSimpleNextStep(
  marker: Pick<
    EvaluatedMarker,
    'clinical_confirmation_required' | 'severity_class' | 'confirm_with' | 'effect_direction'
  >,
): string {
  if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
    return 'Consider clinical confirmation.';
  }
  if (marker.confirm_with.length > 0) {
    return 'Review the suggested follow-up.';
  }
  if (marker.severity_class === 'high_risk' || marker.severity_class === 'moderate_risk' || marker.severity_class === 'low_risk' || marker.effect_direction === 'risk') {
    return 'Compare with symptoms, history, and relevant labs.';
  }
  switch (marker.effect_direction) {
    case 'protective':
      return 'Use this as background context with your health history.';
    case 'context_dependent':
      return 'Consider diet, medications, and lifestyle context.';
    case 'trait':
      return 'Compare this with your lived experience.';
    default:
      return 'Review the details for personal relevance.';
  }
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
