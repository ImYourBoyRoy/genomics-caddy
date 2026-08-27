import { describe, expect, it } from 'vitest';
import { normalizeUiLabel } from './uiLabels';

describe('UI label normalization', () => {
  it('treats emoji and punctuation as presentation details', () => {
    expect(normalizeUiLabel('🏥 Clinical')).toBe('clinical');
    expect(normalizeUiLabel('Data & updates')).toBe('data updates');
  });

  it('keeps labels deterministic across whitespace and case changes', () => {
    expect(normalizeUiLabel('  Highest   Priority FIRST  ')).toBe('highest priority first');
  });
});
