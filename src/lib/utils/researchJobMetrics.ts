// ./src/lib/utils/researchJobMetrics.ts
/**
 * Pure helpers for research sweep job progress UI (speed, ETA, pipeline phases).
 */

export const MAX_PLAUSIBLE_SPEED = 15;

export type PipelineStep = { key: string; label: string; match: string };

export const PIPELINE_STEPS: PipelineStep[] = [
  { key: "gnomad", label: "gnomAD", match: "gnomad" },
  { key: "api", label: "API prefetch", match: "api prefetch" },
  { key: "prepare", label: "Prepare", match: "prepare" },
  { key: "embed", label: "Embed", match: "embedding" },
  { key: "qdrant", label: "Upsert", match: "qdrant" },
];

export type PhaseRow = {
  key: string;
  label: string;
  avgMs: number;
  totalMs: number;
  count: number;
  cacheHitPct: number | null;
};

export type PhaseMetricSnap = {
  avg_ms: number;
  total_ms: number;
  count: number;
  cache_hit_rate?: number | null;
};

export type SweepPhaseMetrics = {
  embed: PhaseMetricSnap;
  gnomad: PhaseMetricSnap;
  pubmed: PhaseMetricSnap;
  gtex: PhaseMetricSnap;
  vep: PhaseMetricSnap;
  clinvar: PhaseMetricSnap;
  gwas: PhaseMetricSnap;
  qdrant: PhaseMetricSnap;
};

export function isResumeOrCheckPhase(phase: string | null | undefined): boolean {
  if (!phase) return false;
  const p = phase.toLowerCase();
  return p.includes("resume") || p.includes("checking qdrant");
}

export function isPrefetchPhase(phase: string | null | undefined): boolean {
  if (!phase) return false;
  return phase.toLowerCase().includes("prefetch");
}

export function formatActivityPhase(phase: string | null | undefined): string {
  if (!phase) return "Preparing batch";
  return phase;
}

export function pipelineStepIndex(phase: string | null | undefined): number {
  if (!phase) return -1;
  const p = phase.toLowerCase();
  const idx = PIPELINE_STEPS.findIndex((s) => p.includes(s.match));
  return idx >= 0 ? idx : -1;
}

export function plausibleSpeed(s: number | null | undefined): number | null {
  if (s == null || Number.isNaN(s) || s <= 0 || s > MAX_PLAUSIBLE_SPEED) return null;
  return s;
}

export function formatDuration(totalSeconds: number): string {
  const sec = Math.max(0, Math.round(totalSeconds));
  if (sec < 60) return `${sec}s`;
  const minutes = Math.floor(sec / 60);
  const remSec = sec % 60;
  if (minutes < 60) return `${minutes}m ${remSec}s`;
  const hours = Math.floor(minutes / 60);
  const remMin = minutes % 60;
  return `${hours}h ${remMin}m`;
}

export function liveRsidFromActivity(message: string | null | undefined): string | null {
  if (!message) return null;
  const match = message.match(/\brs\d+\b/i);
  return match ? match[0] : null;
}

export function cacheHitPct(rate: number | null | undefined): number | null {
  if (rate == null || Number.isNaN(rate)) return null;
  return Math.round(rate * 100);
}

export function phaseRow(key: string, label: string, snap: PhaseMetricSnap): PhaseRow {
  return {
    key,
    label,
    avgMs: snap.avg_ms,
    totalMs: snap.total_ms,
    count: snap.count,
    cacheHitPct: cacheHitPct(snap.cache_hit_rate),
  };
}

