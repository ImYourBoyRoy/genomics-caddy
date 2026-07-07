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
    inferActivityPhase,
    formatRemainingCount,
    sessionSpeedFromJob,
    secondsSince,
    sweepStallHint,
  } from "../../utils/researchJobMetrics";

  interface Props {
    job: ResearchJob;
    pipelineTuning?: PipelineTuningPublic | null;
    scopePreview?: ResearchScopePreview | null;
    qdrantSampleCount?: number | null;
    collectionVectorCount?: number | null;
  }

  let {
    job,
    pipelineTuning = null,
    scopePreview = null,
    qdrantSampleCount = null,
    collectionVectorCount = null,
  }: Props = $props();

  let activityPhase = $derived.by(() => {
    void liveProgress.tick;
    return inferActivityPhase(
      liveProgress.activityPhase,
      job.current_source,
      liveProgress.lastActivityMessage,
    );
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
        (batchElapsedSecs != null && batchElapsedSecs > 0) ||
        (job.current_source?.toLowerCase().includes("prepare") ?? false) ||
        (job.current_source?.toLowerCase().includes("qdrant") ?? false))
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
    const baseline = speedBaseline ?? job.enriched_count;
    const now = Date.now();
    const throughput = Math.max(0, job.enriched_count - baseline);
    speedSamples = [...speedSamples, { t: now, count: throughput }].filter((s) => s.t >= now - 120_000);
  });

  $effect(() => {
    if (job.status !== "running") return;
    const id = setInterval(() => {
      tick = Date.now();
    }, 1000);
    return () => clearInterval(id);
  });

  let sessionSpeed = $derived(sessionSpeedFromJob(job));
  let rolling = $derived(rollingSpeedFromSamples(speedSamples));
  let batchSpeed = $derived(batchPrepareSpeed(activityPhase, batchElapsedSecs, batchPrepared, batchPrefetchDone));
  let speed = $derived(
    effectiveSpeed(rolling, recentProgressSpeed, batchSpeed, progressSpeed, sessionSpeed),
  );
  let etaSec = $derived.by(() => {
    void tick;
    if (job.status !== "running" || speed == null || speed <= 0) return null;
    const remaining = job.total_markers - job.enriched_count;
    return remaining <= 0 ? 0 : remaining / speed;
  });
  let formattedSpeed = $derived(speed != null && !Number.isNaN(speed) ? speed.toFixed(1) : null);
  let speedKind = $derived(
    speedLabel(rolling, recentProgressSpeed, batchSpeed, activityPhase, progressSpeed, sessionSpeed),
  );
  let remainingLabel = $derived(formatRemainingCount(job.enriched_count, job.total_markers));
  let sessionElapsedSecs = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.sessionElapsedSecs ?? job.session_elapsed_secs ?? null;
  });
  let qdrantIndexed = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.qdrantSampleCount ?? job.qdrant_sample_count ?? qdrantSampleCount ?? null;
  });
  let sessionElapsed = $derived.by(() => {
    void tick;
    if (sessionElapsedSecs != null && sessionElapsedSecs > 0) {
      return formatDuration(sessionElapsedSecs);
    }
    if (batchElapsedSecs != null && batchElapsedSecs > 0) {
      return formatDuration(batchElapsedSecs);
    }
    return null;
  });
  let qdrantCoveragePct = $derived.by(() => {
    if (qdrantIndexed == null || job.total_markers <= 0) return null;
    return Math.min(100, Math.round((qdrantIndexed / job.total_markers) * 100));
  });
  let lastUpdateAge = $derived.by(() => {
    void tick;
    const sec = secondsSince(lastProgressAt);
    return sec != null ? formatDuration(sec) : null;
  });
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

  let scanNarrative = $derived.by(() => {
    if (job.status !== "running") {
      if (job.status === "complete") return "Sweep complete — review the live finding feed and Findings Navigator.";
      if (job.status === "paused") return "Sweep paused — resume to continue enriching the queue.";
      return null;
    }
    if (activityPhase?.toLowerCase().includes("qdrant")) {
      return "Saving enriched evidence into Qdrant. Live findings appear after this batch is committed.";
    }
    if (activityPhase?.toLowerCase().includes("embedding")) {
      return "Converting prepared evidence into vectors. Indexed progress updates when the batch is upserted.";
    }
    if (isPrefetchPhase(activityPhase)) {
      return "Fetching remote source context. Slow movement usually means gnomAD/NCBI/GTEx latency, not a frozen app.";
    }
    if (activityPhase) {
      return `Preparing evidence for ${job.current_rsid || liveRsid || "the current batch"}.`;
    }
    return "Scanning the selected queue and checking existing Qdrant cache state.";
  });

  let sourceHealthRows = $derived.by(() => {
    const metrics = phaseMetrics;
    if (!metrics) return [];
    const rows = buildPhaseRows(metrics).slice(0, 6);
    return rows.map((row) => ({
      ...row,
      status:
        activityPhase?.toLowerCase().includes(row.label.toLowerCase().split("/")[0].toLowerCase())
          ? "active"
          : row.avgMs >= 5_000
            ? "slow"
            : "ok",
    }));
  });
