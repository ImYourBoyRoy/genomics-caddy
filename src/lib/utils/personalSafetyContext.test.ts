import { describe, expect, it } from 'vitest';
import {
  normalizePersonalSafetyContext,
  parseContextList,
  personalSafetyContextStorageKey,
} from './personalSafetyContext';

describe('personal safety context', () => {
  it('parses one-item-per-line context without interpreting medical text', () => {
    expect(parseContextList(' fish allergy\n\nFish allergy; magnesium ')).toEqual([
      'fish allergy',
      'magnesium',
    ]);
  });

  it('normalizes legacy string values and rejects malformed fields', () => {
    expect(normalizePersonalSafetyContext({
      medications: 'norethindrone\nlevothyroxine',
      supplements: ['magnesium', 'MAGNESIUM'],
      allergies: null,
      symptoms: 42,
      labObservations: ['ferritin 18 ng/mL'],
      reproductiveIntake: {
        active_ingredients: ' norethindrone  0.35 mg ',
        question_or_belief_to_verify: 'I think this is progesterone-only',
        unknown_field: 'drop me',
      },
      cycleDiary: [{
        id: 'day-1',
        values: {
          entry_date: '2026-08-01',
          mood_behavior_score: '3',
          unknown_field: 'drop me',
        },
      }],
    })).toEqual({
      medications: ['norethindrone', 'levothyroxine'],
      supplements: ['magnesium'],
      allergies: [],
      symptoms: [],
      labObservations: ['ferritin 18 ng/mL'],
      reproductiveIntake: {
        active_ingredients: 'norethindrone 0.35 mg',
        question_or_belief_to_verify: 'I think this is progesterone-only',
      },
      cycleDiary: [{
        id: 'day-1',
        values: {
          entry_date: '2026-08-01',
          mood_behavior_score: '3',
        },
      }],
    });
  });

  it('uses a distinct storage key for each DNA profile', () => {
    expect(personalSafetyContextStorageKey(7)).toBe('genomics_personal_safety_context:7');
    expect(personalSafetyContextStorageKey(null)).toBeNull();
  });
});
