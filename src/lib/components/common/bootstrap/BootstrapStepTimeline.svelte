<!-- ./src/lib/components/common/bootstrap/BootstrapStepTimeline.svelte -->
<script lang="ts">
  import {
    BOOTSTRAP_STEPS,
    phaseProgress,
    stepState,
    type BootstrapPhase,
  } from "./bootstrapPhases";

  interface Props {
    phase: BootstrapPhase;
  }

  let { phase }: Props = $props();

  let progress = $derived(phaseProgress(phase));
</script>

<ol class="step-timeline">
  <div
    class="timeline-track bootstrap-motion"
    style="--progress: {progress}%"
    aria-hidden="true"
  >
    <div class="timeline-fill bootstrap-motion"></div>
  </div>

  {#each BOOTSTRAP_STEPS as step, index}
    {@const state = stepState(phase, step.id)}
    <li
      class="step step-{state} bootstrap-motion"
      style="animation-delay: {index * 0.08}s"
    >
      <span class="step-node" aria-hidden="true">
        {#if state === "done"}
          <span class="node-check bootstrap-motion">✓</span>
        {:else if state === "error"}
          <span class="node-error">!</span>
        {:else if state === "active"}
          <span class="node-active bootstrap-motion"></span>
        {:else}
          <span class="node-pending"></span>
        {/if}
      </span>
      <span class="step-copy">
        <span class="step-icon">{step.icon}</span>
        <span class="step-label">{step.label}</span>
      </span>
    </li>
  {/each}
</ol>

<style>
  .step-timeline {
    position: relative;
    list-style: none;
    padding: 0 0 0 14px;
    margin: 0 0 1rem;
    max-width: 440px;
    width: 100%;
    text-align: left;
  }

  .timeline-track {
    position: absolute;
    left: 22px;
    top: 8px;
    bottom: 8px;
    width: 2px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
    overflow: hidden;
    transform-origin: top;
  }

  .timeline-fill {
    width: 100%;
    height: var(--progress);
    background: linear-gradient(
      180deg,
      var(--accent),
      color-mix(in srgb, var(--success) 70%, var(--accent))
    );
    border-radius: 2px;
    transition: height 0.55s cubic-bezier(0.22, 1, 0.36, 1);
    box-shadow: 0 0 12px var(--bootstrap-glow-soft);
  }

  .step {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 0;
    animation: bootstrap-fade-up 0.55s ease both;
  }

  .step-node {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    z-index: 1;
  }

  .node-pending {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid var(--border-color);
  }

  .node-active {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 14px var(--bootstrap-glow);
    animation: bootstrap-orb-pulse 1.1s ease-in-out infinite;
  }

  .node-check {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(16, 185, 129, 0.18);
    color: #86efac;
    font-size: 0.65rem;
    display: grid;
    place-items: center;
    animation: bootstrap-step-pop 0.45s ease both;
    border: 1px solid rgba(16, 185, 129, 0.35);
  }

  .node-error {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(239, 68, 68, 0.18);
    color: #fca5a5;
    font-size: 0.7rem;
    font-weight: 700;
    display: grid;
    place-items: center;
  }

  .step-copy {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82rem;
    color: var(--text-secondary);
    transition: color 0.35s ease, transform 0.35s ease;
  }

  .step-active .step-copy {
    color: var(--text-primary);
    font-weight: 600;
    transform: translateX(2px);
  }

  .step-done .step-copy {
    color: #86efac;
  }

  .step-error .step-copy {
    color: #fca5a5;
  }

  .step-icon {
    font-size: 0.95rem;
    line-height: 1;
  }
</style>
