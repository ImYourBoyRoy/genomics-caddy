<!-- ./src/lib/components/research/ResearchJobProgressPanel.svelte -->
<script lang="ts">
  import { untrack } from "svelte";
  import type { ResearchJob, PipelineTuningPublic, ResearchScopePreview } from "../../types/research";
  import Tooltip from "../common/Tooltip.svelte";
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
    return liveProgress.batchTotal ?? pipelineTuning?.enrich_batch_size ?? null;
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
      : 0,
  );

  let activePipelineIndex = $derived(pipelineStepIndex(activityPhase));
  let batchDone = $derived(computeBatchDone(activityPhase, batchPrepared, batchPrefetchDone, batchTotal));
  let batchLabel = $derived(computeBatchLabel(activityPhase));
  let interimPct = $derived(interimProgressPct(job, progressPct, batchTotal, batchDone));

  let showBatchProgress = $derived(
    job.status === "running" &&
      ((batchTotal != null && batchTotal > 0) ||
        activityPhase != null ||
        isResumeOrCheckPhase(activityPhase) ||
        (batchElapsedSecs != null && batchElapsedSecs > 0) ||
        (job.current_source?.toLowerCase().includes("prepare") ?? false) ||
        (job.current_source?.toLowerCase().includes("qdrant") ?? false)),
  );

  let batchPct = $derived(
    batchTotal != null && batchTotal > 0
      ? Math.min(100, Math.round((batchDone / batchTotal) * 100))
      : job.status === "running"
        ? Math.min(90, Math.max(8, (batchElapsedSecs ?? 0) * 2))
        : 0,
  );

  let queueMismatch = $derived(scopeQueueMismatch(scopePreview, job));
  let liveRsid = $derived(liveRsidFromActivity(lastActivityMessage));
  let phaseRows = $derived(buildPhaseRows(phaseMetrics));

  type SpeedSample = { t: number; count: number };
  let speedSamples = $state<SpeedSample[]>([]);
  let speedBaseline = $state<number | null>(null);
  let tick = $state(Date.now());
  let showDiagnostics = $state(false);

  $effect(() => {
    if (job.status !== "running") {
      speedSamples = [];
      speedBaseline = null;
      return;
    }
    if (speedBaseline == null) speedBaseline = job.enriched_count;
    void tick;
    void job.enriched_count;
    const baseline = speedBaseline ?? job.enriched_count;
    const now = Date.now();
    const throughput = Math.max(0, job.enriched_count - baseline);
    const prev = untrack(() => speedSamples);
    speedSamples = [...prev, { t: now, count: throughput }].filter((s) => s.t >= now - 120_000);
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
    if (etaSec == null) {
      const phase = activityPhase?.toLowerCase() ?? "";
      if (phase.includes("bootstrap")) {
        return "After bootstrap";
      }
      if (job.enriched_count === 0 && job.status === "running") {
        return "After first batch";
      }
      return "Calibrating…";
    }
    if (etaSec <= 0) return "Done";
    return formatDuration(etaSec);
  });
  let prefetchLabel = $derived(
    prefetchElapsedLabel(job.status, activityPhase, batchElapsedSecs, batchDone, batchTotal),
  );
  let batchStall = $derived(batchStallHint(showBatchProgress, activityPhase, batchElapsedSecs, batchDone));
  let sweepStall = $derived(sweepStallHint(job, batchElapsedSecs, batchDone, lastProgressAt));

  let scanNarrative = $derived.by(() => {
    if (job.status !== "running") {
      if (job.status === "complete") return "Sweep complete — review live findings and the Findings Navigator.";
      if (job.status === "paused") {
        return "Paused at checkpoint. Resume skips vectors already indexed in Qdrant.";
      }
      return null;
    }
    const phase = activityPhase?.toLowerCase() ?? "";
    if (phase.includes("bootstrap")) {
      return "Scanning which queued variants are already indexed — overall % stays on the known Qdrant count until this scan finishes.";
    }
    if (phase.includes("qdrant")) {
      return "Committing this batch to Qdrant — indexed count updates when upsert finishes.";
    }
    if (phase.includes("embedding")) {
      return "Embedding prepared evidence. Indexed progress updates after Qdrant upsert.";
    }
    if (isPrefetchPhase(activityPhase)) {
      return "Fetching remote source context (gnomAD / NCBI / GTEx). Slow here is usually API latency.";
    }
    if (activityPhase) {
      return `Preparing evidence for ${job.current_rsid || liveRsid || "this batch"}.`;
    }
    return "Scanning the queue and checking which variants are already in Qdrant.";
  });

  let checkpointAge = $derived.by(() => {
    void tick;
    if (!job.last_updated) return null;
    const sec = Math.max(0, Math.floor(Date.now() / 1000) - job.last_updated);
    return formatDuration(sec);
  });

  let sourceHealthRows = $derived.by(() => {
    const metrics = phaseMetrics;
    if (!metrics) return [];
    return buildPhaseRows(metrics)
      .slice(0, 6)
      .map((row) => ({
        ...row,
        status: activityPhase?.toLowerCase().includes(row.label.toLowerCase().split("/")[0].toLowerCase())
          ? "active"
          : row.avgMs >= 5_000
            ? "slow"
            : "ok",
      }));
  });

  let batchesRemaining = $derived.by(() => {
    const size = pipelineTuning?.enrich_batch_size ?? batchTotal;
    if (!size || size <= 0 || job.total_markers <= 0) return null;
    return Math.ceil(Math.max(0, job.total_markers - job.enriched_count) / size);
  });

  let speedDisplay = $derived.by(() => {
    if (formattedSpeed !== null) {
      return { label: "Speed", value: `${formattedSpeed} var/s`, note: speedKind };
    }
    if (prefetchLabel || isResumeOrCheckPhase(activityPhase)) {
      return {
        label: isResumeOrCheckPhase(activityPhase) ? "Setup" : "Prefetch",
        value: prefetchLabel ?? (batchElapsedSecs != null ? formatDuration(batchElapsedSecs) : "…"),
        note: isResumeOrCheckPhase(activityPhase) ? activityPhase : "remote APIs",
      };
    }
    if (batchElapsedSecs != null && batchElapsedSecs > 0) {
      return {
        label: "Working",
        value: formatDuration(batchElapsedSecs),
        note: formatActivityPhase(activityPhase),
      };
    }
    if (sessionElapsed) {
      return { label: "Session", value: sessionElapsed, note: "this run" };
    }
    return { label: "Speed", value: "Calibrating…", note: "first batch" };
  });
