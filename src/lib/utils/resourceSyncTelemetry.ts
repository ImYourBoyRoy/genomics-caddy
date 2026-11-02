// ./src/lib/utils/resourceSyncTelemetry.ts
/*
Purpose: Pure reducer that folds the backend's reference-resource events
(`offline:sync_phase`, `offline:download_progress`, `offline:import_progress`)
into one structured view-model for the loading overlay. This replaces parsing
human-readable status strings: the overlay reads real byte counters, row counts,
and tier/asset structure instead.
Honesty rules (see AGENTS.md): speed and ETA exclude resumed bytes (delegated to
updateDownloadProgress); unknown download sizes and row-index imports never get
a fabricated percentage or ETA - only values the backend actually reported.
Usage: `state = reduceResourceSync(state, event)` for each backend event.
*/

import {
  updateDownloadProgress,
  type DownloadProgressSample,
} from "./offlineDownloadProgress";

export type ResourceOpKind = "download" | "index";

export interface ResourceOp {
  /** `${assetId}:${kind}`; stable so the pipeline clock can time it. */
  id: string;
  assetId: string;
  label: string;
  kind: ResourceOpKind;
  state: "active" | "done";
  download: DownloadProgressSample | null;
  rowsProcessed: number | null;
  rowsPerSecond: number | null;
  /** Backend-reported 0-100 percent; null when the backend does not know. */
  percent: number | null;
  etaSeconds: number | null;
  message: string;
}

export interface ResourceSyncTelemetry {
  tier: number | null;
  tierAssetCount: number | null;
  /** Assets inspected so far in the current tier. */
  tierAssetsSeen: number;
  /** Assets found already current (no download or index needed). */
  skipped: number;
  /** Assets downloaded (or built) and waiting for their turn to index. */
  queued: string[];
  /** Asset currently being compared with the remote copy, before any transfer. */
  checking: { assetId: string; label: string } | null;
  ops: ResourceOp[];
  /** Latest human-readable status line from the backend. */
  message: string;
}

export type ResourceSyncEvent =
  | {
      type: "phase";
      assetId: string;
      label?: string | null;
      phase: string;
      state?: string | null;
      message: string;
      tier?: number | null;
      assetCount?: number | null;
    }
  | {
      type: "download";
      assetId: string;
      label: string;
      bytesDone: number;
      totalBytes: number;
      resumedBytes?: number;
      at: number;
    }
  | {
      type: "import";
      assetId: string;
      rowsProcessed: number;
      rowsPerSecond?: number | null;
      percent?: number | null;
      etaSeconds?: number | null;
      message: string;
    };

export function emptyResourceSync(): ResourceSyncTelemetry {
  return {
    tier: null,
    tierAssetCount: null,
    tierAssetsSeen: 0,
    skipped: 0,
    queued: [],
    checking: null,
    ops: [],
    message: "",
  };
}

/** `gwas_catalog` -> `Gwas catalog` when the backend sent no label. */
export function humanizeAssetId(assetId: string): string {
  const words = assetId.replace(/[_-]+/g, " ").trim();
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : "Reference resource";
}

function opId(assetId: string, kind: ResourceOpKind): string {
  return `${assetId}:${kind}`;
}

function newOp(assetId: string, label: string, kind: ResourceOpKind): ResourceOp {
  return {
    id: opId(assetId, kind),
    assetId,
    label,
    kind,
    state: "active",
    download: null,
    rowsProcessed: null,
    rowsPerSecond: null,
    percent: null,
    etaSeconds: null,
    message: "",
  };
}

function completeActive(ops: ResourceOp[], except?: string): ResourceOp[] {
  return ops.map((op) => (op.state === "active" && op.id !== except ? { ...op, state: "done" } : op));
}

