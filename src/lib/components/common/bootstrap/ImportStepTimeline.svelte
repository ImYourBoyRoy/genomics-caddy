<!-- ./src/lib/components/common/bootstrap/ImportStepTimeline.svelte -->
<script lang="ts">
  import {
    IMPORT_STEPS,
    importStepState,
    importTimelineProgress,
    type ImportPhase,
    type ImportStepId,
  } from "../../../utils/importProgress";

  interface Props {
    phase: ImportPhase;
    failedStep?: ImportStepId | null;
  }

  let { phase, failedStep = null }: Props = $props();
  let progress = $derived(importTimelineProgress(phase));
</script>

<div class="import-timeline-wrap">
  <div
    class="import-timeline-track bootstrap-motion"
    style="--progress: {progress}%"
    aria-hidden="true"
  >
    <div class="import-timeline-fill bootstrap-motion"></div>
  </div>

  <ol class="import-step-timeline" aria-label="DNA import progress">
    {#each IMPORT_STEPS as step, index}
      {@const state = importStepState(phase, step.id, failedStep)}
      <li
        class="import-step import-step-{state} bootstrap-motion"
        style="animation-delay: {index * 0.07}s"
      >
        <span class="import-step-node" aria-hidden="true">
          {#if state === "done"}
            <span class="import-node-check bootstrap-motion">✓</span>
          {:else if state === "error"}
            <span class="import-node-error">!</span>
          {:else if state === "active"}
            <span class="import-node-active bootstrap-motion"></span>
          {:else}
            <span class="import-node-pending"></span>
          {/if}
        </span>
        <span class="import-step-copy">
          <span class="import-step-icon" aria-hidden="true">{step.icon}</span>
          <span class="import-step-label">{step.label}</span>
          {#if state === "active"}
            <span class="import-step-state">in progress</span>
          {:else if state === "done"}
            <span class="import-step-state">complete</span>
          {:else if state === "error"}
            <span class="import-step-state">needs attention</span>
          {/if}
        </span>
      </li>
    {/each}
  </ol>
</div>

<style>
  .import-timeline-wrap {
    position: relative;
    width: 100%;
    max-width: 440px;
  }

  .import-step-timeline {
    list-style: none;
    padding: 0 0 0 14px;
    margin: 0 0 1rem;
    width: 100%;
    text-align: left;
  }

  .import-timeline-track {
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

  .import-timeline-fill {
    width: 100%;
    height: var(--progress);
    background: linear-gradient(180deg, var(--accent), var(--success));
    border-radius: 2px;
    transition: height 0.55s cubic-bezier(0.22, 1, 0.36, 1);
    box-shadow: 0 0 12px var(--bootstrap-glow-soft);
  }

  .import-step {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 30px;
    padding: 5px 0;
  }

  .import-step-node {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    z-index: 1;
  }

  .import-node-pending {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid var(--border-color);
  }

  .import-node-active {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 14px var(--bootstrap-glow);
    animation: bootstrap-orb-pulse 1.1s ease-in-out infinite;
  }

  .import-node-check,
  .import-node-error {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 0.65rem;
    font-weight: 700;
  }

  .import-node-check {
    background: rgba(16, 185, 129, 0.18);
    color: #86efac;
    border: 1px solid rgba(16, 185, 129, 0.35);
    animation: bootstrap-step-pop 0.45s ease both;
  }

  .import-node-error {
    background: rgba(239, 68, 68, 0.18);
    color: #fca5a5;
    border: 1px solid rgba(239, 68, 68, 0.35);
  }

  .import-step-copy {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    color: var(--text-secondary);
    transition: color 0.35s ease, transform 0.35s ease;
  }

  .import-step-active .import-step-copy {
    color: var(--text-primary);
    font-weight: 600;
    transform: translateX(2px);
  }

  .import-step-done .import-step-copy {
    color: #86efac;
  }

  .import-step-error .import-step-copy {
    color: #fca5a5;
  }

  .import-step-icon {
    width: 1.1rem;
    flex-shrink: 0;
    font-size: 0.85rem;
    line-height: 1;
    text-align: center;
  }

  .import-step-label {
    font-size: 0.78rem;
  }

  .import-step-state {
    margin-left: auto;
    flex-shrink: 0;
    color: var(--text-secondary);
    font-family: var(--font-mono), monospace;
    font-size: 0.62rem;
    font-weight: 500;
    opacity: 0.78;
  }

  .import-step-error .import-step-state {
    color: #fca5a5;
  }

  @media (max-width: 520px) {
    .import-step-state {
      display: none;
    }
  }
</style>
