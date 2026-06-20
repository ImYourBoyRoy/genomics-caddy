<!-- ./src/lib/components/research/ResearchJobProgressPanel.svelte -->
<script lang="ts">
  import type { ResearchJob, PipelineTuningPublic, ResearchScopePreview } from "../../types/research";
  import { liveProgress } from "../../research/liveProgress.svelte";
  import {
    PIPELINE_STEPS,
    batchCounterDone as computeBatchDone,
    batchCounterLabel as computeBatchLabel,
    batchPrepareSpeed,
    batchStallHint,
    buildPhaseRows,
    effectiveSpeed,
    formatActivityPhase,
    formatDuration,
    interimProgressPct,
    isPrefetchPhase,
    isResumeOrCheckPhase,
    liveRsidFromActivity,
    pipelineStepIndex,
    prefetchElapsedLabel,
    rollingSpeedFromSamples,
    scopeQueueMismatch,
    speedLabel,
    sweepStallHint,
  } from "../../utils/researchJobMetrics";

  interface Props {
    job: ResearchJob;
    pipelineTuning?: PipelineTuningPublic | null;
    scopePreview?: ResearchScopePreview | null;
  }

  let { job, pipelineTuning = null, scopePreview = null }: Props = $props();

  let activityPhase = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.activityPhase;
  });
  let batchPrepared = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.batchPrepared;
  });
  let batchPrefetchDone = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.batchPrefetchDone;
  });
  let batchTotal = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.batchTotal;
  });
  let batchElapsedSecs = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.batchElapsedSecs;
  });
  let lastActivityMessage = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.lastActivityMessage;
  });
  let lastProgressAt = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.lastProgressAt;
  });
  let phaseMetrics = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.phaseMetrics;
  });
  let progressSpeed = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.progressSpeed;
  });
  let recentProgressSpeed = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.recentProgressSpeed;
  });

  let progressPct = $derived(
    job.total_markers > 0
      ? Math.min(100, Math.round((job.enriched_count / job.total_markers) * 100))
      : 0
  );

  let activePipelineIndex = $derived(pipelineStepIndex(activityPhase));
  let batchDone = $derived(computeBatchDone(activityPhase, batchPrepared, batchPrefetchDone, batchTotal));
  let batchLabel = $derived(computeBatchLabel(activityPhase));
  let interimPct = $derived(interimProgressPct(job, progressPct, batchTotal, batchDone));

  let showBatchProgress = $derived(
    job.status === "running" &&
      (batchTotal != null && batchTotal > 0 ||
        activityPhase != null ||
        isResumeOrCheckPhase(activityPhase) ||
        (batchElapsedSecs != null && batchElapsedSecs > 0))
  );

  let batchPct = $derived(
    batchTotal != null && batchTotal > 0
      ? Math.min(100, Math.round((batchDone / batchTotal) * 100))
      : 0
  );

  let queueMismatch = $derived(scopeQueueMismatch(scopePreview, job));
  let liveRsid = $derived(liveRsidFromActivity(lastActivityMessage));
  let phaseRows = $derived(buildPhaseRows(phaseMetrics));

  type SpeedSample = { t: number; count: number };
  let speedSamples = $state<SpeedSample[]>([]);
  let speedBaseline = $state<number | null>(null);
  let tick = $state(Date.now());

  $effect(() => {
    if (job.status !== "running") {
      speedSamples = [];
      speedBaseline = null;
      return;
    }
    if (speedBaseline == null) speedBaseline = job.enriched_count;
    void tick;
    const now = Date.now();
    const throughput = Math.max(0, job.enriched_count - speedBaseline);
    speedSamples = [...speedSamples, { t: now, count: throughput }].filter((s) => s.t >= now - 120_000);
  });

  $effect(() => {
    if (job.status !== "running") return;
    const id = setInterval(() => {
      tick = Date.now();
    }, 5000);
    return () => clearInterval(id);
  });

  let rolling = $derived(rollingSpeedFromSamples(speedSamples));
  let batchSpeed = $derived(batchPrepareSpeed(activityPhase, batchElapsedSecs, batchPrepared, batchPrefetchDone));
  let speed = $derived(effectiveSpeed(rolling, recentProgressSpeed, batchSpeed, progressSpeed));
  let etaSec = $derived.by(() => {
    void tick;
    if (job.status !== "running" || speed == null || speed <= 0) return null;
    const remaining = job.total_markers - job.enriched_count;
    return remaining <= 0 ? 0 : remaining / speed;
  });
  let formattedSpeed = $derived(speed != null && !Number.isNaN(speed) ? speed.toFixed(1) : null);
  let speedKind = $derived(speedLabel(rolling, recentProgressSpeed, batchSpeed, activityPhase, progressSpeed));
  let formattedEta = $derived.by(() => {
    if (etaSec == null) return "Estimating…";
    if (etaSec <= 0) return "Done";
    return formatDuration(etaSec);
  });
  let prefetchLabel = $derived(
    prefetchElapsedLabel(job.status, activityPhase, batchElapsedSecs, batchDone, batchTotal)
  );
  let batchStall = $derived(batchStallHint(showBatchProgress, activityPhase, batchElapsedSecs, batchDone));
  let sweepStall = $derived(sweepStallHint(job, batchElapsedSecs, batchDone, lastProgressAt));
