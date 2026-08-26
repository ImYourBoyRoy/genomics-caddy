import { describe, expect, it } from 'vitest';
import { buildClinicalHandoffMarkdown } from './aiExport';
import type { PersonalSafetyContext } from './personalSafetyContext';

describe('clinical handoff export', () => {
  it('includes structured cycle and hormone context as self-reported, non-genetic context', () => {
    const personalSafetyContext: PersonalSafetyContext = {
      medications: [],
      supplements: [],
      allergies: [],
      symptoms: [],
      labObservations: [],
      reproductiveIntake: {
        active_ingredients: 'norethindrone 0.35 mg',
        symptom_onset_timing: 'at the end of the luteal phase',
        question_or_belief_to_verify: 'I think estrogen spikes; please verify',
        clinical_questions_or_findings: 'possible adenomyosis; ask about appropriate evaluation',
      },
    };

    const markdown = buildClinicalHandoffMarkdown(
      [],
      { name: 'Example', genetic_sex: 'unknown' } as never,
      'test-model',
      {},
      personalSafetyContext,
      'system prompt',
      { sections: [] },
    );

    expect(markdown).toContain('Structured cycle and hormone context (self-reported)');
    expect(markdown).toContain('Active ingredient(s) exactly as labeled');
    expect(markdown).toContain('norethindrone 0.35 mg');
    expect(markdown).toContain('possible adenomyosis');
    expect(markdown).toContain('does not establish a diagnosis, hormone level');
  });
});
