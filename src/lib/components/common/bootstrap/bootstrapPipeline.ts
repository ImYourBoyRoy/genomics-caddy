// ./src/lib/components/common/bootstrap/bootstrapPipeline.ts
/*
Purpose: Pure builders that turn raw startup / import state into the
PipelineStep view-model drawn by PipelineStepList. Keeping this logic out of
the Svelte files makes the wording and progress math unit testable.
Inputs: current phase, live status text, backend report progress, and clock
lookups (finished-step durations and active-step elapsed time).
*/

import type { ReportGenerationProgress } from "../../../types/genomics";
import {
  IMPORT_STEPS,
  importStepState,
  type ImportPhase,
  type ImportStepId,
} from "../../../utils/importProgress";
import { formatTransferSize } from "../../../utils/offlineDownloadProgress";
import type { ResourceOp, ResourceSyncTelemetry } from "../../../utils/resourceSyncTelemetry";
import {
  clampFraction,
  formatCount,
  reportCountLabel,
  reportStepFraction,
} from "../../../utils/loadingTelemetry";
import type { PipelineStep, PipelineStepState } from "../loading/pipelineTypes";
import {
  BOOTSTRAP_STEPS,
  PHASE_ACCENT,
  stepState,
  type BootstrapPhase,
  type DbProgress,
} from "./bootstrapPhases";

export interface ClockLookup {
  durationMs: (id: string) => number | null;
  stepElapsedMs: (id: string) => number | null;
}

export interface StartupPipelineInput {
  phase: BootstrapPhase;
  /** Last phase that was not `error`; marks where a failure happened. */
  failedPhase?: BootstrapPhase | null;
  message: string;
  error?: string;
  dbProgress: DbProgress | null;
  reportProgress: ReportGenerationProgress | null;
  clock: ClockLookup;
}

export function buildStartupSteps(input: StartupPipelineInput): PipelineStep[] {
  const { phase, failedPhase = null, message, error = "", dbProgress, reportProgress, clock } = input;
  return BOOTSTRAP_STEPS.map((step) => {
    const state = stepState(phase, step.id, failedPhase);
    const out: PipelineStep = {
      id: step.id,
      label: step.label,
      state,
      accent: PHASE_ACCENT[step.id],
      durationMs: state === "done" ? clock.durationMs(step.id) : null,
      elapsedMs: state === "active" ? clock.stepElapsedMs(step.id) : null,
    };
    if (state === "error") {
      out.detail = error || message || "This step stopped unexpectedly.";
      return out;
    }
    if (state !== "active") return out;

    if (step.id === "db" && dbProgress) {
      out.detail = dbProgress.detail ? `${dbProgress.label} · ${dbProgress.detail}` : dbProgress.label;
      out.fraction = (dbProgress.index + 0.5) / dbProgress.total;
      out.meta = `${dbProgress.index + 1} / ${dbProgress.total}`;
    } else if (step.id === "report") {
      const fraction = reportStepFraction(reportProgress);
      out.detail = reportCountLabel(reportProgress) ?? reportProgress?.status ?? message;
      out.fraction = fraction;
      out.meta = fraction === null ? null : `${Math.round(fraction * 100)}%`;
    } else {
      out.detail = message;
      out.fraction = null;
    }
    return out;
  });
}

const IMPORT_ACCENTS = ["#2dd4bf", "#22d3ee", "#38bdf8", "#818cf8", "#a78bfa", "#34d399", "#fbbf24"];

export interface ImportPipelineInput {
  phase: ImportPhase;
  failedStep?: ImportStepId | null;
  status: string;
  clock: ClockLookup;
}

export function buildImportSteps(input: ImportPipelineInput): PipelineStep[] {
  const { phase, failedStep = null, status, clock } = input;
  return IMPORT_STEPS.map((step, index) => {
    const state: PipelineStepState = importStepState(phase, step.id, failedStep);
    return {
      id: step.id,
      label: step.label,
      state,
      accent: IMPORT_ACCENTS[index % IMPORT_ACCENTS.length],
      detail: state === "active" || state === "error" ? status : null,
      fraction: null,
      durationMs: state === "done" ? clock.durationMs(step.id) : null,
      elapsedMs: state === "active" ? clock.stepElapsedMs(step.id) : null,
    };
  });
}

export interface ResourcePipelineInput {
  message: string;
  clock: ClockLookup;
  /** Structured backend events; null falls back to the status line only. */
  telemetry?: ResourceSyncTelemetry | null;
}

/** Ledger rows kept on screen; older finished operations scroll away. */
export const RESOURCE_VISIBLE_OPS = 5;

const RESOURCE_ACCENT = {
  scan: PHASE_ACCENT.stats,
  check: "#2dd4bf",
  download: "#38bdf8",
  index: "#a78bfa",
} as const;

function downloadDetail(op: ResourceOp): { detail: string; fraction: number | null; meta: string | null } {
  const sample = op.download;
  if (!sample) return { detail: "Starting download…", fraction: null, meta: null };
  const size =
    sample.totalBytes > 0
      ? `${formatTransferSize(sample.bytesDone)} of ${formatTransferSize(sample.totalBytes)}`
      : `${formatTransferSize(sample.bytesDone)} downloaded`;
  const speed =
    sample.speedBytesPerSecond !== null && sample.speedSamples > 0
      ? ` · ${formatTransferSize(sample.speedBytesPerSecond)}/s`
      : "";
  const resumed = sample.resumedBytes > 0 ? ` · resumed from ${formatTransferSize(sample.resumedBytes)}` : "";
  const known = sample.percent >= 0;
  return {
    detail: `${size}${speed}${resumed}`,
    fraction: known ? clampFraction(sample.percent / 100) : null,
    meta: known ? `${sample.percent}%` : null,
  };
}

