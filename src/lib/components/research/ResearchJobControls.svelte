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
  import { getVectorResearchDiagnostics } from "../../api/tauri";

  import { jobStatusClass, jobStatusText } from "../../utils/researchJobMetrics";

  import {

    computeRunReadiness,

    type OllamaConnectionState,

  } from "../../utils/researchRunReadiness";

  import ResearchJobProgressPanel from "./ResearchJobProgressPanel.svelte";

  import ResearchJobActionsPanel from "./ResearchJobActionsPanel.svelte";

  import { liveProgress } from "../../research/liveProgress.svelte";



  interface Props {

    selectedSample: { id: number; name: string };

    job?: ResearchJob | null;

    config?: QdrantConfigPublic | null;

    connectionStatus?: QdrantConnectionStatus | null;

    connectionActivity?: ConnectionActivity;

    ollamaUrl?: string;

    ollamaStatus?: OllamaConnectionState;

    researchScope?: ResearchScopeConfig;

    scopePreview?: ResearchScopePreview | null;

    scopePreviewLoading?: boolean;

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

    ollamaStatus = "untested",

    researchScope = DEFAULT_RESEARCH_SCOPE,

    scopePreview = null,

    scopePreviewLoading = false,

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

  let pipelineTuning = $state<PipelineTuningPublic | null>(null);
  let embeddingModelMismatch = $state(false);
  let indexEmbeddingModel = $state<string | null>(null);

  $effect(() => {
    const sampleId = selectedSample.id;
    if (!sampleId) {
      embeddingModelMismatch = false;
      indexEmbeddingModel = null;
      return;
    }
    void getVectorResearchDiagnostics(sampleId)
      .then((d) => {
        embeddingModelMismatch = d.embedding_model_mismatch === true;
        indexEmbeddingModel = d.index_embedding_model ?? null;
      })
      .catch(() => {
        embeddingModelMismatch = false;
        indexEmbeddingModel = null;
      });
  });

  let readiness = $derived(

    computeRunReadiness({

      configUrl: config?.url,

      connectionStatus: connectionStatus ?? null,

      connectionActivity,

      ollamaStatus,

      gnomadSweepReady,

      gnomadSourceEnabled: researchScope?.enrichment_sources?.gnomad === true,

      scopePreview,

      scopePreviewLoading,

      job: job ?? null,

      embeddingModelMismatch,

      indexEmbeddingModel,

      configuredEmbeddingModel: config?.embedding_model ?? null,

    })

  );

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



  let qdrantSampleCount = $derived.by(() => {

    void liveProgress.tick;

    return (

      liveProgress.qdrantSampleCount ??

      job?.qdrant_sample_count ??

      null

    );

  });

</script>



<div class="glass-card research-job-card">

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



  {#if readiness.sweepInterrupted}

    <div class="interrupted-banner">

      The sweep loop is not active in this session. Resume to continue from the last saved checkpoint,

      or cancel to reset controls.

    </div>

  {/if}



  {#if job && job.status !== "idle"}

    <ResearchJobProgressPanel

      {job}

      {pipelineTuning}

      {scopePreview}

      qdrantSampleCount={qdrantSampleCount}

      collectionVectorCount={connectionStatus?.vectors_count ?? null}

    />

  {/if}



  <ResearchJobActionsPanel

    {job}

    {config}

    {connectionStatus}

    readiness={readiness}

    {isStarting}

    {isCreatingCollection}

    {onStart}

    {onPause}

    {onResume}

    {onCancel}

    {onCreateCollection}

  />

</div>


