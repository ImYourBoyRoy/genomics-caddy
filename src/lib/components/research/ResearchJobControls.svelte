<!-- ./src/lib/components/research/ResearchJobControls.svelte -->
<script lang="ts">
  import type {
    ResearchJob,
    QdrantConfigPublic,
    QdrantConnectionStatus,
    ConnectionActivity,
    PipelineTuningPublic,
    ResearchScopeConfig,
    ResearchScopePreview,
  } from "../../types/research";
  import { DEFAULT_RESEARCH_SCOPE } from "../../types/research";
  import { previewPipelineTuning } from "../../api/tauri";
  import { jobStatusClass, jobStatusText } from "../../utils/researchJobMetrics";
  import ResearchJobProgressPanel from "./ResearchJobProgressPanel.svelte";
  import ResearchJobActionsPanel from "./ResearchJobActionsPanel.svelte";

  interface Props {
    selectedSample: { id: number; name: string };
    job?: ResearchJob | null;
    config?: QdrantConfigPublic | null;
    connectionStatus?: QdrantConnectionStatus | null;
    connectionActivity?: ConnectionActivity;
    ollamaUrl?: string;
    researchScope?: ResearchScopeConfig;
    scopePreview?: ResearchScopePreview | null;
    gnomadSweepReady?: boolean;
    isStarting?: boolean;
    isCreatingCollection?: boolean;
    onStart?: (options?: { forceReenrich?: boolean }) => void;
    onPause?: () => void;
    onResume?: () => void;
    onCancel?: () => void;
    onCreateCollection?: () => Promise<void>;
  }

  let {
    selectedSample,
    job = null,
    config = null,
    connectionStatus = null,
    connectionActivity = { phase: "idle", message: "Ready" },
    ollamaUrl = "http://localhost:11434",
    researchScope = DEFAULT_RESEARCH_SCOPE,
    scopePreview = null,
    gnomadSweepReady = true,
    isStarting = false,
    isCreatingCollection = false,
    onStart,
    onPause,
    onResume,
    onCancel,
    onCreateCollection,
  }: Props = $props();

  let statusText = $derived(jobStatusText(job));
  let statusClass = $derived(jobStatusClass(job));

  let isConnectionPending = $derived(
    connectionActivity.phase === "checking-qdrant" ||
      connectionActivity.phase === "checking-ollama" ||
      (connectionActivity.phase === "loading-scope" && !connectionStatus)
  );

  let connectionHint = $derived.by(() => {
    if (connectionActivity.phase === "checking-qdrant") return "Checking Qdrant server…";
    if (connectionActivity.phase === "checking-ollama") return "Checking Ollama embedding models…";
    if (connectionActivity.phase === "loading-scope") return "Counting sweep scope markers…";
    if (!connectionStatus) return "Click Test Connections in Vector DB Connection to verify Qdrant.";
    return null;
  });

  let canStart = $derived(
    !!(connectionStatus &&
      connectionStatus.success &&
      connectionStatus.collection_exists &&
      !isConnectionPending &&
      gnomadSweepReady)
  );

  let gnomadBlockHint = $derived.by(() => {
    if (researchScope?.enrichment_sources?.gnomad) return null;
    if (gnomadSweepReady) return null;
    return "Complete gnomAD setup in Research Scope (one-click tabix index download) before starting.";
  });

  let pipelineTuning = $state<PipelineTuningPublic | null>(null);

  $effect(() => {
    const url = ollamaUrl;
    const qdrant = config?.url ?? "";
    const fast =
      researchScope?.enrichment_sources?.gnomad !== true &&
      !researchScope?.enrichment_sources?.clinvar_live &&
      !researchScope?.enrichment_sources?.pubmed &&
      !researchScope?.enrichment_sources?.gtex &&
      !researchScope?.enrichment_sources?.vep_dbsnp &&
      !researchScope?.enrichment_sources?.secondary;
    if (!url || !qdrant) {
      pipelineTuning = null;
      return;
    }
    void previewPipelineTuning(url, qdrant, fast)
      .then((t) => {
        pipelineTuning = t;
      })
      .catch((e: unknown) => {
        console.warn("Pipeline tuning preview failed:", e);
        pipelineTuning = null;
      });
  });
</script>

<div class="glass-card">
  <div class="card-header-row">
    <h2 class="card-title">Run Controller</h2>
    {#if job}
      <span class="status-badge status-{statusClass}">{statusText}</span>
    {:else}
      <span class="status-badge status-idle">Not Started</span>
    {/if}
  </div>

  <div class="sample-display">
    Target Genome: <strong class="accent-text">{selectedSample.name}</strong>
  </div>

  {#if job && job.status !== "idle"}
    <ResearchJobProgressPanel {job} {pipelineTuning} {scopePreview} />
  {/if}

  <ResearchJobActionsPanel
    {job}
    {config}
    {connectionStatus}
    {connectionActivity}
    {canStart}
    connectionHint={connectionHint}
    gnomadBlockHint={gnomadBlockHint}
    {isStarting}
    {isCreatingCollection}
    {onStart}
    {onPause}
    {onResume}
    {onCancel}
    {onCreateCollection}
  />
</div>

<style src="../../styles/components/research-job-controls.css"></style>
