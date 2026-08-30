<!-- ./src/lib/components/common/AppBootstrapScreen.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus } from "../../types/genomics";
  import { PHASE_ORDER, type BootstrapPhase } from "./bootstrap/bootstrapPhases";
  import BootstrapHelixBackdrop from "./bootstrap/BootstrapHelixBackdrop.svelte";
  import BootstrapPhaseOrb from "./bootstrap/BootstrapPhaseOrb.svelte";
  import BootstrapActivityPulse from "./bootstrap/BootstrapActivityPulse.svelte";
  import BootstrapStepTimeline from "./bootstrap/BootstrapStepTimeline.svelte";
  import BootstrapStatReveal from "./bootstrap/BootstrapStatReveal.svelte";
  import "../../styles/bootstrap-animations.css";

  export type { BootstrapPhase };

  interface Props {
    phase: BootstrapPhase;
    message: string;
    status?: AppBootstrapStatus | null;
    error?: string;
  }

  let { phase, message, status = null, error = "" }: Props = $props();

  let isError = $derived(phase === "error");
  let isReady = $derived(phase === "ready");

  /** Optional (n of m) progress from status messages — no elapsed timer. */
  interface ProgressEstimate {
    percent: number;
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
    return {
      percent: Math.round((current / total) * 100),
      current,
      total,
    };
  });

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
  <BootstrapHelixBackdrop {phase} />

  <div class="bootstrap-content">
    <BootstrapPhaseOrb {phase} />

    <h1 class="title bootstrap-motion">
      {#if isReady}
        Genomics Caddy Ready
      {:else if isError}
        Startup Interrupted
      {:else}
        Starting Genomics Caddy
      {/if}
    </h1>

    <p class="lead bootstrap-motion">
      {#if isReady}
        Your local genome workspace is loaded and waiting.
      {:else if isError}
        Check the database path and try restarting the app.
      {:else}
        Preparing your local database and profile — this may take a moment on first launch.
      {/if}
    </p>

    <BootstrapActivityPulse {phase} message={isError ? error || "Startup failed" : message} {isError} />

    {#if progressInfo}
      <div class="inner-progress-container bootstrap-motion" aria-label="Task progress">
        <div class="inner-progress-bar">
          <div class="inner-progress-fill" style="width: {progressInfo.percent}%"></div>
        </div>
        <div class="inner-progress-stats">
          <span>{progressInfo.percent}% ({progressInfo.current}/{progressInfo.total})</span>
        </div>
      </div>
    {/if}

    {#if phase === "db" || (PHASE_ORDER.indexOf(phase) >= PHASE_ORDER.indexOf("db") && phase !== "ready")}
      <div class="substep-checklist bootstrap-motion">
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

    {#if status && !isError}
      <BootstrapStatReveal {status} {phase} />
    {/if}
  </div>
</div>

<style>
  .bootstrap-screen {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 100%;
    padding: 2.5rem 1.5rem;
    text-align: center;
    overflow: hidden;
    isolation: isolate;
    box-sizing: border-box;
  }

  .bootstrap-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 640px;
    padding: 2rem 2.25rem 1.75rem;
    border: 1px solid var(--border-color);
    border-radius: 20px;
    background: rgba(8, 12, 24, 0.72);
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.36), inset 0 1px 0 rgba(255, 255, 255, 0.05);
    backdrop-filter: blur(18px);
    box-sizing: border-box;
    animation: bootstrap-fade-up 0.65s ease both;
  }

  .title {
    font-size: 1.35rem;
    margin: 0;
    color: var(--text-primary);
    animation: bootstrap-fade-up 0.7s ease both 0.05s;
  }

  .lead {
    max-width: 520px;
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin: 0.35rem 0 0.25rem;
    animation: bootstrap-fade-up 0.7s ease both 0.1s;
  }

  .bootstrap-screen.ready .bootstrap-content {
    animation: bootstrap-ready-glow 1.8s ease-in-out 1;
  }

  .inner-progress-container {
    width: 100%;
    max-width: 440px;
    margin: -0.25rem auto 1rem;
    animation: bootstrap-fade-up 0.55s ease both;
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
    justify-content: space-between;
    font-size: 0.72rem;
    color: var(--text-secondary);
    font-family: var(--font-mono), monospace;
  }

  .substep-checklist {
    width: 100%;
    max-width: 440px;
    margin: 0.5rem auto 1.5rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    text-align: left;
    box-sizing: border-box;
    animation: bootstrap-fade-up 0.55s ease both;
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
</style>
