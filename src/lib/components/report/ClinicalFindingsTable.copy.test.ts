import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ClinicalFindingsTable.svelte', import.meta.url), 'utf8');

describe('ClinicalFindingsTable copy', () => {
  it('keeps repeated next-step guidance concise', () => {
    expect(source).toContain("return 'Review need for clinical confirmation.'");
    expect(source).toContain("return 'Interpret with history and current guidance.'");
    expect(source).toContain("return 'Confirmation needed';");
    expect(source).toContain("return 'Contextual result';");
    expect(source).toContain("return 'Review blocked';");
    expect(source).not.toContain('Discuss medical-grade confirmation before making health decisions.');
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
});
