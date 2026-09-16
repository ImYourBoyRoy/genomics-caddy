/**
 * Group exact marker interpretations that describe the same family-level
 * context. The source text is retained; consumers decide whether to display
 * it once or inline for technical review.
 */

export interface SharedInterpretation {
  key: string;
  text: string;
  count: number;
}

export interface InterpretationLike {
  interpretation?: string | null;
}

export function normalizeSharedInterpretation(value: string | null | undefined): string {
  return (value || '').trim().replace(/\s+/g, ' ').toLowerCase();
}

export function getSharedInterpretations<T extends InterpretationLike>(
  markers: readonly T[],
): SharedInterpretation[] {
  const groups = new Map<string, SharedInterpretation>();
  for (const marker of markers) {
    const text = marker.interpretation?.trim() || '';
    const key = normalizeSharedInterpretation(text);
    if (!key) continue;
    const existing = groups.get(key);
    if (existing) existing.count += 1;
    else groups.set(key, { key, text, count: 1 });
  }

  return Array.from(groups.values())
    .filter((item) => item.count > 1)
    .sort((left, right) => right.count - left.count || left.text.localeCompare(right.text));
}
