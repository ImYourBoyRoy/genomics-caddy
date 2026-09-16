import { describe, expect, it } from 'vitest';
import {
  callabilityStateForResult,
  callabilityExplanation,
  callabilityStateLabel,
  normalizeCallabilityState,
  orientationStateForResult,
  orientationStateLabel,
} from './callability';

describe('callability state', () => {
  it('distinguishes evaluator capability from missing raw data', () => {
    expect(callabilityStateForResult('snp', 'Verified')).toBe('callable');
    expect(callabilityStateForResult('snp', 'NotInRawFile')).toBe('not_present');
    expect(callabilityStateForResult('hla_tag', 'NotInRawFile')).toBe('not_callable');
  });

  it('marks quality and orientation gates as blocked', () => {
    expect(callabilityStateForResult('snp', 'OrientationMismatch')).toBe('blocked');
    expect(callabilityStateForResult('snp', 'blocked_raw_call')).toBe('blocked');
    expect(callabilityStateForResult('snp', 'NotEvaluated')).toBe('not_callable');
  });

  it('normalizes only the published wire states and gives human-readable labels', () => {
    expect(normalizeCallabilityState('NOT_CALLABLE')).toBe('not_callable');
    expect(normalizeCallabilityState('unexpected')).toBeUndefined();
    expect(callabilityStateLabel('not_callable')).toBe('Not callable from this DNA representation');
    expect(callabilityStateLabel(undefined)).toBe('Callability not established');
    expect(callabilityExplanation('not_callable')).toContain('different assay or a multi-marker model');
    expect(callabilityExplanation('not_present')).toContain('No usable call');
  });

  it('keeps orientation outcome orthogonal to assertion and callability status', () => {
    expect(orientationStateForResult('Verified', false)).toBe('not_required');
    expect(orientationStateForResult('Verified', true)).toBe('verified');
    expect(orientationStateForResult('UnverifiedOrientation', true)).toBe('unverified');
    expect(orientationStateForResult('UnverifiedOrientation', true, false)).toBe('unverified');
    expect(orientationStateForResult('OrientationMismatch', true)).toBe('mismatch');
    expect(orientationStateLabel('mismatch')).toBe('Orientation mismatch');
  });
});
