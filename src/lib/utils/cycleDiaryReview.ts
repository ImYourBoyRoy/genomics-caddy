/**
 * Produce a bounded, observation-only summary of the local cycle diary.
 *
 * The thresholds and wording are resource-authored. This module counts only
 * values the person entered; it does not fill missing days, infer a cycle
 * phase or hormone level, or turn co-occurrence into a diagnosis.
 */

import cycleSupport from '../marker-packs/cycle_support_guidance.json';
import { normalizeCycleDiary, type CycleDiaryEntry } from './cycleDiary';

const REVIEW_SCHEMA = cycleSupport.review_schema;
const METRIC_THRESHOLDS = REVIEW_SCHEMA.metric_thresholds as Record<string, number>;
const MINIMUM_CYCLE_DAY_OBSERVATIONS = Number.isInteger(REVIEW_SCHEMA.minimum_observations_for_cycle_day_comparison)
  && REVIEW_SCHEMA.minimum_observations_for_cycle_day_comparison > 0
  ? REVIEW_SCHEMA.minimum_observations_for_cycle_day_comparison
  : 2;

export interface CycleDiaryMetricReview {
  id: string;
  field_id: string;
  label: string;
  threshold: number;
  recorded_days: number;
  elevated_days: number;
}

export interface CycleDiaryCycleDayMetricReview {
  metric_id: string;
  field_id: string;
  label: string;
  threshold: number;
  recorded_days: number;
  elevated_days: number;
  observed_share_percent: number | null;
  minimum_observations: number;
  enough_observations: boolean;
}

export interface CycleDiaryCycleDayReview {
  cycle_day: number;
  observation_count: number;
  /** Counts retained for compatibility with the first diary-review payload. */
  bleeding_days: number;
  mood_behavior_days: number;
  pain_headache_days: number;
  metrics: CycleDiaryCycleDayMetricReview[];
}

export interface CycleDiaryReview {
  entry_count: number;
  observed_date_count: number;
  first_observed_date: string | null;
  last_observed_date: string | null;
  observation_span_days: number | null;
  cycle_day_observation_count: number;
  metrics: CycleDiaryMetricReview[];
  co_occurrence: {
    mood_behavior_with_bleeding_days: number;
    pain_headache_with_bleeding_days: number;
  };
  cycle_day_summary: CycleDiaryCycleDayReview[];
  notes: readonly string[];
}

function numericValue(entry: CycleDiaryEntry, fieldId: string): number | null {
  const value = Number(entry.values[fieldId as keyof typeof entry.values]);
  return Number.isFinite(value) ? value : null;
}

function dateTimestamp(value: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const timestamp = Date.parse(`${value}T00:00:00Z`);
  return Number.isFinite(timestamp) ? timestamp : null;
}

function daysBetween(start: string, end: string): number | null {
  const startTimestamp = dateTimestamp(start);
  const endTimestamp = dateTimestamp(end);
  if (startTimestamp === null || endTimestamp === null) return null;
  return Math.round((endTimestamp - startTimestamp) / 86_400_000);
}

function metricThreshold(metric: typeof REVIEW_SCHEMA.metrics[number]): number {
  return METRIC_THRESHOLDS[metric.threshold_key] ?? Number.POSITIVE_INFINITY;
}

/**
 * Summarize diary observations without treating absent entries as normal or
 * assigning biological meaning to a user-entered cycle-day number.
 */
