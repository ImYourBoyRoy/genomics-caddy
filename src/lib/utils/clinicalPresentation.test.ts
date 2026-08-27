import { describe, expect, it } from 'vitest';
import { formatClinicalFollowUp } from './clinicalPresentation';

describe('clinical presentation helpers', () => {
  it('omits empty authored follow-up labels', () => {
    expect(formatClinicalFollowUp(['  ', '', '  lab review  '])).toBe('lab review');
  });

  it('keeps up to three authored actions in their source order', () => {
    expect(formatClinicalFollowUp(['one', 'two', 'three'])).toBe('one; two; three');
  });

  it('summarizes longer action lists without losing their count', () => {
    expect(formatClinicalFollowUp(['one', 'two', 'three', 'four', 'five'])).toBe('one; two; three (+2 more)');
  });

  it('returns an empty value when no usable label exists', () => {
    expect(formatClinicalFollowUp([])).toBe('');
    expect(formatClinicalFollowUp([' ', '\t'])).toBe('');
  });
});
