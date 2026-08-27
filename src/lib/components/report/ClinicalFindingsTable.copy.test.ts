import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./ClinicalFindingsTable.svelte', import.meta.url), 'utf8');

describe('ClinicalFindingsTable copy', () => {
  it('keeps repeated next-step guidance concise', () => {
    expect(source).toContain("return 'Review need for clinical confirmation.'");
    expect(source).toContain("return 'Interpret with history and current guidance.'");
    expect(source).not.toContain('Discuss medical-grade confirmation before making health decisions.');
  });

  it('stacks the clinical table before the sidebar-constrained widths become cramped', () => {
    expect(source).toContain('@media (max-width: 1100px)');
    expect(source).toContain('.clinical-findings-table {\n      min-width: 0;');
    expect(source).toContain('.clinical-findings-table thead {');
  });
});
