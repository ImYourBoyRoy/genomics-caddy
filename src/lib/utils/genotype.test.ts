import { describe, expect, it } from 'vitest';
import { isCallableGenotype, isVariantDetected, normalizeGenotype } from './genotype';
import type { EvaluatedMarker } from '../types/genomics';

describe('genotype call-state normalization', () => {
  it('accepts complete nucleotide calls and normalizes whitespace/case', () => {
    expect(isCallableGenotype(' ag ')).toBe(true);
    expect(normalizeGenotype(' ag ')).toBe('AG');
  });

  it('rejects no-call and unknown-base representations as callable data', () => {
    for (const value of ['', '--', 'A-', 'A?', '00', 'NN', 'AN']) {
      expect(isCallableGenotype(value), value).toBe(false);
      expect(normalizeGenotype(value), value).toBe('--');
    }
  });

  it('keeps descriptive synthetic fixtures compatible with report utilities', () => {
    expect(isCallableGenotype('SYNTHETIC_CALL')).toBe(true);
    expect(normalizeGenotype('SYNTHETIC_CALL')).toBe('SYNTHETIC_CALL');
  });

  it('does not treat blocked or non-evaluated assertions as detected variants', () => {
    const marker = {
      user_genotype: 'AA',
      effect_allele: 'A',
      assertion_status: 'NotEvaluated',
      interpretation_allowed: false,
    } as unknown as EvaluatedMarker;
    expect(isVariantDetected(marker)).toBe(false);
  });
});
