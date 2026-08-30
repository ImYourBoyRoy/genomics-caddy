/**
 * Canonical, profile-scoped user context.
 *
 * This record is deliberately separate from the generated DNA report. It is
 * supplied by the profile owner and is only used by Connected Chat and the
 * export pipeline. The legacy per-profile keys remain readable and writable
 * for compatibility; no stored context is deleted during migration.
 */

import {
  EMPTY_PERSONAL_SAFETY_CONTEXT,
  loadPersonalSafetyContext,
  normalizePersonalSafetyContext,
  savePersonalSafetyContext,
  type PersonalSafetyContext,
} from './personalSafetyContext';
import {
  loadReproductiveContext,
  saveReproductiveContext,
} from './reproductiveContext';

export const PROFILE_CONTEXT_VERSION = 1;

export interface ProfileContextNotes {
  goals: string;
  challenges: string;
  relevantBodySystems: string;
  reproductiveHormoneContext: string;
  diet: string;
  diagnoses: string;
  supportiveTests: string;
}

export interface ProfileExportPreferences {
  includeRawGenotypesInClinician: boolean;
  includeRawGenotypesInAi: boolean;
}

export interface ProfileContext {
  version: number;
  selectedReproductiveContext: string;
  notes: ProfileContextNotes;
  safety: PersonalSafetyContext;
  exportPreferences: ProfileExportPreferences;
}

export interface LegacyAiProfileImport extends Partial<ProfileContextNotes> {
  safety: Pick<PersonalSafetyContext, 'medications' | 'supplements' | 'labObservations'>;
}

export const EMPTY_PROFILE_CONTEXT: ProfileContext = {
  version: PROFILE_CONTEXT_VERSION,
  selectedReproductiveContext: '',
  notes: {
    goals: '',
    challenges: '',
    relevantBodySystems: '',
    reproductiveHormoneContext: '',
    diet: '',
    diagnoses: '',
    supportiveTests: '',
  },
  safety: { ...EMPTY_PERSONAL_SAFETY_CONTEXT },
  exportPreferences: {
    includeRawGenotypesInClinician: true,
    includeRawGenotypesInAi: true,
  },
};

const STORAGE_PREFIX = 'genomics_profile_context:';
const LEGACY_AI_PROFILE_KEY = 'genomics_user_biohacking_profile';

function clean(value: unknown, max = 2000): string {
  return String(value ?? '').replace(/\s+/g, ' ').trim().slice(0, max);
}

function normalizeNotes(value: unknown): ProfileContextNotes {
  const source = value && typeof value === 'object'
    ? value as Partial<Record<keyof ProfileContextNotes, unknown>>
    : {};
  return {
    goals: clean(source.goals),
    challenges: clean(source.challenges),
    relevantBodySystems: clean(source.relevantBodySystems),
    reproductiveHormoneContext: clean(source.reproductiveHormoneContext),
    diet: clean(source.diet),
    diagnoses: clean(source.diagnoses),
    supportiveTests: clean(source.supportiveTests),
  };
}

export function normalizeProfileContext(value: unknown): ProfileContext {
  const source = value && typeof value === 'object'
    ? value as Partial<ProfileContext>
    : {};
  const rawExportPreferences = source.exportPreferences && typeof source.exportPreferences === 'object'
    ? source.exportPreferences as Partial<ProfileExportPreferences>
    : {};
  return {
    version: PROFILE_CONTEXT_VERSION,
    selectedReproductiveContext: clean(source.selectedReproductiveContext, 120),
    notes: normalizeNotes(source.notes),
    safety: source.safety
      ? normalizePersonalSafetyContext(source.safety)
      : { ...EMPTY_PERSONAL_SAFETY_CONTEXT },
    exportPreferences: {
      includeRawGenotypesInClinician: rawExportPreferences.includeRawGenotypesInClinician !== false,
      includeRawGenotypesInAi: rawExportPreferences.includeRawGenotypesInAi !== false,
    },
  };
}

function storageKey(sampleId?: number | null): string | null {
  return sampleId == null ? null : `${STORAGE_PREFIX}${sampleId}`;
}

/** Load the new record and fall back to the existing profile-scoped stores. */
export function loadProfileContext(sampleId?: number | null): ProfileContext {
  const key = storageKey(sampleId);
  if (key && typeof localStorage !== 'undefined') {
    try {
      const stored = localStorage.getItem(key);
      if (stored) return normalizeProfileContext(JSON.parse(stored));
    } catch {
      // Fall through to the legacy stores.
    }
  }

  return normalizeProfileContext({
    selectedReproductiveContext: loadReproductiveContext(sampleId),
    safety: loadPersonalSafetyContext(sampleId),
  });
}

/** Persist the canonical record and mirror legacy fields for compatibility. */
export function saveProfileContext(sampleId: number | null | undefined, value: ProfileContext): void {
  const normalized = normalizeProfileContext(value);
  const key = storageKey(sampleId);
  if (key && typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem(key, JSON.stringify(normalized));
    } catch {
      // Legacy stores below may still be available.
    }
  }
  saveReproductiveContext(sampleId, normalized.selectedReproductiveContext);
  savePersonalSafetyContext(sampleId, normalized.safety);
}

/** Read the old global AI profile without attaching it to any genome. */
export function loadUnassignedLegacyAiProfile(): LegacyAiProfileImport | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const value = JSON.parse(localStorage.getItem(LEGACY_AI_PROFILE_KEY) || 'null');
    if (!value || typeof value !== 'object') return null;
    const source = value as Record<string, unknown>;
    const notes = normalizeNotes(source);
    const safety = normalizePersonalSafetyContext({
      medications: source.medications,
      supplements: source.supplements,
      labObservations: source.bloodwork,
    });
    const hasNotes = Object.values(notes).some(Boolean);
    const hasSafety = safety.medications.length > 0 || safety.supplements.length > 0 || safety.labObservations.length > 0;
    return hasNotes || hasSafety
      ? {
          ...notes,
          safety: {
            medications: safety.medications,
            supplements: safety.supplements,
            labObservations: safety.labObservations,
          },
        }
      : null;
  } catch {
    return null;
  }
}

export function profileContextHasContent(context: ProfileContext): boolean {
  const notesHaveValues = Object.values(context.notes).some((value) => Boolean(value.trim()));
  const safety = context.safety;
  const structuredDietValues = Object.values(safety.dietaryProfile || {}).flatMap((value) =>
    Array.isArray(value) ? value : [value],
  );
  const reproductiveIntakeValues = Object.values(safety.reproductiveIntake || {});
  const safetyHaveValues = [
    ...safety.medications,
    ...safety.supplements,
    ...safety.allergies,
    ...safety.symptoms,
    ...safety.labObservations,
    ...(safety.cycleDiary || []),
    ...structuredDietValues,
    ...reproductiveIntakeValues,
  ].some((value) => Boolean(String(value ?? '').trim()));
  return Boolean(context.selectedReproductiveContext || notesHaveValues || safetyHaveValues);
}
