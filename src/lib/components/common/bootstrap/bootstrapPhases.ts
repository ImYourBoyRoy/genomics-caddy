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
  icon: string;
}

export const BOOTSTRAP_STEPS: BootstrapStep[] = [
  { id: "db", label: "Open database & migrations", icon: "🗄️" },
  { id: "stats", label: "Load genotype & reference counts", icon: "📊" },
  { id: "profile", label: "Select genome profile", icon: "🧬" },
  { id: "report", label: "Generate trait report", icon: "📋" },
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

export const PHASE_ACCENT: Record<BootstrapPhase, string> = {
  db: "var(--accent)",
  stats: "#818cf8",
  profile: "#34d399",
  report: "#fbbf24",
  ready: "var(--success)",
  error: "var(--danger)",
};

export type StepVisualState = "pending" | "active" | "done" | "error";

export function stepState(
  phase: BootstrapPhase,
  stepId: BootstrapPhase
): StepVisualState {
  if (phase === "error") {
    return stepId === "db" ? "error" : "pending";
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
