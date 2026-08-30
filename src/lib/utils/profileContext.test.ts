import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  EMPTY_PROFILE_CONTEXT,
  loadProfileContext,
  loadUnassignedLegacyAiProfile,
  profileContextHasContent,
  saveProfileContext,
  type ProfileContext,
} from './profileContext';
import { personalSafetyContextStorageKey } from './personalSafetyContext';
import { reproductiveContextStorageKey } from './reproductiveContext';

function createStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; },
    clear() { values.clear(); },
    getItem(key) { return values.get(String(key)) ?? null; },
    key(index) { return Array.from(values.keys())[index] ?? null; },
    removeItem(key) { values.delete(String(key)); },
    setItem(key, value) { values.set(String(key), String(value)); },
  };
}

function context(overrides: Partial<ProfileContext> = {}): ProfileContext {
  return {
    ...EMPTY_PROFILE_CONTEXT,
    notes: { ...EMPTY_PROFILE_CONTEXT.notes },
    safety: { ...EMPTY_PROFILE_CONTEXT.safety },
    ...overrides,
  };
}

describe('profile context workspace', () => {
  let previousStorage: Storage | undefined;

  beforeEach(() => {
    previousStorage = globalThis.localStorage;
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: createStorage() });
  });

  afterEach(() => {
    if (previousStorage) {
      Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: previousStorage });
    } else {
      Reflect.deleteProperty(globalThis, 'localStorage');
    }
  });

  it('keeps saved context isolated by sample ID', () => {
    saveProfileContext(11, context({
      notes: { ...EMPTY_PROFILE_CONTEXT.notes, goals: 'Sleep and recovery' },
      safety: { ...EMPTY_PROFILE_CONTEXT.safety, medications: ['Medication A'] },
    }));
    saveProfileContext(12, context({
      notes: { ...EMPTY_PROFILE_CONTEXT.notes, goals: 'Training support' },
      safety: { ...EMPTY_PROFILE_CONTEXT.safety, medications: ['Medication B'] },
    }));

    expect(loadProfileContext(11).notes.goals).toBe('Sleep and recovery');
    expect(loadProfileContext(11).safety.medications).toEqual(['Medication A']);
    expect(loadProfileContext(12).notes.goals).toBe('Training support');
    expect(loadProfileContext(12).safety.medications).toEqual(['Medication B']);
    expect(loadProfileContext(11).notes.goals).not.toBe(loadProfileContext(12).notes.goals);
  });

  it('reads legacy profile-scoped context without deleting it', () => {
    localStorage.setItem(reproductiveContextStorageKey(21), 'cyclic_mood_symptoms');
    localStorage.setItem(personalSafetyContextStorageKey(21)!, JSON.stringify({
      symptoms: ['cyclic mood changes'],
      cycleDiary: [{ id: 'entry-1', values: { entry_date: '2026-08-01' } }],
    }));

    const loaded = loadProfileContext(21);

    expect(loaded.selectedReproductiveContext).toBe('cyclic_mood_symptoms');
    expect(loaded.safety.symptoms).toEqual(['cyclic mood changes']);
    expect(loaded.safety.cycleDiary).toHaveLength(1);
    expect(localStorage.getItem(reproductiveContextStorageKey(21))).toBe('cyclic_mood_symptoms');
    expect(localStorage.getItem(personalSafetyContextStorageKey(21)!)).toContain('cyclic mood changes');
  });

  it('keeps a legacy global AI profile unassigned until explicitly imported', () => {
    localStorage.setItem('genomics_user_biohacking_profile', JSON.stringify({
      goals: 'Legacy goal',
      supplements: 'Legacy supplement',
      medications: 'Legacy medication',
      bloodwork: 'Legacy lab note',
    }));

    expect(loadUnassignedLegacyAiProfile()?.goals).toBe('Legacy goal');
    expect(loadUnassignedLegacyAiProfile()?.safety.supplements).toEqual(['Legacy supplement']);
    expect(loadUnassignedLegacyAiProfile()?.safety.medications).toEqual(['Legacy medication']);
    expect(loadUnassignedLegacyAiProfile()?.safety.labObservations).toEqual(['Legacy lab note']);
    expect(loadProfileContext(31)).toEqual(EMPTY_PROFILE_CONTEXT);
    expect(profileContextHasContent(loadProfileContext(31))).toBe(false);
  });
});
