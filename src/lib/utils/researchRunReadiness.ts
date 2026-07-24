// ./src/lib/utils/researchRunReadiness.ts
/**
 * Central readiness rules for vector research sweep actions.
 * Keeps button enable/disable and status hints consistent across Run Controller UI.
 */

import type {
  ConnectionActivity,
  QdrantConnectionStatus,
  ResearchJob,
  ResearchScopePreview,
} from "../types/research";

export type OllamaConnectionState = "untested" | "testing" | "live" | "dead";

export interface RunReadinessInput {
  configUrl?: string;
  connectionStatus: QdrantConnectionStatus | null;
  connectionActivity: ConnectionActivity;
  ollamaStatus: OllamaConnectionState;
  gnomadSweepReady: boolean;
  gnomadSourceEnabled: boolean;
  scopePreview: ResearchScopePreview | null;
  scopePreviewLoading: boolean;
  job: ResearchJob | null;
  /** When true, block start until collection is re-embedded with the configured model. */
  embeddingModelMismatch?: boolean;
  indexEmbeddingModel?: string | null;
  configuredEmbeddingModel?: string | null;
}

export interface RunReadiness {
  isConnectionPending: boolean;
  qdrantReady: boolean;
  ollamaReady: boolean;
  scopeHasMarkers: boolean;
  sweepActive: boolean;
  sweepPausing: boolean;
  sweepInterrupted: boolean;
  fullyPaused: boolean;
  canStart: boolean;
  canResume: boolean;
  canPause: boolean;
  canCancel: boolean;
  /** Primary hint under action buttons (blocker or status). */
  primaryHint: string | null;
  /** Secondary warning (gnomAD, scope empty, etc.). */
  secondaryHint: string | null;
}

function connectionPending(activity: ConnectionActivity): boolean {
  return (
    activity.phase === "checking-qdrant" ||
    activity.phase === "checking-ollama" ||
    activity.phase === "loading-scope"
  );
}

export function computeRunReadiness(input: RunReadinessInput): RunReadiness {
  const {
    configUrl,
    connectionStatus,
    connectionActivity,
    ollamaStatus,
    gnomadSweepReady,
    gnomadSourceEnabled,
    scopePreview,
    scopePreviewLoading,
    job,
    embeddingModelMismatch = false,
    indexEmbeddingModel = null,
    configuredEmbeddingModel = null,
  } = input;

  const isConnectionPending =
    connectionPending(connectionActivity) || ollamaStatus === "testing";

  const qdrantReady = !!(
    connectionStatus?.success && connectionStatus.collection_exists
  );

  const ollamaReady = ollamaStatus === "live";

  const scopeHasMarkers = (scopePreview?.total_unique ?? 0) > 0;

  const sweepActive =
    job?.loop_active === true ||
    (job?.status === "running" && job.loop_active !== false);
  const sweepPausing = job?.status === "paused" && job.loop_active === true;
  const sweepInterrupted = job?.status === "running" && job.loop_active === false;
  const fullyPaused = job?.status === "paused" && job.loop_active !== true;
  // Cancel persists idle immediately while the cooperative loop may still be winding down.
  const sweepWindingDown =
    job?.status === "idle" &&
    job.loop_active === true &&
    (job.error_message ?? "").toLowerCase().includes("cancel");

  const infrastructureReady =
    !!configUrl?.trim() &&
    !isConnectionPending &&
    qdrantReady &&
    ollamaReady &&
    !embeddingModelMismatch &&
    (!gnomadSourceEnabled || gnomadSweepReady) &&
    // Allow Start during a recount when we already have a non-empty preview.
    // First load still waits until preview returns (total_unique stays 0 until then).
    scopeHasMarkers;

  const canStart = infrastructureReady && !sweepActive && !sweepPausing && !sweepWindingDown;

  const canResume =
    infrastructureReady && (fullyPaused || sweepInterrupted) && !sweepActive && !sweepWindingDown;

  const canPause = sweepActive && !sweepWindingDown;
  const canCancel =
    sweepActive || sweepPausing || fullyPaused || sweepInterrupted || sweepWindingDown;

  let primaryHint: string | null = null;
  let secondaryHint: string | null = null;

  if (connectionActivity.phase === "checking-qdrant") {
    primaryHint = "Checking Qdrant server…";
  } else if (connectionActivity.phase === "checking-ollama" || ollamaStatus === "testing") {
    primaryHint = "Checking Ollama embedding models…";
  } else if (scopePreviewLoading && !scopeHasMarkers) {
    primaryHint = "Counting sweep scope markers…";
  } else if (isConnectionPending) {
    primaryHint = "Verifying connections…";
  } else if (!configUrl?.trim()) {
    primaryHint = "Set a Qdrant server URL in connection settings.";
  } else if (!connectionStatus) {
    primaryHint = "Run Test Connections or wait for the automatic check to finish.";
  } else if (!connectionStatus.success) {
    primaryHint = connectionStatus.error || "Qdrant connection failed.";
  } else if (!connectionStatus.collection_exists) {
    primaryHint = `Collection not found on Qdrant — create it or fix the collection name.`;
  } else if (ollamaStatus === "dead") {
    primaryHint = "Ollama is unreachable or returned no models — check the embed server URL.";
  } else if (ollamaStatus === "untested") {
    primaryHint = "Ollama not verified yet — run Test Connections.";
  } else if (embeddingModelMismatch) {
    const indexed = indexEmbeddingModel ?? "unknown";
    const configured = configuredEmbeddingModel ?? "unknown";
    primaryHint = `Embedding model mismatch — collection indexed with "${indexed}" but config uses "${configured}". Re-embed or update the model before starting.`;
  } else if (gnomadSourceEnabled && !gnomadSweepReady) {
    secondaryHint =
      "Complete gnomAD setup in Research Scope (tabix index download) before starting.";
  } else if (scopePreview && scopePreview.total_unique === 0) {
    secondaryHint =
      "No variants match the selected scopes — enable at least one queue or sync GWAS reference data.";
  } else if (sweepWindingDown) {
    primaryHint = "Cancelling sweep — waiting for the worker to stop…";
  } else if (sweepInterrupted) {
    primaryHint =
      "Sweep was interrupted (app closed or backend stopped). Resume to continue from the last checkpoint.";
  }

  return {
    isConnectionPending,
    qdrantReady,
    ollamaReady,
    scopeHasMarkers,
    sweepActive,
    sweepPausing,
    sweepInterrupted,
    fullyPaused,
    canStart,
    canResume,
    canPause,
    canCancel,
    primaryHint,
    secondaryHint,
  };
}
