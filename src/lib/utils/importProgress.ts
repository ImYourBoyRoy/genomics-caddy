/**
 * Import progress vocabulary shared by the desktop import orchestrator and
 * its full-screen pending view.
 *
 * The native importer intentionally reports human-readable status strings so
 * the same event stream can remain useful in the compact sidebar and in the
 * detailed loading waterfall. Keep this mapping tolerant of small wording
 * changes while preserving a deterministic visual sequence.
 */

export type ImportPhase =
  | "idle"
  | "preview"
  | "awaiting-confirmation"
  | "reading"
  | "parsing"
  | "liftover"
  | "ingesting"
  | "indexing"
  | "profile"
  | "report"
  | "ready"
  | "error";

export type ImportStepId =
  | "validate"
  | "parse"
  | "liftover"
  | "store"
  | "index"
  | "profile"
  | "report";

export type ImportStepVisualState = "pending" | "active" | "done" | "error";

export interface ImportStep {
  id: ImportStepId;
  label: string;
  icon: string;
}

export const IMPORT_STEPS: ImportStep[] = [
  { id: "validate", label: "Validate DNA export", icon: "✓" },
  { id: "parse", label: "Read and parse records", icon: "📄" },
  { id: "liftover", label: "Normalize genome coordinates", icon: "↔" },
  { id: "store", label: "Store genotype data", icon: "🗄️" },
  { id: "index", label: "Build variant placement index", icon: "⌕" },
  { id: "profile", label: "Load imported profile", icon: "🧬" },
  { id: "report", label: "Prepare profile report", icon: "📋" },
];

const STEP_ORDER: ImportStepId[] = IMPORT_STEPS.map((step) => step.id);

function activeStepForPhase(phase: ImportPhase): ImportStepId | null {
  switch (phase) {
    case "preview":
      return "validate";
    case "awaiting-confirmation":
      return null;
    case "reading":
    case "parsing":
      return "parse";
    case "liftover":
      return "liftover";
    case "ingesting":
      return "store";
    case "indexing":
      return "index";
    case "profile":
      return "profile";
    case "report":
      return "report";
    default:
      return null;
  }
}

export function importStepForPhase(phase: ImportPhase): ImportStepId | null {
  return activeStepForPhase(phase);
}

export function importStepState(
  phase: ImportPhase,
  stepId: ImportStepId,
  failedStep: ImportStepId | null = null,
): ImportStepVisualState {
  if (phase === "ready") return "done";

  // Validation is complete while the user reviews the preview. No import
  // work is running until the explicit confirmation is accepted.
  if (phase === "awaiting-confirmation") {
    return stepId === "validate" ? "done" : "pending";
  }

  const activeStep = activeStepForPhase(phase);
  if (phase === "error") {
    const failedIndex = failedStep === null ? -1 : STEP_ORDER.indexOf(failedStep);
    const stepIndex = STEP_ORDER.indexOf(stepId);
    if (failedIndex >= 0 && stepIndex < failedIndex) return "done";
    if (failedIndex >= 0 && stepIndex === failedIndex) return "error";
    return "pending";
  }

  if (activeStep === null) return "pending";
  const activeIndex = STEP_ORDER.indexOf(activeStep);
  const stepIndex = STEP_ORDER.indexOf(stepId);
  if (stepIndex < activeIndex) return "done";
  if (stepIndex === activeIndex) return "active";
  return "pending";
}

export function importTimelineProgress(phase: ImportPhase): number {
  if (phase === "ready") return 100;
  if (phase === "error") return 0;
  if (phase === "awaiting-confirmation") return Math.round(100 / STEP_ORDER.length);
  const activeStep = activeStepForPhase(phase);
  if (activeStep === null) return 0;
  const index = STEP_ORDER.indexOf(activeStep);
  // Keep the active step visibly in progress; 100% belongs to the explicit
  // ready checkpoint after the profile report has loaded.
  return Math.round((index / STEP_ORDER.length) * 100);
}

/** Convert native progress copy into the visual phase used by the waterfall. */
export function importPhaseForProgress(status: string, percentage: number): ImportPhase {
  const normalized = status.toLowerCase();

  if (normalized.includes("building variant placement")) return "indexing";
  if (normalized.includes("committing") || normalized.includes("ingesting")) return "ingesting";
  if (normalized.includes("liftover") || normalized.includes("coordinate")) return "liftover";
  if (normalized.includes("parsing") || normalized.includes("unpacking")) return "parsing";
  if (normalized.includes("reading") || normalized.includes("loading file")) return "reading";

  if (percentage >= 95) return "indexing";
  if (percentage >= 45) return "liftover";
  if (percentage >= 15) return "parsing";
  if (percentage > 0) return "reading";
  return "preview";
}
