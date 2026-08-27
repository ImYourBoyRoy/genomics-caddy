import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ClinicalFindingsTable.svelte', import.meta.url), 'utf8');

describe('ClinicalFindingsTable copy', () => {
  it('keeps repeated next-step guidance concise', () => {
    expect(source).toContain('function authoredFollowUp(marker: EvaluatedMarker): string');
    expect(source).toContain('const visibleLabels = labels.slice(0, 3).join(\'; \');');
    expect(source).toContain('return remainingCount > 0 ? `${visibleLabels} (+${remainingCount} more)` : visibleLabels;');
    expect(source).toContain('const followUp = authoredFollowUp(marker);');
    expect(source).toContain('if (followUp) return followUp;');
    expect(source).toContain("return 'Review need for clinical confirmation.'");
    expect(source).toContain("return 'Interpret with history and current guidance.'");
    expect(source).toContain("return 'Confirmation needed';");
    expect(source).toContain("return 'Contextual result';");
    expect(source).toContain("return 'Review blocked';");
    expect(source).not.toContain('Discuss medical-grade confirmation before making health decisions.');
    expect(source).not.toContain("return 'Review the listed follow-up.'");
  });

  it('keeps the table caption structural instead of repeating the global reminder', () => {
    expect(source).toContain('<caption>Structured findings for clinical review.</caption>');
    expect(source).not.toContain('Consumer-array calls are not diagnostic.');
    expect(source).not.toContain('getClaimFrame(');
  });

  it('stacks the clinical table before the sidebar-constrained widths become cramped', () => {
    expect(source).toContain('@media (max-width: 1100px)');
    expect(source).toContain('.clinical-findings-table {\n      min-width: 0;');
    expect(source).toContain('.clinical-findings-table thead {');
  });

  it('lets expanded technical details fit inside narrow stacked rows', () => {
    expect(source).toContain('.clinical-details {\n      min-width: 0;\n      max-width: 100%;');
    expect(source).toContain('.clinical-details dl {\n      min-width: 0;\n      max-width: 100%;');
    expect(source).toContain('grid-template-columns: minmax(6rem, 0.45fr) minmax(0, 1fr);');
  });

  it('uses real mobile field labels instead of CSS-only pseudo content', () => {
    expect(source).toContain('<span class="clinical-mobile-label">Finding</span>');
    expect(source).toContain('<span class="clinical-mobile-label">DNA result</span>');
    expect(source).toContain('<span class="clinical-mobile-label">Next helpful step</span>');
    expect(source).toContain('.clinical-mobile-label {\n    display: none;');
    expect(source).toContain('.clinical-findings-table tbody td::before {\n      content: none;');
  });

  it('separates DNA, clinical, self-reported, and clinician-entered data sources', () => {
    expect(source).toContain('<summary>Data sources in this report</summary>');
    expect(source).toContain('<dt>Consumer DNA array</dt>');
    expect(source).toContain('<dt>Clinical confirmation</dt>');
    expect(source).toContain('<dt>Self-reported context</dt>');
    expect(source).toContain('<dt>Clinician-entered information</dt>');
    expect(source).toContain('.clinical-provenance');
  });

  it('keeps the clinical table markup structurally balanced', () => {
    expect(source.match(/<tbody>/g)).toHaveLength(1);
    expect(source.match(/<\/tbody>/g)).toHaveLength(1);
  });
});
