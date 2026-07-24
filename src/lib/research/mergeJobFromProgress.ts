// ./src/lib/research/mergeJobFromProgress.ts
/**
 * Merge a research:progress event into the UI job snapshot.
 * Pure helper (no Svelte runes) so cancel races can be unit-tested.
 */

import type { ResearchJob, ResearchProgress } from "../types/research";

function isSettledStatus(status: string): boolean {
  return status === "idle" || status === "complete" || status === "error";
}

export function mergeJobFromProgress(
  job: ResearchJob | null,
  payload: ResearchProgress,
  sampleId: number,
): ResearchJob {
  // Stale in-flight running/paused pulses must not resurrect a cancelled/completed job.
  // Cancel used to lose to a late "paused" emit and flip the UI to Resume.
  if (
    job &&
    job.job_id === payload.job_id &&
    isSettledStatus(job.status) &&
    !isSettledStatus(payload.status)
  ) {
    return job;
  }

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

  const terminal =
    payload.status === "idle" ||
    payload.status === "paused" ||
    payload.status === "complete" ||
    payload.status === "error";

  return {
    ...base,
    status: payload.status as ResearchJob["status"],
    enriched_count: payload.enriched_count,
    total_markers: payload.total_markers,
    current_rsid: payload.current_rsid ?? base.current_rsid,
    current_source: payload.current_source ?? base.current_source,
    last_updated: Math.floor(Date.now() / 1000),
    error_message:
      payload.status === "idle" && payload.message?.toLowerCase().includes("cancel")
        ? payload.message
        : base.error_message,
    loop_active: payload.status === "running" ? true : terminal ? false : base.loop_active,
  };
}
