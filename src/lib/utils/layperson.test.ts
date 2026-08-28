import { describe, expect, it } from 'vitest';
import hormonePack from '../marker-packs/hormones_reproductive.json';
import {
  DEFAULT_LAYPERSON_TRANSLATION,
  getCompactDietNotes,
  getCompactGuidanceText,
  getCompactSimpleMeaning,
  getCompactSupplementName,
  getCompactSupplementReason,
  getSimpleFindingTitle,
  getSimpleNextStep,
  getLaypersonTranslation,
  LAYPERSON_MAP,
} from './layperson';

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

  it('keeps Simple cards concise while retaining the full authored meaning for details and exports', () => {
    const translation = {
      simpleImpact: 'Blood sugar research context',
      simpleMeaning: 'This marker has been studied in insulin signaling. It does not diagnose diabetes.',
    };

    expect(getCompactSimpleMeaning(translation)).toBe('This marker has been studied in insulin signaling.');
  });

  it('falls back to the plain title when a meaning contains only generic guardrails', () => {
    const translation = {
      simpleImpact: 'Research pathway context',
      simpleMeaning: 'This DNA result does not predict whether you have a condition or tell you what treatment to use.',
    };

    expect(getCompactSimpleMeaning(translation)).toBe('Research pathway context');
  });

  it('keeps domain-specific measurement limits visible', () => {
    const translation = {
      simpleImpact: 'Steroid-signaling research context',
      simpleMeaning: 'This marker is studied in steroid signaling. It does not measure current estrogen.',
    };

    expect(getCompactSimpleMeaning(translation)).toBe(
      'This marker is studied in steroid signaling. It does not measure current estrogen.',
    );
  });

  it('keeps technical gene names out of the primary Simple finding title', () => {
    expect(getSimpleFindingTitle('Altered DNA repair (BRCA1 gene)')).toBe('Altered DNA repair');
    expect(getSimpleFindingTitle('ESR1 hormone-response research marker')).toBe('Hormone response research context');
    expect(getSimpleFindingTitle('9p21 cardiovascular-association marker')).toBe('Cardiovascular research context');
    expect(getSimpleFindingTitle('FBN1/connective-tissue association marker')).toBe('Connective tissue research context');
    expect(getSimpleFindingTitle('CYP2D6*4 no-function allele component')).toBe('No function allele component');
    expect(getSimpleFindingTitle('A biological pathway studied in genetic research.')).toBe('A biological pathway studied in genetic research.');
  });

  it('keeps the technical word marker out of Simple titles', () => {
    const titles = Object.values(LAYPERSON_MAP).map((translation) => getSimpleFindingTitle(translation.simpleImpact));
    expect(titles.some((title) => /\bmarker\b/i.test(title))).toBe(false);
    expect(titles.some((title) => /[-_]/.test(title))).toBe(false);
    expect(getSimpleFindingTitle('research-only marker')).toBe('Research only finding');
  });

  it('keeps dashboard and card next steps direction-aware and concise', () => {
    const base = {
      clinical_confirmation_required: false,
      severity_class: 'benign' as const,
      confirm_with: [],
    };

    expect(getSimpleNextStep({ ...base, effect_direction: 'context_dependent' })).toBe('Consider diet, medications, and lifestyle context.');
    expect(getSimpleNextStep({ ...base, effect_direction: 'trait' })).toBe('Compare this with your lived experience.');
    expect(getSimpleNextStep({ ...base, effect_direction: 'protective' })).toBe('Use this as background context with your health history.');
    expect(getSimpleNextStep({ ...base, effect_direction: 'risk', severity_class: 'low_risk' })).toBe('Compare with symptoms, history, and relevant labs.');
    expect(getSimpleNextStep({ ...base, effect_direction: 'trait', clinical_confirmation_required: true })).toBe('Consider clinical confirmation.');
  });

  it('surfaces a bounded summary of the authored follow-up items', () => {
    const base = {
      clinical_confirmation_required: false,
      severity_class: 'benign' as const,
      confirm_with: [],
    };

    expect(getSimpleNextStep({
      ...base,
      effect_direction: 'risk',
      confirm_with: ['baseline serum tryptase', 'event tryptase', 'allergist/immunologist review', 'extra item'],
    })).toBe('Review baseline serum tryptase, event tryptase, or allergist/immunologist review (+1 more in details).');
    expect(getSimpleNextStep({
      ...base,
      effect_direction: 'risk',
      confirm_with: ['ferritin;'],
    })).toBe('Review ferritin.');
    expect(getSimpleNextStep({
      ...base,
      clinical_confirmation_required: true,
      effect_direction: 'risk',
      confirm_with: ['validated clinical laboratory test', 'genetic counseling'],
    })).toBe('Confirm with validated clinical laboratory test, or genetic counseling.');
    expect(getSimpleNextStep({
      ...base,
      effect_direction: 'risk',
      confirm_with: ['a very long authored follow-up label that should remain readable in the first-screen action queue', 'second check', 'third check'],
    })).toContain('more in details');
  });

  it('removes repeated actionability framing from secondary guidance text', () => {
    expect(getCompactGuidanceText(
      'Do not make this change unless symptoms, labs, or clinician guidance support it: Starting a restrictive diet',
    )).toBe('Starting a restrictive diet');
    expect(getCompactGuidanceText(
      'General low-risk option, not a genotype prescription: Eat a varied diet',
    )).toBe('Eat a varied diet');
    expect(getCompactGuidanceText(
      '• Consider only if symptoms, labs, or personal goals support it: Track the pattern\n• Confirm clinically before another step',
    )).toBe('• Track the pattern\n• Confirm clinically before another step');
  });

  it('rewrites long dietary prompts into concise Simple-view labels', () => {
    expect(getCompactGuidanceText(
      "Iron-containing foods that fit the person's diet while the reason for an abnormal iron result is clarified",
    )).toBe('Iron-rich foods, if they fit your diet');
    expect(getCompactGuidanceText(
      'Treating an iron-status marker as proof of iron deficiency, iron overload, or anemia',
    )).toBe("Don't treat an iron marker as a diagnosis");
    expect(getCompactGuidanceText(
      'Review measured Lp(a), ApoB, LDL-C, triglycerides, blood pressure, and family history together rather than reading one SNP in isolation',
    )).toBe('Review lipids, blood pressure, and family history together');
  });

  it('reduces aggregate dietary notes to unique plain-language bullets', () => {
    expect(getCompactDietNotes(
      '• TMPRSS6/TF: TMPRSS6 and TF markers can modestly shift population iron or hemoglobin traits, but they do not distinguish deficiency.\n• FUT2/TCN2: FUT2, TCN2, and CUBN markers can point to a B12-status or absorption question, but effects are modest.\n• FUT2/TCN2: FUT2, TCN2, and CUBN markers can point to a B12-status or absorption question, but effects are modest.',
    )).toEqual([
      'Iron SNPs are modest context; iron studies and bleeding history matter more.',
      'B12 SNPs are modest context; B12, CBC, symptoms, diet, and medicines matter more.',
    ]);
  });

  it('compacts supplement labels and attribution for Simple rows', () => {
    expect(getCompactSupplementName('Iron supplementation only after reviewing ferritin and clinician guidance'))
      .toBe('Iron supplementation');
    expect(getCompactSupplementName('Review calcium, vitamin D, magnesium, vitamin K, and any bone-health product against measured status'))
      .toBe('Bone-health supplement review');

    expect(getCompactSupplementReason(
      'Based on your TMPRSS6/TF variant (rs3811647); Discuss with a clinician or pharmacist before starting: Iron supplementation only after reviewing ferritin, and clinician or pharmacist guidance',
    )).toBe('Iron supplementation only after reviewing ferritin');
  });
});
