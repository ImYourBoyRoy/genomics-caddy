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

  it('supports privacy-safe keyboard interaction checks for local QA', () => {
    expect(source).toContain('focusText: (text: string)');
    expect(source).toContain('pressKey: (key: string)');
    expect(source).toContain('function focusByVisibleText(text: string)');
    expect(source).toContain('function pressAllowlistedKey(key: string)');
    expect(source).toContain("summary, input, select, textarea, [role=\"button\"]");
    expect(source).toContain('document.activeElement !== hit.el');
    expect(source).toContain("['Escape', 'Tab', 'Enter', ' '].includes(normalizedKey)");
    expect(source).toContain('activeElement.dispatchEvent(new KeyboardEvent');
  });

  it('can activate native disclosure summaries during fixture QA', () => {
    expect(source).toContain('button, a, summary, [role="button"], .tab-btn, .card-header');
    expect(source).toContain('button, a[href], summary, input, select, textarea');
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

  it('exposes only aggregate landmark and ARIA relationship metrics', () => {
    expect(source).toContain('export interface AgentUiAccessibilityMetrics');
    expect(source).toContain('accessibility: AgentUiAccessibilityMetrics;');
    expect(source).toContain('function collectAccessibilityMetrics()');
    expect(source).toContain("document.querySelectorAll('main.main-content').length");
    expect(source).toContain("document.querySelectorAll('aside#data-sidebar[aria-label]').length");
    expect(source).toContain("document.querySelectorAll('.focus-toolbar[role=\"toolbar\"][aria-label]').length");
    expect(source).toContain("document.querySelectorAll('[role=\"tabpanel\"][aria-labelledby]').length");
    expect(source).toContain('boundSectionToggleCount');
    expect(source).toContain('guidanceToggleCount');
    expect(source).toContain('boundGuidanceToggleCount');
    expect(source).toContain('expandedControlCount');
    expect(source).toContain('boundExpandedControlCount');
    expect(source).toContain("focusControl?.getAttribute('aria-controls') === 'data-sidebar'");
    expect(source).not.toContain('accessibility.textContent');
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
    expect(source).toContain('actionQueueItemMaxWidth: number | null;');
    expect(source).toContain('actionQueueItemCount: number;');
    expect(source).toContain('connectionsLaunchHeight: number | null;');
    expect(source).toContain('liftoverStatusHeight: number | null;');
    expect(source).toContain('markerGridWidth: number | null;');
    expect(source).toContain('markerGridColumnGap: number | null;');
    expect(source).toContain('markerCardMinWidth: number | null;');
    expect(source).toContain('markerCardMaxWidth: number | null;');
    expect(source).toContain('expandedSectionNames: string[];');
    expect(source).toContain('clinicalProvenanceCount: number;');
    expect(source).toContain('themeControlInToolbar: boolean;');
    expect(source).toContain("document.querySelector<HTMLElement>('.action-queue-list')");
    expect(source).toContain("document.querySelector('.focus-toolbar .theme-toggle') !== null");
    expect(source).toContain("themeMode: 'system' | 'light' | 'dark' | null;");
    expect(source).toContain('document.documentElement.dataset.theme');
    expect(source).not.toContain('actionQueue.textContent');
    expect(source).toContain("actionQueue?.querySelectorAll('.action-queue-item').length ?? 0");
    expect(source).toContain('markerGrid ? Math.round(markerGrid.getBoundingClientRect().width) : null');
    expect(source).toContain('getGridColumnGap(markerGrid)');
    expect(source).toContain("getWidthBounds('.marker-card')");
    expect(source).toContain("getWidthBounds('.action-queue-item')");
    expect(source).toContain("document.querySelectorAll<HTMLButtonElement>('.section-toggle[aria-expanded=\"true\"]')");
    expect(source).toContain('expandedSectionNames,');
    expect(source).toContain("clinicalProvenanceCount: document.querySelectorAll('.clinical-provenance').length");
    expect(source).toContain('connectionsLaunch ? Math.round(connectionsLaunch.getBoundingClientRect().height) : null');
    expect(source).toContain('liftoverStatus ? Math.round(liftoverStatus.getBoundingClientRect().height) : null');
  });
});
