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
