// ./src/lib/components/common/loading/pipelineTypes.ts
/*
Purpose: Shared view-model for the loading telemetry components
(PipelineStepList, PipelineProgressBar, LoadingTelemetryStrip) so startup,
resource sync, and DNA import all describe their progress the same way.
*/

export type PipelineStepState = "pending" | "active" | "done" | "error";

export interface PipelineStep {
  id: string;
  label: string;
  state: PipelineStepState;
  /** CSS color for the step's node, rail, and progress segment. */
  accent: string;
  /** Live one-line activity shown only while the step is active (or the error text). */
  detail?: string | null;
  /** Short right-aligned readout while active, e.g. `42%` or `2 / 5`. */
  meta?: string | null;
  /** 0-1 progress inside an active step; null/undefined means indeterminate. */
  fraction?: number | null;
  /** Final duration, shown as a chip once the step is done. */
  durationMs?: number | null;
  /** Live elapsed time of the active step. */
  elapsedMs?: number | null;
}

export interface PipelineSegment {
  id: string;
  state: PipelineStepState;
  accent: string;
  fraction: number | null;
}

export function segmentsFromSteps(steps: PipelineStep[]): PipelineSegment[] {
  return steps.map((step) => ({
    id: step.id,
    state: step.state,
    accent: step.accent,
    fraction:
      step.state === "done"
        ? 1
        : step.state === "active"
          ? typeof step.fraction === "number"
            ? step.fraction
            : null
          : 0,
  }));
}

/** Overall 0-100 across equal-weight steps; active steps contribute their fraction. */
export function overallPercent(steps: PipelineStep[]): number {
  if (steps.length === 0) return 0;
  const total = segmentsFromSteps(steps).reduce(
    (sum, segment) => sum + (segment.fraction ?? (segment.state === "active" ? 0.15 : 0)),
    0,
  );
  return Math.round(Math.min(1, total / steps.length) * 100);
}
