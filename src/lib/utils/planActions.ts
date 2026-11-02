// ./src/lib/utils/planActions.ts
/*
Purpose: Turn the existing ActionablePlan into one flat, domain-tagged action
list so the Report rows and the Plan tab describe the same next steps.
Run: imported by ReviewFirstList.svelte and PlanView.svelte; unit-tested by
planActions.test.ts.
Inputs: ActionablePlan (deriveActionablePlan) and TopFinding rows.
Outputs: PlanActionItem records, per-finding action lookups, review-first
area groups. Pure functions; no genotype values are read or serialized.
Notes: Wording comes from the authored plan data. This module only routes and
tags it; evidence tiers, avoid lists, and confirmation language are preserved.
*/

import type { ActionablePlan, TopFinding } from './actionabilityEngine';
import { getCompactSupplementName, getCompactSupplementReason } from './layperson';

export const ACTION_DOMAINS = ['tests', 'medicines', 'conditions', 'diet', 'exercise', 'supplements'] as const;
export type ActionDomain = (typeof ACTION_DOMAINS)[number];

export const ACTION_DOMAIN_LABELS: Record<ActionDomain, string> = {
  tests: 'Tests',
  medicines: 'Medicines',
  conditions: 'Conditions',
  diet: 'Diet',
  exercise: 'Exercise',
  supplements: 'Supplements',
};

/** do = lean in, avoid = steer away, discuss = confirm with a professional first. */
export type ActionPolarity = 'do' | 'avoid' | 'discuss';

export interface PlanActionItem {
  id: string;
  domain: ActionDomain;
  polarity: ActionPolarity;
  text: string;
  detail: string;
  /** Marker identifiers (link ids and rsids) that this action is derived from. */
  markerIds: string[];
  genes: string[];
  /** Short evidence label for the evidence meter, when the plan provides one. */
  evidence: string;
}

export type PlanActionsByDomain = Record<ActionDomain, PlanActionItem[]>;

function uniqueStrings(values: readonly (string | undefined | null)[]): string[] {
  return [...new Set(values.map((value) => (value ?? '').trim()).filter(Boolean))];
}

/** Flatten the plan into domain-tagged actions. Order within a domain follows the plan. */
export function buildPlanActionItems(plan: ActionablePlan): PlanActionItem[] {
  const items: PlanActionItem[] = [];

  for (const lab of plan.labTests) {
    items.push({
      id: `tests:${lab.canonical_id}`,
      domain: 'tests',
      polarity: lab.tier === 'optional' ? 'do' : 'discuss',
      text: lab.name,
      detail: lab.purpose || lab.reason,
      markerIds: uniqueStrings(lab.basis_marker_ids),
      genes: uniqueStrings(lab.basis_genes),
      evidence: lab.tier,
    });
  }

  for (const pathway of plan.medicationPathways) {
    items.push({
      id: `medicines:${pathway.id}`,
      domain: 'medicines',
      polarity: 'discuss',
      text: pathway.label,
      detail: pathway.detail,
      markerIds: uniqueStrings(pathway.matchedMarkerLinkIds),
      genes: uniqueStrings(pathway.genes),
      evidence: pathway.direction_label,
    });
  }

  for (const condition of plan.conditionEvidence) {
    items.push({
      id: `conditions:${condition.id}`,
      domain: 'conditions',
      polarity: 'discuss',
      text: condition.label,
      detail: condition.plain_meaning,
      markerIds: uniqueStrings(condition.matched_marker_link_ids),
      genes: uniqueStrings(condition.genes),
      evidence: condition.relative_signal_label || condition.evidence_label,
    });
  }

  for (const [polarity, list] of [['do', plan.diet.favorItems], ['avoid', plan.diet.avoidItems]] as const) {
    for (const item of list) {
      items.push({
        id: `diet:${polarity}:${item.recommendation_id}`,
        domain: 'diet',
        polarity,
        text: item.name,
        detail: item.why_it_appears,
        markerIds: uniqueStrings(item.basis_marker_link_ids),
        genes: uniqueStrings(item.basis_genes),
        evidence: item.evidence_level,
      });
    }
  }

  for (const item of plan.activity.recommendationItems) {
    items.push({
      id: `exercise:${item.recommendation_id}`,
      domain: 'exercise',
      polarity: item.kind === 'build' ? 'do' : item.kind === 'watch' ? 'avoid' : 'discuss',
      text: item.name,
      detail: item.why_it_appears,
      markerIds: uniqueStrings(item.basis_marker_link_ids),
      genes: uniqueStrings(item.basis_genes),
      evidence: item.evidence_level,
    });
  }

  for (const [polarity, list] of [['do', plan.supplements], ['avoid', plan.supplementAvoid]] as const) {
    for (const item of list) {
      items.push({
        id: `supplements:${polarity}:${item.recommendation_id}`,
        domain: 'supplements',
        polarity,
        text: getCompactSupplementName(item.name),
        detail: getCompactSupplementReason(item.reason || item.why_it_appears),
        markerIds: uniqueStrings(item.basis_marker_link_ids),
        genes: uniqueStrings(item.basis_genes),
        evidence: item.evidence_level,
      });
    }
  }

  return items;
}

export function groupActionsByDomain(items: readonly PlanActionItem[]): PlanActionsByDomain {
  const grouped = {} as PlanActionsByDomain;
  for (const domain of ACTION_DOMAINS) grouped[domain] = [];
  for (const item of items) grouped[item.domain].push(item);
  return grouped;
}

function findingMarkerIds(finding: TopFinding): Set<string> {
  return new Set(uniqueStrings([finding.link_id, finding.rsid, ...finding.source_marker_ids]));
}

/** Actions whose DNA basis includes this finding, matched by marker id (never by genotype). */
export function actionsForFinding(items: readonly PlanActionItem[], finding: TopFinding): PlanActionItem[] {
  const ids = findingMarkerIds(finding);
  return items.filter((item) => item.markerIds.some((id) => ids.has(id)));
}

/** One representative action per domain so a collapsed row stays scannable. */
export function actionDomainsForFinding(items: readonly PlanActionItem[], finding: TopFinding): ActionDomain[] {
  const present = new Set(actionsForFinding(items, finding).map((item) => item.domain));
  return ACTION_DOMAINS.filter((domain) => present.has(domain));
}

export interface ReviewFirstGroup {
  /** Stable key derived from the finding section name. */
  id: string;
  label: string;
  findings: { finding: TopFinding; rank: number }[];
}

/**
 * Group the ranked top findings by report section while keeping global rank
 * order inside each group and between groups (a group sorts by its best rank).
 */
export function groupFindingsByArea(findings: readonly TopFinding[], limit = 9): ReviewFirstGroup[] {
  const groups = new Map<string, ReviewFirstGroup>();
  findings.slice(0, limit).forEach((finding, index) => {
    const label = finding.section_name || 'Other findings';
    const id = label.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'other';
    const group = groups.get(id) ?? { id, label, findings: [] };
    group.findings.push({ finding, rank: index + 1 });
    groups.set(id, group);
  });
  return [...groups.values()];
}

/** Segments (0-4) for the evidence meter from the best tier band among a finding's markers. */
export function evidenceMeterLevel(tiers: readonly string[]): number {
  const levels: Record<string, number> = { A: 4, B: 3, C: 2, D: 1 };
  return tiers.reduce((best, tier) => Math.max(best, levels[String(tier || '').trim().charAt(0).toUpperCase()] ?? 0), 0);
}
