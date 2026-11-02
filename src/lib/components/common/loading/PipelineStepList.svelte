<!-- ./src/lib/components/common/loading/PipelineStepList.svelte -->
<script lang="ts">
  /*
  Purpose: Vertical step list for loading surfaces. Each step shows a state
  node (hollow, spinning arc, drawn check, or cross), its label, a duration chip
  once finished, and - only while active - a live detail line with an optional
  inline progress bar. Expanding and collapsing the detail row uses a grid-row
  transition so swapping the active step never causes a layout jump.
  Inputs: steps (see pipelineTypes), ariaLabel.
  Outputs: an ordered list; `aria-current="step"` marks the active row.
  */
  import type { PipelineStep } from "./pipelineTypes";
  import { formatStepDuration, clampFraction } from "../../../utils/loadingTelemetry";
  import "../../../styles/components/loading-telemetry.css";
  import "../../../styles/components/pipeline-steps.css";

  interface Props {
    steps: PipelineStep[];
    ariaLabel: string;
  }

  let { steps, ariaLabel }: Props = $props();

  const stateWord: Record<PipelineStep["state"], string> = {
    pending: "waiting",
    active: "in progress",
    done: "complete",
    error: "needs attention",
  };
</script>

<ol class="ps-list" aria-label={ariaLabel}>
  {#each steps as step, index (step.id)}
    <li
      class="ps-step ps-{step.state}"
      class:ps-last={index === steps.length - 1}
      style="--step-accent: {step.accent}"
      aria-current={step.state === "active" ? "step" : undefined}
    >
      <span class="ps-rail" aria-hidden="true">
        <i class="ps-rail-fill"></i>
        {#if step.state === "active"}<i class="ps-rail-flow"></i>{/if}
      </span>

      <span class="ps-node" aria-hidden="true">
        <svg viewBox="0 0 24 24">
          <circle class="ps-node-ring" cx="12" cy="12" r="8.5" />
          {#if step.state === "active"}
            <circle class="ps-node-arc" cx="12" cy="12" r="8.5" />
            <circle class="ps-node-core" cx="12" cy="12" r="3" />
          {:else if step.state === "done"}
            <circle class="ps-node-fill" cx="12" cy="12" r="8.5" />
            <path class="ps-node-check" d="M7.8 12.4l2.9 2.9 5.6-6.1" />
          {:else if step.state === "error"}
            <circle class="ps-node-fill" cx="12" cy="12" r="8.5" />
            <path class="ps-node-cross" d="M9 9l6 6M15 9l-6 6" />
          {/if}
        </svg>
      </span>

      <div class="ps-body">
        <div class="ps-head">
          <span class="ps-label">{step.label}</span>
          <span class="ps-sr">{stateWord[step.state]}</span>
          {#if step.state === "done" && step.durationMs != null}
            <span class="ps-chip ps-chip-done">{formatStepDuration(step.durationMs)}</span>
          {:else if step.state === "active"}
            {#if step.meta}<span class="ps-meta">{step.meta}</span>{/if}
            {#if step.elapsedMs != null}
              <span class="ps-chip ps-chip-live">{formatStepDuration(step.elapsedMs)}</span>
            {/if}
          {:else if step.state === "error"}
            <span class="ps-chip ps-chip-error">stopped</span>
          {/if}
        </div>

        <div class="ps-reveal" class:ps-open={(step.state === "active" || step.state === "error") && !!step.detail}>
          <div class="ps-reveal-inner">
            <p class="ps-detail" role={step.state === "active" ? "status" : undefined}>{step.detail ?? ""}</p>
            <span class="ps-mini" class:ps-mini-indeterminate={step.state === "active" && step.fraction == null}>
              <i style="--mini-fill: {clampFraction(step.fraction ?? 0)}"></i>
            </span>
          </div>
        </div>
      </div>
    </li>
  {/each}
</ol>