</script>

{#if pipelineTuning}
  <div class="tuning-banner">
    <span class="tuning-profile">{pipelineTuning.profile}</span>
    {#if pipelineTuning.ollama_latency_ms != null || pipelineTuning.qdrant_latency_ms != null}
      <span>
        latency
        {#if pipelineTuning.ollama_latency_ms != null}ollama {pipelineTuning.ollama_latency_ms}ms{/if}
        {#if pipelineTuning.qdrant_latency_ms != null}· qdrant {pipelineTuning.qdrant_latency_ms}ms{/if}
      </span>
    {/if}
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
  {#if qdrantIndexed != null && qdrantIndexed > 0}
    <div class="qdrant-baseline-banner">
      <span>
        <strong>{qdrantIndexed.toLocaleString()}</strong> variants for this sample already in Qdrant
        {#if qdrantCoveragePct != null}
          ({qdrantCoveragePct}% of this queue)
        {/if}
        {#if collectionVectorCount != null}
          · collection total {collectionVectorCount.toLocaleString()}
        {/if}
      </span>
      <span class="qdrant-baseline-note">
        Sweep queue progress below counts variants processed this run (skips + new upserts).
      </span>
    </div>
  {/if}

  <div class="progress-labels">
    <span>This sweep — indexed in Qdrant</span>
    <span class="progress-value">
      {job.enriched_count.toLocaleString()} / {job.total_markers.toLocaleString()} ({interimPct}%)
      · {remainingLabel}
    </span>
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
          {batchDone} / {batchTotal ?? "…"} {batchLabel}
          {#if batchElapsedSecs != null}· {formatDuration(batchElapsedSecs)}{/if}
          {#if batchTotal != null && batchTotal > 0}
            ({batchPct}%)
          {/if}
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
    {#if scanNarrative}
      <div class="scan-narrative mt-2">
        <strong>What is happening:</strong>
        <span>{scanNarrative}</span>
      </div>
    {/if}

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
      {:else if batchElapsedSecs != null && batchElapsedSecs > 0}
        <span class="meta-item">
          Working: <strong>{formatDuration(batchElapsedSecs)}</strong>
          <span class="speed-kind">({formatActivityPhase(activityPhase)})</span>
        </span>
      {:else if sessionElapsed}
        <span class="meta-item">
          Session: <strong>{sessionElapsed}</strong>
          <span class="speed-kind">(this run)</span>
        </span>
      {:else}
        <span class="meta-item">Speed: <strong>Estimating…</strong></span>
      {/if}
      <span class="meta-item">ETA: <strong>{formattedEta}</strong></span>
      {#if lastUpdateAge}
        <span class="meta-item">Updated: <strong>{lastUpdateAge} ago</strong></span>
      {/if}
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

  {#if job.status === "running" && sourceHealthRows.length > 0}
    <div class="source-health-panel mt-2">
      <div class="source-health-header">
        <span>Source health</span>
        <span class="source-health-note">based on this session's observed timings</span>
      </div>
      <div class="source-health-grid">
        {#each sourceHealthRows as row (row.key)}
          <div class="source-health-row status-{row.status}">
            <span class="source-health-dot"></span>
            <span class="source-health-label">{row.label}</span>
            <span class="source-health-stat">{row.avgMs.toFixed(0)} ms avg · {row.count}×</span>
          </div>
        {/each}
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
