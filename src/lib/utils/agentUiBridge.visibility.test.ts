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

  it('does not treat display-contents wrappers or fixed overlays as report bounds', () => {
    expect(source).toContain("if (style.position === 'fixed') return true;");
    expect(source).toContain("['hidden', 'clip'].includes(parentStyle.overflow)");
    expect(source).toContain('function getLayoutParent(element: HTMLElement, root: HTMLElement)');
    expect(source).toContain("getComputedStyle(parent).display === 'contents'");
    expect(source).toContain('const parent = getLayoutParent(element, root);');
    expect(source).toContain('parent?.getBoundingClientRect().width');
    expect(source).not.toContain('const parentWidth = parent?.clientWidth');
  });

  it('provides an unambiguous section-toggle action for local QA', () => {
    expect(source).toContain('clickSection: (sectionName: string)');
    expect(source).toContain("document.querySelectorAll<HTMLButtonElement>('.section-toggle')");
    expect(source).toContain('candidate.querySelector(\'.section-heading-label\')');
    expect(source).toContain('clickSectionByName(sectionName)');
    expect(source).toContain('const wasExpanded = button.getAttribute(\'aria-expanded\') === \'true\';');
    expect(source).toContain('${wasExpanded ? \'collapsed\' : \'expanded\'}: ${sectionName}');
  });

  it('exposes aggregate tooltip bounds without exposing tooltip content', () => {
    expect(source).toContain('export interface AgentUiTooltipMetrics');
    expect(source).toContain('tooltip: AgentUiTooltipMetrics;');
    expect(source).toContain('function collectTooltipMetrics()');
    expect(source).toContain('withinViewportCount');
    expect(source).toContain('accessiblePanelCount');
    expect(source).toContain('maxRightOverflow');
    expect(source).not.toContain('panel.textContent');
  });

  it('exposes aggregate focus geometry without exposing report content', () => {
    expect(source).toContain('focusControlOverlapsContent: boolean;');
    expect(source).toContain('focusControlBottom: number | null;');
    expect(source).toContain('firstContentTop: number | null;');
    expect(source).toContain("document.querySelector<HTMLElement>('.focus-toggle')");
    expect(source).toContain("document.querySelector<HTMLElement>('.main-content > *')");
  });

  it('exposes only aggregate toolbar and action-queue geometry plus the selected theme mode', () => {
    expect(source).toContain('actionQueueWidth: number | null;');
    expect(source).toContain('themeControlInToolbar: boolean;');
    expect(source).toContain("document.querySelector<HTMLElement>('.action-queue-list')");
    expect(source).toContain("document.querySelector('.focus-toolbar .theme-toggle') !== null");
    expect(source).toContain("themeMode: 'system' | 'light' | 'dark' | null;");
    expect(source).toContain('document.documentElement.dataset.theme');
    expect(source).not.toContain('actionQueue.textContent');
  });
});
