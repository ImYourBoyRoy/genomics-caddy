import { describe, expect, it } from 'vitest';
import {
  buildLabRequestListText,
  canonicalizeLabName,
  deriveActionablePlan,
  deriveAllergySensitivityGuidance,
  priorityMetadataForMarker,
  type LabTest,
} from './actionabilityEngine';
import type { GeneratedReport, EvaluatedMarker } from '../types/genomics';
import type { PersonalSafetyContext } from './personalSafetyContext';

function marker(overrides: Partial<EvaluatedMarker>): EvaluatedMarker {
  return {
    link_id: 'test:marker',
    rsid: 'rs1801133',
    gene: 'MTHFR',
    variant_name: 'test marker',
    chromosome: '1',
    position: null,
    user_genotype: 'TT',
    normalized_genotype: null,
    effect_allele: 'T',
    effect_count: 2,
    impact: 'test impact',
    evidence_tier: 'B_replicated_common_marker',
    interpretation: 'test interpretation',
    effect_direction: 'risk',
    severity_class: 'moderate_risk',
    assertion_status: 'Verified',
    interpretation_allowed: true,
    sources: [],
    db_enriched_sources: [],
    clinvar_significance: null,
    clinvar_conditions: null,
    clinvar_review_status: null,
    gwas_top_trait: null,
    gwas_best_pvalue: null,
    gwas_association_count: null,
    population_af: null,
    population_rarity: null,
    confirm_with: [],
    do_not_claim: [],
    ...overrides,
  };
}

