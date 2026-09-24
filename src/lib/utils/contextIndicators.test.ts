import { describe, expect, it } from 'vitest';
import { contextLabels, getContextIndicators } from './contextIndicators';

describe('context indicators', () => {
  it('keeps explicit life-stage tags distinct and ordered', () => {
    expect(contextLabels(['pregnancy', 'postpartum', 'lactation'])).toEqual([
      'Pregnancy-related',
      'Postpartum-related',
      'Lactation-related',
    ]);
  });

  it('uses explicit tags instead of broad legacy scope fallbacks', () => {
    expect(getContextIndicators(['postpartum'], 'all_bodies_pregnancy_lactation_context').map((item) => item.id))
      .toEqual(['postpartum']);
  });

  it('keeps older scoped packs visible without requiring a pack migration', () => {
    expect(contextLabels(undefined, 'all_bodies_pregnancy_lactation_context')).toEqual([
      'Pregnancy-related',
      'Lactation-related',
    ]);
    expect(contextLabels(undefined, 'menstrual_cycle_context')).toEqual(['Menstrual-cycle-related']);
  });

  it('does not create a context indicator for an all-users marker', () => {
    expect(getContextIndicators([], 'all')).toEqual([]);
    expect(getContextIndicators()).toEqual([]);
  });

  it('preserves unknown biological scope as a safe fallback label', () => {
    const [indicator] = getContextIndicators(undefined, 'custom_biological_scope');

    expect(indicator.id).toBe('scope:custom_biological_scope');
    expect(indicator.group).toBe('Biological scope');
    expect(indicator.description).toContain('does not establish');
  });
});
