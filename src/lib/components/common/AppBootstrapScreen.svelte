<!-- ./src/lib/components/common/AppBootstrapScreen.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus } from "../../types/genomics";
  import type { BootstrapPhase } from "./bootstrap/bootstrapPhases";
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

  let startTime = $state(Date.now());
  let elapsedSeconds = $state(0);

  $effect(() => {
    if (phase === "ready" || phase === "error") return;
    const timer = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
    }, 1000);
    return () => clearInterval(timer);
  });

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

    const percent = Math.round((current / total) * 100);
    let etaSeconds: number | null = null;
    if (current > 0 && elapsedSeconds > 0) {
      const secPerItem = elapsedSeconds / current;
      etaSeconds = Math.round((total - current) * secPerItem);
    }
    return { percent, etaSeconds, current, total };
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
          <span>⏱️ {formatTime(elapsedSeconds)} elapsed • {formatEta(progressInfo.etaSeconds)}</span>
        </div>
      </div>
    {:else if phase !== "ready" && phase !== "error" && elapsedSeconds > 0}
      <div class="inner-progress-stats bootstrap-motion" style="margin: -0.25rem auto 1rem; justify-content: center; width: auto; font-size: 0.72rem; font-family: var(--font-mono), monospace;">
        <span>⏱️ {formatTime(elapsedSeconds)} elapsed</span>
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
</style>