function report(markers: EvaluatedMarker[]): GeneratedReport {
  return {
    schema_version: '2.0.0',
    export_format: 'normalized_sparse',
    generated_at: new Date(0).toISOString(),
    title: 'test',
    description: 'test',
    overall_signal_score: 0,
    variants: {},
    user_calls: {},
    category_links: {},
    enrichment: {},
    sections: [
      {
        name: 'Nutrients',
        markers,
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ],
  };
}

describe('actionability engine safety policy', () => {
  it('keeps the dashboard action queue bounded while retaining full report data', () => {
    const markers = Array.from({ length: 11 }, (_, index) => marker({
      rsid: `rs-queue-${index}`,
      gene: `GENE${index}`,
      link_id: `test:queue:${index}`,
      severity_class: 'moderate_risk',
    }));

    const plan = deriveActionablePlan(report(markers));

    expect(plan.topFindings).toHaveLength(9);
  });

  it('removes harmless cancer-language markers from the concern queue', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs-harmless-repair',
      gene: 'BRCA2',
      link_id: 'test:harmless-repair',
      variant_name: 'Harmless DNA repair variant',
      impact: 'Benign DNA repair change',
      interpretation: 'This does not indicate a classic hereditary cancer risk.',
      severity_class: 'moderate_risk',
      effect_count: 1,
    })]));

    expect(plan.topFindings).toHaveLength(0);
  });

  it('groups related clinical medication components into one queue topic', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs-cyp2c9-a',
        gene: 'CYP2C9',
        link_id: 'test:cyp2c9:a',
        variant_name: 'Clinical PGx allele component',
        clinical_confirmation_required: true,
        severity_class: 'moderate_risk',
      }),
      marker({
        rsid: 'rs-cyp2c9-b',
        gene: 'CYP2C9',
        link_id: 'test:cyp2c9:b',
        variant_name: 'Clinical PGx allele component',
        clinical_confirmation_required: true,
        severity_class: 'moderate_risk',
      }),
    ]));

    expect(plan.topFindings).toHaveLength(1);
    expect(plan.topFindings[0]?.priority_group).toBe('clinical-medication:CYP2C9');
    expect(plan.topFindings[0]?.related_marker_count).toBe(2);
  });

  it('groups the same locus across health areas into one canonical queue finding', () => {
    const base = report([]);
    base.sections = [
      {
        name: 'Cardiovascular',
        markers: [marker({
          rsid: 'rs-shared-signal',
          gene: 'SHARED1',
          link_id: 'test:shared:cardio',
          variant_name: 'Shared cardiovascular signal',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Nutrients',
        markers: [marker({
          rsid: 'rs-shared-signal',
          gene: 'SHARED1',
          link_id: 'test:shared:nutrients',
          variant_name: 'Shared nutrient signal',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ];

    const plan = deriveActionablePlan(base);

    expect(plan.topFindings).toHaveLength(1);
    expect(plan.topFindings[0]?.finding_id).toBe('finding-rsid-rs-shared-signal');
    expect(plan.topFindings[0]?.source_marker_ids).toEqual([
      'test:shared:cardio',
      'test:shared:nutrients',
    ]);
    expect(plan.topFindings[0]?.topic_ids).toEqual(['cardiovascular', 'nutrients']);
    expect(plan.topFindings[0]?.health_area_labels).toEqual(['Cardiovascular', 'Nutrients']);
    expect(plan.topFindings[0]?.related_marker_count).toBe(2);
  });

  it('does not turn unknown-only source rows into a priority concern', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs-unknown-only',
      link_id: 'test:unknown-only',
      user_genotype: '--',
      assertion_status: 'NoData',
      interpretation_allowed: true,
      severity_class: 'moderate_risk',
      effect_count: 1,
    })]));

    expect(plan.topFindings).toHaveLength(0);
  });

  it('retains mixed call state when a canonical finding has one known and one unknown source', () => {
    const base = report([]);
    base.sections = [
      {
        name: 'Cardiovascular',
        markers: [marker({
          rsid: 'rs-mixed-signal',
          link_id: 'test:mixed:known',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
      {
        name: 'Nutrients',
        markers: [marker({
          rsid: 'rs-mixed-signal',
          link_id: 'test:mixed:unknown',
          user_genotype: '--',
          assertion_status: 'NotInRawFile',
          interpretation_allowed: false,
          severity_class: 'no_data',
          effect_count: 0,
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      },
    ];

    const plan = deriveActionablePlan(base);

    expect(plan.topFindings[0]?.call_state).toBe('mixed');
    expect(plan.topFindings[0]?.source_marker_ids).toHaveLength(2);
  });

  it('ranks concrete follow-up routes above equally severe generic associations', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs-generic-moderate',
        gene: 'GENERIC1',
        link_id: 'test:generic-moderate',
      }),
      marker({
        rsid: 'rs1801133',
        gene: 'MTHFR',
        link_id: 'test:mthfr-follow-up',
        confirm_with: ['Homocysteine (plasma)'],
      }),
    ]));

    expect(plan.topFindings[0]?.gene).toBe('MTHFR');
  });

  it('keeps severity ordering ahead of concrete follow-up bonuses', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs-high',
        gene: 'HIGH1',
        link_id: 'test:high',
        severity_class: 'high_risk',
      }),
      marker({
        rsid: 'rs-moderate',
        gene: 'MODERATE1',
        link_id: 'test:moderate',
        severity_class: 'moderate_risk',
        confirm_with: ['CBC', 'Ferritin', 'Transferrin saturation'],
      }),
    ]));

    expect(plan.topFindings[0]?.severity_class).toBe('high_risk');
  });

  it('ranks a concrete medication safety route above a high-severity research signal', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs-vague-high',
        gene: 'RESEARCH1',
        link_id: 'test:vague-high',
        variant_name: 'Research-only association',
        evidence_tier: 'D_research_only',
        effect_direction: 'context_dependent',
        severity_class: 'high_risk',
      }),
      marker({
        rsid: 'rs-medication-safety',
        gene: 'CYP2C9',
        link_id: 'test:medication-safety',
        variant_name: 'Drug-specific clinical PGx allele component',
        interpretation: 'A medication dose component requiring clinical confirmation.',
        severity_class: 'moderate_risk',
        clinical_confirmation_required: true,
      }),
    ]));

    expect(plan.topFindings[0]?.gene).toBe('CYP2C9');
    expect(plan.topFindings[0]?.priority_reason).toBe('Check before exposure');
    expect(plan.topFindings[0]?.priority_urgency).toBe('safety');
    expect(plan.topFindings[0]?.priority_tone).toBe('danger');
    expect(plan.topFindings[1]?.priority_reason).toBe('Worth discussing');
    expect(plan.topFindings[1]?.priority_tone).toBe('warning');
  });

  it('labels lower-evidence context without turning it into a danger state', () => {
    const priority = priorityMetadataForMarker(marker({
      severity_class: 'context_dependent',
      evidence_tier: 'D_research_only',
      effect_direction: 'context_dependent',
    }));

    expect(priority.reason).toBe('Research context');
    expect(priority.urgency).toBe('research');
    expect(priority.tone).toBe('info');
  });

  it('describes protective context as potentially favorable rather than guaranteed benefit', () => {
    const priority = priorityMetadataForMarker(marker({
      severity_class: 'protective',
      effect_direction: 'protective',
    }));

    expect(priority.reason).toBe('Potentially favorable context');
    expect(priority.urgency).toBe('favorable');
    expect(priority.tone).toBe('positive');
  });

  it('qualifies diet guidance and never revives the MTHFR folic-acid avoidance myth', () => {
    const plan = deriveActionablePlan(report([marker({})]));
    expect(plan.diet.favor).toContain('Folate-rich foods (leafy greens, beans, eggs)');
    expect(plan.advancedGuidance.some((item) => item.startsWith('Consider only if symptoms'))).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('Folic acid fortified foods');
    expect(plan.supplements).toHaveLength(0);
    expect(plan.safetyNotes.some((note) => note.includes('More markers increase coverage'))).toBe(true);
  });

  it('keeps food limits concise while preserving clinical-confirmation guidance for advanced consumers', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs1800562',
        gene: 'HFE',
        evidence_tier: 'A_clinically_relevant_rare_variant',
        severity_class: 'high_risk',
      }),
    ]));
    expect(plan.diet.avoid).toContain('Red meat');
    expect(plan.diet.avoid.join(' ')).not.toContain('raw DNA');
    expect(plan.advancedGuidance.some((item) => item.startsWith('Do not make this change from raw DNA'))).toBe(true);
    expect(plan.supplementAvoid.some((item) => item.name === 'Iron-containing supplements')).toBe(true);
  });

  it('retains a broad DNA-linked food and supplement menu without mixing in generic warnings', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'APOE', rsid: 'rs429358', interpretation: 'APOE4 lipid context; not diagnostic.' }),
      marker({ gene: 'FADS1', rsid: 'rs174547', interpretation: 'FADS1 fatty-acid conversion context; not diagnostic.' }),
      marker({ gene: 'LCT/MCM6', rsid: 'rs4988235', interpretation: 'Lactase persistence context; not a milk-allergy diagnosis.' }),
      marker({ gene: 'VDR', rsid: 'rs2228570', interpretation: 'Vitamin-D pathway context; not diagnostic.' }),
      marker({ gene: 'COMT', rsid: 'rs4680', interpretation: 'slow COMT catecholamine context; not diagnostic.' }),
    ]));

    expect(plan.diet.favor.length).toBeGreaterThanOrEqual(12);
    expect(plan.diet.favor.join(' ')).toMatch(/fish|walnuts|olive oil|beans|vitamin|magnesium/i);
    expect(plan.diet.avoid.join(' ')).toMatch(/red meat|saturated|caffeine|coconut/i);
    expect(plan.supplements.some((item) => item.name === 'Omega-3 (fish or algae oil)')).toBe(true);
    expect(plan.supplements.some((item) => item.name === 'DHA/EPA (algae or fish oil)')).toBe(true);
    expect(plan.supplements.some((item) => item.name === 'Magnesium')).toBe(true);
    expect(plan.supplementAvoid.some((item) => /megadose vitamin d or calcium/i.test(item.name))).toBe(true);
    expect(plan.diet.favor.join(' ')).not.toMatch(/diagnos|raw DNA|genotype|SNP/i);
    expect(plan.diet.avoid.join(' ')).not.toMatch(/diagnos|raw DNA|genotype|SNP/i);
  });

  it('retains provenance for DNA-linked foods and supplements while keeping legacy labels available', () => {
    const plan = deriveActionablePlan(report([
      marker({
        gene: 'APOE',
        rsid: 'rs429358',
        interpretation: 'APOE4 lipid context.',
      }),
      marker({
        gene: 'FADS1',
        rsid: 'rs174547',
        interpretation: 'FADS1 fatty-acid conversion context.',
      }),
    ]));

    const fish = plan.diet.favorItems.find((item) => item.name.includes('Fatty fish'));
    const omega3 = plan.supplements.find((item) => item.basis_rule_ids.includes('fads_fatty_acids'));

    expect(fish).toMatchObject({
      category: 'food',
      basis_rule_ids: ['apoe_lipid'],
      basis_topic_ids: ['apoe_lipid'],
      basis_marker_ids: ['rs429358'],
      basis_genes: ['APOE'],
      evidence_level: 'Moderate evidence',
    });
    expect(fish?.why_it_appears).toContain('APOE');
    expect(fish?.when_relevant).toContain('symptoms');
    expect(omega3?.category).toBe('supplement');
    expect(omega3?.basis_rule_ids).toContain('fads_fatty_acids');
    expect(omega3?.basis_marker_ids).toContain('rs174547');
    expect(plan.diet.favor).toContain(fish?.name);
    expect(plan.supplements.map((item) => item.name)).toContain(omega3?.name);
  });

  it('retains matched marker provenance for DNA-linked activity recommendations', () => {
    const plan = deriveActionablePlan({
      ...report([]),
      sections: [{
        name: 'Muscle Performance & Recovery',
        markers: [marker({
          link_id: 'test:actn3',
          rsid: 'rs1815739',
          gene: 'ACTN3',
          variant_name: 'ACTN3 performance context',
          interpretation: 'ACTN3 muscle-performance context.',
        })],
        section_signal_score: 0,
        summary: {} as GeneratedReport['sections'][number]['summary'],
      }],
    });

    const domain = plan.activity.relevantDomains.find((item) => item.id === 'muscle_performance_recovery');
    const buildItem = plan.activity.recommendationItems.find((item) =>
      item.kind === 'build' && item.basis_topic_ids.includes('muscle_performance_recovery'));

    expect(domain?.matched_marker_ids).toEqual(['test:actn3']);
    expect(domain?.matched_genes).toEqual(['ACTN3']);
    expect(buildItem).toMatchObject({
      category: 'activity',
      basis_topic_ids: ['muscle_performance_recovery'],
      basis_marker_ids: ['test:actn3'],
      basis_genes: ['ACTN3'],
    });
  });

  it('routes B12 and vitamin-D pathway markers to measured status and supplement safety', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs602662',
        gene: 'FUT2',
        interpretation: 'FUT2 B12-status context marker; not diagnostic.',
      }),
      marker({
        rsid: 'rs10741657',
        gene: 'CYP2R1',
        interpretation: 'Vitamin-D status context marker; not diagnostic.',
      }),
    ]));

    expect(plan.labTests.some((test) => test.name === 'Serum or plasma vitamin B12')).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Methylmalonic acid'))).toBe(true);
    expect(plan.labTests.some((test) => test.name === '25-hydroxyvitamin D [25(OH)D]')).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('vitamin B12 deficiency');
    expect(plan.advancedGuidance.some((item) => item.includes('vitamin B12 deficiency'))).toBe(true);
    expect(plan.supplementSafety.relevantRules.map((rule) => rule.id)).toEqual(expect.arrayContaining([
      'b12_status',
      'vitamin_d',
    ]));
  });

  it('canonicalizes common lab aliases and keeps the request list concise', () => {
    const a1c = canonicalizeLabName('A1c');
    const hba1c = canonicalizeLabName('HbA1c');
    expect(a1c.canonical_id).toBe(hba1c.canonical_id);
    expect(a1c.name).toBe('HbA1c');
    expect(a1c.purpose).toContain('blood-glucose');

    const lab = (name: string, reason: string, category = 'Metabolic'): LabTest => ({
      ...canonicalizeLabName(name),
      reason,
      reason_topics: [reason],
      basis_rule_ids: ['test_rule'],
      basis_marker_ids: ['rs-test'],
      basis_genes: ['TEST1'],
      urgency: 'consider',
      tier: 'discuss',
      requires_counselor: false,
      category,
    });
    const request = buildLabRequestListText([
      lab('A1c', 'DNA-linked glucose pathway'),
      lab('HbA1c', 'Duplicate glucose pathway'),
      lab('ApoB', 'DNA-linked lipid pathway', 'Heart & lipids'),
    ]);

    expect(request).toContain('HbA1c — DNA-linked glucose pathway');
    expect(request).not.toContain('Duplicate glucose pathway');
    expect(request).toContain('ApoB (Apolipoprotein B) — DNA-linked lipid pathway');
  });

  it('routes alcohol and caffeine response markers to exposure-aware guardrails', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs1229984',
        gene: 'ADH1B',
        interpretation: 'ADH1B alcohol response context; not diagnostic.',
      }),
      marker({
        rsid: 'rs5751876',
        gene: 'ADORA2A',
        interpretation: 'ADORA2A caffeine response context; not diagnostic.',
      }),
    ]));
    const avoidText = plan.diet.avoid.join(' ');
    expect(avoidText).toContain('Alcohol when it causes adverse symptoms');
    expect(avoidText).toContain('High-dose or late-day caffeine');
    expect(avoidText).not.toContain('universal caffeine limit');
    expect(plan.advancedGuidance.join(' ')).toContain('heavier drinking');
    expect(plan.advancedGuidance.join(' ')).toContain('universal caffeine limit');
    expect(plan.diet.favor.join(' ')).toContain('Alcohol-free options');
  });

  it('routes metabolic and iron-status markers to measured follow-up', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs2237897',
        gene: 'KCNQ1',
        interpretation: 'KCNQ1 glucose-stimulated insulin secretion context; not diagnostic.',
      }),
      marker({
        rsid: 'rs328',
        gene: 'LPL',
        interpretation: 'LPL lipoprotein and triglyceride context; not diagnostic.',
      }),
      marker({
        rsid: 'rs855791',
        gene: 'TMPRSS6',
        interpretation: 'TMPRSS6 iron-status context; not diagnostic.',
      }),
      marker({
        rsid: 'rs3811647',
        gene: 'TF',
        interpretation: 'TF transferrin and iron context; not diagnostic.',
      }),
      marker({
        rsid: 'rs17782313',
        gene: 'MC4R',
        interpretation: 'MC4R satiety context; not diagnostic.',
      }),
    ]));
    expect(plan.labTests.some((test) => test.name === 'HbA1c')).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('triglycerides'))).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Complete blood count (CBC)')).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Ferritin and transferrin saturation')).toBe(true);
    expect(plan.supplements.some((item) => item.reason.includes('Iron supplementation only'))).toBe(true);
    expect(plan.supplementSafety.relevantRules.map((rule) => rule.id)).toContain('iron_status');
    expect(plan.diet.favor.join(' ')).toMatch(/fiber-rich foods/i);
    expect(plan.diet.avoid.join(' ')).not.toContain('fixed calorie target');
    expect(plan.advancedGuidance.join(' ')).toContain('fixed calorie target');
  });

  it('routes bone, inflammatory-bowel, and celiac markers to measured clinical follow-up', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs3736228',
        gene: 'LRP5',
        interpretation: 'LRP5 bone mineral-density context; not diagnostic.',
      }),
      marker({
        rsid: 'rs2066844',
        gene: 'NOD2',
        interpretation: 'NOD2 Crohn disease susceptibility context; not diagnostic.',
      }),
      marker({
        rsid: 'PANEL_CELIAC_HLA_DQ2_DQ8',
        gene: 'HLA-DQA1/HLA-DQB1',
        severity_class: 'confirmation_required',
        interpretation: 'Clinical HLA typing is required; this panel is not diagnostic.',
      }),
    ]));
    const dietAvoidance = plan.diet.avoid.join(' ');
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('DXA/BMD'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('fecal calprotectin'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('tTG-IgA'))).toBe(true);
    expect(dietAvoidance).not.toContain('lifelong gluten avoidance');
    expect(plan.advancedGuidance.join(' ')).toContain('lifelong gluten avoidance');
    expect(plan.supplements.some((item) => item.name.includes('bone-health'))).toBe(true);
    expect(medicationText).toContain('raw DNA');
  });

  it('keeps rare bone and fructose panels at the clinical-confirmation boundary', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'PANEL_OSTEOGENESIS_IMPERFECTA',
        gene: 'COL1A1/COL1A2/IFITM5/CRTAP/P3H1',
        severity_class: 'confirmation_required',
        interpretation: 'Clinical sequencing and phenotype are required; this panel is not diagnostic.',
      }),
      marker({
        rsid: 'PANEL_HYPOPHOSPHATASIA',
        gene: 'ALPL',
        severity_class: 'confirmation_required',
        interpretation: 'Low alkaline phosphatase and clinical findings require confirmation; raw DNA is not enough.',
      }),
      marker({
        rsid: 'PANEL_HEREDITARY_FRUCTOSE_INTOLERANCE',
        gene: 'ALDOB',
        severity_class: 'confirmation_required',
        interpretation: 'Clinical ALDOB sequencing is required; this panel is not diagnostic.',
      }),
    ]));
    const avoidance = plan.diet.avoid.join(' ');
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('bone-fragility gene-panel'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('ALPL sequencing'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('ALDOB sequencing'))).toBe(true);
    expect(avoidance).toBe('');
    expect(plan.advancedGuidance.join(' ')).toContain('fructose challenge');
    expect(plan.advancedGuidance.join(' ')).toContain('osteogenesis imperfecta');
    expect(medicationText).toContain('consumer-array panel');
    expect(medicationText).not.toContain('Start enzyme');
  });

  it('routes rare allergy, kidney, metabolic, nutrient, and neurologic panels safely', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'PANEL_MAST_CELL_MEDIATOR_CONTEXT',
        gene: 'TPSAB1/KIT/CPA3/HDC',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_HEREDITARY_ANGIOEDEMA',
        gene: 'SERPING1/F12/PLG/ANGPT1/KNG1',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_ALPHA1_ANTITRYPSIN_DEFICIENCY',
        gene: 'SERPINA1',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_APOL1_KIDNEY_RISK',
        gene: 'APOL1',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_MONOGENIC_RENAL_TUBULOPATHY',
        gene: 'SLC12A3/SLC12A1/KCNJ1/CLCNKB/WNK1/WNK4',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_MODY_MONOGENIC_DIABETES',
        gene: 'GCK/HNF1A/HNF4A/HNF1B/INS/PDX1/NEUROD1',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_NON_HFE_IRON_OVERLOAD',
        gene: 'HAMP/HJV/TFR2/SLC40A1',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_WILSON_DISEASE',
        gene: 'ATP7B',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_RARE_NUTRIENT_TRANSPORT_DISORDERS',
        gene: 'SLC19A2/SLC19A3/SLC46A1/SLC52A2/SLC52A3',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_MONOGENIC_MIGRAINE_RED_FLAGS',
        gene: 'CACNA1A/ATP1A2/SCN1A/PRRT2',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'PANEL_PAIN_CHANNELopathy_RED_FLAGS',
        gene: 'SCN9A/SCN10A/SCN11A/TRPA1/TRPV1',
        severity_class: 'confirmation_required',
      }),
    ]));
    const avoidance = plan.diet.avoid.join(' ');
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('serum tryptase'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('C4'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('alpha-1 antitrypsin'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('APOL1 G1/G2'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('potassium'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HbA1c'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('transferrin saturation'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Ceruloplasmin'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('pain-neuropathy sequencing'))).toBe(true);
    expect(avoidance).toBe('');
    expect(plan.advancedGuidance.join(' ')).toContain('low-histamine diet');
    expect(plan.advancedGuidance.join(' ')).toContain('diagnose or exclude Wilson disease');
    expect(plan.advancedGuidance.join(' ')).toContain('megadose vitamins');
    expect(avoidance).not.toContain('Do not avoid solely');
    expect(plan.advancedGuidance.join(' ')).toContain('Do not make this change from raw DNA');
    expect(medicationText).toContain('raw DNA');
    expect(medicationText).not.toContain('select therapy');
  });

  it('routes high-impact cardiovascular and hereditary-cancer markers to confirmation', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs76992529',
        gene: 'TTR',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'ARRHYTHMOGENIC_CARDIOMYOPATHY_PANEL',
        gene: 'PKP2/DSP/DSG2/DSC2/JUP/TMEM43/DES/FLNC/PLN',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'rs555607708',
        gene: 'CHEK2',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'rs34612342',
        gene: 'MUTYH',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'rs1801155',
        gene: 'APC',
        severity_class: 'confirmation_required',
      }),
      marker({
        rsid: 'rs138213197',
        gene: 'HOXB13',
        severity_class: 'confirmation_required',
      }),
    ]));
    const avoidance = plan.diet.avoid.join(' ');
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('TTR genotyping'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('rhythm monitoring'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('CHEK2 sequencing'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('MUTYH sequencing'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('APC testing'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HOXB13 testing'))).toBe(true);
    expect(avoidance).toBe('');
    expect(plan.advancedGuidance.join(' ')).toContain('diagnose amyloidosis');
    expect(plan.advancedGuidance.join(' ')).toContain('classic familial adenomatous polyposis');
    expect(medicationText).toContain('raw DNA');
    expect(medicationText).not.toContain('select cardiac');
  });

  it('keeps reproductive research panels symptom- and context-gated', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY',
        gene: 'ESR1/ESR2/PGR/ALLO_GABAA/ESC_EZ',
      }),
      marker({
        rsid: 'PANEL_PCOS_REPRODUCTIVE_METABOLIC_CONTEXT',
        gene: 'FSHR/LHCGR/SHBG/CYP17A1/INSR/METABOLIC',
      }),
      marker({
        rsid: 'PANEL_ENDOMETRIOSIS_CONTEXT',
        gene: 'WNT4/VEZT/IL1A/ESR1/MULTI_GENE',
      }),
      marker({
        rsid: 'PANEL_ADENOMYOSIS_RESEARCH_GAP',
        gene: 'ESR1/PGR/WNT4/VEZT/HRH1/SSPN/MULTI_GENE',
        severity_class: 'confirmation_required',
      }),
    ]), { reproductiveContext: 'cyclic_mood_symptoms' });
    const avoidance = plan.diet.avoid.join(' ');
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('HbA1c'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('transvaginal ultrasound'))).toBe(true);
    expect(avoidance).toBe('');
    expect(plan.advancedGuidance.join(' ')).toContain('estrogen spike');
    expect(plan.advancedGuidance.join(' ')).toContain('diagnose PCOS');
    expect(plan.advancedGuidance.join(' ')).toContain('diagnose or exclude endometriosis');
    expect(medicationText).toContain('exact contraceptive');
    expect(medicationText).toContain('raw DNA');
  });

  it('surfaces cycle-aware activity and medication guardrails for reproductive context', () => {
    const plan = deriveActionablePlan({
      ...report([marker({
        rsid: 'PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY',
        gene: 'ESR1/ESR2/PGR',
        sex_scope: 'menstrual_cycle_context',
      })]),
      sections: [{
        ...report([]).sections[0],
        name: 'Menstrual Cycle, Hormones & Reproductive Context',
        markers: [marker({
          rsid: 'PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY',
          gene: 'ESR1/ESR2/PGR',
          sex_scope: 'menstrual_cycle_context',
        })],
      }],
    }, { reproductiveContext: 'menstrual_cycle' });
    expect(plan.activity.relevantDomains.some((domain) => domain.id === 'hormones_reproductive')).toBe(true);
    expect(plan.activity.stopAndEscalate.some((item) => item.includes('Chest pain'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('contraceptive'))).toBe(true);
    expect(plan.medication.askFor.some((item) => item.includes('active ingredient'))).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_phase_and_symptom_timing')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'heavy_bleeding_pelvic_pain')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_linked_pain_headache_context')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_nutrition_activity_context')).toBe(true);
  });

  it('surfaces reproductive activity guardrails from explicit context even without a matching SNP', () => {
    const plan = deriveActionablePlan(report([]), { reproductiveContext: 'menstrual_cycle' });

    expect(plan.activity.relevantDomains.map((domain) => domain.id)).toContain('hormones_reproductive');
    expect(plan.activity.relevantDomains.find((domain) => domain.id === 'hormones_reproductive')?.favor.join(' '))
      .toContain('cycle-aware');
  });

  it('routes personal symptoms and lab context to activity safety without turning them into genotype findings', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: ['wheezing during exercise', 'daytime sleepiness'],
        labObservations: ['eGFR 45 mL/min/1.73 m²'],
      },
    });
    const domainIds = plan.activity.relevantDomains.map((domain) => domain.id);

    expect(domainIds).toEqual(expect.arrayContaining([
      'respiratory_airway',
      'sleep_recovery',
      'kidney_fluid_electrolytes',
    ]));
    expect(plan.topFindings).toHaveLength(0);
  });

  it('surfaces context-relevant supplement safety rules without inventing a supplement need', () => {
    const cyclePlan = deriveActionablePlan(report([]), { reproductiveContext: 'menstrual_cycle' });
    const cycleRuleIds = cyclePlan.supplementSafety.relevantRules.map((rule) => rule.id);
    expect(cycleRuleIds).toEqual(expect.arrayContaining(['iron', 'magnesium', 'calcium', 'vitamin_b6']));
    expect(cyclePlan.supplements).toHaveLength(0);

    const neutralPlan = deriveActionablePlan(report([]));
    expect(neutralPlan.supplementSafety.relevantRules.map((rule) => rule.id)).not.toContain('iron');
  });

  it('routes an explicitly named B6 product to the neuropathy guardrail', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: ['pyridoxine 50 mg'],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const b6Rule = plan.supplementSafety.relevantRules.find((rule) => rule.id === 'vitamin_b6');
    expect(b6Rule?.avoid.join(' ')).toContain('neurologic symptoms');
    expect(b6Rule?.sources).toContain('nih_ods_vitamin_b6');
  });

  it('keeps PGx medication guidance at the review boundary', () => {
    const plan = deriveActionablePlan({
      ...report([]),
      sections: [{
        ...report([]).sections[0],
        name: 'Pharmacogenomics (PGx)',
        markers: [marker({ gene: 'CYP2C19', rsid: 'rs4244285' })],
      }],
    });
    expect(plan.medication.rules.some((item) => item.includes('raw DNA alone'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('HLA'))).toBe(true);
    expect(plan.pgxGuidance.relevantGenes.map((gene) => gene.id)).toContain('CYP2C19');
    expect(plan.pgxGuidance.relevantGenes.find((gene) => gene.id === 'CYP2C19')?.clinical_next_step)
      .toContain('clinical PGx interpretation');
    expect(plan.pgxGuidance.policy.display_rule).toContain('never assign');
  });

  it('surfaces named medication pathways without turning raw SNPs into prescriptions', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C19', rsid: 'rs4244285' }),
      marker({ gene: 'CYP2D6', rsid: 'rs3892097' }),
      marker({ gene: 'SLCO1B1', rsid: 'rs4149056' }),
      marker({ gene: 'VKORC1', rsid: 'rs9923231' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('clopidogrel');
    expect(medicationText).toContain('warfarin');
    expect(medicationText).toContain('statin');
    expect(medicationText).toContain('codeine or tramadol');
    expect(medicationText).toContain('tamoxifen');
    expect(medicationText).toContain('antidepressant');
    expect(medicationText).toContain('raw DNA');
    expect(medicationText).not.toContain('Start clopidogrel');
    expect(medicationText).not.toContain('Change warfarin dose');

    expect(plan.medicationPathways.map((pathway) => pathway.label)).toEqual(expect.arrayContaining([
      'Clopidogrel',
      'Warfarin',
      'Statins',
      'Codeine & tramadol',
      'Tamoxifen',
      'SSRIs & related antidepressants',
    ]));
    expect(plan.medicationPathways.every((pathway) => !pathway.detail.toLowerCase().includes('raw dna'))).toBe(true);
  });

  it('routes CYP2C9 to NSAID exposure and menstrual-pain safety review', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C9', rsid: 'rs1057910', variant_name: 'CYP2C9*3' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP2C9') && test.name.includes('NSAID'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Kidney function/eGFR'))).toBe(true);
    expect(medicationText).toContain('menstrual or other pain');
    expect(medicationText).toContain('raw-array data');
    expect(medicationText).not.toContain('dose this NSAID');
  });

  it('routes CYP3A5 to transplant-only tacrolimus monitoring', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP3A5', rsid: 'rs776746', variant_name: 'CYP3A5*3' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP3A5') && test.name.includes('tacrolimus'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('whole-blood trough'))).toBe(true);
    expect(medicationText).toContain('transplant care');
    expect(medicationText).toContain('cannot determine a tacrolimus dose');
  });

  it('routes additional high-impact PGx markers to bounded clinical safety review', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'BCHE', rsid: 'rs1803274', variant_name: 'BCHE anesthesia context' }),
      marker({ gene: 'UGT1A1', rsid: 'rs887829', variant_name: 'UGT1A1*28 tag' }),
      marker({ gene: 'NAT2', rsid: 'rs1801280', variant_name: 'NAT2*5 component' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('pseudocholinesterase'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('UGT1A1') && test.name.includes('irinotecan'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('NAT2') && test.name.includes('hydralazine'))).toBe(true);
    expect(medicationText).toContain('succinylcholine');
    expect(medicationText).toContain('irinotecan');
    expect(medicationText).toContain('hydralazine');
    expect(medicationText).not.toContain('Change hydralazine dose to');
  });

  it('routes nutrient supplement safety from related pathway markers', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'DIO2', rsid: 'rs225014' }),
      marker({ gene: 'VDR', rsid: 'rs2228570' }),
      marker({ gene: 'SLC30A8', rsid: 'rs13266634' }),
      marker({ gene: 'SLC12A3', rsid: 'rs13333226' }),
      marker({ gene: 'VKORC1', rsid: 'rs9923231' }),
    ]));
    const ruleIds = plan.supplementSafety.relevantRules.map((rule) => rule.id);

    expect(ruleIds).toEqual(expect.arrayContaining(['iodine', 'calcium', 'zinc', 'potassium', 'vitamin_k']));
  });

  it('applies per-profile medication, supplement, allergy, symptom, and lab context', () => {
    const personalSafetyContext: PersonalSafetyContext = {
      medications: ['Norethindrone progestin-only birth control', 'warfarin'],
      supplements: ['zinc'],
      allergies: ['fish allergy'],
      symptoms: ['mood changes in the late luteal phase'],
      labObservations: ['ferritin 18 ng/mL (2026-05-10)'],
    };
    const plan = deriveActionablePlan(report([]), { personalSafetyContext });
    const medicationText = plan.medication.rules.join(' ');
    const supplementRuleIds = plan.supplementSafety.relevantRules.map((rule) => rule.id);

    expect(medicationText).toContain('contraceptive product');
    expect(supplementRuleIds).toContain('zinc');
    expect(supplementRuleIds).toContain('omega3');
    expect(supplementRuleIds).toContain('vitamin_k');
    expect(plan.personalContext.medications).toEqual(personalSafetyContext.medications);
    expect(plan.personalContext.symptoms).toEqual(personalSafetyContext.symptoms);
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('measured phenotype outranks');
    expect(plan.cycleSupport.relevantDomains).toHaveLength(0);
  });

  it('routes explicit food requirements through resource-backed safety rules', () => {
    const plan = deriveActionablePlan(report([]), {
      reproductiveContext: 'menstrual_cycle',
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        dietaryProfile: {
          hard_exclusions: ['pork'],
          allergies_confirmed: ['fish'],
          allergies_suspected: ['sesame'],
          religious_cultural_profiles: ['halal_compatible'],
          ethical_preference_profiles: [],
          medical_diet_profiles: [],
          goals: ['PMDD_cycle'],
        },
      },
    });
    const ruleIds = plan.foodSafety.relevantRules.map((rule) => rule.id);

    expect(plan.foodSafety.explicitExclusions).toEqual(['pork']);
    expect(plan.foodSafety.confirmedAllergies).toEqual(['fish']);
    expect(plan.foodSafety.suspectedAllergies).toEqual(['sesame']);
    expect(ruleIds).toEqual(expect.arrayContaining([
      'RULE_EXPLICIT_FOOD_EXCLUSION_RESPECT',
      'RULE_ALLERGY_MAJOR_FOOD_STRICT_AVOIDANCE',
      'RULE_HALAL_COMPATIBLE',
      'RULE_PMDD_CYCLE_STABILITY_NUTRITION_OVERLAY',
      'RULE_LOW_IRON_OR_ANEMIA_CONTEXT',
    ]));
    expect(plan.foodSafety.notes.join(' ')).toContain('not automatically confirmed allergies');
    expect(plan.foodSafety.notes.join(' ')).toContain('not a genetic finding');
  });

  it('does not treat a non-food allergy entry as a major food allergy route', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        dietaryProfile: {
          hard_exclusions: [],
          allergies_confirmed: ['penicillin'],
          allergies_suspected: [],
          religious_cultural_profiles: [],
          ethical_preference_profiles: [],
          medical_diet_profiles: [],
          goals: [],
        },
      },
    });

    expect(plan.foodSafety.relevantRules.map((rule) => rule.id)).not.toContain('RULE_ALLERGY_MAJOR_FOOD_STRICT_AVOIDANCE');
  });

  it('suppresses conflicting food suggestions and routes dietary allergies into supplement safety', () => {
    const plan = deriveActionablePlan(report([
      marker({
        gene: 'APOE',
        rsid: 'rs429358',
        interpretation: 'APOE4 context',
      }),
    ]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        dietaryProfile: {
          hard_exclusions: [],
          allergies_confirmed: ['fish'],
          allergies_suspected: [],
          religious_cultural_profiles: [],
          ethical_preference_profiles: [],
          medical_diet_profiles: [],
          goals: [],
        },
      },
    });

    expect(plan.foodSafety.suppressedSuggestions.length).toBeGreaterThan(0);
    expect(plan.foodSafety.suppressedSuggestions.join(' ')).toContain('fish');
    expect(plan.diet.favor.join(' ')).not.toContain('fatty fish');
    expect(plan.supplementSafety.relevantRules.map((rule) => rule.id)).toContain('omega3');
    expect(plan.foodSafety.conflictNotes.join(' ')).toContain('reported as a reaction');
  });

  it('recognizes an exact progestin ingredient without inferring that it is contraception', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: ['norethindrone'],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).toContain('active ingredients');
    expect(medicationText).not.toContain('contraceptive product');
  });

  it('uses structured ingredient context for label review without inferring contraception', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        reproductiveIntake: {
          active_ingredients: 'norethindrone 0.35 mg',
          question_or_belief_to_verify: 'I think this is progesterone-only',
        },
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).toContain('active ingredients');
    expect(medicationText).not.toContain('contraceptive product');
    expect(plan.personalContext.reproductiveIntake?.active_ingredients).toBe('norethindrone 0.35 mg');
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('Structured cycle and hormone details');
  });

  it('keeps diary observations separate from genotype guidance', () => {
    const plan = deriveActionablePlan(report([]), {
      personalSafetyContext: {
        medications: [],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
        cycleDiary: [{ id: 'day-1', values: { entry_date: '2026-08-01', mood_behavior_score: '3' } }],
      },
    });

    expect(plan.personalContext.cycleDiary?.[0].values.mood_behavior_score).toBe('3');
    expect(plan.personalContext.priorityNotes.join(' ')).toContain('Missing entries are not symptom-free days');
    expect(plan.cycleSupport.diaryReview?.metrics.find((metric) => metric.id === 'mood_behavior_impact'))
      .toEqual(expect.objectContaining({ recorded_days: 1, elevated_days: 1 }));
    expect(plan.cycleSupport.diaryReview?.notes.join(' ')).toContain('hormone cause');
    expect(plan.supplements).toHaveLength(0);
  });

  it('separates general hormone therapy from contraceptive composition warnings', () => {
    const plan = deriveActionablePlan(report([]), {
      reproductiveContext: 'hormone_therapy_context',
      personalSafetyContext: {
        medications: ['testosterone cypionate'],
        supplements: [],
        allergies: [],
        symptoms: [],
        labObservations: [],
      },
    });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('current hormone-therapy product');
    expect(medicationText).not.toContain('contraceptive product');
    expect(plan.cycleSupport.relevantDomains.map((domain) => domain.id)).toEqual([
      'exogenous_hormone_medication_context',
      'measured_hormone_context',
      'general_symptom_day_support',
    ]);
  });

  it('keeps preconception medication review separate from fertility prediction', () => {
    const plan = deriveActionablePlan(report([]), { reproductiveContext: 'preconception_fertility' });
    const medicationText = plan.medication.rules.join(' ');

    expect(medicationText).toContain('planning pregnancy or pursuing fertility care');
    expect(plan.cycleSupport.relevantDomains.map((domain) => domain.id)).toContain('preconception_fertility_context');
    expect(plan.cycleSupport.relevantDomains.find((domain) => domain.id === 'preconception_fertility_context')?.context)
      .toContain('cannot measure ovarian reserve');
  });

  it('keeps cycle-linked pain and migraine support at the clinical-context boundary', () => {
    const plan = deriveActionablePlan(report([]), { reproductiveContext: 'cycle_linked_pain_headache' });
    const domain = plan.cycleSupport.relevantDomains.find((item) => item.id === 'cycle_linked_pain_headache_context');

    expect(domain?.context).toContain('cannot diagnose');
    expect(domain?.support_options.join(' ')).toContain('DNA result');
    expect(plan.medication.rules.join(' ')).toContain('contraceptive product');
  });

  it('connects confirmed thrombophilia context to contraceptive review without prescribing a change', () => {
    const plan = deriveActionablePlan({
      ...report([]),
      sections: [{
        ...report([]).sections[0],
        name: 'Cardiovascular Health',
        markers: [marker({
          gene: 'F5',
          rsid: 'rs6025',
          severity_class: 'confirmation_required',
          clinical_confirmation_required: true,
        })],
      }],
    });
    expect(plan.medication.rules.some((item) => item.includes('whether it contains estrogen'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('Factor V Leiden'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('do not change medication'))).toBe(true);
  });

  it('turns metabolic markers into conditional glucose and activity follow-up', () => {
    const base = report([marker({
      rsid: 'rs7903146',
      gene: 'TCF7L2',
      interpretation: 'Common TCF7L2 glucose and insulin-secretion association',
    })]);
    const plan = deriveActionablePlan({
      ...base,
      sections: [{ ...base.sections[0], name: 'Metabolic Health' }],
    });

    expect(plan.diet.favor.some((item) => /fiber-rich foods/i.test(item))).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'HbA1c')).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Fasting plasma glucose')).toBe(true);
    expect(plan.activity.relevantDomains.some((domain) => domain.id === 'metabolic')).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('diabetes diagnosis');
    expect(plan.advancedGuidance.join(' ')).toContain('diabetes diagnosis');
  });

  it('uses LPA context to request phenotype testing without prescribing therapy', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs10455872',
      gene: 'LPA',
      interpretation: 'LPA lipoprotein(a) association; measured Lp(a) remains actionable',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Lipoprotein(a)') && test.category === 'Heart & lipids')).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('ApoB'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('statin'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('raw DNA'))).toBe(true);
  });

  it('routes thyroid pathway markers to measured thyroid follow-up', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs225014',
      gene: 'DIO2',
      interpretation: 'Thyroid conversion context marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name === 'TSH')).toBe(true);
    expect(plan.labTests.some((test) => test.name === 'Free T4')).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('High-dose iodine');
    expect(plan.advancedGuidance.join(' ')).toContain('High-dose iodine');
    expect(plan.medication.rules.some((item) => item.includes('levothyroxine'))).toBe(true);
  });

  it('routes sleep markers to symptom tracking and sleep-apnea evaluation', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs1801260',
      gene: 'CLOCK',
      interpretation: 'Circadian timing association marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Sleep study'))).toBe(true);
    expect(plan.diet.favor.join(' ')).not.toContain('consistent sleep opportunity');
    expect(plan.advancedGuidance.join(' ')).toContain('consistent sleep opportunity');
    expect(plan.medication.rules.some((item) => item.includes('chronotype'))).toBe(true);
  });

  it('keeps food-allergy actionability exposure-led rather than genotype-led', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs10156191',
      gene: 'AOC1',
      interpretation: 'Histamine-processing pathway context marker; not diagnostic.',
    })]));

    expect(plan.labTests.some((test) => test.name.includes('Allergist-directed'))).toBe(true);
    expect(plan.diet.avoid.join(' ')).not.toContain('Permanent food elimination');
    expect(plan.advancedGuidance.join(' ')).toContain('Permanent food elimination');
    expect(plan.medication.rules.some((item) => item.includes('AOC1'))).toBe(true);
  });

  it('builds a categorized allergy map with linked DNA meaning and exposure examples', () => {
    const allergyReport = report([
      marker({ rsid: 'rs20541', gene: 'IL13', variant_name: 'IL13 allergic inflammation locus' }),
      marker({ rsid: 'rs11591147_FLG_LOF_PANEL', gene: 'FLG', variant_name: 'Skin barrier panel' }),
      marker({ rsid: 'PANEL_FOOD_ALLERGY_CONTEXT', gene: 'HLA/IL4/IL13/FLG/STAT6', variant_name: 'Food allergy context panel' }),
      marker({
        rsid: 'HLA-B*57:01',
        gene: 'HLA-B',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        variant_name: 'HLA-B*57:01',
      }),
    ]);

    const allergy = deriveAllergySensitivityGuidance(allergyReport);
    const plan = deriveActionablePlan(allergyReport);

    expect(allergy.hasDnaSignal).toBe(true);
    expect(allergy.dnaContexts.map((context) => context.id)).toEqual(expect.arrayContaining([
      'atopy_ige',
      'skin_barrier',
      'food_allergy_context',
    ]));
    const atopy = allergy.dnaContexts.find((context) => context.id === 'atopy_ige');
    expect(atopy?.signal_label).toBe('Immune-allergy signal');
    expect(atopy?.relevance).toContain('seasonal allergies');
    expect(atopy?.matched_genes).toContain('IL13');
    expect(atopy?.matched_marker_link_ids).toContain('test:marker');
    expect(allergy.medicationSafety.map((route) => route.id)).toContain('abacavir_hypersensitivity');
    const abacavir = allergy.medicationSafety.find((route) => route.id === 'abacavir_hypersensitivity');
    expect(abacavir?.signal_label).toContain('Medication alert');
    expect(abacavir?.relevance).toContain('abacavir');
    expect(abacavir?.matched_genes).toContain('HLA-B');
    expect(allergy.exposureChecklists).toHaveLength(5);
    expect(allergy.exposureChecklists.find((group) => group.id === 'foods')?.items.map((item) => item.label))
      .toEqual(expect.arrayContaining(['Peanut', 'Tree nuts', 'Sesame']));
    expect(allergy.exposureChecklists.find((group) => group.id === 'physical_triggers')?.items.map((item) => item.label))
      .toEqual(expect.arrayContaining(['Cold', 'Sun / UV']));
    expect(plan.allergy).toEqual(allergy);
  });

  it('does not invent allergy contexts from unrelated markers', () => {
    const allergy = deriveAllergySensitivityGuidance(report([marker({ rsid: 'rs1801133', gene: 'MTHFR' })]));

    expect(allergy.hasDnaSignal).toBe(false);
    expect(allergy.dnaContexts).toHaveLength(0);
    expect(allergy.medicationSafety).toHaveLength(0);
    expect(allergy.exposureChecklists.length).toBeGreaterThan(0);
  });

  it('counts each linked allergy finding once when a report repeats it across sections', () => {
    const repeated = marker({
      link_id: 'test:allergy-repeat',
      rsid: 'rs20541',
      gene: 'IL13',
      variant_name: 'IL13 allergic inflammation locus',
    });
    const base = report([]);
    const allergy = deriveAllergySensitivityGuidance({
      ...base,
      sections: [
        { ...base.sections[0], name: 'Allergy A', markers: [repeated] },
        { ...base.sections[0], name: 'Allergy B', markers: [repeated] },
      ],
    });
    const atopy = allergy.dnaContexts.find((context) => context.id === 'atopy_ige');

    expect(atopy?.matched_marker_count).toBe(1);
    expect(atopy?.matched_marker_ids).toEqual(['rs20541']);
    expect(atopy?.matched_marker_link_ids).toEqual(['test:allergy-repeat']);
    expect(allergy.matchedMarkerCount).toBe(1);
  });

  it('connects high-impact PGx findings to drug-specific clinical review', () => {
    const plan = deriveActionablePlan(report([
      marker({
        rsid: 'rs3918290',
        gene: 'DPYD',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'DPYD*2A context marker; confirm clinically before fluoropyrimidine therapy.',
      }),
      marker({
        rsid: 'rs116855232',
        gene: 'NUDT15',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'NUDT15 thiopurine context marker; confirm clinically.',
      }),
      marker({
        rsid: 'HLA-B*57:01',
        gene: 'HLA-B',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'HLA drug-hypersensitivity context marker; confirm with clinical HLA typing.',
      }),
      marker({
        rsid: 'PANEL_G6PD_DEFICIENCY',
        gene: 'G6PD',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'G6PD deficiency context panel; confirm clinically.',
      }),
      marker({
        rsid: 'PANEL_RYR1_CACNA1S_MALIGNANT_HYPERTHERMIA',
        gene: 'RYR1/CACNA1S',
        severity_class: 'confirmation_required',
        clinical_confirmation_required: true,
        interpretation: 'Malignant-hyperthermia susceptibility context panel; confirm clinically.',
      }),
    ]));

    expect(plan.labTests.some((test) => test.name.includes('DPYD'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('TPMT/NUDT15'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HLA'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('G6PD'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('RYR1/CACNA1S'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('fluorouracil'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('thiopurine'))).toBe(true);
    expect(plan.medication.rules.some((item) => item.includes('abacavir'))).toBe(true);
  });

  it('routes exact HLA alleles to the matching drug pathway without cross-triggering allele rules', () => {
    const abacavirPlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-B*57:01',
      gene: 'HLA-B',
      variant_name: 'HLA-B*57:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-B*57:01 context; confirm with clinical HLA typing.',
    })]));
    const allopurinolPlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-B*58:01',
      gene: 'HLA-B',
      variant_name: 'HLA-B*58:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-B*58:01 context; confirm with clinical HLA typing.',
    })]));
    const carbamazepinePlan = deriveActionablePlan(report([marker({
      rsid: 'HLA-A*31:01',
      gene: 'HLA-A',
      variant_name: 'HLA-A*31:01',
      severity_class: 'confirmation_required',
      clinical_confirmation_required: true,
      interpretation: 'HLA-A*31:01 context; confirm with clinical HLA typing.',
    })]));

    expect(abacavirPlan.medication.rules.some((item) => item.includes('validated clinical HLA-B*57:01 result'))).toBe(true);
    expect(abacavirPlan.labTests.some((test) => test.name.includes('HLA-B*57:01') && test.name.includes('abacavir'))).toBe(true);
    expect(allopurinolPlan.medication.rules.some((item) => item.includes('validated HLA-B*58:01 status'))).toBe(true);
    expect(allopurinolPlan.labTests.some((test) => test.name.includes('HLA-B*58:01') && test.name.includes('allopurinol'))).toBe(true);
    expect(allopurinolPlan.medication.rules.some((item) => item.includes('validated clinical HLA-B*57:01 result'))).toBe(false);
    expect(carbamazepinePlan.medication.rules.some((item) => item.includes('carbamazepine or oxcarbazepine'))).toBe(true);
    expect(carbamazepinePlan.labTests.some((test) => test.name.includes('HLA-B*15:02') && test.name.includes('HLA-A*31:01'))).toBe(true);
  });

  it('routes additional clinical PGx pathways without turning them into raw-array prescriptions', () => {
    const plan = deriveActionablePlan(report([
      marker({ gene: 'CYP2C19', rsid: 'rs4244285' }),
      marker({ gene: 'CYP2B6', rsid: 'rs3745274' }),
      marker({ gene: 'CYP2C9', rsid: 'rs1057910' }),
      marker({ gene: 'HLA-B', rsid: 'HLA-B*15:02' }),
    ]));
    const medicationText = plan.medication.rules.join(' ');

    expect(plan.labTests.some((test) => test.name.includes('CYP2C19') && test.name.includes('omeprazole'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('CYP2B6') && test.name.includes('efavirenz'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('CYP2C9') && test.name.includes('phenytoin'))).toBe(true);
    expect(plan.labTests.some((test) => test.name.includes('HLA-B*15:02') && test.name.includes('phenytoin'))).toBe(true);
    expect(medicationText).toContain('proton-pump inhibitor');
    expect(medicationText).toContain('efavirenz');
    expect(medicationText).toContain('phenytoin');
    expect(medicationText).not.toContain('Change phenytoin dose to');
  });

  it('does not infer reproductive applicability from hormone marker text', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'rs2234693',
      gene: 'ESR1',
      variant_name: 'estrogen receptor alpha marker',
      sex_scope: 'menstrual_cycle_context',
      interpretation: 'Hormone context marker; not diagnostic.',
      effect_direction: 'context_dependent',
    })]));

    expect(plan.cycleSupport.relevantDomains).toHaveLength(0);
    expect(plan.medication.rules.some((item) => item.includes('Raw DNA cannot identify a person'))).toBe(false);
  });

  it('supports an explicitly selected androgen context without showing menstrual guidance', () => {
    const plan = deriveActionablePlan(report([marker({
      rsid: 'PANEL_AR_CAG_REPEAT_CALLOUT',
      gene: 'AR',
      sex_scope: 'androgen_reproductive_context',
      effect_direction: 'context_dependent',
    })]), { reproductiveContext: 'androgen_reproductive' });

    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'androgen_reproductive_context')).toBe(true);
    expect(plan.cycleSupport.relevantDomains.some((domain) => domain.id === 'cycle_phase_and_symptom_timing')).toBe(false);
  });
});
