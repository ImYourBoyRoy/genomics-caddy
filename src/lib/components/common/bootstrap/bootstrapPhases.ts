// ./src/lib/components/common/bootstrap/bootstrapPhases.ts
/*
Purpose: Bootstrap phase metadata, step labels, and in-flight ticker copy.
*/

export type BootstrapPhase =
  | "db"
  | "stats"
  | "profile"
  | "report"
  | "ready"
  | "error";

export interface BootstrapStep {
  id: BootstrapPhase;
  label: string;
}

export const BOOTSTRAP_STEPS: BootstrapStep[] = [
  { id: "db", label: "Open database & migrations" },
  { id: "stats", label: "Index genotypes & reference catalogs" },
  { id: "profile", label: "Load your genome profile" },
  { id: "report", label: "Scan markers & build report" },
];

export const DB_TICKER_MESSAGES = [
  "Opening local database…",
  "Applying schema migrations…",
  "Seeding evidence library…",
  "Syncing GWAS reference catalog…",
  "Optimizing SQLite indexes…",
  "Verifying genotype tables…",
];

export const PHASE_ORDER: BootstrapPhase[] = [
  "db",
  "stats",
  "profile",
  "report",
  "ready",
];

// Literal colors: the loader is intentionally dark and independent of the report theme.
export const PHASE_ACCENT: Record<BootstrapPhase, string> = {
  db: "#2dd4bf",
  stats: "#818cf8",
  profile: "#34d399",
  report: "#fbbf24",
  ready: "#34d399",
  error: "#f87171",
};

export interface DbProgress {
  /** Zero-based index of the active database sub-step. */
  index: number;
  total: number;
  label: string;
  detail: string | null;
}

export const DB_SUB_STEPS = [
  "Connect to database",
  "Apply schema migrations",
  "Migrate reference mappings",
  "Seed evidence library",
  "Normalize rsIDs & optimize indexes",
] as const;

/**
 * Map the native database status line (or the idle ticker copy) to a sub-step.
 * Unknown lines resolve to `null` so the caller keeps its last known position.
 */
export function describeDbProgress(message: string): DbProgress | null {
  const msg = message.trim();
  if (!msg) return null;
  const make = (index: number, detail: string | null = null): DbProgress => ({
    index,
    total: DB_SUB_STEPS.length,
    label: DB_SUB_STEPS[index],
    detail: detail && detail.length > 0 ? detail : null,
  });
  if (/^(connecting to|opening) local database/i.test(msg)) return make(0);
  if (/applying schema migrations/i.test(msg)) return make(1);
  if (/migrating reference (schema mappings|database table)/i.test(msg)) {
    return make(2, msg.replace(/^migrating reference database table:\s*/i, "").replace(/…$/, ""));
  }
  if (/seeding evidence library/i.test(msg)) {
    return make(3, msg.replace(/^seeding evidence library:?\s*/i, "").replace(/…$/, ""));
  }
  if (/normalizing rsids|vacuuming|syncing gwas|optimizing sqlite|verifying genotype/i.test(msg)) {
    return make(4, msg.replace(/…$/, ""));
  }
  return null;
}

export type StepVisualState = "pending" | "active" | "done" | "error";

export function stepState(
  phase: BootstrapPhase,
  stepId: BootstrapPhase,
  failedPhase: BootstrapPhase | null = null
): StepVisualState {
  if (phase === "error") {
    // Mark the step that was running when startup failed; earlier steps stay done.
    const failedIdx = PHASE_ORDER.indexOf(failedPhase ?? "db");
    const stepIdx = PHASE_ORDER.indexOf(stepId);
    const safeFailedIdx = failedIdx < 0 || failedIdx >= PHASE_ORDER.length - 1 ? 0 : failedIdx;
    if (stepIdx < safeFailedIdx) return "done";
    return stepIdx === safeFailedIdx ? "error" : "pending";
  }
  if (phase === "ready") return "done";
  const currentIdx = PHASE_ORDER.indexOf(phase);
  const stepIdx = PHASE_ORDER.indexOf(stepId);
  if (stepIdx < currentIdx) return "done";
  if (stepIdx === currentIdx) return "active";
  return "pending";
}

export function phaseProgress(phase: BootstrapPhase): number {
  if (phase === "error") return 0;
  if (phase === "ready") return 100;
  const idx = PHASE_ORDER.indexOf(phase);
  return Math.max(0, Math.min(100, (idx / (PHASE_ORDER.length - 1)) * 100));
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Yield so the webview can paint between bootstrap phases. */
export function yieldToUi(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}
