import { describe, expect, it } from 'vitest';
import {
  DEFAULT_PRESENTATION_MODE,
  readPresentationMode,
  writePresentationMode,
} from './presentationPreferences';

function storage(values: Record<string, string> = {}) {
  return {
    getItem: (key: string) => values[key] ?? null,
    setItem: (key: string, value: string) => { values[key] = value; },
  };
}

describe('presentation preferences', () => {
  it('defaults missing and malformed values to Simple', () => {
    expect(readPresentationMode(storage(), 'mode')).toBe(DEFAULT_PRESENTATION_MODE);
    expect(readPresentationMode(storage({ mode: 'unknown' }), 'mode')).toBe(DEFAULT_PRESENTATION_MODE);
  });

  it('accepts supported modes and writes only the selected mode', () => {
    const prefs = storage();
    expect(readPresentationMode(storage({ mode: 'clinical' }), 'mode')).toBe('clinical');
    writePresentationMode(prefs, 'mode', 'compare');
    expect(prefs.getItem('mode')).toBe('compare');
  });

  it('migrates the legacy dual mode to compare without resetting the profile', () => {
    expect(readPresentationMode(storage({ mode: 'dual' }), 'mode')).toBe('compare');
  });
});
