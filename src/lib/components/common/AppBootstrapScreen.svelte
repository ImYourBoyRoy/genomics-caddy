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
</style>
