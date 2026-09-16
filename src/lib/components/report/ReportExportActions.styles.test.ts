import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ReportExportActions.svelte', import.meta.url), 'utf8');

describe('report export actions', () => {
  it('makes the AI-ready JSON handoff the primary export', () => {
    expect(source).toContain('aria-labelledby="report-export-title"');
    expect(source).toContain('AI-ready JSON');
    expect(source).toContain('Clinician handoff');
    expect(source).toContain('Personal report');
    expect(source).toContain('AI review bundle');
    expect(source).toContain('Curated pack JSON');
    expect(source).toContain('Full catalog associations');
    expect(source).toContain('Local-only');
  });

  it('keeps technical and secondary exports behind More', () => {
    const moreIndex = source.indexOf('<details class="report-export-more">');
    const aiBundleIndex = source.indexOf('AI review bundle');
    const technicalIndex = source.indexOf('Curated pack JSON');
    expect(moreIndex).toBeGreaterThan(-1);
    expect(aiBundleIndex).toBeGreaterThan(-1);
    expect(aiBundleIndex).toBeLessThan(moreIndex);
    expect(technicalIndex).toBeGreaterThan(moreIndex);
    expect(source).toContain('<summary>More</summary>');
  });
});
