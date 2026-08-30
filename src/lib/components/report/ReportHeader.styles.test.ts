import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/report/ReportHeader.svelte'), 'utf8');
const styleBlock = source.match(/<style>([\s\S]*?)<\/style>/)?.[1] ?? '';
const summaryStyles = styleBlock.match(/\.overall-summary \{[\s\S]*?\n  \}/)?.[0] ?? '';

describe('ReportHeader summary', () => {
  it('does not repeat the title-row sex result in the report body', () => {
    expect(source).not.toContain('health-summary-row');
    expect(source).not.toContain('formatGeneticSexLabel');
    expect(source).not.toContain('Sex estimate from DNA');
  });

  it('keeps Simple mode as a wide, non-sticky mini stats banner', () => {
    expect(source).toContain('<div class="simple-report-overview" aria-label="Report overview">');
    expect(source).toContain('<span class="quality-kicker">Report overview</span>');
    expect(source).toContain('on review board');
    expect(source).toContain('class="report-stat-grid" aria-label="Report overview statistics"');
    expect(source).toContain('Review queue');
    expect(source).toContain('Higher concern');
    expect(source).toContain('Context findings');
    expect(source).toContain('Protective context');
    expect(source).toContain('<span>DNA coverage</span>');
    expect(source).toContain('role="progressbar"');
    expect(source).toContain('class="overview-coverage-meter"');
    expect(source).toContain('coveragePercent');
    expect(source).toContain('--report-dashboard-surface-width');
    expect(source).toContain('box-sizing: border-box;');
    expect(source).not.toContain('Markers Checked');
    expect(source).not.toContain('curated SNPs found');
  });

  it('uses plain-language aggregate summary text in Simple mode', () => {
    const simpleStart = source.indexOf("if (mode === 'simple')");
    const clinicalStart = source.indexOf('const parts = []', simpleStart);
    const simpleMode = source.slice(simpleStart, clinicalStart);

    expect(source).toContain("if (mode === 'simple')");
    expect(source).toContain('const associationCount = high + mod;');
    expect(source).toContain('protective-context ${prot === 1 ? \'finding\' : \'findings\'}');
    expect(source).toContain('research ${associationCount === 1 ? \'finding\' : \'findings\'} to review');
    expect(source).toContain("return simpleParts.join(' · ');");
    expect(source).toContain('computeSummaryLine(generatedReport, presentationMode)');
    expect(simpleMode).not.toContain('stronger association');
  });

  it('keeps the aggregate research count visually secondary to the report title', () => {
    expect(source).toContain('<span class="overall-summary" aria-label="Report finding summary">{summaryLine}</span>');
    expect(summaryStyles).toContain('display: block;');
    expect(summaryStyles).toContain('color: var(--text-secondary);');
    expect(summaryStyles).not.toContain('border:');
    expect(summaryStyles).not.toContain('background:');
  });

  it('keeps long report header content inside its flex layout', () => {
    expect(source).toContain('.simple-report-overview {');
    expect(source).toContain('grid-template-columns: minmax(14rem, 1fr) minmax(0, 2.4fr) minmax(9rem, 13rem);');
    expect(source).toContain('.report-desc {');
    expect(source).toContain('flex: 1 1 auto;');
    expect(source).toContain('min-width: 0;');
    expect(source).toContain('overflow-wrap: anywhere;');
    expect(source).toContain('max-width: 100%;');
  });

  it('does not contain raw color literals in the summary style', () => {
    expect(summaryStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
