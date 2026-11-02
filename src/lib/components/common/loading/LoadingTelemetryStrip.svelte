<!-- ./src/lib/components/common/loading/LoadingTelemetryStrip.svelte -->
<script lang="ts">
  /*
  Purpose: Three-cell readout under the progress bar: a stopwatch (elapsed),
  the current step, and a time-remaining estimate. The estimate shows
  "Measuring" until the estimator has enough signal and never fabricates one.
  Inputs: elapsedMs, running/failed/done flags, step index/count/label, etaSeconds.
  Outputs: markup only; the dial hand sweeps via a CSS animation anchored once
  to the starting offset so it stays smooth between 250 ms clock ticks.
  */
  import { untrack } from "svelte";
  import { formatClock, formatRemaining } from "../../../utils/loadingTelemetry";
  import "../../../styles/components/loading-telemetry.css";

  interface Props {
    elapsedMs: number;
    stepIndex: number;
    /** Null when the total is unknown; the strip then shows the index alone. */
    stepCount: number | null;
    stepTitle?: string;
    stepLabel: string;
    etaSeconds: number | null;
    /** Overall status drives the right-hand cell wording. */
    status?: "running" | "done" | "error";
    etaCaption?: string;
    /** The total is not known (row-index imports); show a dash instead of "measuring". */
    etaUnknown?: boolean;
  }

  let {
    elapsedMs,
    stepIndex,
    stepCount,
    stepTitle = "Step",
    stepLabel,
    etaSeconds,
    status = "running",
    etaCaption = "left in this step",
    etaUnknown = false,
  }: Props = $props();

  // Anchor the dial once; later ticks must not restart the CSS animation.
  const dialOffsetMs = untrack(() => elapsedMs % 60_000);
  const clockText = $derived(formatClock(elapsedMs));
</script>

<dl class="lt-strip" class:lt-strip-done={status === "done"} class:lt-strip-error={status === "error"}>
  <div class="lt-cell">
    <dt>Elapsed</dt>
    <dd class="lt-cell-elapsed">
      <svg class="lt-dial" viewBox="0 0 24 24" aria-hidden="true" class:lt-dial-paused={status !== "running"}>
        <circle class="lt-dial-ring" cx="12" cy="12" r="9.5" />
        <path class="lt-dial-ticks" d="M12 3.5v1.6M20.5 12h-1.6M12 20.5v-1.6M3.5 12h1.6" />
        <g class="lt-dial-hand" style="animation-delay: -{dialOffsetMs}ms">
          <path d="M12 12V5.6" />
        </g>
        <circle class="lt-dial-pin" cx="12" cy="12" r="1.4" />
      </svg>
      <span class="lt-value" aria-label="{clockText} elapsed">{clockText}</span>
    </dd>
  </div>

  <div class="lt-cell">
    <dt>{stepTitle}</dt>
    <dd>
      <span class="lt-value">{stepIndex}{#if stepCount !== null}<span class="lt-value-dim">{" / "}{stepCount}</span>{/if}</span>
      <span class="lt-sub" title={stepLabel}>{stepLabel}</span>
    </dd>
  </div>

  <div class="lt-cell">
    <dt>Remaining</dt>
    <dd>
      {#if status === "done"}
        <span class="lt-value lt-value-ok">Done</span>
        <span class="lt-sub">all steps complete</span>
      {:else if status === "error"}
        <span class="lt-value lt-value-bad">Stopped</span>
        <span class="lt-sub">see details below</span>
      {:else if etaSeconds !== null}
        <span class="lt-value">{formatRemaining(etaSeconds)}</span>
        <span class="lt-sub">{etaCaption}</span>
      {:else if etaUnknown}
        <span class="lt-value lt-value-dim" aria-label="Remaining time unknown">—</span>
        <span class="lt-sub">total not reported</span>
      {:else}
        <span class="lt-value lt-value-measuring" aria-label="Measuring remaining time">
          <span class="lt-dots" aria-hidden="true"><i></i><i></i><i></i></span>
        </span>
        <span class="lt-sub">measuring pace</span>
      {/if}
    </dd>
  </div>
</dl>