</script>

{#if pipelineTuning}
  <div class="tuning-banner">
    <span class="tuning-profile">{pipelineTuning.profile}</span>
    <span class="tuning-pill">batch {pipelineTuning.enrich_batch_size}</span>
    <span class="tuning-pill">{pipelineTuning.prepare_concurrency}× prepare</span>
    {#if pipelineTuning.ollama_latency_ms != null || pipelineTuning.qdrant_latency_ms != null}
      <span class="tuning-latency">
        {#if pipelineTuning.ollama_latency_ms != null}ollama {pipelineTuning.ollama_latency_ms}ms{/if}
        {#if pipelineTuning.qdrant_latency_ms != null}
          {pipelineTuning.ollama_latency_ms != null ? " · " : ""}qdrant {pipelineTuning.qdrant_latency_ms}ms
        {/if}
      </span>
    {/if}
    {#if pipelineTuning.fast_sweep}
      <span class="tuning-fast">fast sweep</span>
    {/if}
    <Tooltip
      label="Adaptive sweep tuning"
      description="Batch size and preparation parallelism adapt from measured Ollama and Qdrant latency."
      triggerClass="tuning-help-trigger"
    >
      <span class="tuning-help" aria-hidden="true">ⓘ</span>
    </Tooltip>
  </div>
{/if}

{#if queueMismatch}
  <div class="scope-mismatch-banner">
    Scope queue is <strong>{queueMismatch.configured.toLocaleString()}</strong> variants,
    but this run is locked at <strong>{queueMismatch.running.toLocaleString()}</strong>.
    Cancel and start a new sweep (or Expand queue after completion) to apply the updated cap.
  </div>
{/if}

{#if job.status === "paused"}
  <div class="checkpoint-banner" role="status">
    <strong>Checkpoint</strong>
    · rsID <code>{job.current_rsid || liveRsid || "—"}</code>
    · {job.enriched_count.toLocaleString()} / {job.total_markers.toLocaleString()}
    {#if qdrantIndexed != null}
      · ~{qdrantIndexed.toLocaleString()} in Qdrant
    {/if}
    {#if checkpointAge}
      · paused {checkpointAge}
    {/if}
  </div>
{/if}

<div class="progress-section mt-2">
  {#if qdrantIndexed != null && qdrantIndexed > 0}
    <div class="qdrant-baseline-banner">
      <span>
        <strong>{qdrantIndexed.toLocaleString()}</strong> already in Qdrant for this sample
        {#if qdrantCoveragePct != null}({qdrantCoveragePct}% of queue){/if}
        {#if collectionVectorCount != null}
          · collection {collectionVectorCount.toLocaleString()}
        {/if}
      </span>
    </div>
  {/if}

  <div class="sweep-hero">
    <div class="sweep-hero-pct">{interimPct}%</div>
    <div class="sweep-hero-copy">
      <div class="progress-labels">
        <span
          >{activityPhase?.toLowerCase().includes("bootstrap")
            ? "Queue coverage (held during bootstrap)"
            : "Sweep progress"}</span
        >
        <span class="progress-value">
          {job.enriched_count.toLocaleString()} / {job.total_markers.toLocaleString()}
          · {remainingLabel}
        </span>
      </div>
      <div class="progress-bar-container progress-bar-lg">
        <div class="progress-bar-fill" style="width: {interimPct}%"></div>
      </div>
    </div>
  </div>

  {#if job.status === "running"}
    <div class="metric-grid mt-2">
      <div class="metric-cell">
        <span class="metric-label">{speedDisplay.label}</span>
        <span class="metric-value">{speedDisplay.value}</span>
        {#if speedDisplay.note}<span class="metric-note">{speedDisplay.note}</span>{/if}
      </div>
      <div class="metric-cell">
        <span class="metric-label">ETA</span>
        <span class="metric-value">{formattedEta}</span>
        <span class="metric-note">remaining queue</span>
      </div>
      <div class="metric-cell">
        <span class="metric-label">Batch size</span>
        <span class="metric-value">{batchTotal ?? pipelineTuning?.enrich_batch_size ?? "—"}</span>
        <span class="metric-note">
          {#if batchesRemaining != null}
            ~{batchesRemaining.toLocaleString()} left
          {:else}
            adaptive
          {/if}
        </span>
      </div>
      {#if lastUpdateAge}
        <div class="metric-cell">
          <span class="metric-label">Updated</span>
          <span class="metric-value">{lastUpdateAge}</span>
          <span class="metric-note">ago</span>
        </div>
      {/if}
    </div>
  {/if}

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
          {#if batchTotal != null && batchTotal > 0}({batchPct}%){/if}
        </span>
      </div>
      <div class="progress-bar-container batch-bar">
        <div
          class="progress-bar-fill batch-fill"
          class:indeterminate={batchTotal == null || batchTotal <= 0}
          style="width: {batchPct}%"
        ></div>
      </div>
      {#if batchStall}
        <p class="batch-stall-hint">{batchStall}</p>
      {/if}
    </div>
  {/if}

  {#if job.status === "running"}
    {#if scanNarrative}
      <div class="scan-narrative mt-2">
        <span>{scanNarrative}</span>
      </div>
    {/if}

    {#if sweepStall}
      <p class="batch-stall-hint">{sweepStall}</p>
    {/if}

    <div class="active-task mt-2">
      <span class="pulse-ring"></span>
      <div class="task-info">
        {#if activityPhase && batchTotal}
          <span class="task-title">{formatActivityPhase(activityPhase)}</span>
          <strong class="task-rsid">{job.current_rsid || "…"}</strong>
          <span class="task-batch">
            {batchDone}/{batchTotal} {batchLabel}
            {#if batchElapsedSecs != null}· {formatDuration(batchElapsedSecs)}{/if}
          </span>
        {:else}
          <span class="task-title">Working</span>
          <strong class="task-rsid">{job.current_rsid || liveRsid || "…"}</strong>
        {/if}
        {#if lastActivityMessage}
          <span class="task-activity">{lastActivityMessage}</span>
        {/if}
      </div>
    </div>

    {#if sourceHealthRows.length > 0 || phaseRows.length > 0}
      <details class="diagnostics-fold mt-2" bind:open={showDiagnostics}>
        <summary>Session diagnostics</summary>
        {#if sourceHealthRows.length > 0}
          <div class="source-health-panel">
            <div class="source-health-header">
              <span>Source health</span>
            </div>
            <div class="source-health-grid">
              {#each sourceHealthRows as row (row.key)}
                <div class="source-health-row status-{row.status}">
                  <span class="source-health-dot"></span>
                  <span class="source-health-label">{row.label}</span>
                  <span class="source-health-stat">{row.avgMs.toFixed(0)} ms · {row.count}×</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
        {#if phaseRows.length > 0}
          <div class="phase-metrics">
            <div class="phase-metrics-header">
              <span>Slowest phases</span>
            </div>
            <ul class="phase-list">
              {#each phaseRows as row (row.key)}
                <li class="phase-row">
                  <span class="phase-label">{row.label}</span>
                  <span class="phase-stats">
                    {row.avgMs.toFixed(0)} ms avg · {row.count}×
                    {#if row.cacheHitPct !== null}· cache {row.cacheHitPct}%{/if}
                  </span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </details>
    {/if}
  {/if}
</div>
