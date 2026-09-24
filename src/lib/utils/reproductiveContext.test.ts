import { describe, expect, it } from 'vitest';
import hormonesReproductive from '../marker-packs/hormones_reproductive.json';
import type { GeneratedReport } from '../types/genomics';
import {
  activeReproductiveContextIds,
  cycleSupportEvidenceLayersForContextIds,
  cycleSupportDomainsForPersonalContext,
  reproductiveDnaCoverageForReport,
  reproductiveContextIdsForProfileText,
  reproductiveContextIdsForPersonalContext,
  reproductiveContextOptionIsSuggestedForGeneticSex,
  reproductiveMarkerContextIds,
  reproductiveMarkerContextRank,
  reproductiveSectionHasContext,
  selectedReproductiveContextOption,
} from './reproductiveContext';

describe('reproductive marker context routing', () => {
  it('groups, but does not hide, context options for known chromosome patterns', () => {
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('menstrual_cycle', 'Female')).toBe(true);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('androgen_reproductive', 'Female')).toBe(false);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('androgen_reproductive', 'Male')).toBe(true);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('menstrual_cycle', 'Male')).toBe(false);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('menstrual_cycle', 'Male-like (XY chromosome pattern)')).toBe(false);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('androgen_reproductive', 'Female-like (XX chromosome pattern; no Y calls observed)')).toBe(false);
    expect(reproductiveContextOptionIsSuggestedForGeneticSex('menstrual_cycle', 'Unknown')).toBe(true);
  });

  it('prioritizes menstrual-cycle markers without hiding other findings', () => {
    expect(reproductiveMarkerContextRank('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY', 'menstrual_cycle')).toBe(3);
    expect(reproductiveMarkerContextRank('rs2234693', 'menstrual_cycle')).toBe(2);
    expect(reproductiveMarkerContextRank('PANEL_AR_CAG_REPEAT_CALLOUT', 'menstrual_cycle')).toBe(0);
    expect(reproductiveMarkerContextIds('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY')).toContain('menstrual_cycle');
  });

  it('routes the same reproductive pack symmetrically across contexts', () => {
    expect(reproductiveMarkerContextRank('PANEL_AR_CAG_REPEAT_CALLOUT', 'androgen_reproductive')).toBe(3);
    expect(reproductiveMarkerContextRank('rs523349', 'androgen_reproductive')).toBe(3);
    expect(reproductiveMarkerContextRank('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY', 'androgen_reproductive')).toBe(0);
    expect(reproductiveMarkerContextRank('PANEL_ADENOMYOSIS_RESEARCH_GAP', 'uterine_pelvic')).toBe(3);
  });

  it('exposes cyclic mood symptoms as phenotype context without turning them into a diagnosis', () => {
    const option = selectedReproductiveContextOption('cyclic_mood_symptoms');

    expect(option?.domain_ids).toEqual([
      'cycle_phase_and_symptom_timing',
      'measured_hormone_context',
      'pmdd_like_mood_symptoms',
      'contraceptive_product_context',
      'contraception_thrombophilia_context',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA');
    expect(reproductiveMarkerContextRank('PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY', 'cyclic_mood_symptoms')).toBe(3);
    expect(reproductiveMarkerContextRank('GUARDRAIL_PMDD_STEROID_SENSITIVITY_NOT_LEVELS', 'cyclic_mood_symptoms')).toBe(3);
    expect(reproductiveMarkerContextRank('GUARDRAIL_CYCLIC_BEHAVIOR_IS_PHENOTYPE', 'cyclic_mood_symptoms')).toBe(3);
  });

  it('corrects the late-luteal estrogen-spike assumption without inferring a current level', () => {
    const phaseGuardrail = hormonesReproductive.markers.find(
      (marker) => marker.rsid === 'GUARDRAIL_MENSTRUAL_PHASE_HORMONE_DIRECTION',
    );

    expect(phaseGuardrail?.interpretation).toContain('both generally decline');
    expect(phaseGuardrail?.do_not_claim).toContain('estrogen spike at the start of menstruation');
    expect(phaseGuardrail?.raw_dna_limitation).toContain('DNA cannot measure a current hormone level');
  });

  it('routes an explicit adenomyosis concern to clinical workup rather than a DNA call', () => {
    const option = selectedReproductiveContextOption('suspected_adenomyosis');

    expect(option?.domain_ids).toEqual([
      'heavy_bleeding_pelvic_pain',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toEqual([]);
    expect(reproductiveMarkerContextRank('GUARDRAIL_ADENOMYOSIS_NOT_CALLABLE_FROM_CONSUMER_SNP', 'suspected_adenomyosis')).toBe(3);
    expect(reproductiveMarkerContextRank('PANEL_ADENOMYOSIS_RESEARCH_GAP', 'suspected_adenomyosis')).toBe(3);
    expect(reproductiveMarkerContextRank('rs184700', 'suspected_adenomyosis')).toBe(3);
    expect(reproductiveMarkerContextRank('rs6025', 'menstrual_cycle')).toBe(3);
    expect(reproductiveMarkerContextIds('rs6025')).toContain('menstrual_cycle');
  });

  it('keeps unknown or unselected context non-directive', () => {
    expect(selectedReproductiveContextOption('')).toBeUndefined();
    expect(reproductiveMarkerContextRank('rs2234693', '')).toBe(0);
    expect(reproductiveMarkerContextRank('future_marker', 'menstrual_cycle')).toBe(1);
  });

  it('uses resource-authored section keywords and marker contexts for prioritization', () => {
    expect(reproductiveSectionHasContext('Menstrual Cycle, Hormones & Reproductive Context')).toBe(true);
    expect(reproductiveSectionHasContext('Sleep & Circadian')).toBe(false);
    expect(reproductiveSectionHasContext('Unclassified section', ['PANEL_PMDD_OVARIAN_STEROID_SENSITIVITY'])).toBe(true);
  });

  it('routes exogenous hormone therapy without assuming identity or anatomy', () => {
    const option = selectedReproductiveContextOption('hormone_therapy_context');

    expect(option?.domain_ids).toEqual([
      'exogenous_hormone_medication_context',
      'measured_hormone_context',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('HORMONE_THERAPY_COMPOSITION_NOT_IN_DNA');
  });

  it('routes menopause context to transition guardrails and research-only markers', () => {
    const option = selectedReproductiveContextOption('menopause_hormone_therapy');

    expect(option?.domain_ids).toContain('menopause_transition_support');
    expect(reproductiveMarkerContextRank('GUARDRAIL_MENOPAUSE_STATUS_NOT_IN_DNA', 'menopause_hormone_therapy')).toBe(3);
    expect(reproductiveMarkerContextRank('PANEL_AGE_AT_NATURAL_MENOPAUSE_RESEARCH', 'menopause_hormone_therapy')).toBe(3);
    expect(reproductiveContextIdsForProfileText('perimenopause hot flashes and night sweats')).toContain('menopause_hormone_therapy');
  });

  it('keeps pregnancy and lactation status as explicit clinical context', () => {
    const option = selectedReproductiveContextOption('pregnancy_postpartum');

    expect(option?.domain_ids).toContain('pregnancy_postpartum_lactation_context');
    expect(option?.medication_rule_ids).toContain('PREGNANCY_LACTATION_MEDICATION_REVIEW_NOT_IN_DNA');
    expect(option?.domain_ids).toContain('postpartum_recovery_support');
  });

  it('routes explicit preconception and fertility goals without making fertility claims', () => {
    const option = selectedReproductiveContextOption('preconception_fertility');

    expect(option?.domain_ids).toEqual([
      'preconception_fertility_context',
      'measured_hormone_context',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('PRECONCEPTION_MEDICATION_REVIEW_NOT_IN_DNA');
    expect(reproductiveMarkerContextRank('GUARDRAIL_FERTILITY_NOT_PREDICTABLE_FROM_CONSUMER_DNA', 'preconception_fertility')).toBe(3);
  });

  it('maps explicit all-body profile goals to only their resource-authored context route', () => {
    expect(reproductiveContextIdsForProfileText('prostate screening and testosterone questions')).toEqual([
      'androgen_reproductive',
    ]);
    expect(reproductiveContextIdsForProfileText('cycle timing, menstrual migraine, and late luteal mood')).toEqual([
      'menstrual_cycle',
      'cycle_linked_pain_headache',
      'cyclic_mood_symptoms',
    ]);
    expect(reproductiveContextIdsForProfileText('general wellness')).toEqual([]);
  });

  it('routes explicit cycle-linked pain and migraine context without using DNA diagnostically', () => {
    const option = selectedReproductiveContextOption('cycle_linked_pain_headache');

    expect(option?.domain_ids).toEqual([
      'cycle_linked_pain_headache_context',
      'heavy_bleeding_pelvic_pain',
      'contraception_thrombophilia_context',
      'cycle_nutrition_activity_context',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA');
    expect(reproductiveMarkerContextRank('GUARDRAIL_MENSTRUAL_PHASE_HORMONE_DIRECTION', 'cycle_linked_pain_headache')).toBe(3);
  });

  it('selects resource-authored evidence layers for menstrual and adenomyosis questions', () => {
    const menstrualLayers = cycleSupportEvidenceLayersForContextIds(['menstrual_cycle']);
    const adenomyosisLayers = cycleSupportEvidenceLayersForContextIds(['suspected_adenomyosis']);

    expect(menstrualLayers.map((layer) => layer.id)).toEqual([
      'current_hormone_state',
      'natural_cycle_timing',
      'genetic_pathway_context',
      'hormone_product_label',
      'thrombophilia_contraception_context',
      'adenomyosis_structural_workup',
    ]);
    expect(menstrualLayers.find((layer) => layer.id === 'natural_cycle_timing')?.summary)
      .toContain('generally decline');
    expect(adenomyosisLayers.find((layer) => layer.id === 'adenomyosis_structural_workup')?.next_step)
      .toContain('transvaginal ultrasound');
  });

  it('reports reproductive marker coverage without returning genotypes', () => {
    const report = {
      sections: [{
        markers: [
          { rsid: 'rs2234693', assertion_status: 'Verified', interpretation_allowed: true, user_genotype: 'AA' },
          { rsid: 'rs6166', assertion_status: 'UnverifiedOrientation', interpretation_allowed: false, user_genotype: 'AG' },
          { rsid: 'rs9340799', assertion_status: 'NotInRawFile', interpretation_allowed: false, user_genotype: '--' },
        ],
      }],
    } as unknown as GeneratedReport;

    const coverage = reproductiveDnaCoverageForReport(report, ['menstrual_cycle']);

    expect(coverage).toEqual(expect.objectContaining({
      tracked_marker_count: 44,
      present_marker_count: 2,
      callable_marker_count: 1,
      unknown_marker_count: 42,
    }));
    expect(coverage).not.toHaveProperty('genotype');
  });

  it('activates only resource-declared contexts from self-reported intake or diary data', () => {
    const hormoneContext = {
      reproductiveIntake: {
        active_ingredients: 'norethindrone 0.35 mg',
        hormone_lab_timing_context: 'cycle day 24; before morning dose',
      },
    };
    const contextIds = reproductiveContextIdsForPersonalContext(hormoneContext);

    expect(contextIds).toEqual(expect.arrayContaining([
      'menstrual_cycle',
      'cyclic_mood_symptoms',
      'ovarian_reproductive',
      'menopause_hormone_therapy',
      'hormone_therapy_context',
      'preconception_fertility',
    ]));
    expect(activeReproductiveContextIds('', hormoneContext)).toEqual(contextIds);
    expect(activeReproductiveContextIds('androgen_reproductive', hormoneContext)).toEqual(['androgen_reproductive']);

    const diaryDomains = cycleSupportDomainsForPersonalContext({
      cycleDiary: [{ id: 'day-1' }],
    });
    expect(diaryDomains.map((domain) => domain.id)).toEqual(expect.arrayContaining([
      'cycle_phase_and_symptom_timing',
      'pmdd_like_mood_symptoms',
      'heavy_bleeding_pelvic_pain',
    ]));
  });
});
