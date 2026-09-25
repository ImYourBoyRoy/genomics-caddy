<!-- ./src/lib/components/common/AppBootstrapScreen.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus } from "../../types/genomics";
  import type { GenomeImportPreview } from "../../types/genomics";
  import { PHASE_ORDER, type BootstrapPhase } from "./bootstrap/bootstrapPhases";
  import BootstrapHelixBackdrop from "./bootstrap/BootstrapHelixBackdrop.svelte";
  import BootstrapPhaseOrb from "./bootstrap/BootstrapPhaseOrb.svelte";
  import BootstrapActivityPulse from "./bootstrap/BootstrapActivityPulse.svelte";
  import BootstrapStepTimeline from "./bootstrap/BootstrapStepTimeline.svelte";
  import ImportStepTimeline from "./bootstrap/ImportStepTimeline.svelte";
  import BootstrapStatReveal from "./bootstrap/BootstrapStatReveal.svelte";
  import type { ImportPhase, ImportStepId } from "../../utils/importProgress";
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
    workspaceActionLabel = "",
    onContinue,
  }: Props = $props();

  let isError = $derived(phase === "error");
  let isReady = $derived(phase === "ready");

  function importVisualPhase(value: ImportPhase): BootstrapPhase {
    if (value === "error") return "error";
    if (value === "ready") return "ready";
    if (value === "report") return "report";
    if (value === "profile") return "profile";
    if (value === "liftover" || value === "ingesting" || value === "indexing") return "profile";
    return "stats";
  }

  let activityPhase = $derived(mode === "import" ? importVisualPhase(importPhase) : phase);
  let activityMessage = $derived(
    mode === "import"
      ? isError
        ? error || importProgress?.status || "DNA import stopped"
        : importProgress?.status || message || "Preparing DNA import…"
      : isError
        ? error || "Startup failed"
        : message,
  );
  let importPercent = $derived(
    Math.max(0, Math.min(100, Math.round(importProgress?.percentage ?? 0))),
  );

  let startTime = $state(Date.now());
  let elapsedSeconds = $state(0);

  $effect(() => {
    if (phase === "ready" || phase === "error") return;
    elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
    const timer = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
    }, 1000);
    return () => clearInterval(timer);
  });

  /** Optional (n of m) progress from status messages, with a live estimate. */
  interface ProgressEstimate {
    percent: number;
    etaSeconds: number | null;
    current: number;
    total: number;
  }

  let progressInfo = $derived.by<ProgressEstimate | null>(() => {
    if (!message) return null;
    const match = message.match(/\((\d+)\s+of\s+(\d+)\)/);
    if (!match) return null;
    const current = parseInt(match[1], 10);
    const total = parseInt(match[2], 10);
    if (isNaN(current) || isNaN(total) || total <= 0) return null;

    const percent = Math.min(100, Math.round((current / total) * 100));
    let etaSeconds: number | null = null;
    if (current > 0 && elapsedSeconds > 0) {
      const secondsPerItem = elapsedSeconds / current;
      etaSeconds = Math.round((total - current) * secondsPerItem);
    }
    return { percent, etaSeconds, current, total };
  });

  let importEtaSeconds = $derived.by<number | null>(() => {
    if (mode !== "import" || importPercent <= 0 || elapsedSeconds <= 0 || importPercent >= 100) return null;
    return Math.max(0, Math.round((elapsedSeconds * (100 - importPercent)) / importPercent));
  });

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }

  function formatEta(seconds: number | null): string {
    if (seconds === null) return "estimating…";
    if (seconds <= 0) return "finishing…";
    if (seconds < 60) return `${seconds}s remaining`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s remaining`;
  }

  interface SubStep {
    label: string;
    state: "pending" | "active" | "done";
    details?: string;
  }

  let dbSubSteps = $derived.by<SubStep[]>(() => {
    const steps: SubStep[] = [
      { label: "Connect to database", state: "pending" },
      { label: "Apply schema migrations", state: "pending" },
      { label: "Migrate reference schema mappings", state: "pending" },
      { label: "Seed evidence library packs", state: "pending" },
      { label: "Normalize rsIDs & optimize indexes", state: "pending" }
    ];

    if (phase === "error") {
      steps[0].state = "error" as any;
      return steps;
    }

    const msg = message || "";

    if (msg.includes("Connecting to local database")) {
      steps[0].state = "active";
    } else if (msg.includes("Applying schema migrations")) {
      steps[0].state = "done";
      steps[1].state = "active";
    } else if (msg.includes("Migrating reference schema mappings") || msg.includes("Migrating reference database table")) {
      steps[0].state = "done";
      steps[1].state = "done";
      steps[2].state = "active";
      if (msg.includes("table")) {
        steps[2].details = msg.replace("Migrating reference database table: ", "");
      }
    } else if (msg.includes("Seeding evidence library")) {
      steps[0].state = "done";
      steps[1].state = "done";
      steps[2].state = "done";
      steps[3].state = "active";
      steps[3].details = msg.replace("Seeding evidence library: ", "");
    } else if (msg.includes("Normalizing rsIDs") || msg.includes("Vacuuming") || msg.includes("Syncing GWAS")) {
      steps[0].state = "done";
      steps[1].state = "done";
      steps[2].state = "done";
      steps[3].state = "done";
      steps[4].state = "active";
      if (msg.includes("GWAS")) {
        steps[4].details = "Syncing GWAS reference catalog…";
      } else if (msg.includes("Vacuuming")) {
        steps[4].details = "Vacuuming user database…";
      } else {
        steps[4].details = "Normalizing rsIDs…";
      }
    } else if (phase === "db") {
      steps[0].state = "active";
    }

    // If we've passed the db phase, they are all done
    const currentPhaseIdx = PHASE_ORDER.indexOf(phase);
    const dbPhaseIdx = PHASE_ORDER.indexOf("db");
    if (currentPhaseIdx > dbPhaseIdx) {
      for (const s of steps) s.state = "done";
    }

    return steps;
  });
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
        Downloading and indexing local catalogs — this may take a while for large references.
      {:else if mode === "import"}
        Reading the file locally, normalizing coordinates, and preparing the selected profile.
      {:else if status?.sample_count === 0}
        Checking your local database and catalogs — import a genome whenever you’re ready.
      {:else}
        Preparing your local database and profile — this may take a moment on first launch.
      {/if}
    </p>

    <BootstrapActivityPulse
      phase={activityPhase}
      message={activityMessage}
      isError={isError || (mode === "import" && importPhase === "error")}
    />

    {#if mode === "import"}
      <div class="inner-progress-container bootstrap-motion" aria-label="DNA import progress" aria-live="polite">
        <div class="inner-progress-bar">
          <div class="inner-progress-fill" style="width: {importPercent}%"></div>
        </div>
        <div class="inner-progress-stats">
          <span>{importPercent}% complete</span>
          <span>⏱️ {formatTime(elapsedSeconds)} elapsed · {formatEta(importEtaSeconds)}</span>
        </div>
      </div>
    {:else if progressInfo}
      <div class="inner-progress-container bootstrap-motion" aria-label="Task progress">
        <div class="inner-progress-bar">
          <div class="inner-progress-fill" style="width: {progressInfo.percent}%"></div>
        </div>
        <div class="inner-progress-stats">
          <span>{progressInfo.percent}% ({progressInfo.current}/{progressInfo.total})</span>
          <span>⏱️ {formatTime(elapsedSeconds)} elapsed · {formatEta(progressInfo.etaSeconds)}</span>
        </div>
      </div>
    {:else if phase !== "ready" && phase !== "error" && elapsedSeconds > 0}
      <div class="inner-progress-stats elapsed-status bootstrap-motion">
        <span>⏱️ {formatTime(elapsedSeconds)} elapsed</span>
      </div>
    {/if}

    {#if mode === "import"}
      <ImportStepTimeline phase={importPhase} failedStep={importFailedStep} />

      {#if importPreview}
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
    {:else}
      {#if phase === "db" || (PHASE_ORDER.indexOf(phase) >= PHASE_ORDER.indexOf("db") && phase !== "ready")}
        <div class="substep-checklist">
          {#each dbSubSteps as step}
            <div class="substep-item step-{step.state}">
              <span class="substep-check">
                {#if step.state === "done"}
                  ✓
                {:else if step.state === "active"}
                  ⏳
                {:else}
                  ○
                {/if}
              </span>
              <span class="substep-label">
                {step.label}
                {#if step.details}
                  <span class="substep-details">{step.details}</span>
                {/if}
              </span>
            </div>
          {/each}
        </div>
      {/if}

      <BootstrapStepTimeline {phase} />
    {/if}

    {#if status && !isError && mode !== "import"}
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

  .inner-progress-container {
    align-self: stretch;
    width: 100%;
    max-width: none;
    margin: 0.85rem 0 0.35rem;
    opacity: 1;
    animation: none;
  }

  .inner-progress-bar {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.35rem;
  }

  .inner-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), #818cf8);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .inner-progress-stats {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 1rem;
    font-size: 0.72rem;
    color: var(--text-secondary);
    font-family: var(--font-mono), monospace;
  }

  .elapsed-status {
    justify-content: center;
    width: auto;
    margin: -0.25rem auto 1rem;
  }

  .substep-checklist {
    align-self: stretch;
    width: 100%;
    max-width: none;
    margin: 0.75rem 0 1rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    text-align: left;
    box-sizing: border-box;
    opacity: 1;
    animation: none;
  }

  .substep-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.75rem;
    transition: all 0.2s ease;
  }

  .substep-item.step-pending {
    color: var(--text-secondary);
    opacity: 0.4;
  }

  .substep-item.step-active {
    color: var(--accent);
    font-weight: 500;
  }

  .substep-item.step-done {
    color: var(--success);
  }

  .substep-check {
    font-family: var(--font-mono), monospace;
    font-size: 0.8rem;
    width: 1rem;
    text-align: center;
  }

  .substep-label {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .substep-details {
    font-size: 0.65rem;
    color: var(--text-secondary);
    opacity: 0.75;
    margin-top: 0.05rem;
    font-family: var(--font-mono), monospace;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
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
