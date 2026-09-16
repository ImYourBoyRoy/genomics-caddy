import { describe, expect, it } from 'vitest';
import {
  normalizeAssertionStatus,
  normalizeCallStatus,
  normalizeReportStatuses,
} from './reportStatuses';
import type { NormalizedReport } from '../types/genomics';

describe('report status normalization', () => {
  it.each([
    ['found', 'Found'],
    ['no_data', 'NoData'],
    ['not_in_raw_file', 'NotInRawFile'],
    ['ambiguous_raw_call', 'AmbiguousRawCall'],
  ])('maps Rust call status %s to the frontend value', (wire, expected) => {
    expect(normalizeCallStatus(wire)).toBe(expected);
  });

  it.each([
    ['verified', 'Verified'],
    ['no_data', 'NoData'],
    ['not_in_raw_file', 'NotInRawFile'],
    ['blocked_raw_call', 'BlockedRawCall'],
    ['unverified_orientation', 'UnverifiedOrientation'],
    ['orientation_mismatch', 'OrientationMismatch'],
    ['ambiguous_alleles', 'AmbiguousAlleles'],
    ['not_evaluated', 'NotEvaluated'],
  ])('maps Rust assertion status %s to the frontend value', (wire, expected) => {
    expect(normalizeAssertionStatus(wire)).toBe(expected);
  });

  it('uses conservative non-call defaults for unknown or absent statuses', () => {
    expect(normalizeCallStatus(undefined)).toBe('NoData');
    expect(normalizeCallStatus('unexpected')).toBe('NoData');
    expect(normalizeAssertionStatus(undefined)).toBe('NotEvaluated');
    expect(normalizeAssertionStatus('unexpected')).toBe('NotEvaluated');
  });

  it('normalizes both report maps without mutating the wire payload', () => {
    const wireReport = {
      user_calls: {
        rs1: { user_genotype: 'AA', call_status: 'found' },
      },
      category_links: {
        link1: { assertion_status: 'verified' },
      },
    } as unknown as NormalizedReport;

    const normalized = normalizeReportStatuses(wireReport);

    expect(normalized.user_calls.rs1.call_status).toBe('Found');
    expect(normalized.category_links.link1.assertion_status).toBe('Verified');
    expect((wireReport.user_calls.rs1 as unknown as { call_status: string }).call_status).toBe('found');
    expect((wireReport.category_links.link1 as unknown as { assertion_status: string }).assertion_status).toBe('verified');
  });
});
