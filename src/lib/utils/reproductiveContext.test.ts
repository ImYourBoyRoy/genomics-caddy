import { describe, expect, it } from 'vitest';
import hormonesReproductive from '../marker-packs/hormones_reproductive.json';
import {
  reproductiveMarkerContextIds,
  reproductiveMarkerContextRank,
  selectedReproductiveContextOption,
} from './reproductiveContext';

describe('reproductive marker context routing', () => {
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
  });

  it('keeps unknown or unselected context non-directive', () => {
    expect(selectedReproductiveContextOption('')).toBeUndefined();
    expect(reproductiveMarkerContextRank('rs2234693', '')).toBe(0);
    expect(reproductiveMarkerContextRank('future_marker', 'menstrual_cycle')).toBe(1);
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

  it('keeps pregnancy and lactation status as explicit clinical context', () => {
    const option = selectedReproductiveContextOption('pregnancy_postpartum');

    expect(option?.domain_ids).toContain('pregnancy_postpartum_lactation_context');
    expect(option?.medication_rule_ids).toContain('PREGNANCY_LACTATION_MEDICATION_REVIEW_NOT_IN_DNA');
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

  it('routes explicit cycle-linked pain and migraine context without using DNA diagnostically', () => {
    const option = selectedReproductiveContextOption('cycle_linked_pain_headache');

    expect(option?.domain_ids).toEqual([
      'cycle_linked_pain_headache_context',
      'heavy_bleeding_pelvic_pain',
      'cycle_nutrition_activity_context',
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('CONTRACEPTIVE_COMPOSITION_NOT_IN_DNA');
    expect(reproductiveMarkerContextRank('GUARDRAIL_MENSTRUAL_PHASE_HORMONE_DIRECTION', 'cycle_linked_pain_headache')).toBe(3);
  });
});
