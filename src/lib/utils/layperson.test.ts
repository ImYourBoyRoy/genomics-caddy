import { describe, expect, it } from 'vitest';
import hormonePack from '../marker-packs/hormones_reproductive.json';
import {
  DEFAULT_LAYPERSON_TRANSLATION,
  getCompactActionMeaning,
  getCompactGuidanceText,
  getCompactSimpleMeaning,
  getCompactSupplementName,
  getCompactSupplementReason,
  getSimpleEvidenceLabel,
  getSimpleFindingCopy,
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

    expect(hormoneRsids).toHaveLength(49);
    for (const rsid of hormoneRsids) {
      expect(LAYPERSON_MAP[rsid]?.simpleMeaning, rsid).toBeTruthy();
    }
    expect(DEFAULT_LAYPERSON_TRANSLATION.simpleMeaning).toContain('probabilistic association');
  });

  it('explains fibroid GWAS loci as research associations, not personal predictions', () => {
    for (const rsid of ['rs4785384', 'rs1812264', 'rs7907606', 'rs7124615', 'rs10835889']) {
      const meaning = LAYPERSON_MAP[rsid]?.simpleMeaning || '';
      expect(meaning, rsid).toContain('group-level association');
      expect(meaning, rsid).toContain('not a personal');
      expect(meaning, rsid).toContain('cannot confirm or rule out fibroids');
      expect(meaning, rsid).toContain('predict IUD response');
    }
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

    expect(metabolic.simpleImpact).toBe('Blood sugar and body-weight signal');
    expect(reproductive.simpleImpact).toBe('Thyroid and immune signal');
    expect(metabolic.simpleMeaning).toContain('glucose or HbA1c');
    expect(reproductive.simpleMeaning).toContain('measured thyroid or immune tests');
    expect(metabolic.simpleMeaning).not.toMatch(/diagnos|raw DNA|genotype/i);
    expect(reproductive.simpleMeaning).not.toMatch(/diagnos|raw DNA|genotype/i);
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

    expect(translation.simpleImpact).toBe('Genetic research signal');
    expect(translation.simpleMeaning).toContain('biological pathway studied in research');
    expect(translation.simpleMeaning).not.toContain('does not predict whether you have a condition');
    expect(translation.simpleMeaning).not.toContain('A clinical test may be needed');
    expect(translation.reviewAction).toBe('Review the related clinical test route.');
    expect(translation.evidenceLabel).toBe('Clinical follow-up');
    expect(translation.simpleMeaning).not.toContain('This marker has been studied in pathway research.');
    expect(translation.simpleMeaning).not.toContain('does not measure current pathway activity');
  });

  it('uses an identifier-aware template for non-rsid clinical panels', () => {
    const translation = getLaypersonTranslation({
      rsid: 'PANEL_EXAMPLE_CLINICAL_ROUTE',
      gene: 'EXAMPLE',
      variant_name: 'Example hereditary condition panel',
      variant_type: 'gene_panel',
      impact: 'Clinical panel context',
      interpretation: 'A targeted clinical panel may be relevant.',
      raw_dna_limitation: 'A consumer array does not provide the full panel result.',
      clinical_confirmation_required: true,
    });

    expect(translation.simpleImpact).toBe('Clinical gene-panel signal');
    expect(translation.plainTitle).toBe('Clinical gene-panel signal');
    expect(translation.signal).toContain('Example hereditary condition panel');
    expect(translation.whyItMatters).toContain('symptoms, family history');
    expect(translation.reviewAction).toContain('targeted clinical testing');
    expect(translation.simpleMeaning).not.toContain('dedicated plain-English explanation');
    expect(translation.isFallback).not.toBe(true);
  });

  it('keeps Simple cards concise while retaining the full authored meaning for details and exports', () => {
    const translation = {
      simpleImpact: 'Blood sugar research context',
      simpleMeaning: 'This marker has been studied in insulin signaling. It does not diagnose diabetes.',
    };

    expect(getCompactSimpleMeaning(translation)).toBe('This marker has been studied in insulin signaling.');
  });

  it('keeps the useful association when a claim boundary trails the same sentence', () => {
    expect(getCompactSimpleMeaning({
      simpleImpact: 'Heart rhythm research context',
      simpleMeaning: 'This marker has been associated with cardiac conduction, but it does not diagnose an arrhythmia.',
    })).toBe('This marker has been associated with cardiac conduction');
  });

  it('keeps the dashboard action reason to one plain-language sentence', () => {
    expect(getCompactActionMeaning({
      simpleImpact: 'SLC30A8 beta-cell association marker',
      simpleMeaning: 'This SLC30A8 marker is studied in zinc transport and pancreatic beta-cell function. Any association is probabilistic; it does not measure insulin release or diagnose diabetes.',
    })).toBe('This finding is studied in zinc transport and pancreatic beta-cell function.');
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
    expect(getSimpleFindingTitle('CYP2D6*4 no-function allele component')).toBe('Medication processing context');
    expect(getSimpleFindingTitle('A biological pathway studied in genetic research.')).toBe('A biological pathway studied in genetic research.');
  });

  it('puts the direction before a concise topic instead of repeating research-context boilerplate', () => {
    const copy = getSimpleFindingCopy(
      {
        gene: 'TCF7L2',
        variant_name: 'TCF7L2 glucose association marker',
        impact: 'Higher glucose susceptibility research signal',
        evidence_tier: 'B_replicated_common_marker',
        effect_direction: 'risk',
        clinical_confirmation_required: false,
        severity_class: 'moderate_risk',
        confirm_with: [],
      },
      {
        simpleImpact: 'Blood sugar and metabolism research context',
        simpleMeaning: 'This marker has been studied in glucose and insulin-related traits.',
        plainTitle: 'Blood sugar and metabolism research context',
      },
    );

    expect(copy.plain_title).toBe('Higher susceptibility — Blood sugar and metabolism');
    expect(copy.direction_label).toBe('Higher susceptibility');
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
    expect(getSimpleNextStep({ ...base, effect_direction: 'trait', clinical_confirmation_required: true })).toBe('Review the related clinical test route.');
  });

  it('normalizes legacy translations into the complete Simple finding contract', () => {
    const copy = getSimpleFindingCopy(
      {
        evidence_tier: 'Tier B — replicated association',
        effect_direction: 'risk',
        clinical_confirmation_required: false,
        severity_class: 'moderate_risk',
        confirm_with: ['HbA1c'],
      },
      {
        simpleImpact: 'Blood sugar pathway',
        simpleMeaning: 'This marker is associated with insulin signaling. Pair it with measured glucose and HbA1c.',
      },
    );

    expect(copy).toEqual({
      plain_title: 'Blood sugar pathway',
      direction_label: 'Higher susceptibility',
      signal: 'This marker is associated with insulin signaling.',
      why_it_matters: 'Pair it with measured glucose and HbA1c.',
      review_action: 'Review HbA1c.',
      evidence_label: 'Replicated research signal',
      is_fallback: false,
    });
  });

  it('prefers authored structured Simple fields and keeps evidence labels compact', () => {
    const copy = getSimpleFindingCopy(
      {
        evidence_tier: 'Tier A — clinical confirmation',
        effect_direction: 'risk',
        clinical_confirmation_required: true,
        severity_class: 'confirmation_required',
        confirm_with: ['validated testing'],
      },
      {
        simpleImpact: 'Technical legacy title',
        simpleMeaning: 'Legacy meaning that should not replace authored copy.',
        plainTitle: 'Medication safety signal',
        signal: 'A medication-related safety signal is present.',
        whyItMatters: 'It may affect how a specific medicine is reviewed.',
        reviewAction: 'Check the matching medication before use.',
        evidenceLabel: 'Clinical safety route',
      },
    );

    expect(copy).toMatchObject({
      plain_title: 'Medication safety signal',
      signal: 'This marker is associated with a possible medication-sensitivity signal.',
      why_it_matters: 'It may affect how a specific medicine is reviewed.',
      review_action: 'Check the matching medication before use.',
      evidence_label: 'Clinical safety route',
    });
    expect(getSimpleEvidenceLabel('Tier E — exploratory')).toBe('Early or limited research');
  });

  it('makes named warfarin direction visible instead of using a generic response title', () => {
    const lower = getSimpleFindingCopy(
      {
        gene: 'VKORC1',
        variant_name: 'VKORC1 -1639G>A warfarin sensitivity marker',
        impact: 'Lower VKORC1 expression / lower warfarin dose requirement',
        interpretation: 'A-carriers generally require lower initial doses.',
        evidence_tier: 'A_pharmacogenomic_actionable',
        effect_direction: 'risk',
        clinical_confirmation_required: true,
        severity_class: 'confirmation_required',
        confirm_with: ['INR', 'clinical dosing algorithm'],
      },
      {
        simpleImpact: 'Warfarin response context',
        simpleMeaning: 'This DNA finding is one part of how warfarin response can vary.',
        plainTitle: 'Warfarin response context',
        signal: 'This DNA finding is one part of how warfarin response can vary.',
      },
    );

    expect(lower.plain_title).toBe('Warfarin dosing — lower-dose requirement signal');
    expect(lower.direction_label).toBe('Lower-dose requirement tendency');
    expect(lower.signal).toContain('lower warfarin dose requirement');
    expect(lower.why_it_matters).toContain('CYP2C9, VKORC1, and CYP4F2');
  });

  it('shows a higher-dose warfarin direction when the authored marker supports it', () => {
    const higher = getSimpleFindingCopy(
      {
        gene: 'CYP4F2',
        variant_name: 'CYP4F2 V433M warfarin dose marker',
        impact: 'Reduced vitamin K oxidation / higher warfarin dose requirement tendency',
        interpretation: 'The T allele can increase warfarin dose requirements modestly.',
        evidence_tier: 'A_pharmacogenomic_actionable',
        effect_direction: 'risk',
        clinical_confirmation_required: true,
        severity_class: 'confirmation_required',
        confirm_with: ['INR'],
      },
      {
        simpleImpact: 'Warfarin response context',
        simpleMeaning: 'This DNA finding is one part of how warfarin response can vary.',
      },
    );

    expect(higher.plain_title).toBe('Warfarin dosing — higher-dose requirement signal');
    expect(higher.direction_label).toBe('Higher-dose requirement tendency');
  });

  it('does not call an undirected medication component a susceptibility finding', () => {
    const copy = getSimpleFindingCopy(
      {
        gene: 'CYP2C9',
        variant_name: 'Clinical PGx allele component',
        impact: 'Medication dose component',
        interpretation: 'A complete clinical result is needed.',
        evidence_tier: 'A_pharmacogenomic_actionable',
        effect_direction: 'risk',
        clinical_confirmation_required: true,
        severity_class: 'confirmation_required',
        confirm_with: ['clinical PGx panel'],
      },
      {
        simpleImpact: 'Medication processing context',
        simpleMeaning: 'This DNA finding may influence how some medicines are processed.',
      },
    );

    expect(copy.plain_title).toBe('Medication processing — direction incomplete from this result');
    expect(copy.direction_label).toBe('Direction incomplete');
  });

  it('keeps the ten audited PGx translations specific in Simple mode', () => {
    const cases = [
      ['rs35742686', 'CYP2D6', 'CYP2D6*3'],
      ['rs5030655', 'CYP2D6', 'CYP2D6*6'],
      ['rs56337013', 'CYP2C19', 'CYP2C19*5'],
      ['rs72558186', 'CYP2C19', 'CYP2C19*6'],
      ['rs72552267', 'CYP2C19', 'CYP2C19*7'],
      ['rs41291556', 'CYP2C19', 'CYP2C19*8'],
      ['rs11045819', 'SLCO1B1', 'SLCO1B1'],
      ['rs186364861', 'NUDT15', 'NUDT15'],
      ['rs1801280', 'NAT2', 'NAT2'],
      ['rs1799930', 'NAT2', 'NAT2'],
    ] as const;

    for (const [rsid, gene, pathway] of cases) {
      const translation = LAYPERSON_MAP[rsid];
      const copy = getSimpleFindingCopy({
        gene,
        variant_name: `${pathway} allele component`,
        impact: `${gene} reduced-function component`,
        evidence_tier: 'A_clinical_guideline',
        effect_direction: 'risk',
        clinical_confirmation_required: true,
        severity_class: 'confirmation_required',
        confirm_with: ['clinical PGx panel'],
        variant_type: 'pharmacogenomic_snp',
      }, translation);

      expect(copy.plain_title, rsid).toContain(gene);
      expect(copy.plain_title, rsid).not.toMatch(/medication processing context|medication safety context/i);
      expect(copy.signal, rsid).toContain(gene);
      expect(copy.review_action, rsid).toMatch(/PGx|testing/i);
    }
  });

  it('keeps high-visibility medication and neurobiology findings specific', () => {
    const marker = (gene: string, variant_name: string, impact: string, effect_direction: 'risk' | 'context_dependent') => ({
      gene,
      variant_name,
      impact,
      evidence_tier: 'A_clinical_guideline',
      effect_direction,
      clinical_confirmation_required: gene === 'CYP2C9',
      severity_class: 'confirmation_required' as const,
      confirm_with: ['clinical review'],
      variant_type: gene === 'CYP2C9' ? 'pharmacogenomic_snp' : 'snp',
    });

    const cyp2c9 = getSimpleFindingCopy(
      marker('CYP2C9', 'CYP2C9*3 reduced-function warfarin marker', 'Reduced CYP2C9 enzyme clearance', 'risk'),
      LAYPERSON_MAP.rs1057910,
    );
    expect(cyp2c9.plain_title).toBe('CYP2C9*3 — reduced medicine clearance');
    expect(cyp2c9.signal).toContain('reduced-function CYP2C9*3');

    const maoa = getSimpleFindingCopy(
      marker('MAOA', 'MAOA coding variant', 'Lower monoamine oxidase A enzyme activity', 'context_dependent'),
      LAYPERSON_MAP.rs6323,
    );
    expect(maoa.plain_title).toBe('MAOA lower-activity stress-response signal');
    expect(maoa.signal).toContain('lower-activity MAOA');

    const drd3 = getSimpleFindingCopy(
      marker('DRD3', 'Ser9Gly', 'Dopamine D3 receptor binding/limbic reward candidate', 'context_dependent'),
      LAYPERSON_MAP.rs6280,
    );
    expect(drd3.plain_title).toBe('DRD3 dopamine-reward and movement-sensitivity signal');
    expect(drd3.signal).toContain('reward sensitivity and movement-side-effect');
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
