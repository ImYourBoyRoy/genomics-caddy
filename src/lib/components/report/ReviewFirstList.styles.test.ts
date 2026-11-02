// ./src/lib/components/report/ReviewFirstList.styles.test.ts
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { ACTION_DOMAINS } from '../../utils/planActions';

const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');
const rows = read('./ReviewFirstList.svelte');
const plan = read('./PlanView.svelte');
const rowsCss = read('../../styles/components/finding-rows.css');
const planCss = read('../../styles/components/plan-view.css');

describe('Review-first rows and Plan view', () => {
  it('renders disclosure rows with accessible expansion state', () => {
    expect(rows).toContain('aria-expanded={open}');
    expect(rows).toContain('aria-controls={open ? `review-detail-${finding.finding_id}` : undefined}');
    expect(rows).toContain('aria-labelledby="review-first-title"');
    expect(rows).toContain('data-priority={rank}');
    expect(rows).toContain('data-concern={concernFor(finding)}');
    expect(rows).toContain('This points you to');
  });

  it('never repeats safety boilerplate inside rows or plan items', () => {
    for (const source of [rows, plan]) {
      expect(source).not.toMatch(/not a diagnosis|talk to your doctor|consult a (?:doctor|clinician)/i);
    }
  });

  it('keeps stylesheets token-only and under the 500-line budget', () => {
    for (const css of [rowsCss, planCss]) {
      expect(css).not.toMatch(/#[0-9a-f]{3,8}\b/i);
      expect(css).not.toMatch(/\brgba?\(/i);
      expect(css.split('\n').length).toBeLessThanOrEqual(500);
    }
  });

  it('styles a tag for every action domain', () => {
    for (const domain of ACTION_DOMAINS) {
      expect(rowsCss).toContain(`.domain-tag[data-domain="${domain}"]`);
    }
  });

  it('exposes filters as a labelled pressed-state group and keeps proof navigation', () => {
    expect(plan).toContain('role="group" aria-label="Filter plan by area"');
    expect(plan).toContain('aria-pressed={activeDomain === domain}');
    expect(plan).toContain('View proof');
    expect(plan).toContain('buildLabRequestListText(plan.labTests)');
  });
});
