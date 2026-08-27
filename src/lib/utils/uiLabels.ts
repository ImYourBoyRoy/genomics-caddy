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

/** Keep legacy chromosome-call descriptions out of primary result surfaces. */
export function formatGeneticSexLabel(value: string | null | undefined): string {
  const label = value?.trim() ?? '';
  if (!label) return 'Unknown';

  if (/^female\b/i.test(label)) return 'Female';
  if (/^male\b/i.test(label)) return 'Male';
  if (/^uncertain\b/i.test(label)) return 'Uncertain';
  if (/^unknown\b/i.test(label)) return 'Unknown';
  return label;
}
