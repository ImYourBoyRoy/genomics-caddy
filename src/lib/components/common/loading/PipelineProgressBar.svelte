<!-- ./src/lib/components/common/loading/PipelineProgressBar.svelte -->
<script lang="ts">
  /*
  Purpose: Segmented progress bar, one segment per pipeline step. Finished
  segments are solid, the active one fills with real progress (or sweeps while
  the amount is unknown), and the readout shows the overall percentage.
  Inputs: segments, percent (0-100), label, optional caption text.
  Outputs: a single progressbar; fills animate transform only (compositor).
  */
  import type { PipelineSegment } from "./pipelineTypes";
  import "../../../styles/components/loading-telemetry.css";

  interface Props {
    segments: PipelineSegment[];
    percent: number | null;
    label: string;
    caption?: string;
    determinate?: boolean;
  }

  let { segments, percent, label, caption = "", determinate = true }: Props = $props();
</script>

<div class="lt-progress">
  <div class="lt-progress-head">
    <span class="lt-progress-caption">{caption || label}</span>
    <span class="lt-progress-percent">
      {#if percent === null}<small>working</small>{:else}{percent}<small>%</small>{/if}
    </span>
  </div>
  <div
    class="lt-bar"
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={determinate && percent !== null ? percent : undefined}
  >
    {#each segments as segment (segment.id)}
      <span
        class="lt-seg lt-seg-{segment.state}"
        class:lt-seg-indeterminate={segment.state === "active" && segment.fraction === null}
        style="--seg-accent: {segment.accent}; --seg-fill: {segment.fraction ?? 0}"
      >
        <i class="lt-seg-fill"></i>
      </span>
    {/each}
  </div>
</div>
