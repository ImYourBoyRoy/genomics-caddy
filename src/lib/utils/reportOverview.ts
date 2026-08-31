import type { GeneratedReport } from '../types/genomics';
import { buildCanonicalFindingGroups, type CanonicalFindingGroup } from './findingIdentity';

export interface ReportOverviewStats {
  /** Unique, called signals that have a meaningful review or context route. */
  priority: number;
  higherConcern: number;
  context: number;
  protective: number;
  /** Unique signals that are present in the panel but not called in this file. */
  unassessed: number;
}

const PRIORITY_SEVERITIES = new Set(['high_risk', 'confirmation_required', 'moderate_risk']);
const HIGHER_CONCERN_SEVERITIES = new Set(['high_risk', 'confirmation_required']);
const CONTEXT_SEVERITIES = new Set(['low_risk', 'trait', 'context_dependent']);
const PROTECTIVE_SEVERITIES = new Set(['protective']);

function hasAny(values: readonly string[], candidates: Set<string>): boolean {
  return values.some((value) => candidates.has(value));
}

/**
 * Calculate overview counts from canonical findings, not raw pack rows.
 * Cross-pack reuse remains available in Clinical/technical views but does not
 * make the Simple overview look like one signal was many separate findings.
 */
export function computeReportOverviewStats(report: Pick<GeneratedReport, 'sections'>): ReportOverviewStats {
  const stats: ReportOverviewStats = {
    priority: 0,
    higherConcern: 0,
    context: 0,
    protective: 0,
    unassessed: 0,
  };

  for (const group of buildCanonicalFindingGroups(report)) {
    if (group.callState === 'unknown') {
      stats.unassessed += 1;
      continue;
    }

    // Higher concern is a subset of the review queue, not a competing bucket.
    // Keeping both counts here lets the banner explain the queue without
    // under-counting the findings that deserve the strongest attention.
    if (hasAny(group.severityClasses, PRIORITY_SEVERITIES)) {
      stats.priority += 1;
      if (hasAny(group.severityClasses, HIGHER_CONCERN_SEVERITIES)) {
        stats.higherConcern += 1;
      }
      continue;
    }

    if (hasAny(group.severityClasses, PROTECTIVE_SEVERITIES)) {
      stats.protective += 1;
      continue;
    }

    // Only authored context classes belong in the context bucket. Benign and
    // other non-actionable technical rows remain available in the full report
    // without inflating the user-facing overview.
    if (hasAny(group.severityClasses, CONTEXT_SEVERITIES)) {
      stats.context += 1;
    }
  }

  return stats;
}
