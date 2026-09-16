/**
 * Report status compatibility helpers.
 *
 * Rust serializes report enums as snake_case. The frontend historically used
 * PascalCase values in its internal model and fixtures. Keep that internal
 * contract stable while normalizing both forms at the report boundary. This
 * also makes status-sensitive selectors safe for older cached payloads and
 * direct test fixtures.
 */

import type { AssertionStatus, CallStatus, NormalizedReport } from '../types/genomics';

const CALL_STATUS_MAP: Record<string, CallStatus> = {
  Found: 'Found',
  found: 'Found',
  NoData: 'NoData',
  no_data: 'NoData',
  NotInRawFile: 'NotInRawFile',
  not_in_raw_file: 'NotInRawFile',
  AmbiguousRawCall: 'AmbiguousRawCall',
  ambiguous_raw_call: 'AmbiguousRawCall',
};

const ASSERTION_STATUS_MAP: Record<string, AssertionStatus> = {
  Verified: 'Verified',
  verified: 'Verified',
  NoData: 'NoData',
  no_data: 'NoData',
  NotInRawFile: 'NotInRawFile',
  not_in_raw_file: 'NotInRawFile',
  BlockedRawCall: 'BlockedRawCall',
  blocked_raw_call: 'BlockedRawCall',
  UnverifiedOrientation: 'UnverifiedOrientation',
  unverified_orientation: 'UnverifiedOrientation',
  OrientationMismatch: 'OrientationMismatch',
  orientation_mismatch: 'OrientationMismatch',
  AmbiguousAlleles: 'AmbiguousAlleles',
  ambiguous_alleles: 'AmbiguousAlleles',
  NotEvaluated: 'NotEvaluated',
  not_evaluated: 'NotEvaluated',
};

export function normalizeCallStatus(value: unknown): CallStatus {
  return CALL_STATUS_MAP[String(value ?? '')] || 'NoData';
}

export function normalizeAssertionStatus(value: unknown): AssertionStatus {
  return ASSERTION_STATUS_MAP[String(value ?? '')] || 'NotEvaluated';
}

export function isVerifiedAssertionStatus(value: unknown): boolean {
  return normalizeAssertionStatus(value) === 'Verified';
}

/** Normalize enum fields once at the Rust-to-Svelte report boundary. */
export function normalizeReportStatuses(report: NormalizedReport): NormalizedReport {
  return {
    ...report,
    user_calls: Object.fromEntries(
      Object.entries(report.user_calls || {}).map(([rsid, call]) => [
        rsid,
        {
          ...call,
          call_status: normalizeCallStatus(call.call_status),
        },
      ]),
    ),
    category_links: Object.fromEntries(
      Object.entries(report.category_links || {}).map(([linkId, link]) => [
        linkId,
        {
          ...link,
          assertion_status: normalizeAssertionStatus(link.assertion_status),
        },
      ]),
    ),
  };
}
