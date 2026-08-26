/**
 * Structured, user-supplied context that can change how genomic findings are
 * interpreted safely.
 *
 * This is deliberately separate from genotype data. It is stored per imported
 * DNA profile so medication, supplement, allergy, symptom, and lab context
 * cannot silently follow a different person's sample.
 */

import { normalizeReproductiveIntake, type ReproductiveIntakeValues } from './reproductiveIntake';
import { normalizeCycleDiary, type CycleDiaryEntry } from './cycleDiary';

/**
 * Explicit food requirements from the user-diet profile schema. These values
 * are kept separate from genotype and are never inferred from a DNA file.
 */
export interface PersonalDietaryProfile {
  hard_exclusions: string[];
  allergies_confirmed: string[];
  allergies_suspected: string[];
  religious_cultural_profiles: string[];
  ethical_preference_profiles: string[];
  medical_diet_profiles: string[];
  goals: string[];
}

export interface PersonalSafetyContext {
  medications: string[];
  supplements: string[];
  allergies: string[];
  symptoms: string[];
  labObservations: string[];
  dietaryProfile?: PersonalDietaryProfile;
  reproductiveIntake?: ReproductiveIntakeValues;
  cycleDiary?: CycleDiaryEntry[];
}

export const EMPTY_PERSONAL_SAFETY_CONTEXT: PersonalSafetyContext = {
  medications: [],
  supplements: [],
  allergies: [],
  symptoms: [],
  labObservations: [],
};

export const EMPTY_PERSONAL_DIETARY_PROFILE: PersonalDietaryProfile = {
  hard_exclusions: [],
  allergies_confirmed: [],
  allergies_suspected: [],
  religious_cultural_profiles: [],
  ethical_preference_profiles: [],
  medical_diet_profiles: [],
  goals: [],
};

const STORAGE_PREFIX = 'genomics_personal_safety_context:';
const MAX_ITEMS_PER_FIELD = 50;
const MAX_ITEM_LENGTH = 240;

function cleanItem(value: unknown): string {
  return String(value ?? '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, MAX_ITEM_LENGTH);
}

function dedupeItems(values: unknown[]): string[] {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const value of values) {
    const clean = cleanItem(value);
    const key = clean.toLocaleLowerCase();
    if (!clean || seen.has(key)) continue;
    seen.add(key);
    result.push(clean);
    if (result.length >= MAX_ITEMS_PER_FIELD) break;
  }
  return result;
}

/** Parse the one-item-per-line editor without trying to interpret medical text. */
export function parseContextList(value: string): string[] {
  return dedupeItems(String(value || '').split(/[\r\n;]+/));
}

/** Normalize persisted data and tolerate older or manually edited local data. */
export function normalizePersonalSafetyContext(value: unknown): PersonalSafetyContext {
  const source = value && typeof value === 'object'
    ? value as Partial<Record<keyof PersonalSafetyContext, unknown>>
    : {};

  const list = (field: keyof PersonalSafetyContext): string[] => {
    const raw = source[field];
    if (Array.isArray(raw)) return dedupeItems(raw);
    if (typeof raw === 'string') return parseContextList(raw);
    return [];
  };

  const reproductiveIntake = normalizeReproductiveIntake(source.reproductiveIntake);
  const cycleDiary = normalizeCycleDiary(source.cycleDiary);
  const dietarySource = source.dietaryProfile && typeof source.dietaryProfile === 'object'
    ? source.dietaryProfile as Record<string, unknown>
    : {};
  const dietaryProfile = Object.fromEntries(
    Object.keys(EMPTY_PERSONAL_DIETARY_PROFILE).map((field) => {
      const raw = dietarySource[field];
      return [field, Array.isArray(raw) ? dedupeItems(raw) : typeof raw === 'string' ? parseContextList(raw) : []];
    }),
  ) as unknown as PersonalDietaryProfile;
  const hasDietaryProfile = Object.values(dietaryProfile).some((items) => items.length > 0);
  return {
    medications: list('medications'),
    supplements: list('supplements'),
    allergies: list('allergies'),
    symptoms: list('symptoms'),
    labObservations: list('labObservations'),
    ...(hasDietaryProfile ? { dietaryProfile } : {}),
    ...(Object.keys(reproductiveIntake).length > 0 ? { reproductiveIntake } : {}),
    ...(cycleDiary.length > 0 ? { cycleDiary } : {}),
  };
}

export function personalSafetyContextStorageKey(sampleId?: number | null): string | null {
  return sampleId == null ? null : `${STORAGE_PREFIX}${sampleId}`;
}

export function loadPersonalSafetyContext(sampleId?: number | null): PersonalSafetyContext {
  const key = personalSafetyContextStorageKey(sampleId);
  if (!key || typeof localStorage === 'undefined') return { ...EMPTY_PERSONAL_SAFETY_CONTEXT };

  try {
    return normalizePersonalSafetyContext(JSON.parse(localStorage.getItem(key) || '{}'));
  } catch {
    return { ...EMPTY_PERSONAL_SAFETY_CONTEXT };
  }
}

export function savePersonalSafetyContext(
  sampleId: number | null | undefined,
  context: PersonalSafetyContext,
): void {
  const key = personalSafetyContextStorageKey(sampleId);
  if (!key || typeof localStorage === 'undefined') return;

  try {
    localStorage.setItem(key, JSON.stringify(normalizePersonalSafetyContext(context)));
  } catch {
    // A blocked or full browser storage area must not interrupt report use.
  }
}
