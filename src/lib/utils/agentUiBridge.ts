// ./src/lib/utils/agentUiBridge.ts
/*
Purpose: Dev/agent UI control surface for MCP and automated QA.
Responsibilities:
- Register window.__GENOMICS_CADDY_UI__ with snapshot + action helpers.
- Listen for Tauri `agent-ui-request` events and reply via `agent-ui-response`.
Key Inputs: Page state accessors provided by +page.svelte.
Key Outputs: Structured UI snapshots and action acknowledgements.
Operational Notes: Active in all builds; actions are local-only (no network). Safe for agent QA.
*/

import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { normalizeUiLabel } from './uiLabels';

export type AgentUiTab =
  | 'report'
  | 'map'
  | 'discovery'
  | 'browser'
  | 'mcp'
  | 'agent'
  | 'research'
  | 'ai';

export interface AgentUiLayoutMetrics {
  viewportWidth: number;
  viewportHeight: number;
  documentClientWidth: number;
  documentScrollWidth: number;
  mainContentWidth: number | null;
  mainContentScrollWidth: number | null;
  sidebarWidth: number | null;
  markerGridColumnCount: number | null;
  markerCardCount: number;
  clinicalTableCount: number;
  activePresentationMode: 'simple' | 'clinical' | 'compare' | null;
  focusMode: boolean;
  overflowingElements: Array<{
    tag: string;
    classes: string[];
    clientWidth: number;
    scrollWidth: number;
    boundingWidth: number;
    rightOverflow: number;
    parentWidth: number;
    parentWidthOverflow: number;
    parentClasses: string[];
  }>;
}

export interface AgentUiSnapshot {
  title: string;
  activeTab: string;
  sample: { id: number; name: string; genetic_sex: string } | null;
  hasReport: boolean;
  reportScore: number | null;
  reportTitle: string | null;
  sectionCount: number;
  labFollowupsCollapsed: boolean | null;
  matchedAlleleLabelPresent: boolean;
  dietaryAlignmentPresent: boolean;
  dualExportButtonsPresent: boolean;
  urgentBadgeCount: number;
  visibleTextSample: string[];
  layout: AgentUiLayoutMetrics;
  href: string;
  capturedAt: string;
}

export interface AgentUiControllers {
  getActiveTab: () => string;
  setActiveTab: (tab: string) => void;
  getSelectedSample: () => { id: number; name: string; genetic_sex: string } | null;
  getReport: () => {
    overall_signal_score?: number;
    title?: string;
    sections?: unknown[];
  } | null;
  expandLabFollowups?: () => void;
  collapseLabFollowups?: () => void;
}

declare global {
  interface Window {
    __GENOMICS_CADDY_UI__?: {
      snapshot: () => AgentUiSnapshot;
      setTab: (tab: string) => { ok: true; activeTab: string };
      expandLabs: () => { ok: boolean; detail: string };
      clickText: (text: string) => { ok: boolean; detail: string };
      queryText: (text: string) => { ok: boolean; count: number; samples: string[] };
    };
  }
}

function collectVisibleText(limit = 40): string[] {
  const out: string[] = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let node = walker.nextNode();
  while (node && out.length < limit) {
    if (!isPublicUiTextNode(node)) {
      node = walker.nextNode();
      continue;
    }
    const text = (node.textContent || '').replace(/\s+/g, ' ').trim();
    if (text.length >= 3) out.push(text.slice(0, 120));
    node = walker.nextNode();
  }
  return out;
}

/**
 * Keep automation output useful without exposing raw genetic values.
 * Technical disclosures and clinical finding rows can contain genotype calls,
 * including when a <details> element is collapsed or visually clipped.
 */
const PRIVATE_UI_SELECTORS = [
  '.genotype-val',
  '.genotype-tag',
  '.technical-details',
  '.clinical-table-wrap',
  '[data-private-genetic-value="true"]',
  '[data-sensitive-genotype="true"]',
].join(', ');

