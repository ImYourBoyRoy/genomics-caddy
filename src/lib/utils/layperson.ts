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
  /** Optional action written for the first-screen Simple queue. */
  simpleNextStep?: string;
  /** Optional structured fields for the Simple finding contract. */
  plainTitle?: string;
  signal?: string;
  whyItMatters?: string;
  reviewAction?: string;
  evidenceLabel?: string;
  /** True when the safe generic fallback is being used instead of authored copy. */
  isFallback?: boolean;
}

interface LaypersonTranslationGroup {
  id: string;
  rsids: string[];
  simpleImpact: string;
  simpleMeaning: string;
  simpleNextStep?: string;
  plainTitle?: string;
  signal?: string;
  whyItMatters?: string;
  reviewAction?: string;
  evidenceLabel?: string;
}

interface LaypersonTranslationResourceEntry extends LaypersonTranslationGroup {
  rsid: string;
}

function buildLaypersonMap(): Record<string, LaypersonTranslation> {
  const map: Record<string, LaypersonTranslation> = {};
  for (const translation of laypersonTranslations.translations as LaypersonTranslationResourceEntry[]) {
    map[translation.rsid.trim().toLowerCase()] = {
      simpleImpact: translation.simpleImpact,
      simpleMeaning: translation.simpleMeaning,
      ...(translation.simpleNextStep ? { simpleNextStep: translation.simpleNextStep } : {}),
      ...(translation.plainTitle ? { plainTitle: translation.plainTitle } : {}),
      ...(translation.signal ? { signal: translation.signal } : {}),
      ...(translation.whyItMatters ? { whyItMatters: translation.whyItMatters } : {}),
      ...(translation.reviewAction ? { reviewAction: translation.reviewAction } : {}),
      ...(translation.evidenceLabel ? { evidenceLabel: translation.evidenceLabel } : {}),
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
          ...(group.simpleNextStep ? { simpleNextStep: group.simpleNextStep } : {}),
          ...(group.plainTitle ? { plainTitle: group.plainTitle } : {}),
          ...(group.signal ? { signal: group.signal } : {}),
          ...(group.whyItMatters ? { whyItMatters: group.whyItMatters } : {}),
          ...(group.reviewAction ? { reviewAction: group.reviewAction } : {}),
          ...(group.evidenceLabel ? { evidenceLabel: group.evidenceLabel } : {}),
        };
      }
    }
  }
  return map;
}

export const LAYPERSON_MAP = buildLaypersonMap();

export const DEFAULT_LAYPERSON_TRANSLATION: LaypersonTranslation = laypersonTranslations.fallback;

const GENERIC_TITLE_PREFIXES = new Set(['DNA', 'RNA', 'SNP']);

function humanizeSimpleTitle(title: string): string {
  const normalized = title
    .replace(/-association\b/gi, ' research')
    .replace(/\bresearch-only marker\b/gi, 'research-only finding')
    .replace(/\bcontext marker\b/gi, 'context')
    .replace(/\b(?:association|research) marker\b/gi, 'research context')
    .replace(/\bmarker\b/gi, 'research context')
    .replace(/\bresearch\s+research\b/gi, 'research')
    .replace(/[-_]+/g, ' ')
    .replace(/\s{2,}/g, ' ')
    .trim();

  return normalized ? `${normalized[0].toUpperCase()}${normalized.slice(1)}` : normalized;
}

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

  // Star-allele, haplotype, and HLA component names are useful in Clinical
  // mode but are not meaningful first-screen labels. Keep the Simple queue
  // focused on the medication question while the exact allele remains in
  // Technical data.
  if (/(?:allele|haplotype)\s+component|clinical\s+pgx|\bHLA[-*]|oxidative-medication-safety/i.test(withoutGeneSuffix)) {
    if (/warfarin/i.test(withoutGeneSuffix)) return 'Warfarin response context';
    if (/statin|SLCO1B1/i.test(withoutGeneSuffix)) return 'Statin medication context';
    if (/DPYD|fluoropyrimidine|chemotherapy/i.test(withoutGeneSuffix)) return 'Chemotherapy medication context';
    if (/TPMT|NUDT15|thiopurine/i.test(withoutGeneSuffix)) return 'Thiopurine medication context';
    if (/HLA|hypersensitivity|safety/i.test(withoutGeneSuffix)) return 'Medication safety context';
    return 'Medication processing context';
  }

  const leadingTechnical = withoutGeneSuffix.match(
    /^((?:[A-Z]{2,}[A-Z0-9]*(?:\*[A-Z0-9]+)?(?:\/(?:[A-Z]{2,}[A-Z0-9]*(?:\*[A-Z0-9]+)?)|\/(?=[a-z]))?|[0-9]+p[0-9.]+))\s*(.+)$/,
  );

  if (!leadingTechnical || GENERIC_TITLE_PREFIXES.has(leadingTechnical[1])) {
    return humanizeSimpleTitle(withoutGeneSuffix);
  }

  return humanizeSimpleTitle(leadingTechnical[2].trim() || withoutGeneSuffix);
}

