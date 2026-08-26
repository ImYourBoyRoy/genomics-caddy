import { describe, expect, it } from 'vitest';
import {
  CYCLE_DIARY_SCHEMA,
  cycleDiaryAppliesToContext,
  normalizeCycleDiary,
  normalizeCycleDiaryEntry,
  populatedCycleDiaryFields,
} from './cycleDiary';

describe('resource-backed cycle diary', () => {
  it('normalizes a dated entry, filters unknown fields, and rejects invalid numeric scores', () => {
    const entry = normalizeCycleDiaryEntry({
      id: '  day-one  ',
      values: {
        entry_date: ' 2026-08-01 ',
        bleeding_level: '5',
        mood_behavior_score: ' 3 ',
        mood_behavior_notes: '  needs quiet  ',
        unknown: 'do not persist',
      },
    });

    expect(entry).toEqual({
      id: 'day-one',
      values: {
        entry_date: '2026-08-01',
        mood_behavior_score: '3',
        mood_behavior_notes: 'needs quiet',
      },
    });
    expect(entry).not.toBeNull();
    expect(populatedCycleDiaryFields(entry!.values).map(({ field }) => field.id)).toEqual([
      'entry_date',
      'mood_behavior_score',
      'mood_behavior_notes',
    ]);
    expect(normalizeCycleDiaryEntry({ values: { mood_behavior_score: '2' } })).toBeNull();
    expect(normalizeCycleDiaryEntry({ values: { entry_date: 'not-a-date' } })).toBeNull();
  });

  it('keeps the diary bounded and only enables it for explicit cycle contexts', () => {
    const entries = Array.from({ length: CYCLE_DIARY_SCHEMA.retention_limit + 5 }, (_, index) => ({
      id: `entry-${index}`,
      values: { entry_date: `2026-08-${String((index % 28) + 1).padStart(2, '0')}` },
    }));

    expect(normalizeCycleDiary(entries)).toHaveLength(CYCLE_DIARY_SCHEMA.retention_limit);
    expect(cycleDiaryAppliesToContext('menstrual_cycle')).toBe(true);
    expect(cycleDiaryAppliesToContext('androgen_reproductive')).toBe(false);
    expect(cycleDiaryAppliesToContext('')).toBe(false);
  });
});