function isPublicUiTextNode(node: Node): boolean {
  const element = node.parentElement;
  if (!element || element.closest(PRIVATE_UI_SELECTORS)) return false;
  if (element.closest('[hidden], [aria-hidden="true"], [inert]')) return false;

  // A closed disclosure keeps its summary visible while hiding its body.
  const closedDetails = element.closest('details:not([open])');
  if (closedDetails && !element.closest('summary')) return false;

  const style = getComputedStyle(element);
  return style.display !== 'none' && style.visibility !== 'hidden';
}

function getGridColumnCount(element: HTMLElement | null): number | null {
  if (!element) return null;
  const template = getComputedStyle(element).gridTemplateColumns.trim();
  if (!template || template === 'none') return null;
  return template.split(/\s+/).length;
}

function getActivePresentationMode(): AgentUiLayoutMetrics['activePresentationMode'] {
  const activeButton = document.querySelector<HTMLElement>('.view-mode-btn[aria-pressed="true"]');
  const label = (activeButton?.textContent || '').toLowerCase();
  if (label.includes('simple')) return 'simple';
  if (label.includes('clinical')) return 'clinical';
  if (label.includes('compare')) return 'compare';
  return null;
}

function isIgnoredLayoutElement(element: HTMLElement): boolean {
  if (element.closest('[hidden], [aria-hidden="true"], [inert]')) return true;
  if (element.closest('details:not([open])') && !element.closest('summary')) return true;
  if (element.closest('.clinical-findings-table thead')) return true;
  if (element.classList.contains('sr-only')) return true;

  const style = getComputedStyle(element);
  if (style.display === 'none' || style.visibility === 'hidden') return true;
  return style.position === 'absolute' && element.clientWidth <= 1 && element.clientHeight <= 1;
}

function collectOverflowingElements(root: HTMLElement | null): AgentUiLayoutMetrics['overflowingElements'] {
  if (!root) return [];

  const rootRight = root.getBoundingClientRect().right;

  return Array.from(root.querySelectorAll<HTMLElement>('*'))
    .filter((element) => !isIgnoredLayoutElement(element))
    .map((element) => {
      const rect = element.getBoundingClientRect();
      const parent = element.parentElement;
      const parentWidth = parent?.clientWidth ?? root.clientWidth;
      const overflow = element.scrollWidth - element.clientWidth;
      const rightOverflow = rect.right - rootRight;
      const parentWidthOverflow = rect.width - parentWidth;
      return { element, overflow, rightOverflow, boundingWidth: rect.width, parentWidth, parentWidthOverflow, parentClasses: parent ? Array.from(parent.classList).slice(0, 4) : [] };
    })
    .filter(({ overflow, rightOverflow, parentWidthOverflow }) => overflow > 1 || rightOverflow > 1 || parentWidthOverflow > 1)
    .sort((a, b) => Math.max(b.overflow, b.rightOverflow, b.parentWidthOverflow) - Math.max(a.overflow, a.rightOverflow, a.parentWidthOverflow))
    .slice(0, 12)
    .map(({ element, rightOverflow, boundingWidth, parentWidth, parentWidthOverflow, parentClasses }) => ({
      tag: element.tagName.toLowerCase(),
      classes: Array.from(element.classList).slice(0, 4),
      clientWidth: Math.round(element.clientWidth),
      scrollWidth: Math.round(element.scrollWidth),
      boundingWidth: Math.round(boundingWidth),
      rightOverflow: Math.max(0, Math.round(rightOverflow)),
      parentWidth: Math.round(parentWidth),
      parentWidthOverflow: Math.max(0, Math.round(parentWidthOverflow)),
      parentClasses,
    }));
}

