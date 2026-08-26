/**
 * Resource-backed normalization for optional cycle and hormone context.
 *
 * These values are deliberately user-supplied phenotype and medication
 * context. They are never merged into genotype evidence or interpreted as a
 * diagnosis, hormone measurement, or medication recommendation.
 */

import cycleSupport from '../marker-packs/cycle_support_guidance.json';

export const REPRODUCTIVE_INTAKE_SCHEMA = cycleSupport.intake_schema;
export type ReproductiveIntakeSchema = typeof REPRODUCTIVE_INTAKE_SCHEMA;
export type ReproductiveIntakeGroup = ReproductiveIntakeSchema['groups'][number];
export type ReproductiveIntakeField = ReproductiveIntakeSchema['fields'][number];
export type ReproductiveIntakeValues = Partial<Record<ReproductiveIntakeField['id'], string>>;

const MAX_VALUE_LENGTH = 500;

function cleanValue(value: unknown): string {
  return String(value ?? '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, MAX_VALUE_LENGTH);
}

/** Keep only fields declared by the resource-authored intake schema. */
export function normalizeReproductiveIntake(value: unknown): ReproductiveIntakeValues {
  const source = value && typeof value === 'object'
    ? value as Record<string, unknown>
    : {};
  const normalized: ReproductiveIntakeValues = {};

  for (const field of REPRODUCTIVE_INTAKE_SCHEMA.fields) {
    const clean = cleanValue(source[field.id]);
    if (clean) normalized[field.id] = clean;
  }

  return normalized;
}

export function reproductiveIntakeValue(
  values: ReproductiveIntakeValues | undefined,
  fieldId: ReproductiveIntakeField['id'],
): string {
  return values?.[fieldId] || '';
}

export function hasReproductiveIntake(values: ReproductiveIntakeValues | undefined): boolean {
  return Object.keys(values || {}).length > 0;
}

/**
 * Show context-specific groups only after the person explicitly selects that
 * context. Groups without context_ids remain common to every route; with no
 * selection, all groups stay available in the profile editor.
 */
export function reproductiveIntakeGroupsForContext(
  reproductiveContext?: string | null,
): ReproductiveIntakeGroup[] {
  const normalizedContext = String(reproductiveContext || '').trim().toLowerCase();
  return REPRODUCTIVE_INTAKE_SCHEMA.groups.filter((group) => {
    const contextIds = group.context_ids as readonly string[] | undefined;
    return !normalizedContext || !contextIds?.length || contextIds.includes(normalizedContext);
  });
}

/** Return only populated fields in the schema's display order. */
export function populatedReproductiveIntake(
  values: ReproductiveIntakeValues | undefined,
): Array<{ field: ReproductiveIntakeField; value: string }> {
  return REPRODUCTIVE_INTAKE_SCHEMA.fields.flatMap((field) => {
    const value = reproductiveIntakeValue(values, field.id);
    return value ? [{ field, value }] : [];
  });
}
