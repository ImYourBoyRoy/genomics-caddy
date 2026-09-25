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

  it('keeps chromosome-pattern results concise while preserving inconclusive states', () => {
    expect(formatGeneticSexLabel('Female-like (XX chromosome pattern; no Y calls observed)')).toBe('Female-like');
    expect(formatGeneticSexLabel('Female')).toBe('Female-like');
    expect(formatGeneticSexLabel('Male')).toBe('Male-like');
    expect(formatGeneticSexLabel('Unknown (Y chromosome not observed)')).toBe('Unknown');
    expect(formatGeneticSexLabel('Uncertain (limited sex-chromosome calls)')).toBe('Inconclusive');
    expect(formatGeneticSexLabel('Inconclusive (mixed X/Y chromosome calls)')).toBe('Inconclusive');
    expect(formatGeneticSexLabel('XX chromosome pattern; no Y calls observed')).toBe('Unknown');
  });
});
