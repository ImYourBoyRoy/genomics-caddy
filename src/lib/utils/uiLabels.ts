/**
 * Normalize user-facing labels for deterministic local UI automation.
 * Emoji and punctuation are presentation details, not part of the label's
 * semantic match.
 */
export function normalizeUiLabel(value: string): string {
  return value
    .normalize('NFKC')
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim()
    .toLowerCase();
}

/** Keep chromosome-call descriptions concise in primary result surfaces. */
export function formatGeneticSexLabel(value: string | null | undefined): string {
  const label = value?.trim() ?? '';
  if (!label) return 'Unknown';

  // These are deliberately chromosome-pattern labels. They are not a
  // person's gender identity, anatomy, fertility, hormone state, or a
  // diagnosis of mosaicism/chimerism.
  if (/^female(?:-like)?\b/i.test(label)) return 'Female-like';
  if (/^male(?:-like)?\b/i.test(label)) return 'Male-like';
  if (/^(?:inconclusive|uncertain)\b/i.test(label)) return 'Inconclusive';
  if (/^unknown\b/i.test(label)) return 'Unknown';
  return 'Unknown';
}
