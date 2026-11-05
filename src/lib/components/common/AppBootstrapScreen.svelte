<!-- ./src/lib/components/common/AppBootstrapScreen.svelte -->
<script lang="ts">
  /*
  Purpose: Full-screen startup / reference-sync / DNA-import loading surface.
  Shows one calm panel: headline, overall segmented progress, an
  elapsed / step / remaining readout, and a live step list where the active step
  narrates what is happening right now and finished steps report how long they
  took. The DNA helix sits behind it on the right.
  Inputs: phase + message (startup), reportProgress (backend report events),
  import* props (DNA import), mode.
  Outputs: markup only; the clock and step math live in utils/pipelineClock and
  bootstrap/bootstrapPipeline so they stay testable.
  */
  import { untrack } from "svelte";
  import type {
    AppBootstrapStatus,
    GenomeImportPreview,
    ReportGenerationProgress,
  } from "../../types/genomics";
  import {
    BOOTSTRAP_STEPS,
    describeDbProgress,
    stepState,
    type BootstrapPhase,
    type DbProgress,
  } from "./bootstrap/bootstrapPhases";
  import {
    buildImportSteps,
    buildResourceSteps,
    buildStartupSteps,
    reportEtaSample,
    resourceFocus,
    stepStates,
  } from "./bootstrap/bootstrapPipeline";
  import BootstrapHelixBackdrop from "./bootstrap/BootstrapHelixBackdrop.svelte";
  import BootstrapPhaseOrb from "./bootstrap/BootstrapPhaseOrb.svelte";
  import BootstrapStatReveal from "./bootstrap/BootstrapStatReveal.svelte";
  import LoadingTelemetryStrip from "./loading/LoadingTelemetryStrip.svelte";
  import PipelineProgressBar from "./loading/PipelineProgressBar.svelte";
  import PipelineStepList from "./loading/PipelineStepList.svelte";
  import {
    overallPercent,
    segmentsFromSteps,
    type PipelineSegment,
    type PipelineStep,
  } from "./loading/pipelineTypes";
  import { IMPORT_STEPS, importStepState, type ImportPhase, type ImportStepId } from "../../utils/importProgress";
  import { parseMessageProgress } from "../../utils/loadingTelemetry";
  import { PipelineClock } from "../../utils/pipelineClock.svelte";
  import type { ResourceSyncTelemetry } from "../../utils/resourceSyncTelemetry";
  import "../../styles/bootstrap-animations.css";

  export type { BootstrapPhase };

  interface Props {
    phase: BootstrapPhase;
    message: string;
    status?: AppBootstrapStatus | null;
    error?: string;
    mode?: "startup" | "resources" | "import";
    importPhase?: ImportPhase;
    importProgress?: { percentage: number; status: string } | null;
    importPreview?: GenomeImportPreview | null;
    importProfileName?: string;
    importFailedStep?: ImportStepId | null;
    showWorkspaceAction?: boolean;
    /** Live backend report progress while the startup report step runs. */
    reportProgress?: ReportGenerationProgress | null;
    /** Structured reference-sync telemetry (bytes, rows, tier/asset counts). */
    resourceSync?: ResourceSyncTelemetry | null;
    workspaceActionLabel?: string;
    onContinue?: () => void;
  }

  let {
    phase,
    message,
    status = null,
    error = "",
    mode = "startup",
    importPhase = "idle",
    importProgress = null,
    importPreview = null,
    importProfileName = "",
    importFailedStep = null,
    showWorkspaceAction = false,
    reportProgress = null,
    resourceSync = null,
    workspaceActionLabel = "",
    onContinue,
  }: Props = $props();

  let isError = $derived(phase === "error");
  let isReady = $derived(phase === "ready");

  const NO_TIMINGS = { durationMs: () => null, stepElapsedMs: () => null };

  // Startup is timed from webview launch (honest total); other modes from mount.
  const clock = new PipelineClock(
    untrack(() => (mode === "startup" ? Date.now() - performance.now() : Date.now())),
  );

  $effect(() => {
    clock.start();
    return () => clock.stop();
  });

  // Remember where startup was when it failed so the right step shows the error.
  let lastLivePhase = $state<BootstrapPhase>("db");
  $effect(() => {
    if (phase !== "error") lastLivePhase = phase;
  });

  // Database sub-steps only ever move forward, even if the idle ticker re-fires.
  let dbProgress = $state<DbProgress | null>(null);
  $effect(() => {
    if (mode !== "startup" || phase !== "db") return;
    const next = describeDbProgress(message);
    if (!next) return;
    untrack(() => {
      if (!dbProgress || next.index >= dbProgress.index) dbProgress = next;
    });
  });

  let importPercent = $derived(
    isReady && mode === "import"
      ? 100
      : Math.max(0, Math.min(100, Math.round(importProgress?.percentage ?? 0))),
  );

  let activityMessage = $derived(
    mode === "import"
      ? isError
        ? error || importProgress?.status || "DNA import stopped"
        : importProgress?.status || message || "Preparing DNA import…"
      : isError
        ? error || "Startup failed"
        : message,
  );

  // Step states drive the clock (durations); they must not depend on the clock.
  let stateMap = $derived.by(() => {
    if (mode === "import") {
      return Object.fromEntries(
        IMPORT_STEPS.map((step) => [step.id, importStepState(importPhase, step.id, importFailedStep)]),
      );
    }
    if (mode === "resources") {
      // Same builder as the visible list, minus timings, so the clock never feeds itself.
      return stepStates(buildResourceSteps({ message, clock: NO_TIMINGS, telemetry: resourceSync }));
    }
    const failed = isError ? lastLivePhase : null;
    return Object.fromEntries(BOOTSTRAP_STEPS.map((step) => [step.id, stepState(phase, step.id, failed)]));
  });

  $effect(() => {
    clock.sync(stateMap);
  });

  $effect(() => {
    if (isReady || isError || (mode === "import" && (importPhase === "ready" || importPhase === "error"))) {
      clock.freeze();
    }
  });

  let steps = $derived.by<PipelineStep[]>(() => {
    if (mode === "import") {
      return buildImportSteps({
        phase: importPhase,
        failedStep: importFailedStep,
        status: activityMessage,
        clock,
      });
    }
    if (mode === "resources") return buildResourceSteps({ message, clock, telemetry: resourceSync });
    return buildStartupSteps({
      phase,
      failedPhase: isError ? lastLivePhase : null,
      message,
      error: error || "",
      dbProgress,
      reportProgress,
      clock,
    });
  });

  let segments = $derived.by<PipelineSegment[]>(() => {
    if (mode === "import") {
      const state = isError ? "error" : isReady ? "done" : "active";
      const accent = state === "error" ? "#f87171" : steps[Math.max(0, steps.findIndex((s) => s.state === "active"))]?.accent ?? "#2dd4bf";
      return [{ id: "import", state, accent, fraction: importPercent / 100 }];
    }
    if (mode === "resources") {
      // Only the live operation has honest progress; unknown amounts sweep.
      const live = steps.find((step) => step.state === "active");
      return [
        {
          id: "resources",
          state: "active",
          accent: live?.accent ?? "#2dd4bf",
          fraction: typeof live?.fraction === "number" ? live.fraction : null,
        },
      ];
    }
    return segmentsFromSteps(steps);
  });

  let percent = $derived.by<number | null>(() => {
    if (mode === "import") return importPercent;
    if (mode === "resources") {
      const fraction = segments[0]?.fraction;
      return typeof fraction === "number" ? Math.round(fraction * 100) : null;
    }
    if (isReady) return 100;
    return overallPercent(steps);
  });

  let determinate = $derived(percent !== null);
  let resourceView = $derived(resourceFocus(resourceSync, steps));

  let activeIndex = $derived(steps.findIndex((step) => step.state === "active"));
  let errorIndex = $derived(steps.findIndex((step) => step.state === "error"));
  let focusIndex = $derived(activeIndex >= 0 ? activeIndex : errorIndex >= 0 ? errorIndex : 0);
  let finished = $derived(isReady || (mode === "import" && importPhase === "ready"));
  let failed = $derived(isError || (mode === "import" && importPhase === "error"));
  let stepIndex = $derived(
    mode === "resources" ? resourceView.stepIndex : finished ? steps.length : focusIndex + 1,
  );
  let stepLabel = $derived(
    mode === "resources"
      ? resourceView.stepLabel
      : mode === "import" && segments.length === 1 && !finished && activeIndex < 0
        ? "Waiting for confirmation"
        : finished
        ? "All steps complete"
        : (steps[focusIndex]?.label ?? ""),
  );

  // Import ETA is measured from the moment real percentages start arriving.
  let importStartOffsetMs = $state<number | null>(null);
  $effect(() => {
    if (mode !== "import" || importPercent <= 0) return;
    untrack(() => {
      if (importStartOffsetMs === null) importStartOffsetMs = clock.elapsedMs;
    });
  });

  $effect(() => {
    if (finished || failed) {
      clock.observeEta("settled", null);
      return;
    }
    if (mode === "import") {
      const measured = importStartOffsetMs === null ? 0 : clock.elapsedMs - importStartOffsetMs;
      clock.observeEta(
        "import",
        importPercent > 0 && importPercent < 100
          ? { fraction: importPercent / 100, elapsedMs: measured }
          : null,
      );
      return;
    }
    if (mode === "resources") {
      // Download/index ETAs come straight from the backend counters (resume-aware).
      clock.observeEta("resources", null);
      return;
    }
    const sample = phase === "report" ? reportEtaSample(reportProgress) : null;
    clock.observeEta(`startup:${phase}:${reportProgress?.phase ?? ""}`, sample);
  });

  let etaCaption = $derived(
    mode === "import" ? "left to import" : mode === "resources" ? resourceView.etaCaption : "left in this scan",
  );
  let etaSeconds = $derived(mode === "resources" ? resourceView.etaSeconds : clock.eta);
  let barCaption = $derived(
    mode === "import"
      ? "Import progress"
      : mode === "resources"
        ? (steps.find((step) => step.state === "active")?.label ?? "Reference sync")
        : "Startup progress",
  );
  let stepListLabel = $derived(
    mode === "import" ? "DNA import steps" : mode === "resources" ? "Reference sync steps" : "Startup steps",
  );
