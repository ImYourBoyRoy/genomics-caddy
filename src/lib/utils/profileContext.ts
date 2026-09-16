/**
 * Canonical, profile-scoped user context.
 *
 * This record is deliberately separate from the generated DNA report. It is
 * supplied by the profile owner and is only used by Connected Chat and the
 * export pipeline. The legacy per-profile keys remain readable and writable
 * for compatibility. The old global AI profile is never attached implicitly
 * and is removed only after the user explicitly imports or dismisses it.
 */

import {
  EMPTY_PERSONAL_SAFETY_CONTEXT,
  loadPersonalSafetyContext,
  normalizePersonalSafetyContext,
  personalSafetyContextStorageKey,
  savePersonalSafetyContext,
  type PersonalSafetyContext,
} from './personalSafetyContext';
import {
  loadReproductiveContext,
  reproductiveContextStorageKey,
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
  /** @deprecated AI and clinician handoffs always include raw calls. */
  includeRawGenotypesInClinician: boolean;
  /** @deprecated AI and clinician handoffs always include raw calls. */
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
  return {
    version: PROFILE_CONTEXT_VERSION,
    selectedReproductiveContext: clean(source.selectedReproductiveContext, 120),
    notes: normalizeNotes(source.notes),
    safety: source.safety
      ? normalizePersonalSafetyContext(source.safety)
      : { ...EMPTY_PERSONAL_SAFETY_CONTEXT },
    exportPreferences: {
      // Keep the old persisted shape readable, but no longer allow it to
      // suppress the raw calls required for technical handoffs.
      includeRawGenotypesInClinician: true,
      includeRawGenotypesInAi: true,
    },
  };
}

function storageKey(sampleId?: number | null): string | null {
  return sampleId == null ? null : `${STORAGE_PREFIX}${sampleId}`;
}

/**
 * Remove browser state that belongs exclusively to a deleted profile.
 *
 * The native sample database is the source of truth for genotype and report
 * data, but these keys can otherwise resurrect stale context, chat selection,
 * or presentation state if a profile ID is reused. The global active-session
 * key is cleared only when the deleted profile was selected.
 */
export function clearProfileScopedStorage(
  sampleId: number,
  clearGlobalActiveSession = false,
): number {
  if (typeof localStorage === 'undefined') return 0;

  const exactKeys = new Set([
    storageKey(sampleId),
    reproductiveContextStorageKey(sampleId),
    personalSafetyContextStorageKey(sampleId),
    `genomics_active_session_id_${sampleId}`,
    `genomics_presentation_mode_${sampleId}`,
    `genomics_dashboard_collapsed_v2_${sampleId}`,
  ].filter((key): key is string => Boolean(key)));
  const prefixes = [`section-collapsed-v2-${sampleId}-`];
  const keysToRemove: string[] = [];

  try {
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key && (exactKeys.has(key) || prefixes.some((prefix) => key.startsWith(prefix)))) {
        keysToRemove.push(key);
      }
    }
    if (clearGlobalActiveSession && localStorage.getItem('genomics_active_session_id') !== null) {
      keysToRemove.push('genomics_active_session_id');
    }
  } catch {
    return 0;
  }

  let removed = 0;
  for (const key of new Set(keysToRemove)) {
    try {
      if (localStorage.getItem(key) !== null) {
        localStorage.removeItem(key);
        removed += 1;
      }
    } catch {
      // Storage can be unavailable or quota-restricted; native deletion has
      // already completed, so keep the cleanup best-effort and non-blocking.
    }
  }
  return removed;
}

/** Clear only browser pointers to chat sessions after a native profile replace.
 * Entered Context/Diary is intentionally preserved by replacement semantics.
 */
export function clearProfileScopedSessionState(
  sampleId: number,
  clearGlobalActiveSession = false,
): number {
  if (typeof localStorage === 'undefined') return 0;
  const keys = [`genomics_active_session_id_${sampleId}`];
  if (clearGlobalActiveSession) keys.push('genomics_active_session_id');
  let removed = 0;
  for (const key of keys) {
    try {
      if (localStorage.getItem(key) !== null) {
        localStorage.removeItem(key);
        removed += 1;
      }
    } catch {
      // Native replacement remains authoritative if browser storage is unavailable.
    }
  }
  return removed;
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

/** Remove the old unassigned global AI profile after an explicit user choice. */
export function clearUnassignedLegacyAiProfile(): boolean {
  if (typeof localStorage === 'undefined') return false;
  try {
    const present = localStorage.getItem(LEGACY_AI_PROFILE_KEY) !== null;
    localStorage.removeItem(LEGACY_AI_PROFILE_KEY);
    return present;
  } catch {
    return false;
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