</script>

{#if pipelineTuning}
  <div class="tuning-banner">
    <span class="tuning-profile">{pipelineTuning.profile} profile</span>
    <span>batch {pipelineTuning.enrich_batch_size}</span>
    <span>parallel prepare {pipelineTuning.prepare_concurrency}</span>
    {#if pipelineTuning.fast_sweep}
      <span class="tuning-fast">fast sweep</span>
    {/if}
  </div>
{/if}

{#if queueMismatch}
  <div class="scope-mismatch-banner">
    Scope queue is <strong>{queueMismatch.configured.toLocaleString()}</strong> variants,
    but this run is locked at <strong>{queueMismatch.running.toLocaleString()}</strong>.
    Cancel and start a new sweep (or Expand queue after completion) to apply the updated cap.
  </div>
{/if}

<div class="progress-section mt-2">
  <div class="progress-labels">
    <span>Indexed in Qdrant</span>
    <span class="progress-value">{job.enriched_count} / {job.total_markers} ({interimPct}%)</span>
  </div>
  <div class="progress-bar-container">
    <div class="progress-bar-fill" style="width: {interimPct}%"></div>
  </div>

  {#if showBatchProgress}
    <div class="batch-progress mt-2">
      {#if job.status === "running" && activePipelineIndex >= 0 && !isResumeOrCheckPhase(activityPhase)}
        <div class="pipeline-steps" aria-label="Batch pipeline">
          {#each PIPELINE_STEPS as step, i (step.key)}
            <div
              class="pipeline-step"
              class:done={activePipelineIndex > i}
              class:active={activePipelineIndex === i}
              class:pending={activePipelineIndex < i}
            >
              <span class="pipeline-dot"></span>
              <span class="pipeline-label">{step.label}</span>
            </div>
          {/each}
        </div>
      {/if}
      <div class="progress-labels">
        <span>Current batch — {formatActivityPhase(activityPhase)}</span>
        <span class="progress-value">
          {batchDone} / {batchTotal} {batchLabel}
          {#if batchElapsedSecs != null}· {formatDuration(batchElapsedSecs)}{/if}
          ({batchPct}%)
        </span>
      </div>
      <div class="progress-bar-container batch-bar">
        <div class="progress-bar-fill batch-fill" style="width: {batchPct}%"></div>
      </div>
      <p class="batch-hint">
        Indexed count updates only after embed + Qdrant upsert finish for this batch. Prefetch
        and prepare progress are shown above while the batch is in flight.
      </p>
      {#if batchStall}
        <p class="batch-stall-hint">{batchStall}</p>
      {/if}
    </div>
  {/if}

  {#if job.status === "running"}
    <div class="progress-meta mt-1">
      {#if formattedSpeed !== null}
        <span class="meta-item">
          Speed: <strong>{formattedSpeed} var/s</strong>
          <span class="speed-kind">({speedKind})</span>
        </span>
      {:else if prefetchLabel || isResumeOrCheckPhase(activityPhase)}
        <span class="meta-item">
          {isResumeOrCheckPhase(activityPhase) ? "Setup" : "Prefetch"}:
          <strong>{prefetchLabel ?? (batchElapsedSecs != null ? formatDuration(batchElapsedSecs) : "…")}</strong>
          <span class="speed-kind">({isResumeOrCheckPhase(activityPhase) ? activityPhase : "gnomAD/API"})</span>
        </span>
      {:else}
        <span class="meta-item">Speed: <strong>Estimating…</strong></span>
      {/if}
      <span class="meta-item">ETA: <strong>{formattedEta}</strong></span>
    </div>

    {#if sweepStall}
      <p class="batch-stall-hint">{sweepStall}</p>
    {/if}

    <div class="active-task mt-2">
      <span class="pulse-ring"></span>
      <div class="task-info">
        {#if activityPhase && batchTotal}
          <span class="task-title">{formatActivityPhase(activityPhase)}</span>
          <strong class="task-rsid">{job.current_rsid || "..."}</strong>
          <span class="task-batch">
            {batchDone}/{batchTotal} {batchLabel}
            {#if batchElapsedSecs != null}· {formatDuration(batchElapsedSecs)}{/if}
          </span>
        {:else}
          <span class="task-title">Enriching Variant:</span>
          <strong class="task-rsid">{job.current_rsid || liveRsid || "…"}</strong>
        {/if}
        {#if job.current_source}
          <span class="task-source">via {job.current_source}</span>
        {/if}
        {#if lastActivityMessage}
          <span class="task-activity">{lastActivityMessage}</span>
        {/if}
      </div>
    </div>
  {/if}

  {#if job.status === "running" && phaseRows.length > 0}
    <div class="phase-metrics mt-2">
      <div class="phase-metrics-header">
        <span>Slowest phases (session cumulative)</span>
      </div>
      <ul class="phase-list">
        {#each phaseRows as row (row.key)}
          <li class="phase-row">
            <span class="phase-label">{row.label}</span>
            <span class="phase-stats">
              {row.totalMs.toFixed(0)} ms total · {row.avgMs.toFixed(0)} ms avg · {row.count}×
              {#if row.cacheHitPct !== null}· cache {row.cacheHitPct}%{/if}
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