export interface SimpleFindingCopy {
  plain_title: string;
  signal: string;
  why_it_matters: string;
  review_action: string;
  evidence_label: string;
  is_fallback: boolean;
}

function firstMeaningSentence(text: string): string {
  return text.split(/(?<=[.!?])\s+/)[0]?.trim() || text.trim();
}

function meaningAfterFirstSentence(text: string): string {
  const first = firstMeaningSentence(text);
  return text.slice(first.length).trim();
}

/** Keep evidence understandable without repeating the full evidence policy. */
export function getSimpleEvidenceLabel(evidenceTier: string): string {
  const tier = evidenceTier.trim().toUpperCase();
  if (/PHARMACOGENOMIC|DRUG|CLINICAL|HIGH.?STAKE|TIER\s*A\b/.test(tier)) return 'Clinical or medication evidence';
  if (/TIER\s*B\b|REPLICAT|STRONG/.test(tier)) return 'Replicated research signal';
  if (/TIER\s*C\b|MODERATE/.test(tier)) return 'Moderate research signal';
  if (/TIER\s*[DE]\b|LIMITED|EXPLOR/.test(tier)) return 'Early or limited research';
  return 'Research context';
}

/**
 * Normalize legacy two-field translations into the Simple-mode contract.
 * Authored structured fields win; the legacy meaning remains the source for
 * the useful signal when a resource has not migrated yet.
 */
export function getSimpleFindingCopy(
  marker: Pick<
    EvaluatedMarker,
    'evidence_tier' | 'effect_direction' | 'clinical_confirmation_required' | 'severity_class' | 'confirm_with'
  >,
  translation: LaypersonTranslation,
): SimpleFindingCopy {
  const compactMeaning = getCompactSimpleMeaning(translation);
  const sentence = firstMeaningSentence(compactMeaning);
  const remainder = meaningAfterFirstSentence(compactMeaning);
  const plainTitle = translation.plainTitle?.trim() || getSimpleFindingTitle(translation.simpleImpact);
  const signal = translation.signal?.trim() || sentence || plainTitle;
  const whyItMatters = translation.whyItMatters?.trim()
    || remainder
    || (signal === plainTitle
      ? 'A useful context signal to compare with the related health measure, symptom pattern, or goal.'
      : signal);
  const reviewAction = translation.reviewAction?.trim() || getSimpleNextStep(marker, translation);

  return {
    plain_title: plainTitle,
    signal,
    why_it_matters: whyItMatters,
    review_action: reviewAction,
    evidence_label: translation.evidenceLabel?.trim() || getSimpleEvidenceLabel(marker.evidence_tier),
    is_fallback: translation.isFallback === true,
  };
}

const GENERIC_GUARDRAIL_PATTERNS = [
  /\bdoes not (?:diagnose|prove)\b/i,
  /\bdoes not predict whether you have (?:a )?condition\b/i,
  /\bnot (?:a|an) (?:diagnosis|treatment|prescription)\b/i,
  /\b(?:requires|needs?) (?:a|an|the)?\s*(?:doctor|clinician|clinical|medical-grade|healthcare)\b.*\bbefore\b/i,
  /\bbefore making (?:health|medical) decisions\b/i,
];

const COMPACT_GUIDANCE_PREFIX = /(?:Do not make this change from raw DNA; confirm the finding clinically first:|Only consider after clinical confirmation and individualized advice:|Do not make this change unless symptoms, labs, or clinician guidance support it:|Consider only if symptoms, labs, or personal goals support it:|General health consideration, not a genotype-specific restriction:|General low-risk option, not a genotype prescription:)\s*/gi;

