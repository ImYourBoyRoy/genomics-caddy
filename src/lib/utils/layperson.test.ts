import { describe, expect, it } from 'vitest';
import hormonePack from '../marker-packs/hormones_reproductive.json';
import { DEFAULT_LAYPERSON_TRANSLATION, LAYPERSON_MAP } from './layperson';

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

  it('keeps every numeric hormone-pack marker understandable in plain English', () => {
    const hormoneRsids = hormonePack.markers
      .map((marker) => marker.rsid)
      .filter((rsid) => /^rs\d+$/i.test(rsid));

    expect(hormoneRsids).toHaveLength(43);
    for (const rsid of hormoneRsids) {
      expect(LAYPERSON_MAP[rsid]?.simpleMeaning, rsid).toBeTruthy();
    }
    expect(DEFAULT_LAYPERSON_TRANSLATION.simpleMeaning).toContain('probabilistic association');
  });
});
