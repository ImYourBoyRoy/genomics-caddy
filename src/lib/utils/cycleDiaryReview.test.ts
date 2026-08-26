import { describe, expect, it } from 'vitest';
import { reviewCycleDiary } from './cycleDiaryReview';
import type { CycleDiaryEntry } from './cycleDiary';

function entry(date: string, values: Record<string, string>): CycleDiaryEntry {
  return { id: date, values: { entry_date: date, ...values } };
}

describe('observation-only cycle diary review', () => {
  it('counts recorded and co-occurring observations without filling missing days', () => {
    const review = reviewCycleDiary([
      entry('2026-08-01', { cycle_day: '1', bleeding_level: '3', mood_behavior_score: '2' }),
      entry('2026-08-02', { cycle_day: '2', bleeding_level: '4', pain_headache_score: '3' }),
      entry('2026-08-05', { cycle_day: '5', mood_behavior_score: '0' }),
    ]);

    expect(review).not.toBeNull();
    expect(review?.entry_count).toBe(3);
    expect(review?.observed_date_count).toBe(3);
    expect(review?.observation_span_days).toBe(4);
    expect(review?.metrics).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: 'mood_behavior_impact', recorded_days: 2, elevated_days: 1, threshold: 2 }),
      expect.objectContaining({ id: 'pain_headache_impact', recorded_days: 1, elevated_days: 1, threshold: 2 }),
      expect.objectContaining({ id: 'heavy_bleeding', recorded_days: 2, elevated_days: 2, threshold: 3 }),
    ]));
    expect(review?.co_occurrence).toEqual({
      mood_behavior_with_bleeding_days: 1,
      pain_headache_with_bleeding_days: 1,
    });
    expect(review?.cycle_day_summary).toEqual([
      expect.objectContaining({ cycle_day: 1, observation_count: 1, bleeding_days: 1, mood_behavior_days: 1 }),
      expect.objectContaining({ cycle_day: 2, observation_count: 1, bleeding_days: 1, pain_headache_days: 1 }),
      expect.objectContaining({ cycle_day: 5, observation_count: 1 }),
    ]);
  });

  it('returns no review for empty or invalid diary input and never exposes genotype data', () => {
    expect(reviewCycleDiary([])).toBeNull();
    expect(reviewCycleDiary([{ id: 'bad', values: { entry_date: 'not-a-date', genotype: 'AA' } }])).toBeNull();
  });

  it('compares repeated user-entered cycle days only among recorded observations', () => {
    const review = reviewCycleDiary([
      entry('2026-08-01', { cycle_day: '24', mood_behavior_score: '2' }),
      entry('2026-08-29', { cycle_day: '24', mood_behavior_score: '3' }),
      entry('2026-09-26', { cycle_day: '24', mood_behavior_score: '0' }),
    ]);

    const day = review?.cycle_day_summary.find((candidate) => candidate.cycle_day === 24);
    const mood = day?.metrics.find((metric) => metric.metric_id === 'mood_behavior_impact');

    expect(day?.observation_count).toBe(3);
    expect(mood).toEqual(expect.objectContaining({
      recorded_days: 3,
      elevated_days: 2,
      observed_share_percent: 67,
      minimum_observations: 2,
      enough_observations: true,
    }));
    expect(review?.notes.join(' ')).toContain('not a population probability');
  });
});