const SIMPLE_GUIDANCE_REWRITES: Array<[RegExp, string]> = [
  [/^Iron-containing foods.*$/i, 'Iron-rich foods, if they fit your diet'],
  [/^B12-containing foods.*$/i, 'B12-rich or fortified foods'],
  [/^Use measured B12 status.*$/i, 'Check B12 status before considering a supplement'],
  [/^A balanced eating pattern built around fiber-rich foods.*$/i, 'Fiber-rich foods, adequate protein, and minimally processed carbs'],
  [/^Regular physical activity progressed gradually.*$/i, 'Regular activity, adjusted for ability, symptoms, and recovery'],
  [/^Use a heart-healthy eating pattern.*$/i, 'Heart-healthy eating and regular activity'],
  [/^Review measured Lp\(a\).*$/i, 'Review lipids, blood pressure, and family history together'],
  [/^An exposure and symptom log.*$/i, 'Track exposures, timing, symptoms, and co-factors'],
  [/^Adequate protein and calcium-rich.*$/i, 'Adequate protein, calcium-rich foods, and safe resistance activity'],
  [/^Track low-trauma fractures.*$/i, 'Review fracture history, bone pain, hormone context, and family history'],
  [/^Use a symptom, stool, food, and medication log.*$/i, 'Track digestive symptoms, bleeding, fever, weight, and timing'],
  [/^Maintain adequate hydration.*$/i, 'Stay hydrated; get guidance before restrictive diets'],
  [/^Treating an iron-status marker.*$/i, "Don't treat an iron marker as a diagnosis"],
  [/^Starting or avoiding iron supplements.*$/i, "Don't change iron supplements based on fatigue or one SNP"],
  [/^Treating a FUT2.*$/i, "Don't treat a B12 marker as proof of deficiency"],
  [/^Using folate or an energy formula.*$/i, "Don't mask B12 symptoms with folate or energy formulas"],
  [/^Treating a common metabolic SNP.*$/i, "Don't treat a metabolic SNP as diabetes"],
  [/^Using a highly restrictive diet.*$/i, "Don't use restrictive diets to chase a genotype"],
  [/^Assuming a protective-leaning LDL.*$/i, "Don't skip lipid monitoring"],
  [/^Using a genotype as a substitute.*$/i, "Don't replace measured lipids with genotype"],
  [/^High-dose iodine or selenium.*$/i, 'Avoid high-dose iodine or selenium without guidance'],
  [/^Permanent food elimination.*$/i, "Don't eliminate foods or do challenges based only on DNA"],
  [/^Using a bone SNP.*$/i, "Don't use a bone SNP to diagnose or prescribe treatment"],
  [/^High-dose calcium.*$/i, 'Avoid high-dose bone supplements or abrupt treatment changes'],
  [/^Using an IBD-associated SNP.*$/i, "Don't use an IBD SNP to diagnose or choose treatment"],
  [/^Long-term restrictive diets.*$/i, "Don't use restrictive diets, probiotics, enzymes, or detox products as DNA treatment"],
];

/**
 * Remove repeated actionability framing from secondary Simple guidance lists.
 * The section heading provides the shared context; authored plan data remains
 * unchanged for clinical and AI consumers.
 */
