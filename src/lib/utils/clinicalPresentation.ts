const CLINICAL_FOLLOW_UP_LIMIT = 3;

/**
 * Keep the structured Clinical table actionable without making long follow-up
 * lists widen or dominate a row. Empty labels are omitted; the full list
 * remains available through the finding's technical details.
 */
export function formatClinicalFollowUp(labels: readonly string[]): string {
  const normalizedLabels = labels
    .map((label) => label.trim())
    .filter(Boolean);
  if (normalizedLabels.length === 0) return '';

  const visibleLabels = normalizedLabels.slice(0, CLINICAL_FOLLOW_UP_LIMIT).join('; ');
  const remainingCount = normalizedLabels.length - CLINICAL_FOLLOW_UP_LIMIT;
  return remainingCount > 0 ? `${visibleLabels} (+${remainingCount} more)` : visibleLabels;
}
