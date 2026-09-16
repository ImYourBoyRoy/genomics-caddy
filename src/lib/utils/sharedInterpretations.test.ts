import { describe, expect, it } from 'vitest';
import { getSharedInterpretations, normalizeSharedInterpretation } from './sharedInterpretations';

describe('shared interpretation grouping', () => {
  it('normalizes whitespace and case for stable grouping', () => {
    expect(normalizeSharedInterpretation('  Shared  pathway\ncontext. ')).toBe('shared pathway context.');
  });

  it('returns repeated family context once with its occurrence count', () => {
    const result = getSharedInterpretations([
      { interpretation: 'Hormone pathway context.' },
      { interpretation: '  hormone pathway context. ' },
      { interpretation: 'Marker-specific result.' },
    ]);

    expect(result).toEqual([{
      key: 'hormone pathway context.',
      text: 'Hormone pathway context.',
      count: 2,
    }]);
  });

  it('does not discard unique or empty marker interpretations', () => {
    expect(getSharedInterpretations([
      { interpretation: '' },
      { interpretation: null },
      { interpretation: 'Unique context.' },
    ])).toEqual([]);
  });
});