function collectLayoutMetrics(): AgentUiLayoutMetrics {
  const mainContent = document.querySelector<HTMLElement>('.main-content');
  const sidebar = document.querySelector<HTMLElement>('.sidebar');
  const markerGrid = document.querySelector<HTMLElement>('.markers-grid');

  return {
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    documentClientWidth: document.documentElement.clientWidth,
    documentScrollWidth: document.documentElement.scrollWidth,
    mainContentWidth: mainContent?.getBoundingClientRect().width ?? null,
    mainContentScrollWidth: mainContent?.scrollWidth ?? null,
    sidebarWidth: sidebar?.getBoundingClientRect().width ?? null,
    markerGridColumnCount: getGridColumnCount(markerGrid),
    markerCardCount: document.querySelectorAll('.marker-card').length,
    clinicalTableCount: document.querySelectorAll('.clinical-table-wrap').length,
    activePresentationMode: getActivePresentationMode(),
    focusMode: document.querySelector('.app-layout.focus-mode') !== null,
    overflowingElements: collectOverflowingElements(mainContent),
  };
}

function clickByVisibleText(text: string): { ok: boolean; detail: string } {
  const needle = normalizeUiLabel(text);
  if (!needle) return { ok: false, detail: 'empty text' };

  const candidates = Array.from(
    document.querySelectorAll<HTMLElement>('button, a, [role="button"], .tab-btn, .card-header')
  );
  // Prefer enabled matches so automation does not "click" disabled Start/Cancel shells.
  const ranked = candidates
    .map((el) => {
      const visibleLabel = el.closest(PRIVATE_UI_SELECTORS)
        ? ''
        : (el.innerText || el.textContent || '').replace(/\s+/g, ' ').trim();
      const accessibleLabel = el.getAttribute('aria-label')?.replace(/\s+/g, ' ').trim() || '';
      const label = [visibleLabel, accessibleLabel]
        .filter(Boolean)
        .join(' · ')
        .toLowerCase();
      const exact = normalizeUiLabel(visibleLabel) === needle || normalizeUiLabel(accessibleLabel) === needle;
      if (!normalizeUiLabel(label).includes(needle)) return null;
      const disabled =
        (el as HTMLButtonElement).disabled === true ||
        el.getAttribute('aria-disabled') === 'true' ||
        el.hasAttribute('disabled');
      return { el, label, disabled, exact };
    })
    .filter((x): x is { el: HTMLElement; label: string; disabled: boolean; exact: boolean } => !!x)
    .sort((a, b) => Number(a.disabled) - Number(b.disabled) || Number(b.exact) - Number(a.exact));

  const hit = ranked[0];
  if (!hit) return { ok: false, detail: `no clickable element containing "${text}"` };
  if (hit.disabled) {
    return { ok: false, detail: `matched disabled control: ${hit.label.slice(0, 80)}` };
  }
  hit.el.click();
  return { ok: true, detail: `clicked: ${hit.label.slice(0, 80)}` };
}

