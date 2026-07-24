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

export type AgentUiTab =
  | 'report'
  | 'map'
  | 'discovery'
  | 'browser'
  | 'mcp'
  | 'agent'
  | 'research'
  | 'ai';

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
    const text = (node.textContent || '').replace(/\s+/g, ' ').trim();
    if (text.length >= 3) out.push(text.slice(0, 120));
    node = walker.nextNode();
  }
  return out;
}

function clickByVisibleText(text: string): { ok: boolean; detail: string } {
  const needle = text.trim().toLowerCase();
  if (!needle) return { ok: false, detail: 'empty text' };

  const candidates = Array.from(
    document.querySelectorAll<HTMLElement>('button, a, [role="button"], .tab-btn, .card-header')
  );
  // Prefer enabled matches so automation does not "click" disabled Start/Cancel shells.
  const ranked = candidates
    .map((el) => {
      const label = (el.innerText || el.textContent || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!label.includes(needle)) return null;
      const disabled =
        (el as HTMLButtonElement).disabled === true ||
        el.getAttribute('aria-disabled') === 'true' ||
        el.hasAttribute('disabled');
      return { el, label, disabled };
    })
    .filter((x): x is { el: HTMLElement; label: string; disabled: boolean } => !!x)
    .sort((a, b) => Number(a.disabled) - Number(b.disabled));

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