</script>

<div
  class="bootstrap-screen bootstrap-motion"
  class:ready={isReady}
  class:error={isError}
>
  <div
    class="bootstrap-content"
    style="opacity:1;visibility:visible;color:#eef2ff;background:#0c1220;"
  >
    <!-- Inline headline styles: never rely on delayed CSS to reveal status text. -->
    <p
      class="startup-kicker"
      style="margin:0;color:#7dd3fc;font-size:0.68rem;font-weight:800;letter-spacing:0.08em;text-transform:uppercase;"
    >
      {mode === "import" ? "DNA import" : mode === "resources" ? "Reference sync" : "Startup"}
    </p>

    <BootstrapPhaseOrb {phase} />

    <h1 class="title" style="margin:0.35rem 0 0;color:#eef2ff;font-size:1.35rem;font-weight:700;line-height:1.25;">
      {#if isReady}
        {mode === "import" ? "DNA Profile Ready" : "Genomics Caddy Ready"}
      {:else if isError}
        {mode === "import" ? "DNA Import Needs Attention" : "Startup Interrupted"}
      {:else if mode === "resources"}
        Preparing Reference Resources
      {:else if mode === "import"}
        Importing DNA Profile
      {:else}
        Starting Genomics Caddy
      {/if}
    </h1>

    <p class="lead" style="margin:0.45rem 0 0;color:#c7d2fe;font-size:0.92rem;line-height:1.45;font-weight:600;">
      {#if isReady}
        {mode === "import"
          ? `Loaded ${importProfileName || "the imported profile"} and prepared its report.`
          : "Your local genome workspace is loaded and waiting."}
      {:else if isError}
        {mode === "import"
          ? "The import stopped safely. No incomplete profile was left behind; return to the workspace to retry."
          : "Check the database path and try restarting the app."}
      {:else if mode === "resources"}
        Downloading and indexing local catalogs — this may take a while for large references.{#if resourceSync?.skipped}{" "}{resourceSync.skipped} already up to date.{/if}
      {:else if mode === "import"}
        Reading the file locally, normalizing coordinates, and preparing the selected profile.
      {:else if status?.sample_count === 0}
        Checking your local database and catalogs — import a genome whenever you’re ready.
      {:else}
        Preparing your local database and profile — this may take a moment on first launch.
      {/if}
    </p>

    <div class="loading-telemetry" role="group" aria-label={barCaption} aria-busy={!finished && !failed}>
      <PipelineProgressBar
        {segments}
        {percent}
        caption={barCaption}
        label={barCaption}
        explanation={mode === "startup"
          ? "An estimate across the four startup steps. The active step below shows marker-check progress; this is not the percentage of your whole DNA that was scanned."
          : ""}
        {determinate}
      />
      <LoadingTelemetryStrip
        elapsedMs={clock.elapsedMs}
        {stepIndex}
        stepCount={mode === "resources" ? resourceView.stepCount : steps.length}
        stepTitle={mode === "resources" ? resourceView.stepTitle : "Step"}
        {stepLabel}
        {etaSeconds}
        etaUnknown={mode === "resources" && resourceView.etaUnknown}
        status={failed ? "error" : finished ? "done" : "running"}
        {etaCaption}
      />
    </div>

    <div class="loading-steps">
      <PipelineStepList {steps} ariaLabel={stepListLabel} />
    </div>

    {#if mode === "import" && importPreview}
      <div class="import-details bootstrap-motion" aria-label="DNA import details">
        <div class="import-details-heading">
          <span>Import details</span>
          <span class="import-local-badge">On-device</span>
        </div>
        <dl class="import-details-grid">
          <div>
            <dt>Profile</dt>
            <dd>{importProfileName || "New profile"}</dd>
          </div>
          <div>
            <dt>Source</dt>
            <dd title={importPreview.source_file_name}>{importPreview.source_file_name}</dd>
          </div>
          <div>
            <dt>Format</dt>
            <dd>{importPreview.diagnostics.vendor} · {importPreview.diagnostics.delimiter}</dd>
          </div>
          <div>
            <dt>Records</dt>
            <dd>
              {importPreview.diagnostics.accepted_rows.toLocaleString()} /
              {importPreview.diagnostics.total_rows.toLocaleString()} accepted
            </dd>
          </div>
          <div>
            <dt>Source build</dt>
            <dd>{importPreview.diagnostics.source_build}</dd>
          </div>
          <div>
            <dt>Liftover</dt>
            <dd>{importPreview.liftover_available ? "Available" : "Not installed"}</dd>
          </div>
        </dl>
        {#if importPreview.diagnostics.malformed_rows > 0 || importPreview.diagnostics.duplicate_rows > 0 || importPreview.diagnostics.warnings.length > 0}
          <p class="import-details-note">
            {importPreview.diagnostics.malformed_rows.toLocaleString()} malformed ·
            {importPreview.diagnostics.duplicate_rows.toLocaleString()} duplicates ·
            {importPreview.diagnostics.warnings.length.toLocaleString()} warnings were recorded in the preview.
          </p>
        {/if}
      </div>
    {/if}

    {#if status && !isError && mode === "startup"}
      <BootstrapStatReveal {status} {phase} />
    {/if}

    {#if showWorkspaceAction}
      <button type="button" class="workspace-action bootstrap-motion" onclick={() => onContinue?.()}>
        {workspaceActionLabel || (mode === "import" ? "Return to workspace" : "Continue in workspace")}
      </button>
    {/if}
  </div>

  <aside class="bootstrap-helix-slot" aria-hidden="true">
    <BootstrapHelixBackdrop {phase} />
  </aside>
</div>

<style>
  .bootstrap-screen {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    min-height: 100%;
    min-height: 100dvh;
    padding: 2rem clamp(1rem, 3.5vw, 2.75rem);
    text-align: center;
    overflow: visible;
    box-sizing: border-box;
  }

  .bootstrap-content {
    position: relative;
    z-index: 2;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: min(100%, 38rem);
    max-width: 38rem;
    padding: 1.55rem 1.65rem 1.45rem;
    border: 1px solid rgba(191, 219, 254, 0.34);
    border-radius: 20px;
    /* Solid panel with literal colors. Do not fade this in: fill-mode:both
       plus a blocked UI thread during native DB bootstrap held the card at
       opacity 0, leaving only the DNA helix visible. */
    background: #0c1220;
    color: #eef2ff;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.36), inset 0 1px 0 rgba(255, 255, 255, 0.05);
    box-sizing: border-box;
    opacity: 1;
    visibility: visible;
    transform: none;
    animation: none;
  }

  .bootstrap-helix-slot {
    position: absolute;
    inset: 0;
    z-index: 1;
    pointer-events: none;
  }

  .title {
    font-size: 1.35rem;
    margin: 0;
    color: #eef2ff;
    opacity: 1;
    animation: none;
  }

  .lead {
    max-width: 34rem;
    color: #c7d2fe;
    font-size: 0.92rem;
    margin: 0.45rem 0 0;
    opacity: 1;
    animation: none;
    text-align: center;
  }

  .import-details {
    align-self: stretch;
    width: 100%;
    max-width: 440px;
    margin: -0.15rem auto 1rem;
    padding: 0.8rem 0.9rem;
    border: 1px solid rgba(125, 211, 252, 0.16);
    border-radius: 10px;
    background: rgba(3, 7, 18, 0.55);
    text-align: left;
    box-sizing: border-box;
    opacity: 1;
  }

  .import-details-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.65rem;
    color: var(--text-primary);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .import-local-badge {
    padding: 0.18rem 0.4rem;
    border: 1px solid rgba(52, 211, 153, 0.3);
    border-radius: 999px;
    color: #86efac;
    font-family: var(--font-mono), monospace;
    font-size: 0.58rem;
    font-weight: 600;
    letter-spacing: 0;
    text-transform: none;
  }

  .import-details-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.55rem 0.75rem;
    margin: 0;
  }

  .import-details-grid > div {
    min-width: 0;
  }

  .import-details-grid dt {
    margin-bottom: 0.12rem;
    color: var(--text-secondary);
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .import-details-grid dd {
    margin: 0;
    overflow: hidden;
    color: var(--text-primary);
    font-family: var(--font-mono), monospace;
    font-size: 0.68rem;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .import-details-note {
    margin: 0.7rem 0 0;
    padding-top: 0.6rem;
    border-top: 1px solid rgba(148, 163, 184, 0.12);
    color: var(--text-secondary);
    font-size: 0.65rem;
    line-height: 1.4;
  }

  .loading-telemetry {
    display: flex;
    align-self: stretch;
    flex-direction: column;
    gap: 0.85rem;
    width: 100%;
    margin-top: 1.1rem;
  }

  .loading-steps {
    align-self: stretch;
    width: 100%;
    margin: 0.9rem 0 0.3rem;
  }

  .workspace-action {
    min-height: 2.5rem;
    margin-top: 0.25rem;
    padding: 0.55rem 0.9rem;
    border: 1px solid var(--border-strong);
    border-radius: 0.55rem;
    background: var(--surface-control);
    color: var(--text-primary);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
    opacity: 1;
  }

  .workspace-action:hover {
    border-color: var(--focus-ring);
    background: var(--surface-subtle);
  }

  .workspace-action:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  @media screen and (max-height: 800px) {
    .bootstrap-screen {
      place-items: start center;
      padding-top: clamp(1rem, 3vh, 1.5rem);
      padding-bottom: clamp(1rem, 3vh, 1.5rem);
    }
  }

  @media (max-width: 720px) {
    .bootstrap-screen {
      padding: 1.25rem 0.8rem;
    }

    .bootstrap-helix-slot {
      display: block;
    }

    .bootstrap-content {
      padding: 1.5rem 1rem 1.25rem;
    }

    .import-details-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
