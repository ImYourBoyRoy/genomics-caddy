/**
 * Presentation policy for caveats carried by genomic resources.
 *
 * Resource fields remain complete for clinical and AI exports. This module
 * decides where those fields belong in the product so ordinary research
 * findings do not become a repeated warning wall.
 */

export type WarningKind =
  | 'actionable_safety'
  | 'clinical_review'
  | 'research_context'
  | 'general_interpretation'
  | 'operational_status'
  | 'legal_privacy';

export type WarningAudience = 'simple' | 'clinical' | 'ai' | 'footer' | 'legal';

export interface ClassifiedWarning {
  text: string;
  kind: WarningKind;
  dedupeKey: string;
}
const ACTIONABLE_PATTERNS = [
  /hypersensitivity/i,
  /severe skin reaction/i,
  /anaphylaxis/i,
  /urgent(?:ly)?/i,
  /seek (?:urgent|immediate) care/i,
  /avoid\s+(?:abacavir|allopurinol|carbamazepine|oxcarbazepine|phenytoin)/i,
];

const CLINICAL_REVIEW_PATTERNS = [
  /clinical (?:review|testing|confirmation|laboratory|pgx)/i,
  /confirm(?:atory| with)?/i,
  /clinician|pharmacist|genetic counselor|specialist/i,
  /INR|HbA1c|ApoB|Lp\(a\)|ferritin|transferrin|CBC|TSH|MMA/i,
  /imaging|diplotype|HLA typing|copy-number|enzyme activity/i,
];

const LEGAL_PRIVACY_PATTERNS = [
  /privacy|sensitive health|share it only|terms|legal/i,
  /generated locally|local data|export responsibility/i,
];

const OPERATIONAL_PATTERNS = [
  /download|resource|catalog|import|update|reload|database|sync|available/i,
];

const GENERAL_INTERPRETATION_PATTERNS = [
  /not (?:a )?diagnos(?:is|tic)|does not diagnose|cannot diagnose/i,
  /raw DNA|consumer array|genotype alone|does not establish|not a prescription/i,
  /association|probabilistic|context[- ]dependent|does not predict/i,
];

function normalize(text: string): string {
  return text.toLowerCase().replace(/\s+/g, ' ').trim().replace(/[.!?]+$/, '');
}

/** Classify one source string from the most concrete route to the broadest. */
export function classifyWarning(text: string): WarningKind {
  const value = text.trim();
  if (!value) return 'research_context';
  if (ACTIONABLE_PATTERNS.some((pattern) => pattern.test(value))) return 'actionable_safety';
  if (LEGAL_PRIVACY_PATTERNS.some((pattern) => pattern.test(value))) return 'legal_privacy';
  if (OPERATIONAL_PATTERNS.some((pattern) => pattern.test(value))) return 'operational_status';
  if (CLINICAL_REVIEW_PATTERNS.some((pattern) => pattern.test(value))) return 'clinical_review';
  if (GENERAL_INTERPRETATION_PATTERNS.some((pattern) => pattern.test(value))) return 'general_interpretation';
  return 'research_context';
}

export function classifyWarnings(texts: ReadonlyArray<string>): ClassifiedWarning[] {
  return texts
    .map((text) => text.trim())
    .filter(Boolean)
    .map((text) => ({ text, kind: classifyWarning(text), dedupeKey: normalize(text) }));
}

export function dedupeWarnings(warnings: ReadonlyArray<ClassifiedWarning>): ClassifiedWarning[] {
  const seen = new Set<string>();
  return warnings.filter((warning) => {
    if (seen.has(warning.dedupeKey)) return false;
    seen.add(warning.dedupeKey);
    return true;
  });
}

/**
 * Simple shows only concrete safety/review routes. The shared footer and guide
 * carry the one-time interpretation reminder; they do not repeat per finding.
 */
export function isWarningVisibleInAudience(kind: WarningKind, audience: WarningAudience): boolean {
  if (audience === 'simple') return kind === 'actionable_safety' || kind === 'clinical_review';
  if (audience === 'footer') return kind === 'general_interpretation';
  if (audience === 'legal') return kind === 'legal_privacy';
  if (audience === 'clinical' || audience === 'ai') {
    return kind !== 'operational_status' && kind !== 'legal_privacy';
  }
  return false;
}

export function selectWarningsForAudience(
  texts: ReadonlyArray<string>,
  audience: WarningAudience,
): ClassifiedWarning[] {
  return dedupeWarnings(classifyWarnings(texts)).filter((warning) =>
    isWarningVisibleInAudience(warning.kind, audience),
  );
}