export function reviewCycleDiary(
  entries: readonly CycleDiaryEntry[] | null | undefined,
): CycleDiaryReview | null {
  const normalizedEntries = normalizeCycleDiary(entries || []);
  if (normalizedEntries.length === 0) return null;

  const datedEntries = normalizedEntries
    .map((entry) => ({ entry, date: entry.values.entry_date }))
    .filter((item): item is { entry: CycleDiaryEntry; date: string } =>
      typeof item.date === 'string' && dateTimestamp(item.date) !== null)
    .sort((a, b) => a.date.localeCompare(b.date));
  const observedDates = Array.from(new Set(datedEntries.map((item) => item.date)));
  const firstObservedDate = observedDates[0] || null;
  const lastObservedDate = observedDates[observedDates.length - 1] || null;
  const cycleDayEntries = normalizedEntries.filter((entry) => numericValue(entry, 'cycle_day') !== null);

  const metrics = REVIEW_SCHEMA.metrics.map((metric) => {
    const threshold = metricThreshold(metric);
    const values = normalizedEntries
      .map((entry) => numericValue(entry, metric.field_id))
      .filter((value): value is number => value !== null);
    return {
      id: metric.id,
      field_id: metric.field_id,
      label: metric.label,
      threshold,
      recorded_days: values.length,
      elevated_days: values.filter((value) => value >= threshold).length,
    };
  });

  const thresholdForMetricId = (metricId: string): number => {
    const metric = REVIEW_SCHEMA.metrics.find((candidate) => candidate.id === metricId);
    return metric ? metricThreshold(metric) : Number.POSITIVE_INFINITY;
  };
  const moodThreshold = thresholdForMetricId('mood_behavior_impact');
  const painThreshold = thresholdForMetricId('pain_headache_impact');
  const coOccurrence = normalizedEntries.reduce(
    (summary, entry) => {
      const bleeding = numericValue(entry, 'bleeding_level');
      if (bleeding === null || bleeding < 1) return summary;
      if ((numericValue(entry, 'mood_behavior_score') ?? -Infinity) >= moodThreshold) {
        summary.mood_behavior_with_bleeding_days += 1;
      }
      if ((numericValue(entry, 'pain_headache_score') ?? -Infinity) >= painThreshold) {
        summary.pain_headache_with_bleeding_days += 1;
      }
      return summary;
    },
    { mood_behavior_with_bleeding_days: 0, pain_headache_with_bleeding_days: 0 },
  );

  const cycleDayMap = new Map<number, CycleDiaryCycleDayReview>();
  for (const entry of cycleDayEntries) {
    const cycleDay = numericValue(entry, 'cycle_day');
    if (cycleDay === null) continue;
    const row = cycleDayMap.get(cycleDay) || {
      cycle_day: cycleDay,
      observation_count: 0,
      bleeding_days: 0,
      mood_behavior_days: 0,
      pain_headache_days: 0,
      metrics: [],
    };
    row.observation_count += 1;
    if ((numericValue(entry, 'bleeding_level') ?? 0) >= 1) row.bleeding_days += 1;
    if ((numericValue(entry, 'mood_behavior_score') ?? -Infinity) >= moodThreshold) row.mood_behavior_days += 1;
    if ((numericValue(entry, 'pain_headache_score') ?? -Infinity) >= painThreshold) row.pain_headache_days += 1;
    cycleDayMap.set(cycleDay, row);
  }

  for (const [cycleDay, row] of cycleDayMap.entries()) {
    const entriesForDay = cycleDayEntries.filter((entry) => numericValue(entry, 'cycle_day') === cycleDay);
    row.metrics = REVIEW_SCHEMA.metrics.map((metric) => {
      const threshold = metricThreshold(metric);
      const values = entriesForDay
        .map((entry) => numericValue(entry, metric.field_id))
        .filter((value): value is number => value !== null);
      const recordedDays = values.length;
      const elevatedDays = values.filter((value) => value >= threshold).length;
      return {
        metric_id: metric.id,
        field_id: metric.field_id,
        label: metric.label,
        threshold,
        recorded_days: recordedDays,
        elevated_days: elevatedDays,
        observed_share_percent: recordedDays > 0
          ? Math.round((elevatedDays / recordedDays) * 100)
          : null,
        minimum_observations: MINIMUM_CYCLE_DAY_OBSERVATIONS,
        enough_observations: recordedDays >= MINIMUM_CYCLE_DAY_OBSERVATIONS,
      };
    });
  }

  return {
    entry_count: normalizedEntries.length,
    observed_date_count: observedDates.length,
    first_observed_date: firstObservedDate,
    last_observed_date: lastObservedDate,
    observation_span_days: firstObservedDate && lastObservedDate
      ? daysBetween(firstObservedDate, lastObservedDate)
      : null,
    cycle_day_observation_count: cycleDayEntries.length,
    metrics,
    co_occurrence: coOccurrence,
    cycle_day_summary: Array.from(cycleDayMap.values()).sort((a, b) => a.cycle_day - b.cycle_day),
    notes: REVIEW_SCHEMA.review_notes,
  };
}
