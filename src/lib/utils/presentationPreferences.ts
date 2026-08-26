// ./src/lib/utils/presentationPreferences.ts
/**
 * Small, defensive preference boundary for report presentation mode.
 * Preferences are UI state only and never contain profile or genotype data.
 */

export type PresentationMode = 'simple' | 'clinical' | 'compare';

export const DEFAULT_PRESENTATION_MODE: PresentationMode = 'simple';

export function isPresentationMode(value: string | null | undefined): value is PresentationMode {
  return value === 'simple' || value === 'clinical' || value === 'compare';
}

export function readPresentationMode(storage: Pick<Storage, 'getItem'>, key: string): PresentationMode {
  const stored = storage.getItem(key);
  // `dual` was the pre-redesign name. Read it once as Compare so existing
  // profiles keep their deliberate choice without preserving the old API.
  if (stored === 'dual') return 'compare';
  return isPresentationMode(stored) ? stored : DEFAULT_PRESENTATION_MODE;
}

export function writePresentationMode(
  storage: Pick<Storage, 'setItem'>,
  key: string,
  mode: PresentationMode,
): void {
  storage.setItem(key, mode);
}