export function buildPhaseRows(metrics: SweepPhaseMetrics | null | undefined): PhaseRow[] {
  if (!metrics) return [];
  const rows: PhaseRow[] = [
    phaseRow("embed", "Embed", metrics.embed),
    phaseRow("gnomad", "gnomAD", metrics.gnomad),
    phaseRow("pubmed", "PubMed", metrics.pubmed),
    phaseRow("gtex", "GTEx", metrics.gtex),
    phaseRow("vep", "VEP/dbSNP", metrics.vep),
    phaseRow("clinvar", "ClinVar", metrics.clinvar),
    phaseRow("gwas", "GWAS", metrics.gwas),
    phaseRow("qdrant", "Qdrant", metrics.qdrant),
  ];
  return rows.filter((r) => r.count > 0).sort((a, b) => b.totalMs - a.totalMs);
}

export function batchCounterDone(
  activityPhase: string | null | undefined,
  batchPrepared: number | null | undefined,
  batchPrefetchDone: number | null | undefined,
  batchTotal: number | null | undefined
): number {
  if (isResumeOrCheckPhase(activityPhase)) return 0;
  if (isPrefetchPhase(activityPhase)) return batchPrefetchDone ?? 0;
  const p = activityPhase?.toLowerCase() ?? "";
  if (p.includes("embedding") || p.includes("qdrant")) return batchTotal ?? 0;
  return batchPrepared ?? 0;
}

export function batchCounterLabel(activityPhase: string | null | undefined): string {
  if (isResumeOrCheckPhase(activityPhase)) return "waiting";
  if (isPrefetchPhase(activityPhase)) return "prefetched";
  const p = activityPhase?.toLowerCase() ?? "";
  if (p.includes("embedding")) return "embedded";
  if (p.includes("qdrant")) return "upserting";
  return "prepared";
}

export function interimProgressPct(
  job: { enriched_count: number; total_markers: number; status: string } | null,
  progressPct: number,
  batchTotal: number | null | undefined,
  batchDone: number,
): number {
  if (!job || job.total_markers <= 0) return progressPct;
  const batchT = batchTotal ?? 0;
  if (job.status === "running" && batchT > 0 && batchDone > 0) {
    const batchFraction = batchDone / batchT;
    const base = job.enriched_count / job.total_markers;
    const batchSlice = Math.min(1 / job.total_markers, 0.02);
    return Math.min(100, Math.round((base + batchFraction * batchSlice) * 100));
  }
  return progressPct;
}

export function batchStallHint(
  showBatchProgress: boolean,
  activityPhase: string | null | undefined,
  batchElapsedSecs: number | null | undefined,
  batchDone: number,
): string | null {
  if (!showBatchProgress) return null;
  const elapsed = batchElapsedSecs ?? 0;
  const prefetching = isPrefetchPhase(activityPhase);
  if (elapsed >= 300 && batchDone === 0) {
    if (prefetching) {
      return "Prefetch has made no progress for 5+ minutes — remote gnomAD/API calls may be slow or stuck. Check the terminal for [prefetch] / [gnomad] messages.";
    }
    return "Batch prepare has made no progress for 5+ minutes — remote API calls may be slow or stuck. Check the terminal for [enrich] / [prefetch] messages.";
  }
  if (elapsed >= 120 && batchDone === 0) {
    if (prefetching) {
      return "Still prefetching first variants — gnomAD tabix + NCBI/GTEx can take several minutes per batch on remote hosts.";
    }
    return "Still preparing first variants — enrichment APIs can take several minutes per batch.";
  }
  return null;
}

export function sweepStallHint(
  job: { status: string; enriched_count: number; total_markers: number } | null,
  batchElapsedSecs: number | null | undefined,
  batchDone: number,
  lastProgressAt: number | null | undefined,
): string | null {
  if (!job || job.status !== "running") return null;
  const elapsed = batchElapsedSecs ?? 0;
  const sinceProgress =
    lastProgressAt != null ? (Date.now() - lastProgressAt) / 1000 : elapsed;
  if (sinceProgress >= 600 && batchDone === 0 && job.enriched_count < job.total_markers) {
    return "No indexed progress for 10+ minutes — backend may be stuck on prefetch or prepare. Check the terminal for [sweep] / [enrich] logs or try Pause → Fast index preset → Resume.";
  }
  if (sinceProgress >= 180 && batchDone === 0 && elapsed === 0) {
    return "Waiting for backend activity — scanning chunks or connecting to Qdrant.";
  }
  return null;
}

