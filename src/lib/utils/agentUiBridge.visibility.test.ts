import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./agentUiBridge.ts', import.meta.url), 'utf8');

describe('agent UI bridge visibility boundary', () => {
  it('keeps closed disclosure summaries visible but excludes their hidden bodies', () => {
    expect(source).toContain("const closedDetails = element.closest('details:not([open])');");
    expect(source).toContain('if (closedDetails && !element.closest(\'summary\')) return false;');
  });

  it('reports only safe width metadata for overflowing report descendants', () => {
    expect(source).toContain('overflowingElements: Array<{');
    expect(source).toContain('classes: Array.from(element.classList).slice(0, 4),');
    expect(source).toContain('rightOverflow: Math.max(0, Math.round(rightOverflow)),');
    expect(source).toContain('parentClasses,');
    expect(source).toContain("element.closest('details:not([open])')");
    expect(source).toContain("element.closest('.clinical-findings-table thead')");
    expect(source).toContain("element.classList.contains('sr-only')");
    expect(source).not.toContain('element.textContent');
    expect(source).not.toContain('element.id');
  });

  it('provides an unambiguous section-toggle action for local QA', () => {
    expect(source).toContain('clickSection: (sectionName: string)');
    expect(source).toContain("document.querySelectorAll<HTMLButtonElement>('.section-toggle')");
    expect(source).toContain('candidate.querySelector(\'.section-heading-label\')');
    expect(source).toContain('clickSectionByName(sectionName)');
    expect(source).toContain('const wasExpanded = button.getAttribute(\'aria-expanded\') === \'true\';');
    expect(source).toContain('${wasExpanded ? \'collapsed\' : \'expanded\'}: ${sectionName}');
  });
});
