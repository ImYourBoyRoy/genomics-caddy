// ./src/lib/utils/planActions.test.ts
import { describe, expect, it } from 'vitest';
import type { ActionablePlan, TopFinding } from './actionabilityEngine';
import {
  ACTION_DOMAINS,
  actionDomainsForFinding,
  actionsForFinding,
  buildPlanActionItems,
  evidenceMeterLevel,
  groupActionsByDomain,
  groupFindingsByArea,
} from './planActions';

const rec = (id: string, name: string, linkIds: string[], extra: Record<string, unknown> = {}) => ({
  recommendation_id: id,
  name,
  category: 'food',
  basis_rule_ids: [],
  basis_topic_ids: [],
  basis_marker_ids: linkIds,
  basis_marker_link_ids: linkIds,
  basis_genes: ['GENEA'],
  basis_marker_count: linkIds.length,
  basis_rule_count: 1,
  dna_basis: '',
  why_it_appears: `${name} because of the linked pathway`,
  evidence_level: 'Tier B',
  when_relevant: '',
  conflicts: [],
  ...extra,
});

const plan = {
  labTests: [
    { canonical_id: 'ferritin', name: 'Ferritin', purpose: 'Iron stores', reason: 'r', reason_topics: [], basis_rule_ids: [], basis_marker_ids: ['rs1'], basis_genes: ['HFE'], urgency: 'consider', tier: 'discuss', category: 'Iron' },
    { canonical_id: 'b12', name: 'B12', purpose: 'B12 level', reason: 'r', reason_topics: [], basis_rule_ids: [], basis_marker_ids: ['rs-unrelated'], basis_genes: [], urgency: 'routine', tier: 'optional', category: 'Vitamins' },
  ],
  medicationPathways: [
    { id: 'cyp2c19', label: 'Clopidogrel pathway', direction_label: 'Reduced activation', genes: ['CYP2C19'], detail: 'Ask about alternatives', matchedMarkerCount: 1, matchedMarkerLinkIds: ['link-1'] },
  ],
  conditionEvidence: [
    { id: 'hemochromatosis', label: 'Iron overload pattern', plain_meaning: 'Iron handling signal', matched_marker_link_ids: ['link-1'], genes: ['HFE'], relative_signal_label: 'Higher signal', evidence_label: 'x' },
  ],
  diet: { favorItems: [rec('d1', 'Leafy greens', ['link-1'])], avoidItems: [rec('d2', 'Iron-fortified cereal', ['link-1'])], favor: [], avoid: [] },
  activity: { recommendationItems: [rec('a1', 'Zone-2 base building', ['link-9'], { category: 'activity', kind: 'build' })] },
  supplements: [rec('s1', 'Magnesium glycinate', ['link-1'], { reason: 'Supports relaxation' })],
  supplementAvoid: [rec('s2', 'Iron supplement', ['link-1'], { reason: 'Avoid unless confirmed low' })],
} as unknown as ActionablePlan;

const finding = (overrides: Partial<TopFinding>): TopFinding => ({
  finding_id: 'f', source_marker_ids: [], rsid: 'rs1', gene: 'HFE', link_id: 'link-1', section_name: 'Iron & Blood',
  ...overrides,
} as unknown as TopFinding);

describe('planActions', () => {
  const items = buildPlanActionItems(plan);

  it('tags every action with one of the six domains and keeps the avoid polarity', () => {
    expect(new Set(items.map((item) => item.domain))).toEqual(new Set(ACTION_DOMAINS));
    expect(items.find((item) => item.id === 'diet:avoid:d2')?.polarity).toBe('avoid');
    expect(items.find((item) => item.id === 'supplements:avoid:s2')?.polarity).toBe('avoid');
    expect(items.find((item) => item.id === 'exercise:a1')?.polarity).toBe('do');
  });

  it('routes confirmation-bound domains to discuss', () => {
    expect(items.find((item) => item.domain === 'medicines')?.polarity).toBe('discuss');
    expect(items.find((item) => item.id === 'tests:ferritin')?.polarity).toBe('discuss');
  });

  it('groups by domain with every domain present', () => {
    const grouped = groupActionsByDomain(items);
    expect(Object.keys(grouped)).toEqual([...ACTION_DOMAINS]);
    expect(grouped.diet).toHaveLength(2);
  });

  it('links actions to a finding by link id or rsid, not by gene guesses', () => {
    const linked = actionsForFinding(items, finding({}));
    expect(linked.map((item) => item.id)).toContain('tests:ferritin');
    expect(linked.map((item) => item.id)).toContain('medicines:cyp2c19');
    expect(linked.map((item) => item.id)).not.toContain('tests:b12');
    expect(linked.map((item) => item.id)).not.toContain('exercise:a1');
    expect(actionDomainsForFinding(items, finding({}))).toEqual(['tests', 'medicines', 'conditions', 'diet', 'supplements']);
  });

  it('matches through retained source marker ids for merged findings', () => {
    const merged = finding({ link_id: 'other', rsid: 'rsX', source_marker_ids: ['link-9'] });
    expect(actionsForFinding(items, merged).map((item) => item.id)).toEqual(['exercise:a1']);
  });

  it('groups ranked findings by area and preserves global rank', () => {
    const groups = groupFindingsByArea([
      finding({ finding_id: '1', section_name: 'Iron & Blood' }),
      finding({ finding_id: '2', section_name: 'Sleep' }),
      finding({ finding_id: '3', section_name: 'Iron & Blood' }),
    ]);
    expect(groups.map((group) => group.label)).toEqual(['Iron & Blood', 'Sleep']);
    expect(groups[0].findings.map((row) => row.rank)).toEqual([1, 3]);
    expect(groupFindingsByArea([finding({}), finding({}), finding({})], 2).flatMap((g) => g.findings)).toHaveLength(2);
  });

  it('scores the evidence meter from the best tier band', () => {
    expect(evidenceMeterLevel(['C_x', 'A_clinical_actionable'])).toBe(4);
    expect(evidenceMeterLevel(['D_research'])).toBe(1);
    expect(evidenceMeterLevel(['E_boundary', ''])).toBe(0);
    expect(evidenceMeterLevel([])).toBe(0);
  });
});
