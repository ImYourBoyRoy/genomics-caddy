<!-- ./src/lib/components/research/ResearchJobActionsPanel.svelte -->
<script lang="ts">
  import type { ResearchJob, QdrantConfigPublic, QdrantConnectionStatus, ConnectionActivity } from "../../types/research";

  interface Props {
    job?: ResearchJob | null;
    config?: QdrantConfigPublic | null;
    connectionStatus?: QdrantConnectionStatus | null;
    connectionActivity?: ConnectionActivity;
    canStart?: boolean;
    connectionHint?: string | null;
    gnomadBlockHint?: string | null;
    isStarting?: boolean;
    isCreatingCollection?: boolean;
    onStart?: (options?: { forceReenrich?: boolean }) => void;
    onPause?: () => void;
    onResume?: () => void;
    onCancel?: () => void;
    onCreateCollection?: () => Promise<void>;
  }

  let {
    job = null,
    config = null,
    connectionStatus = null,
    connectionActivity = { phase: "idle", message: "Ready" },
    canStart = false,
    connectionHint = null,
    gnomadBlockHint = null,
    isStarting = false,
    isCreatingCollection = false,
    onStart,
    onPause,
    onResume,
    onCancel,
    onCreateCollection,
  }: Props = $props();

  async function handleCreateCollection() {
    if (onCreateCollection) await onCreateCollection();
  }
</script>

<div class="control-actions mt-3">
  {#if job?.status === "idle" && job.error_message}
    <div class="purge-notice">{job.error_message}</div>
  {/if}
  {#if !job || job.status === "idle"}
    {#if connectionStatus && connectionStatus.success && !connectionStatus.collection_exists}
      <div class="setup-notice">
        <p class="hint warning-hint text-center">
          ⚠️ Qdrant is connected, but collection <strong class="collection-highlight">'{config?.collection || "genomics_evidence"}'</strong> does not exist.
        </p>
        <button class="btn btn-primary w-full mt-2" onclick={handleCreateCollection} disabled={isCreatingCollection}>
          {isCreatingCollection ? "Creating Collection..." : "Create Collection"}
        </button>
      </div>
    {:else}
      <button class="btn btn-primary w-full" onclick={() => onStart?.()} disabled={isStarting || !canStart}>
        {isStarting ? "Initializing..." : "Start Autonomous Sweep"}
      </button>
      {#if connectionHint}
        <p class="hint status-hint">{connectionHint}</p>
      {:else if gnomadBlockHint}
        <p class="hint warning-hint">{gnomadBlockHint}</p>
      {:else if connectionStatus && !connectionStatus.success}
        <p class="hint error-hint">
          ❌ Qdrant Connection Failed: {connectionStatus.error || "Cannot connect to database."}
        </p>
      {/if}
    {/if}
  {:else if job.status === "running" || (job.status === "paused" && job.loop_active)}
    <button class="btn btn-secondary w-full" onclick={() => onPause?.()}>Pause Sweep</button>
    <button class="btn btn-danger w-full mt-2" onclick={() => onCancel?.()}>Cancel Sweep</button>
  {:else if job.status === "paused"}
    {#if connectionStatus && connectionStatus.success && !connectionStatus.collection_exists}
      <div class="setup-notice">
        <p class="hint warning-hint text-center">
          ⚠️ Qdrant collection <strong class="collection-highlight">'{config?.collection || "genomics_evidence"}'</strong> is missing. Create it before resuming.
        </p>
        <button class="btn btn-primary w-full mt-2" onclick={handleCreateCollection} disabled={isCreatingCollection}>
          {isCreatingCollection ? "Creating Collection..." : "Create Collection"}
        </button>
      </div>
    {:else}
      <div class="btn-group w-full">
        <button class="btn btn-primary" onclick={() => onResume?.()} disabled={isStarting || !canStart}>
          {isStarting ? "..." : "Resume Sweep"}
        </button>
        <button class="btn btn-secondary" onclick={() => onStart?.()} disabled={isStarting || !canStart}>Expand queue</button>
        <button class="btn btn-secondary" onclick={() => onStart?.({ forceReenrich: true })} disabled={isStarting || !canStart}>
          Force re-enrich
        </button>
      </div>
      {#if connectionHint}
        <p class="hint status-hint">{connectionHint}</p>
      {:else if gnomadBlockHint}
        <p class="hint warning-hint">{gnomadBlockHint}</p>
      {:else if connectionStatus && !connectionStatus.success}
        <p class="hint error-hint">
          ❌ Qdrant Connection Failed: {connectionStatus.error || "Cannot connect to database."}
        </p>
      {/if}
    {/if}
  {:else if job.status === "complete"}
    <div class="completion-banner">Enrichment complete — markers indexed in Qdrant.</div>
    <p class="hint sweep-hint">
      <strong>Expand queue</strong> enriches newly capped rsIDs (skips already indexed).
      <strong>Supplement missing only</strong> + enable sources (e.g. gnomAD) then Expand queue to backfill skipped data.
      <strong>Force re-enrich</strong> rebuilds the full queue with the latest pipeline.
      Increase GWAS cap first if you want more of your genome overlap researched.
    </p>
    <div class="btn-group w-full mt-2">
      <button class="btn btn-primary" onclick={() => onStart?.()} disabled={!canStart || isStarting}>
        {isStarting ? "..." : "Expand queue"}
      </button>
      <button class="btn btn-secondary" onclick={() => onStart?.({ forceReenrich: true })} disabled={!canStart || isStarting}>
        Force re-enrich
      </button>
    </div>
    {#if connectionStatus && !connectionStatus.success}
      <p class="hint error-hint">Fix the Qdrant connection before running a new sweep.</p>
    {/if}
  {:else if job.status === "error"}
    <div class="error-banner">
      Run failed: {job.error_message || "An error occurred during the sweep."}
    </div>
    <button class="btn btn-primary w-full mt-2" onclick={() => onStart?.()} disabled={!canStart}>Retry Sweep</button>
    {#if connectionStatus && !connectionStatus.success}
      <p class="hint error-hint">Fix the Qdrant connection before retrying.</p>
    {/if}
  {/if}
</div>