function upsertOp(
  ops: ResourceOp[],
  assetId: string,
  label: string,
  kind: ResourceOpKind,
  patch: (op: ResourceOp) => ResourceOp,
): ResourceOp[] {
  const id = opId(assetId, kind);
  const settled = completeActive(ops, id);
  const index = settled.findIndex((op) => op.id === id);
  if (index < 0) return [...settled, patch(newOp(assetId, label, kind))];
  const next = settled.slice();
  next[index] = patch(settled[index]);
  return next;
}

function labelFor(state: ResourceSyncTelemetry, assetId: string, given?: string | null): string {
  if (given) return given;
  const known = state.ops.find((op) => op.assetId === assetId);
  return known?.label ?? humanizeAssetId(assetId);
}

export function reduceResourceSync(
  state: ResourceSyncTelemetry,
  event: ResourceSyncEvent,
): ResourceSyncTelemetry {
  switch (event.type) {
    case "phase": {
      const { assetId, phase } = event;
      const label = labelFor(state, assetId, event.label);
      const message = event.message || state.message;
      if (phase === "bulk" || assetId === "__bulk__") {
        if (typeof event.tier === "number") {
          return {
            ...state,
            tier: event.tier,
            tierAssetCount: event.assetCount ?? null,
            tierAssetsSeen: 0,
            checking: null,
            ops: completeActive(state.ops),
            message,
          };
        }
        return { ...state, message };
      }
      if (phase === "check") {
        return {
          ...state,
          tierAssetsSeen: state.tierAssetsSeen + 1,
          checking: { assetId, label },
          ops: completeActive(state.ops),
          message,
        };
      }
      if (phase === "skip") {
        return {
          ...state,
          skipped: state.skipped + 1,
          checking: state.checking?.assetId === assetId ? null : state.checking,
          message,
        };
      }
      if (phase === "download") {
        return {
          ...state,
          checking: null,
          ops: upsertOp(state.ops, assetId, label, "download", (op) => ({ ...op, message })),
          message,
        };
      }
      if (phase === "import") {
        if (event.state === "running") {
          return {
            ...state,
            checking: null,
            queued: state.queued.filter((id) => id !== assetId),
            ops: upsertOp(state.ops, assetId, label, "index", (op) => ({ ...op, message })),
            message,
          };
        }
        return {
          ...state,
          checking: state.checking?.assetId === assetId ? null : state.checking,
          queued: state.queued.includes(assetId) ? state.queued : [...state.queued, assetId],
          message,
        };
      }
      return { ...state, message };
    }

    case "download": {
      const label = labelFor(state, event.assetId, event.label);
      return {
        ...state,
        checking: null,
        ops: upsertOp(state.ops, event.assetId, label, "download", (op) => ({
          ...op,
          download: updateDownloadProgress(
            op.download ?? undefined,
            {
              bytesDone: event.bytesDone,
              totalBytes: event.totalBytes,
              resumedBytes: event.resumedBytes,
            },
            event.at,
          ),
        })),
      };
    }

    case "import": {
      const label = labelFor(state, event.assetId);
      // The backend reports 0% before it has measured anything (CPU-bound
      // re-indexes, unknown sizes); that is "unknown", not progress.
      const reported =
        typeof event.percent === "number" && Number.isFinite(event.percent)
          ? Math.max(0, Math.min(100, event.percent))
          : null;
      const percent = reported === 0 && event.rowsProcessed === 0 ? null : reported;
      return {
        ...state,
        checking: null,
        queued: state.queued.filter((id) => id !== event.assetId),
        ops: upsertOp(state.ops, event.assetId, label, "index", (op) => ({
          ...op,
          rowsProcessed: event.rowsProcessed,
          rowsPerSecond: event.rowsPerSecond && event.rowsPerSecond > 0 ? event.rowsPerSecond : null,
          percent,
          etaSeconds:
            typeof event.etaSeconds === "number" && event.etaSeconds >= 0 ? event.etaSeconds : null,
          message: event.message || op.message,
        })),
        message: event.message || state.message,
      };
    }
  }
}
