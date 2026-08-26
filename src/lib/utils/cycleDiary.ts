/**
 * Resource-backed local diary for explicitly selected menstrual/cycle contexts.
 *
 * Diary values are self-reported phenotype and treatment context. They are
 * kept separate from genotype evidence and never converted into a diagnosis,
 * hormone measurement, or medication recommendation.
 */

import cycleSupport from '../marker-packs/cycle_support_guidance.json';

export const CYCLE_DIARY_SCHEMA = cycleSupport.diary_schema;
export type CycleDiarySchema = typeof CYCLE_DIARY_SCHEMA;
export type CycleDiaryField = CycleDiarySchema['fields'][number];
export type CycleDiaryValues = Partial<Record<CycleDiaryField['id'], string>>;

export interface CycleDiaryEntry {
  id: string;
  values: CycleDiaryValues;
}

function cleanValue(value: unknown): string {
  return String(value ?? '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 500);
}

function validFieldValue(field: CycleDiaryField, value: string): boolean {
  if (!value || field.input_type !== 'number') return true;
  const numberValue = Number(value);
  if (!Number.isFinite(numberValue)) return false;
  if (field.min !== undefined && numberValue < field.min) return false;
  if (field.max !== undefined && numberValue > field.max) return false;
  return true;
}

export function normalizeCycleDiaryEntry(value: unknown, fallbackId = ''): CycleDiaryEntry | null {
  const source = value && typeof value === 'object'
    ? value as { id?: unknown; values?: unknown }
    : {};
  const rawValues = source.values && typeof source.values === 'object'
    ? source.values as Record<string, unknown>
    : source as unknown as Record<string, unknown>;
  const values: CycleDiaryValues = {};

  for (const field of CYCLE_DIARY_SCHEMA.fields) {
    const clean = cleanValue(rawValues[field.id]);
    if (validFieldValue(field, clean)) {
      if (clean) values[field.id] = clean;
    }
  }

  const entryDate = values.entry_date;
  if (!entryDate || !/^\d{4}-\d{2}-\d{2}$/.test(entryDate)) return null;
  const id = cleanValue(source.id) || fallbackId || entryDate;
  return { id, values };
}

/** Normalize persisted data, discard malformed entries, and cap retention. */
export function normalizeCycleDiary(value: unknown): CycleDiaryEntry[] {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const entries: CycleDiaryEntry[] = [];
  const limit = Number(CYCLE_DIARY_SCHEMA.retention_limit) || 180;

  for (const [index, rawEntry] of value.entries()) {
    const entry = normalizeCycleDiaryEntry(rawEntry, `diary-entry-${index}`);
    if (!entry || seen.has(entry.id)) continue;
    seen.add(entry.id);
    entries.push(entry);
    if (entries.length >= limit) break;
  }
  return entries;
}

export function cycleDiaryAppliesToContext(reproductiveContext?: string | null): boolean {
  const context = String(reproductiveContext || '').trim().toLowerCase();
  return Boolean(context && CYCLE_DIARY_SCHEMA.context_ids.some((id) => id.toLowerCase() === context));
}

export function cycleDiaryAppliesToContexts(contextIds: readonly string[]): boolean {
  return contextIds.some((contextId) => cycleDiaryAppliesToContext(contextId));
}

export function populatedCycleDiaryFields(
  values: CycleDiaryValues,
): Array<{ field: CycleDiaryField; value: string }> {
  return CYCLE_DIARY_SCHEMA.fields.flatMap((field) => {
    const value = values[field.id];
    return value ? [{ field, value }] : [];
  });
}