export function batchPrepareSpeed(
  activityPhase: string | null | undefined,
  batchElapsedSecs: number | null | undefined,
  batchPrepared: number | null | undefined,
  batchPrefetchDone: number | null | undefined,
): number | null {
  if (batchElapsedSecs == null || batchElapsedSecs < 5) return null;
  if (isPrefetchPhase(activityPhase)) {
    const prefetch = batchPrefetchDone ?? 0;
    if (prefetch > 0) return plausibleSpeed(prefetch / batchElapsedSecs);
  } else if (batchPrepared != null && batchPrepared > 0) {
    return plausibleSpeed(batchPrepared / batchElapsedSecs);
  }
  return null;
}

export function rollingSpeedFromSamples(
  samples: { t: number; count: number }[],
): number | null {
  if (samples.length < 2) return null;
  const oldest = samples[0];
  const newest = samples[samples.length - 1];
  const dtSec = (newest.t - oldest.t) / 1000;
  const delta = newest.count - oldest.count;
  if (dtSec < 10 || delta <= 0) return null;
  return plausibleSpeed(delta / dtSec);
}

export function effectiveSpeed(
  rolling: number | null,
  recentProgressSpeed: number | null | undefined,
  batchSpeed: number | null,
  progressSpeed: number | null | undefined,
): number | null {
  return (
    rolling ??
    plausibleSpeed(recentProgressSpeed) ??
    batchSpeed ??
    plausibleSpeed(progressSpeed)
  );
}

export function speedLabel(
  rolling: number | null,
  recentProgressSpeed: number | null | undefined,
  batchSpeed: number | null,
  activityPhase: string | null | undefined,
  progressSpeed: number | null | undefined,
): string {
  if (rolling != null) return "recent";
  if (plausibleSpeed(recentProgressSpeed) != null) return "batch";
  if (batchSpeed != null) {
    return isPrefetchPhase(activityPhase) ? "prefetch" : "preparing";
  }
  if (plausibleSpeed(progressSpeed) != null) return "session";
  return "estimating";
}

export function prefetchElapsedLabel(
  jobStatus: string | undefined,
  activityPhase: string | null | undefined,
  batchElapsedSecs: number | null | undefined,
  batchDone: number,
  batchTotal: number | null | undefined,
): string | null {
  if (jobStatus !== "running") return null;
  const elapsed = batchElapsedSecs ?? 0;
  if (elapsed >= 30 && batchDone === 0 && batchTotal != null && batchTotal > 0 && isPrefetchPhase(activityPhase)) {
    return formatDuration(elapsed);
  }
  return null;
}

export function scopeQueueMismatch(
  scopePreview: { total_unique: number } | null | undefined,
  job: { status: string; total_markers: number } | null,
): { configured: number; running: number } | null {
  if (!scopePreview || !job || job.status === "idle") return null;
  if (scopePreview.total_unique === job.total_markers) return null;
  return { configured: scopePreview.total_unique, running: job.total_markers };
}

export function jobStatusText(job: { status: string; loop_active?: boolean; total_markers: number; enriched_count: number } | null): string {
  if (!job) return "Not Started";
  if (job.status === "paused" && job.loop_active) return "Pausing…";
  if (job.status === "complete" || (job.total_markers > 0 && job.enriched_count >= job.total_markers)) {
    return "Complete";
  }
  return job.status.charAt(0).toUpperCase() + job.status.slice(1);
}

export function jobStatusClass(job: { status: string; loop_active?: boolean; total_markers: number; enriched_count: number } | null): string {
  if (!job) return "idle";
  if (job.status === "complete" || (job.total_markers > 0 && job.enriched_count >= job.total_markers)) {
    return "complete";
  }
  if (job.status === "paused" && job.loop_active) return "running";
  return job.status;
}
