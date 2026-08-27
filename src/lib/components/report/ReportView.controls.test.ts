import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('./ReportView.svelte', import.meta.url), 'utf8');
const theme = readFileSync(new URL('../../styles/theme.css', import.meta.url), 'utf8');

describe('Simple-first report controls', () => {
  it('keeps advanced filters in a disclosure that opens by default outside Simple mode', () => {
    expect(source).toContain('<div class="report-controls no-print">');
    expect(source).toContain('<details class="report-filter-details" open={presentationMode !== \'simple\'}>');
    expect(source).toContain('<summary>Filters &amp; ordering</summary>');
  });

  it('keeps the reading-mode group available outside the advanced filter disclosure', () => {
    const detailsStart = source.indexOf('<details class="report-filter-details"');
    const detailsEnd = source.indexOf('</details>', detailsStart);
    const filterDetails = source.slice(detailsStart, detailsEnd);
    const modeGroupIndex = source.indexOf('<div class="mode-group"');

    expect(filterDetails).toContain('bind:checked={showBenign}');
    expect(filterDetails).toContain('bind:value={tierFilter}');
    expect(filterDetails).toContain('bind:value={sortBy}');
    expect(filterDetails).not.toContain('class="mode-group"');
    expect(modeGroupIndex).toBeGreaterThan(detailsEnd);
  });

  it('gives the disclosure a keyboard-sized target and preserves narrow mode controls', () => {
    const filterSummary = theme.match(/\.report-filter-details > summary \{[\s\S]*?\n\}/)?.[0] ?? '';
    const narrow = theme.match(/@media \(max-width: 720px\) \{[\s\S]*?\n\}/)?.[0] ?? '';

    expect(filterSummary).toContain('min-height: 44px;');
    expect(filterSummary).toContain('var(--surface-subtle)');
    expect(theme).toContain('.report-filter-details[open] .filter-bar');
    expect(narrow).toContain('.report-controls');
    expect(narrow).toContain('.report-controls .view-mode-buttons');
    expect(narrow).toContain('.report-controls .view-mode-btn');
  });

  it('routes section collapse changes through an explicit parent updater', () => {
    expect(source).toContain('function setSectionCollapsed(sectionName: string, isCollapsed: boolean)');
    expect(source).toContain('collapsed={collapsedSections[section.name]}');
    expect(source).toContain('onCollapsedChange={(isCollapsed) => setSectionCollapsed(section.name, isCollapsed)}');
    expect(source).not.toContain('bind:collapsed={collapsedSections[section.name]}');
  });
});
