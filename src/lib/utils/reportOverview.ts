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

/** Finding filters shared by the overview tiles and the Focus chips. */
export type ReportFocus = 'all' | 'priority' | 'higher_concern' | 'context' | 'protective';

const FOCUS_SEVERITIES: Record<Exclude<ReportFocus, 'all'>, Set<string>> = {
  priority: PRIORITY_SEVERITIES,
  higher_concern: HIGHER_CONCERN_SEVERITIES,
  context: CONTEXT_SEVERITIES,
  protective: PROTECTIVE_SEVERITIES,
};

export function severityMatchesReportFocus(severity: string, focus: ReportFocus): boolean {
  return focus === 'all' || FOCUS_SEVERITIES[focus].has(severity);
}

function hasAny(values: readonly string[], candidates: Set<string>): boolean {
  return values.some((value) => candidates.has(value));
}

type OverviewBucket = 'unassessed' | 'priority' | 'protective' | 'context' | null;

function overviewBucket(group: CanonicalFindingGroup): OverviewBucket {
  if (group.callState === 'unknown') return 'unassessed';
  // Higher concern is a subset of the review queue, not a competing bucket.
  if (hasAny(group.severityClasses, PRIORITY_SEVERITIES)) return 'priority';
  if (hasAny(group.severityClasses, PROTECTIVE_SEVERITIES)) return 'protective';
  // Only authored context classes belong in the context bucket. Benign and
  // other non-actionable technical rows remain available in the full report
  // without inflating the user-facing overview.
  if (hasAny(group.severityClasses, CONTEXT_SEVERITIES)) return 'context';
  return null;
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
    const bucket = overviewBucket(group);
    if (!bucket) continue;
    stats[bucket] += 1;
    if (bucket === 'priority' && hasAny(group.severityClasses, HIGHER_CONCERN_SEVERITIES)) {
      stats.higherConcern += 1;
    }
  }

  return stats;
}

/**
 * Marker link IDs whose canonical locus lands in the given overview tile, so
 * a tile filter lists exactly the signals its count describes.
 */
export function reportFocusLinkIds(
  report: Pick<GeneratedReport, 'sections'>,
  focus: Exclude<ReportFocus, 'all'>,
): Set<string> {
  const ids = new Set<string>();
  for (const group of buildCanonicalFindingGroups(report)) {
    const bucket = overviewBucket(group);
    const matches = focus === 'higher_concern'
      ? bucket === 'priority' && hasAny(group.severityClasses, HIGHER_CONCERN_SEVERITIES)
      : bucket === focus;
    if (!matches) continue;
    for (const source of group.sourceMarkers) {
      if (severityMatchesReportFocus(source.marker.severity_class, focus)) ids.add(source.marker.link_id);
    }
  }
  return ids;
}