export function getCompactGuidanceText(text: string): string {
  const compact = text.replace(COMPACT_GUIDANCE_PREFIX, '').trim();
  return SIMPLE_GUIDANCE_REWRITES.find(([pattern]) => pattern.test(compact))?.[1] ?? compact;
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

function compactGuardrailSentence(sentence: string): string {
  const clauses = sentence
    .split(/;\s+/)
    .map((clause) => clause.trim())
    .filter(Boolean);
  const compactClauses = clauses
    .map((clause) => {
      if (!isGenericGuardrail(clause)) return clause;
      const boundary = clause.search(/\b(?:does not diagnose|cannot diagnose|does not prove|does not establish|is not a diagnosis|is not an? treatment|is not an? prescription)\b/i);
      // Keep a useful association clause when the generic boundary is only a
      // trailing clause. A sentence made entirely of a boundary is removed.
      if (boundary > 24) {
        return clause
          .slice(0, boundary)
          .replace(/(?:,\s*|;\s*|\s+)(?:but|and)\s+(?:it|this\s+(?:marker|result))\s*$/i, '')
          .replace(/[\s,;:]+$/, '')
          .trim();
      }
      return '';
    })
    .filter(Boolean);

  return compactClauses.join('; ').trim();
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
    .map(compactGuardrailSentence)
    .filter(Boolean);

  return compact.join(' ') || getSimpleFindingTitle(translation.simpleImpact);
}

/**
 * Keep the first-screen action queue to one useful sentence. The full authored
 * wording remains available on the finding card and in clinical/AI exports;
 * the queue only needs a short reason for why an item was surfaced.
 */
export function getCompactActionMeaning(translation: LaypersonTranslation): string {
  const compact = getCompactSimpleMeaning(translation);
  const firstSentence = compact.split(/(?<=[.!?])\s+/)[0]?.trim() || compact;
  const firstClause = firstSentence.split(/;\s+/)[0]?.trim() || firstSentence;
  return firstClause
    .replace(/^This\s+(?:(?:[A-Z][A-Z0-9]*(?:\/[A-Z][A-Z0-9]*)?)\s+)?(?:common\s+)?(?:marker|variant)\b/i, 'This finding')
    .replace(/^This\s+is\s+a\s+research-only\s+marker\b/i, 'This finding')
    .trim();
}

/**
 * Keep follow-up guidance at the section level in Simple mode. Individual
 * cards explain the signal; this one compact line gives the section a single
 * practical route without repeating the same review language on every card.
 */
export function getSimpleSectionFollowUp(
  markers: ReadonlyArray<Pick<EvaluatedMarker, 'confirm_with' | 'clinical_confirmation_required' | 'severity_class'>>,
): string | null {
  const followUps = Array.from(new Set(
    markers
      .flatMap((marker) => marker.confirm_with || [])
      .map((item) => item.replace(/[.;:]+$/, '').replace(/\s+/g, ' ').trim())
      .filter((item) => item.length > 3)
      .filter((item) => !/^(?:clinical|medical|doctor|clinician|healthcare|specialist)\s+(?:review|confirmation|testing|evaluation)$/i.test(item)),
  ));
  if (followUps.length === 0) return null;

  const visible = followUps.slice(0, 3);
  const summary = visible.length === 1
    ? visible[0]
    : `${visible.slice(0, -1).join(', ')}, or ${visible.at(-1)}`;
  const needsConfirmation = markers.some((marker) =>
    marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required'
  );
  return `${needsConfirmation ? 'Confirm' : 'Review'} ${summary}.`;
}

/** Keep the primary Simple action consistent between cards and the dashboard queue. */
function compactFollowUpSummary(items: string[]): string | null {
  const followUps = items
    .map((item) => item.replace(/[.;:]+$/, '').trim())
    .filter(Boolean);
  if (followUps.length === 0) return null;

  const visibleFollowUps = followUps.slice(0, 3);
  const maxLength = 116;
  const visible: string[] = [];
  for (const item of visibleFollowUps) {
    const candidate = visible.length === 0 ? item : `${visible.join(', ')}, or ${item}`;
    if (candidate.length <= maxLength) {
      visible.push(item);
      continue;
    }
    if (visible.length === 0) {
      const clipped = item.slice(0, maxLength - 1).replace(/\s+\S*$/, '').trim();
      visible.push(`${clipped || item.slice(0, maxLength - 1)}…`);
    }
    break;
  }

  const omittedCount = followUps.length - visible.length;
  const summary = visible.length === 1
    ? visible[0]
    : `${visible.slice(0, -1).join(', ')}, or ${visible.at(-1)}`;
  return omittedCount > 0 ? `${summary} (+${omittedCount} more in details)` : summary;
}

export function getSimpleNextStep(
  marker: Pick<
    EvaluatedMarker,
    'clinical_confirmation_required' | 'severity_class' | 'confirm_with' | 'effect_direction'
  >,
  translation?: Pick<LaypersonTranslation, 'simpleNextStep'>,
): string {
  if (translation?.simpleNextStep) return translation.simpleNextStep;
  if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
    const followUp = compactFollowUpSummary(marker.confirm_with);
    return followUp ? `Confirm with ${followUp}.` : 'Review the related clinical test route.';
  }
  if (marker.confirm_with.length > 0) {
    const followUp = compactFollowUpSummary(marker.confirm_with);
    if (followUp) return `Review ${followUp}.`;
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
  simpleImpact: "Genetic research signal",
  simpleMeaning:
    "This result relates to a biological pathway studied in research; compare it with the relevant health measure, symptoms, or goal.",
  signal: "A biological pathway signal is present.",
  whyItMatters: "Its value is in helping you choose the relevant health measure, symptom pattern, or goal to review.",
  reviewAction: "Review the related health context.",
  evidenceLabel: "Research context",
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
      reviewAction: 'Review the related clinical test route.',
      evidenceLabel: 'Clinical follow-up',
    };
  }

  return { ...SAFE_FALLBACK };
}