export function installAgentUiBridge(controllers: AgentUiControllers): () => void {
  const api = {
    snapshot(): AgentUiSnapshot {
      const report = controllers.getReport();
      const sample = controllers.getSelectedSample();
      const labHeader = Array.from(document.querySelectorAll('.card-header h3')).find((h) =>
        (h.textContent || '').includes('Lab & screening')
      );
      const labCard = labHeader?.closest('.summary-card');
      const labCollapsed = labCard ? labCard.classList.contains('collapsed') : null;

      return {
        title: document.title,
        activeTab: controllers.getActiveTab(),
        sample,
        hasReport: !!report,
        reportScore: report?.overall_signal_score ?? null,
        reportTitle: report?.title ?? null,
        sectionCount: report?.sections?.length ?? 0,
        labFollowupsCollapsed: labCollapsed,
        matchedAlleleLabelPresent: !!document.body.innerText.match(/Matched alleles/i),
        dietaryAlignmentPresent: !!document.body.innerText.match(/Dietary Alignment/i),
        dualExportButtonsPresent:
          !!document.body.innerText.match(/Export curated report JSON/i) &&
          !!document.body.innerText.match(/Export full catalog associations/i),
        urgentBadgeCount: Array.from(document.querySelectorAll('*')).filter((el) => {
          const direct = Array.from(el.childNodes)
            .filter((n) => n.nodeType === Node.TEXT_NODE)
            .map((n) => (n.textContent || '').trim())
            .join(' ');
          return /^\s*URGENT\s*$/i.test(direct);
        }).length,
        visibleTextSample: collectVisibleText(30),
        layout: collectLayoutMetrics(),
        href: location.href,
        capturedAt: new Date().toISOString(),
      };
    },
    setTab(tab: string) {
      controllers.setActiveTab(tab);
      return { ok: true as const, activeTab: controllers.getActiveTab() };
    },
    expandLabs() {
      if (controllers.expandLabFollowups) {
        controllers.expandLabFollowups();
        return { ok: true, detail: 'expandLabFollowups called' };
      }
      const labHeader = Array.from(document.querySelectorAll('.card-header h3')).find((h) =>
        (h.textContent || '').includes('Lab & screening')
      );
      const labCard = labHeader?.closest('.summary-card');
      if (!labCard) {
        return { ok: false, detail: 'Lab follow-ups card not found' };
      }
      if (labCard.classList.contains('collapsed')) {
        return clickByVisibleText('Lab & screening');
      }
      return { ok: true, detail: 'labs already expanded' };
    },
    clickText(text: string) {
      return clickByVisibleText(text);
    },
    queryText(text: string) {
      const needle = text.toLowerCase();
      const hits: string[] = [];
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
      let node = walker.nextNode();
      while (node) {
        if (!isPublicUiTextNode(node)) {
          node = walker.nextNode();
          continue;
        }
        const value = (node.textContent || '').replace(/\s+/g, ' ').trim();
        if (value.length >= 2 && value.toLowerCase().includes(needle)) {
          hits.push(value.slice(0, 160));
          if (hits.length >= 25) break;
        }
        node = walker.nextNode();
      }
      return { ok: true, count: hits.length, samples: hits.slice(0, 10) };
    },
  };

  window.__GENOMICS_CADDY_UI__ = api;

  let unlisten: UnlistenFn | null = null;
  void listen<{ id: string; method: string; args?: Record<string, unknown> }>(
    'agent-ui-request',
    async (event) => {
      const { id, method, args } = event.payload || ({} as { id: string; method: string; args?: unknown });
      try {
        let result: unknown;
        switch (method) {
          case 'snapshot':
            result = api.snapshot();
            break;
          case 'setTab': {
            const tab =
              typeof args === 'string'
                ? args
                : String((args as { tab?: string; arg?: string } | null)?.tab
                    ?? (args as { arg?: string } | null)?.arg
                    ?? '');
            result = api.setTab(tab);
            break;
          }
          case 'expandLabs':
            result = api.expandLabs();
            break;
          case 'clickText': {
            const text =
              typeof args === 'string'
                ? args
                : String((args as { text?: string; arg?: string } | null)?.text
                    ?? (args as { arg?: string } | null)?.arg
                    ?? '');
            result = api.clickText(text);
            break;
          }
          case 'queryText': {
            const text =
              typeof args === 'string'
                ? args
                : String((args as { text?: string; arg?: string } | null)?.text
                    ?? (args as { arg?: string } | null)?.arg
                    ?? '');
            result = api.queryText(text);
            break;
          }
          default:
            await emit('agent-ui-response', {
              id,
              ok: false,
              error: `Unknown UI method: ${method}`,
            });
            return;
        }
        await emit('agent-ui-response', { id, ok: true, result });
      } catch (e) {
        await emit('agent-ui-response', {
          id,
          ok: false,
          error: String(e),
        });
      }
    }
  ).then((fn) => {
    unlisten = fn;
  });

  return () => {
    unlisten?.();
    if (window.__GENOMICS_CADDY_UI__ === api) {
      delete window.__GENOMICS_CADDY_UI__;
    }
  };
}
