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
      dietaryProfile: {
        hard_exclusions: ['pork'],
        allergies_confirmed: ['fish'],
        allergies_suspected: [],
        religious_cultural_profiles: ['halal_compatible'],
        ethical_preference_profiles: [],
        medical_diet_profiles: [],
        goals: ['PMDD_cycle'],
      },
      reproductiveIntake: {
        active_ingredients: 'norethindrone 0.35 mg',
        symptom_onset_timing: 'at the end of the luteal phase',
        question_or_belief_to_verify: 'I think estrogen spikes; please verify',
        clinical_questions_or_findings: 'possible adenomyosis; ask about appropriate evaluation',
      },
      cycleDiary: [{
        id: 'day-1',
        values: {
          entry_date: '2026-08-01',
          mood_behavior_score: '3',
          mood_behavior_notes: 'needs quiet',
        },
      }],
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
    expect(markdown).toContain('Explicit food requirements (self-reported)');
    expect(markdown).toContain('pork');
    expect(markdown).toContain('fish');
    expect(markdown).toContain('Active ingredient(s) exactly as labeled');
    expect(markdown).toContain('norethindrone 0.35 mg');
    expect(markdown).toContain('possible adenomyosis');
    expect(markdown).toContain('does not establish a diagnosis, hormone level');
    expect(markdown).toContain('Daily cycle and symptom diary (self-reported)');
    expect(markdown).toContain('needs quiet');
    expect(markdown).toContain('missing entries are not symptom-free days');
  });

  it('keeps the exported chromosome-pattern label concise when stored data uses a legacy description', () => {
    const markdown = buildClinicalHandoffMarkdown(
      [],
      { name: 'Example', genetic_sex: 'Male-like (XY chromosome pattern)' } as never,
      'test-model',
      {},
      undefined,
      'system prompt',
      { sections: [] },
    );

    expect(markdown).toContain('* **Chromosome pattern:** Male-like');
    expect(markdown).not.toContain('* **Sex:**');
  });

  it('always includes raw genotype calls in the legacy clinician handoff path', () => {
    const markdown = buildClinicalHandoffMarkdown(
      [],
      { name: 'Example', genetic_sex: 'unknown' } as never,
      'test-model',
      {},
      undefined,
      'system prompt without a genotype section',
      {
        sections: [{
          name: 'Technical findings',
          markers: [{ rsid: 'rs-raw', gene: 'RAW1', user_genotype: 'AG', normalized_genotype: 'AG' }],
        }],
      },
    );

    expect(markdown).toContain('## Exact raw genotype calls');
    expect(markdown).toContain('raw call: AG');
  });
});