function indexDetail(op: ResourceOp): { detail: string; fraction: number | null; meta: string | null } {
  const rows =
    op.rowsProcessed !== null && op.rowsProcessed > 0
      ? `${formatCount(op.rowsProcessed)} rows${
          op.rowsPerSecond !== null ? ` · ${formatCount(op.rowsPerSecond)} rows/s` : ""
        }`
      : null;
  const known = op.percent !== null;
  return {
    detail: rows ?? (op.message || "Indexing into the local database…"),
    fraction: known ? clampFraction((op.percent ?? 0) / 100) : null,
    meta: known ? `${Math.round(op.percent ?? 0)}%` : op.rowsProcessed ? formatCount(op.rowsProcessed) : null,
  };
}

/**
 * Reference-resource sync as a live ledger: one row per real operation
 * (download or index) in the order the backend runs them. Only values the
 * backend reported are shown - no invented totals, percentages, or ETAs.
 */
export function buildResourceSteps(input: ResourcePipelineInput): PipelineStep[] {
  const { message, clock, telemetry = null } = input;
  const ops = telemetry?.ops ?? [];
  const hidden = Math.max(0, ops.length - RESOURCE_VISIBLE_OPS);
  const steps: PipelineStep[] = [];

  if (hidden === 0) {
    const scanning = ops.length === 0;
    const checking = telemetry?.checking ?? null;
    steps.push({
      id: "scan",
      label: "Scan local catalogs",
      state: scanning ? "active" : "done",
      accent: RESOURCE_ACCENT.scan,
      detail: scanning
        ? checking
          ? `Comparing ${checking.label} with the remote copy`
          : message || "Checking which reference resources need work…"
        : null,
      meta: scanning && telemetry?.tierAssetCount ? `${telemetry.tierAssetsSeen} / ${telemetry.tierAssetCount}` : null,
      fraction: null,
      durationMs: scanning ? null : clock.durationMs("scan"),
      elapsedMs: scanning ? clock.stepElapsedMs("scan") : null,
    });
  }

  for (const op of ops.slice(hidden)) {
    const active = op.state === "active";
    const view = op.kind === "download" ? downloadDetail(op) : indexDetail(op);
    steps.push({
      id: op.id,
      label: `${op.kind === "download" ? "Download" : "Index"} ${op.label}`,
      state: active ? "active" : "done",
      accent: RESOURCE_ACCENT[op.kind],
      detail: active ? view.detail : null,
      meta: active ? view.meta : null,
      fraction: active ? view.fraction : null,
      durationMs: active ? null : clock.durationMs(op.id),
      elapsedMs: active ? clock.stepElapsedMs(op.id) : null,
    });
  }

  const checking = telemetry?.checking ?? null;
  if (ops.length > 0 && checking) {
    const id = `${checking.assetId}:check`;
    steps.push({
      id,
      label: `Check ${checking.label}`,
      state: "active",
      accent: RESOURCE_ACCENT.check,
      detail: `Comparing ${checking.label} with the remote copy`,
      meta: null,
      fraction: null,
      durationMs: null,
      elapsedMs: clock.stepElapsedMs(id),
    });
  }
  return steps;
}

export interface ResourceFocus {
  etaSeconds: number | null;
  /** True when the backend cannot know the total (row-index imports, unknown sizes). */
  etaUnknown: boolean;
  etaCaption: string;
  stepTitle: string;
  stepIndex: number;
  stepCount: number | null;
  stepLabel: string;
}

/** Readout values for the telemetry strip while reference resources sync. */
export function resourceFocus(telemetry: ResourceSyncTelemetry | null, steps: PipelineStep[]): ResourceFocus {
  const active = telemetry?.ops.find((op) => op.state === "active") ?? null;
  const activeStep = steps.find((step) => step.state === "active");
  let etaSeconds: number | null = null;
  let etaCaption = "left in this step";
  let etaUnknown = false;
  if (active?.kind === "download") {
    etaSeconds = active.download?.etaSeconds ?? null;
    etaCaption = "left to download";
    etaUnknown = active.download !== null && active.download.totalBytes <= 0;
  } else if (active?.kind === "index") {
    etaSeconds = active.etaSeconds;
    etaCaption = "left to index";
    etaUnknown = active.percent === null && active.etaSeconds === null && active.rowsProcessed !== null;
  }
  const tier = telemetry?.tier ?? null;
  return {
    etaSeconds,
    etaUnknown,
    etaCaption,
    stepTitle: tier === null ? "Asset" : `Tier ${tier} asset`,
    stepIndex: Math.max(1, telemetry?.tierAssetsSeen ?? 1),
    stepCount: telemetry?.tierAssetCount ?? null,
    stepLabel: activeStep?.label ?? "Preparing",
  };
}

/** Id -> state map, the shape `PipelineClock.sync` consumes. */
export function stepStates(steps: PipelineStep[]): Record<string, PipelineStepState> {
  return Object.fromEntries(steps.map((step) => [step.id, step.state]));
}

/** ETA sample for the startup report step: per-phase scan rate from the backend clock. */
export function reportEtaSample(
  progress: ReportGenerationProgress | null,
): { fraction: number; elapsedMs: number } | null {
  if (!progress || progress.total <= 0) return null;
  if (progress.phase !== "markers" && progress.phase !== "clinvar") return null;
  return {
    fraction: clampFraction(progress.current / progress.total),
    elapsedMs: progress.phaseElapsedMs,
  };
}
