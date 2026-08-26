import { describe, expect, it } from 'vitest';
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

  it('keeps unknown or unselected context non-directive', () => {
    expect(selectedReproductiveContextOption('')).toBeUndefined();
    expect(reproductiveMarkerContextRank('rs2234693', '')).toBe(0);
    expect(reproductiveMarkerContextRank('future_marker', 'menstrual_cycle')).toBe(1);
  });

  it('routes exogenous hormone therapy without assuming identity or anatomy', () => {
    const option = selectedReproductiveContextOption('hormone_therapy_context');

    expect(option?.domain_ids).toEqual([
      'exogenous_hormone_medication_context',
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
      'general_symptom_day_support',
    ]);
    expect(option?.medication_rule_ids).toContain('PRECONCEPTION_MEDICATION_REVIEW_NOT_IN_DNA');
    expect(reproductiveMarkerContextRank('GUARDRAIL_FERTILITY_NOT_PREDICTABLE_FROM_CONSUMER_DNA', 'preconception_fertility')).toBe(3);
  });
});
