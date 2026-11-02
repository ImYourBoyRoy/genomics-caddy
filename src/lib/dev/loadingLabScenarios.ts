// ./src/lib/dev/loadingLabScenarios.ts
/*
Purpose: Scripted loader states for the permanent /__loading-lab route and its
unit tests. These fixtures never touch real genotype files or network downloads;
they only exercise AppBootstrapScreen / BootstrapOverlay with honest telemetry
shapes the production reducers already understand.
Usage: import { loadingLabScenario, LOADING_LAB_SCENARIO_IDS } from this module.
Dev only: the route that consumes this is gated by import.meta.env.DEV.
*/

import type { ReportGenerationProgress } from "../types/genomics";
import type { BootstrapPhase } from "../components/common/bootstrap/bootstrapPhases";
import type { ImportPhase } from "../utils/importProgress";
import {
  emptyResourceSync,
  reduceResourceSync,
  type ResourceSyncEvent,
  type ResourceSyncTelemetry,
} from "../utils/resourceSyncTelemetry";

export type LoadingLabMode = "startup" | "resources" | "import";

export interface LoadingLabScenario {
  id: string;
  label: string;
  mode: LoadingLabMode;
  phase: BootstrapPhase;
  message: string;
  resourceSync: ResourceSyncTelemetry | null;
  reportProgress: ReportGenerationProgress | null;
  importPhase: ImportPhase;
  importProgress: { percentage: number; status: string } | null;
  importProfileName: string;
  showWorkspaceAction: boolean;
}

const ph = (
  assetId: string,
  label: string,
  phase: string,
  state?: string,
): ResourceSyncEvent => ({
  type: "phase",
  assetId,
  label,
  phase,
  state,
  message: `${phase} ${label}`,
});

const dl = (
  assetId: string,
  label: string,
  done: number,
  total: number,
  at: number,
  resumed = 0,
): ResourceSyncEvent => ({
  type: "download",
  assetId,
  label,
  bytesDone: done,
  totalBytes: total,
  resumedBytes: resumed,
  at,
});

const tier: ResourceSyncEvent = {
  type: "phase",
  assetId: "__bulk__",
  phase: "bulk",
  message: "Sync · preparing tier 1 (6 asset(s))…",
  tier: 1,
  assetCount: 6,
};

const downloadTrail: ResourceSyncEvent[] = [
  tier,
  ph("a", "ClinGen gene validity", "check"),
  ph("a", "ClinGen gene validity", "skip"),
  ph("b", "PharmGKB clinical variants", "check"),
  ph("b", "PharmGKB clinical variants", "download"),
  dl("b", "PharmGKB clinical variants", 2_000_000, 9_000_000, 1_000),
  dl("b", "PharmGKB clinical variants", 9_000_000, 9_000_000, 2_000),
  ph("c", "ClinVar variant summary", "check"),
  ph("c", "ClinVar variant summary", "download"),
];

function fold(events: ResourceSyncEvent[]): ResourceSyncTelemetry {
  return events.reduce(reduceResourceSync, emptyResourceSync());
}

const SCENARIOS: Record<string, LoadingLabScenario> = {
  "res-scan": {
    id: "res-scan",
    label: "Reference sync · scanning",
    mode: "resources",
    phase: "stats",
    message: "Checking ClinGen gene validity…",
    resourceSync: fold([tier, ph("a", "ClinGen gene validity", "check")]),
    reportProgress: null,
    importPhase: "idle",
    importProgress: null,
    importProfileName: "",
    showWorkspaceAction: true,
  },
  "res-download": {
    id: "res-download",
    label: "Reference sync · downloading",
    mode: "resources",
    phase: "stats",
    message: "Step 1/2 · Downloading ClinVar variant summary…",
    resourceSync: fold([
      ...downloadTrail,
      dl("c", "ClinVar variant summary", 40_000_000, 410_000_000, 3_000, 10_000_000),
      dl("c", "ClinVar variant summary", 90_000_000, 410_000_000, 5_000, 10_000_000),
    ]),
    reportProgress: null,
    importPhase: "idle",
    importProgress: null,
    importProfileName: "",
    showWorkspaceAction: true,
  },
  "res-index": {
    id: "res-index",
    label: "Reference sync · indexing",
    mode: "resources",
    phase: "stats",
    message: "Indexing ClinVar · 1240000 rows",
    resourceSync: fold([
      ...downloadTrail,
      dl("c", "ClinVar variant summary", 410_000_000, 410_000_000, 5_000),
      ph("c", "ClinVar variant summary", "import", "running"),
      {
        type: "import",
        assetId: "c",
        rowsProcessed: 1_240_000,
        rowsPerSecond: 88_000,
        percent: null,
        etaSeconds: null,
        message: "Indexing ClinVar · 1240000 rows",
      },
    ]),
    reportProgress: null,
    importPhase: "idle",
    importProgress: null,
    importProfileName: "",
    showWorkspaceAction: true,
  },
  startup: {
    id: "startup",
    label: "App startup · report build",
    mode: "startup",
    phase: "report",
    message: "Scanning markers",
    resourceSync: null,
    reportProgress: {
      sampleId: 1,
      phase: "markers",
      status: "Scanning markers",
      current: 320,
      total: 900,
      matches: 41,
      matchesLabel: "matches",
      elapsedMs: 9000,
      phaseElapsedMs: 4000,
    },
    importPhase: "idle",
    importProgress: null,
    importProfileName: "",
    showWorkspaceAction: false,
  },
  import: {
    id: "import",
    label: "DNA import · storing genotypes",
    mode: "import",
    phase: "stats",
    message: "Storing genotype data · 412,300 rows",
    resourceSync: null,
    reportProgress: null,
    importPhase: "ingesting",
    importProgress: { percentage: 54, status: "Storing genotype data · 412,300 rows" },
    importProfileName: "Demo profile",
    showWorkspaceAction: false,
  },
};

export const LOADING_LAB_SCENARIO_IDS = Object.keys(SCENARIOS);

export const LOADING_LAB_DEFAULT_SCENARIO = "res-download";

export function loadingLabScenario(id: string | null | undefined): LoadingLabScenario {
  if (id && SCENARIOS[id]) return SCENARIOS[id];
  return SCENARIOS[LOADING_LAB_DEFAULT_SCENARIO];
}
