import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(process.cwd(), 'src/lib/components/report/ReportHeader.svelte'), 'utf8');
const badgeStyles = source.slice(source.indexOf('.overall-summary-badge'), source.indexOf('.technical-score-details'));

describe('ReportHeader summary badge', () => {
  it('does not repeat the title-row sex result in the report body', () => {
    expect(source).not.toContain('health-summary-row');
    expect(source).not.toContain('formatGeneticSexLabel');
    expect(source).not.toContain('Sex estimate from DNA');
  });

  it('keeps Simple-mode coverage compact and avoids duplicate technical copy', () => {
    expect(source).toContain('<span class="quality-kicker">DNA coverage</span>');
    expect(source).toContain('<span>markers called</span>');
    expect(source).toContain('<span class="quality-note">Uncalled = unknown</span>');
    expect(source).not.toContain('Markers Checked');
    expect(source).not.toContain('curated SNPs found');
  });

  it('uses plain-language aggregate summary text in Simple mode', () => {
    const simpleStart = source.indexOf("if (mode === 'simple')");
    const clinicalStart = source.indexOf('const parts = []', simpleStart);
    const simpleMode = source.slice(simpleStart, clinicalStart);

    expect(source).toContain("if (mode === 'simple')");
    expect(source).toContain('const associationCount = high + mod;');
    expect(source).toContain('possible protective ${prot === 1 ? \'association\' : \'associations\'}');
    expect(source).toContain("return simpleParts.join(' · ');");
    expect(source).toContain('computeSummaryLine(generatedReport, presentationMode)');
    expect(simpleMode).not.toContain('stronger association');
  });

  it('uses semantic theme tokens for association context', () => {
    expect(badgeStyles).toContain('var(--status-accent-bg)');
    expect(badgeStyles).toContain('var(--status-accent-border)');
    expect(badgeStyles).toContain('var(--status-accent-text)');
  });

  it('keeps long report header content inside its flex layout', () => {
    expect(source).toContain('.report-desc {');
    expect(source).toContain('flex: 1 1 auto;');
    expect(source).toContain('min-width: 0;');
    expect(source).toContain('overflow-wrap: anywhere;');
    expect(source).toContain('max-width: 100%;');
  });

  it('does not contain raw color literals in the badge style', () => {
    expect(badgeStyles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });
});
