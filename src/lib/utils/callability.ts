import callabilityRules from '../marker-packs/callability_rules.json';
import type {
  AssertionStatus,
  CallabilityState,
  ClinicalSemantics,
  MarkerSource,
  OrientationState,
} from '../types/genomics';
import { reportReferenceId } from './reportReferences';

export type CallabilityPolicy = typeof callabilityRules.variant_type_registry[keyof typeof callabilityRules.variant_type_registry];

const registry = callabilityRules.variant_type_registry as Record<string, CallabilityPolicy>;

/**
 * Resolve the resource-authored policy used to explain whether an assertion
 * can be scored from the current raw-DNA representation.
 *
 * Curated marker packs classify every row. The `snp` fallback preserves the
 * existing programmatic report-template contract when a caller omits the
 * optional variant_type field; unknown authored values use the explicit
 * `unspecified` policy and are never treated as scoreable.
 */
export function callabilityPolicyForVariantType(variantType?: string | null): CallabilityPolicy {
  const normalized = String(variantType || '').trim().toLowerCase();
  if (!normalized) return registry.snp;
  return registry[normalized] || registry.unspecified;
}

const CALLABILITY_STATE_MAP: Record<string, CallabilityState> = {
  callable: 'callable',
  not_callable: 'not_callable',
  not_present: 'not_present',
  blocked: 'blocked',
  unknown: 'unknown',
};

export function normalizeCallabilityState(value: unknown): CallabilityState | undefined {
  const normalized = String(value ?? '').trim().toLowerCase();
  return CALLABILITY_STATE_MAP[normalized];
}

/**
 * Compatibility fallback for reports generated before callability_state was
 * added. Policy determines whether the evaluator supports the assertion;
 * assertion status then explains missing calls and quality/orientation gates.
 */
export function callabilityStateForResult(
  variantType: string | null | undefined,
  assertionStatus: AssertionStatus | string | null | undefined,
): CallabilityState {
  if (callabilityPolicyForVariantType(variantType).scoring_policy !== 'snp_allele_count') {
    return 'not_callable';
  }

  const status = String(assertionStatus ?? '').trim().toLowerCase();
  if (status === 'not_evaluated' || status === 'notevaluated') return 'not_callable';
  if (status === 'no_data' || status === 'notinrawfile' || status === 'not_in_raw_file') return 'not_present';
  if (
    status === 'blockedrawcall'
    || status === 'blocked_raw_call'
    || status === 'unverifiedorientation'
    || status === 'unverified_orientation'
    || status === 'orientationmismatch'
    || status === 'orientation_mismatch'
    || status === 'ambiguousalleles'
    || status === 'ambiguous_alleles'
  ) return 'blocked';
  if (status === 'verified') return 'callable';
  return 'unknown';
}

export function callabilityStateLabel(state: CallabilityState | null | undefined): string {
  switch (state) {
    case 'callable': return 'Callable by this evaluator';
    case 'not_callable': return 'Not callable from this DNA representation';
    case 'not_present': return 'No usable call present';
    case 'blocked': return 'Blocked pending call or orientation review';
    default: return 'Callability not established';
  }
}

/** A concise, genotype-free explanation suitable for summary surfaces. */
export function callabilityExplanation(state: CallabilityState | null | undefined): string {
  switch (state) {
    case 'callable': return 'This assertion can be evaluated by the current DNA evaluator.';
    case 'not_callable': return 'This assertion needs a different assay or a multi-marker model, so it was not scored here.';
    case 'not_present': return 'No usable call for this supported assertion was present in the imported file.';
    case 'blocked': return 'Evaluation is paused until the call or allele orientation is verified.';
    default: return 'The evaluator could not establish whether this assertion can be scored.';
  }
}

export function orientationStateForResult(
  assertionStatus: AssertionStatus | string | null | undefined,
  requiresOrientationVerification: boolean | null | undefined,
  markerOrientationVerified?: boolean | null,
): OrientationState {
  const status = String(assertionStatus ?? '').trim().toLowerCase();
  if (status === 'orientationmismatch' || status === 'orientation_mismatch') return 'mismatch';
  if (status === 'unverifiedorientation' || status === 'unverified_orientation') return 'unverified';
  if (status === 'verified') {
    return requiresOrientationVerification || markerOrientationVerified === true
      ? 'verified'
      : 'not_required';
  }
  return requiresOrientationVerification ? 'unknown' : 'not_required';
}

export function orientationStateLabel(state: OrientationState | null | undefined): string {
  switch (state) {
    case 'verified': return 'Verified orientation';
    case 'not_required': return 'Orientation check not required';
    case 'unverified': return 'Orientation not verified';
    case 'mismatch': return 'Orientation mismatch';
    default: return 'Orientation unknown';
  }
}

export interface AssertionIdentityInput {
  rsid: string;
  gene?: string | null;
  variant_name?: string | null;
  variant_type?: string | null;
  source_build?: string | null;
  hgvs?: string | null;
  expected_plus_alleles?: string[] | null;
  effect_allele?: string | null;
  category_id?: string | null;
  sex_scope?: string | null;
  clinical_semantics?: ClinicalSemantics | null;
  reference_ids?: string[] | null;
  sources?: MarkerSource[] | null;
}

/**
 * Stable, inspectable biomedical assertion identity. This is deliberately
 * separate from rsID-based Simple-mode grouping and excludes user genotype
 * values, enrichment, and evaluated severity.
 */
export function buildAssertionKey(input: AssertionIdentityInput): string {
  const sourceAssertion = input.reference_ids?.length
    ? input.reference_ids
    : (input.sources || []).map((source) => reportReferenceId(source));
  const semantics = input.clinical_semantics || {};
  return JSON.stringify({
    version: 1,
    locus: {
      rsid: String(input.rsid || '').trim(),
      gene: String(input.gene || '').trim(),
      source_build: input.source_build || null,
      hgvs: input.hgvs || null,
    },
    allele_definition: {
      variant_type: String(input.variant_type || 'snp').trim().toLowerCase() || 'snp',
      effect_allele: String(input.effect_allele || '').trim(),
      expected_plus_alleles: [...new Set(input.expected_plus_alleles || [])].sort(),
    },
    condition_or_trait: {
      category_id: input.category_id || null,
      label: semantics.condition_label || input.variant_name || null,
      interpretation_class: semantics.interpretation_class || null,
      inheritance_model: semantics.inheritance_model || null,
      clinical_state: semantics.clinical_state || null,
    },
    population_context: {
      sex_scope: input.sex_scope || null,
    },
    assay_requirement: callabilityPolicyForVariantType(input.variant_type).assay_requirement,
    source_assertion: [...new Set(sourceAssertion)].sort(),
  });
}
