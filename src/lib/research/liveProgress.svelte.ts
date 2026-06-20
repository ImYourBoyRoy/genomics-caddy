// ./src/lib/research/liveProgress.svelte.ts
/*
  Shared live sweep progress from Tauri `research:progress` events.
  Single source of truth so +page tab badge and ResearchPanel stay in sync.
*/

import type { ResearchProgress, ResearchJob, SweepPhaseMetricsSnapshot } from "../types/research";

/** Updated by +page / ResearchPanel — avoids stale closures in Tauri listeners. */
export const researchEventContext = $state({
  sampleId: null as number | null,
  debugEnabled: false,
});

/** Refs reassigned from +page $effect — avoids stale closures in Tauri listeners. */
export const jobRefs = {
  getJob: (): ResearchJob | null => null,
  setJob: (_job: ResearchJob) => {},
};

/** Normalize Tauri payload (snake_case or camelCase). */
export function normalizeResearchProgress(
  raw: ResearchProgress | Record<string, unknown>
): ResearchProgress {
  const r = raw as Record<string, unknown>;
  return {
    job_id: String(r.job_id ?? r.jobId ?? ""),
    status: String(r.status ?? "running"),
    enriched_count: Number(r.enriched_count ?? r.enrichedCount ?? 0),
    total_markers: Number(r.total_markers ?? r.totalMarkers ?? 0),
    current_rsid: (r.current_rsid ?? r.currentRsid) as string | undefined,
    current_source: (r.current_source ?? r.currentSource) as string | undefined,
    message: String(r.message ?? ""),
    variants_per_sec: (r.variants_per_sec ?? r.variantsPerSec) as number | null | undefined,
    recent_variants_per_sec: (r.recent_variants_per_sec ?? r.recentVariantsPerSec) as
      | number
      | null
      | undefined,
    phase_metrics: (r.phase_metrics ?? r.phaseMetrics) as SweepPhaseMetricsSnapshot | null | undefined,
    activity_phase: (r.activity_phase ?? r.activityPhase) as string | null | undefined,
    batch_prepared: (r.batch_prepared ?? r.batchPrepared) as number | null | undefined,
    batch_prefetch_done: (r.batch_prefetch_done ?? r.batchPrefetchDone) as number | null | undefined,
    batch_total: (r.batch_total ?? r.batchTotal) as number | null | undefined,
    batch_elapsed_secs: (r.batch_elapsed_secs ?? r.batchElapsedSecs) as number | null | undefined,
  };
}

export const liveProgress = $state({
  activityPhase: null as string | null,
  batchPrepared: null as number | null,
  batchPrefetchDone: null as number | null,
  batchTotal: null as number | null,
  batchElapsedSecs: null as number | null,
  lastActivityMessage: null as string | null,
  lastProgressAt: null as number | null,
  phaseMetrics: null as SweepPhaseMetricsSnapshot | null,
  progressSpeed: null as number | null,
  recentProgressSpeed: null as number | null,
  /** Bumped on every progress apply so $derived subscribers always refresh. */
  tick: 0,
});

export const liveDebugLines = $state<string[]>([]);

export function pushLiveDebugLine(line: string) {
  const stamp = new Date().toLocaleTimeString();
  liveDebugLines.unshift(`[${stamp}] ${line}`);
  if (liveDebugLines.length > 80) {
    liveDebugLines.length = 80;
  }
}

export function resetLiveProgress() {
  liveProgress.activityPhase = null;
  liveProgress.batchPrepared = null;
  liveProgress.batchPrefetchDone = null;
  liveProgress.batchTotal = null;
  liveProgress.batchElapsedSecs = null;
  liveProgress.lastActivityMessage = null;
  liveProgress.lastProgressAt = null;
  liveProgress.phaseMetrics = null;
  liveProgress.progressSpeed = null;
  liveProgress.recentProgressSpeed = null;
  liveProgress.tick = 0;
  liveDebugLines.length = 0;
}

/** Clear speed + batch UI on resume without wiping the last milestone message. */
export function resetLiveProgressForResume() {
  liveProgress.activityPhase = null;
  liveProgress.batchPrepared = null;
  liveProgress.batchPrefetchDone = null;
  liveProgress.batchTotal = null;
  liveProgress.batchElapsedSecs = null;
  liveProgress.progressSpeed = null;
  liveProgress.recentProgressSpeed = null;
}

export function mergeJobFromProgress(
  job: ResearchJob | null,
  payload: ResearchProgress,
  sampleId: number
): ResearchJob {
  const base: ResearchJob = job ?? {
    job_id: payload.job_id,
    sample_id: sampleId,
    status: "idle",
    total_markers: payload.total_markers,
    enriched_count: payload.enriched_count,
    priority_complete: false,
    started_at: Math.floor(Date.now() / 1000),
    last_updated: Math.floor(Date.now() / 1000),
  };

  return {
    ...base,
    status: payload.status as ResearchJob["status"],
    enriched_count: payload.enriched_count,
    total_markers: payload.total_markers,
    current_rsid: payload.current_rsid ?? base.current_rsid,
    current_source: payload.current_source ?? base.current_source,
    last_updated: Math.floor(Date.now() / 1000),
    loop_active:
      payload.status === "running"
        ? true
        : payload.status === "paused" || payload.status === "complete" || payload.status === "error"
          ? false
          : base.loop_active,
  };
}

export function applyLiveProgressFields(payload: ResearchProgress) {
  liveProgress.lastProgressAt = Date.now();
  liveProgress.tick += 1;
  if (payload.message) {
    liveProgress.lastActivityMessage = payload.message;
  }
  if (payload.activity_phase != null) {
    liveProgress.activityPhase = payload.activity_phase;
  }
  if (payload.batch_prepared != null) {
    liveProgress.batchPrepared = payload.batch_prepared;
  }
  if (payload.batch_prefetch_done != null) {
    liveProgress.batchPrefetchDone = payload.batch_prefetch_done;
  }
  if (payload.batch_total != null) {
    liveProgress.batchTotal = payload.batch_total;
  }
  if (payload.batch_elapsed_secs != null) {
    liveProgress.batchElapsedSecs = payload.batch_elapsed_secs;
  }
  if (payload.phase_metrics) {
    liveProgress.phaseMetrics = payload.phase_metrics;
  }
  if (payload.recent_variants_per_sec != null && payload.recent_variants_per_sec > 0) {
    liveProgress.recentProgressSpeed = payload.recent_variants_per_sec;
  }
  if (payload.variants_per_sec != null && payload.variants_per_sec > 0) {
    liveProgress.progressSpeed = payload.variants_per_sec;
  }
}

export function handleResearchProgressEvent(payload: ResearchProgress) {
  const sampleId = researchEventContext.sampleId;
  if (!sampleId || !payload.job_id) return false;

  const prefix = `job_${sampleId}_`;
  if (!payload.job_id.startsWith(prefix)) return false;

  const merged = mergeJobFromProgress(jobRefs.getJob(), payload, sampleId);
  jobRefs.setJob(merged);
  applyLiveProgressFields(payload);

  if (payload.message?.toLowerCase().includes("resumed")) {
    liveProgress.progressSpeed = null;
    liveProgress.recentProgressSpeed = null;
  }
  return true;
}

export function handleResearchDebugEvent(tag: string, message: string) {
  if (!researchEventContext.debugEnabled) return;
  pushLiveDebugLine(`[${tag}] ${message}`);
}
