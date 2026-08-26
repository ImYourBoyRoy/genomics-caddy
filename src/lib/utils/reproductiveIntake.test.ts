import { describe, expect, it } from 'vitest';
import {
  REPRODUCTIVE_INTAKE_SCHEMA,
  hasReproductiveIntake,
  normalizeReproductiveIntake,
  populatedReproductiveIntake,
  reproductiveIntakeGroupsForContext,
} from './reproductiveIntake';

describe('resource-backed reproductive intake', () => {
  it('keeps declared fields, trims values, and drops unknown fields', () => {
    const normalized = normalizeReproductiveIntake({
      active_ingredients: '  norethindrone   0.35 mg  ',
      mood_behavior_symptoms: ' Just leave me alone; avoidant near bleeding ',
      need_for_space_communication_pattern: ' Requests a check-in later; quiet helps ',
      unknown_private_field: 'should not persist',
    });

    expect(normalized).toEqual({
      active_ingredients: 'norethindrone 0.35 mg',
      mood_behavior_symptoms: 'Just leave me alone; avoidant near bleeding',
      need_for_space_communication_pattern: 'Requests a check-in later; quiet helps',
    });
    expect(normalized).not.toHaveProperty('unknown_private_field');
  });

  it('bounds free-text values and preserves schema display order', () => {
    const normalized = normalizeReproductiveIntake({
      clinical_questions_or_findings: 'x'.repeat(600),
      most_recent_bleeding_start: '2026-08-01',
    });

    expect(normalized.clinical_questions_or_findings).toHaveLength(500);
    expect(populatedReproductiveIntake(normalized).map(({ field }) => field.id)).toEqual([
      'most_recent_bleeding_start',
      'clinical_questions_or_findings',
    ]);
    expect(hasReproductiveIntake(normalized)).toBe(true);
    expect(hasReproductiveIntake({})).toBe(false);
  });

  it('contains exact product review and adenomyosis follow-up fields', () => {
    const fieldIds = REPRODUCTIVE_INTAKE_SCHEMA.fields.map((field) => field.id);
    expect(fieldIds).toEqual(expect.arrayContaining([
      'active_ingredients',
      'route_dose_schedule',
      'question_or_belief_to_verify',
      'hormone_lab_timing_context',
      'clinical_questions_or_findings',
    ]));
    expect(REPRODUCTIVE_INTAKE_SCHEMA.do_not_infer.join(' ')).toContain('adenomyosis');
  });

  it('shows cycle and androgen groups according to explicit context without hiding common review fields', () => {
    const menstrualGroups = reproductiveIntakeGroupsForContext('menstrual_cycle').map((group) => group.id);
    const androgenGroups = reproductiveIntakeGroupsForContext('androgen_reproductive').map((group) => group.id);
    const unselectedGroups = reproductiveIntakeGroupsForContext().map((group) => group.id);

    expect(menstrualGroups).toContain('cycle_timing_symptoms');
    expect(menstrualGroups).not.toContain('androgen_body_context');
    expect(androgenGroups).toContain('androgen_body_context');
    expect(androgenGroups).not.toContain('cycle_timing_symptoms');
    expect(menstrualGroups).toContain('hormone_product_review');
    expect(androgenGroups).toContain('hormone_product_review');
    expect(unselectedGroups).toEqual(REPRODUCTIVE_INTAKE_SCHEMA.groups.map((group) => group.id));
  });
});
