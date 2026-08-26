import { describe, expect, it } from 'vitest';
import { LAYPERSON_MAP } from './layperson';

describe('plain-English claim framing', () => {
  it('does not reintroduce deterministic or diagnosis-like wording', () => {
    const deterministicPatterns = [
      /\bdetermines\b/i,
      /\bstrongest\b/i,
      /\bcan cause\b/i,
      /\bcausing\b/i,
      /\bproduces? less\b/i,
      /\b(reduces?|lowers?|slows?) (the|your|a|an)\b/i,
      /\b(makes|make) (you|your|it)\b/i,
      /\bleads? to\b/i,
      /\bprotects? you\b/i,
    ];

    for (const [rsid, translation] of Object.entries(LAYPERSON_MAP)) {
      for (const pattern of deterministicPatterns) {
        expect(`${rsid}: ${translation.simpleMeaning}`).not.toMatch(pattern);
      }
    }
  });

  it('frames hormone translations as research context rather than current hormone measurement', () => {
    expect(LAYPERSON_MAP.rs1801132.simpleMeaning).toContain('does not measure current estrogen');
    expect(LAYPERSON_MAP.rs2234693.simpleMeaning).toContain('does not measure estrogen');
    expect(LAYPERSON_MAP.rs8079626.simpleMeaning).toContain('does not measure progesterone');
  });
});
