import { describe, expect, it } from 'vitest';
import { formatGeneticSexLabel, normalizeUiLabel } from './uiLabels';

describe('UI label normalization', () => {
  it('treats emoji and punctuation as presentation details', () => {
    expect(normalizeUiLabel('🏥 Clinical')).toBe('clinical');
    expect(normalizeUiLabel('Data & updates')).toBe('data updates');
  });

  it('keeps labels deterministic across whitespace and case changes', () => {
    expect(normalizeUiLabel('  Highest   Priority FIRST  ')).toBe('highest priority first');
  });

  it('keeps primary sex results concise while preserving unknown states', () => {
    expect(formatGeneticSexLabel('Female-like (XX chromosome pattern; no Y calls observed)')).toBe('Female');
    expect(formatGeneticSexLabel('Male')).toBe('Male');
    expect(formatGeneticSexLabel('Unknown (Y chromosome not observed)')).toBe('Unknown');
    expect(formatGeneticSexLabel('Uncertain (limited Y chromosome calls)')).toBe('Uncertain');
  });
});
