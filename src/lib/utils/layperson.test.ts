import { describe, expect, it } from 'vitest';
import hormonePack from '../marker-packs/hormones_reproductive.json';
import { DEFAULT_LAYPERSON_TRANSLATION, getLaypersonTranslation, LAYPERSON_MAP } from './layperson';

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

    expect(hormoneRsids).toHaveLength(44);
    for (const rsid of hormoneRsids) {
      expect(LAYPERSON_MAP[rsid]?.simpleMeaning, rsid).toBeTruthy();
    }
    expect(DEFAULT_LAYPERSON_TRANSLATION.simpleMeaning).toContain('probabilistic association');
  });

  it('covers high-value bone and digestive markers with dedicated boundaries', () => {
    for (const rsid of [
      'rs3736228',
      'rs6426749',
      'rs2066844',
      'rs2066847',
      'rs2241880',
      'rs738409',
    ]) {
      expect(LAYPERSON_MAP[rsid]?.simpleMeaning, rsid).toBeTruthy();
    }
    expect(LAYPERSON_MAP.rs3736228.simpleMeaning).toContain('does not diagnose osteoporosis');
    expect(LAYPERSON_MAP.rs2066844.simpleMeaning).toContain('does not diagnose Crohn disease');
    expect(LAYPERSON_MAP.rs738409.simpleMeaning).toContain('does not diagnose fatty liver');
  });

  it('covers high-impact cardiovascular and hereditary-cancer markers with confirmation boundaries', () => {
    for (const rsid of [
      'rs76992529',
      'rs6025',
      'rs1799963',
      'rs11591147',
      'rs6511720',
      'rs599839',
      'rs116843064',
      'rs12190287',
    ]) {
      expect(LAYPERSON_MAP[rsid]?.simpleMeaning, rsid).toBeTruthy();
    }
    expect(LAYPERSON_MAP.rs76992529.simpleMeaning).toContain('not a diagnosis');
    expect(LAYPERSON_MAP.rs6025.simpleMeaning).toContain('current clot');
    expect(LAYPERSON_MAP.rs11591147.simpleMeaning).toContain('lower LDL cholesterol');
    expect(LAYPERSON_MAP.rs12190287.simpleMeaning).toContain('not a diagnosis');
  });

  it('uses authored pack-scoped copy for markers without marker-specific wording', () => {
    const metabolic = getLaypersonTranslation({
      rsid: 'rs7754840',
      gene: 'CDKAL1',
      impact: 'Beta-cell insulin secretion risk locus',
      interpretation: 'Association context only.',
      raw_dna_limitation: 'Clinical context is required.',
      clinical_confirmation_required: false,
    });
    const reproductive = getLaypersonTranslation({
      rsid: 'rs4704397',
      gene: 'TPO',
      impact: 'Thyroid regulation context',
      interpretation: 'Association context only.',
      raw_dna_limitation: 'Clinical context is required.',
      clinical_confirmation_required: false,
    });

    expect(metabolic.simpleImpact).toBe('Blood sugar and metabolism research context');
    expect(reproductive.simpleImpact).toBe('Thyroid and immune research context');
    expect(metabolic.simpleMeaning).toContain('does not diagnose diabetes');
    expect(reproductive.simpleMeaning).toContain('does not diagnose thyroid disease');
    expect(metabolic.isFallback).not.toBe(true);
    expect(reproductive.isFallback).not.toBe(true);
  });

  it('uses a safe non-clinical fallback when no dedicated translation exists', () => {
    const translation = getLaypersonTranslation({
      rsid: 'rs-untranslated',
      gene: 'EXAMPLE',
      impact: 'Example pathway context',
      interpretation: 'This marker has been studied in pathway research.',
      raw_dna_limitation: 'This marker does not measure current pathway activity.',
      clinical_confirmation_required: true,
    });

    expect(translation.simpleImpact).toBe('A biological pathway studied in genetic research.');
    expect(translation.simpleMeaning).toContain('associated with a research finding');
    expect(translation.simpleMeaning).toContain('does not predict whether you have a condition');
    expect(translation.simpleMeaning).toContain('A clinical test may be needed');
    expect(translation.simpleMeaning).not.toContain('This marker has been studied in pathway research.');
    expect(translation.simpleMeaning).not.toContain('does not measure current pathway activity');
  });
});
