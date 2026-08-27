<!-- ./src/lib/components/research/ResearchJobActionsPanel.svelte -->
<script lang="ts">
  import type { ResearchJob, QdrantConfigPublic, QdrantConnectionStatus } from "../../types/research";
  import type { RunReadiness } from "../../utils/researchRunReadiness";
  import Tooltip from "../common/Tooltip.svelte";

  interface Props {
    job?: ResearchJob | null;
    config?: QdrantConfigPublic | null;
    connectionStatus?: QdrantConnectionStatus | null;
    readiness: RunReadiness;
    isStarting?: boolean;
    isCancelling?: boolean;
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
    readiness,
    isStarting = false,
    isCancelling = false,
    isCreatingCollection = false,
    onStart,
    onPause,
    onResume,
    onCancel,
    onCreateCollection,
  }: Props = $props();

  let showCreateCollection = $derived(
    connectionStatus?.success === true && !connectionStatus.collection_exists
  );

  let busy = $derived(isStarting || isCancelling || readiness.isConnectionPending);

  async function handleCreateCollection() {
    if (onCreateCollection) await onCreateCollection();
  }
</script>

<div class="control-actions mt-3">
  {#if job?.status === "idle" && job.error_message}
    <div class="purge-notice">{job.error_message}</div>
  {/if}

  {#if !job || job.status === "idle"}
    {#if showCreateCollection}
      <div class="setup-notice">
        <p class="hint warning-hint text-center">
          Qdrant is connected, but collection
          <strong class="collection-highlight">'{config?.collection || "genomics_evidence"}'</strong>
          does not exist.
        </p>
        <button
          class="btn btn-primary w-full mt-2"
          onclick={handleCreateCollection}
          disabled={isCreatingCollection || readiness.isConnectionPending}
        >
          {isCreatingCollection ? "Creating Collection..." : "Create Collection"}
        </button>
      </div>
    {:else}
      <button
        class="btn btn-primary w-full"
        onclick={() => onStart?.()}
        disabled={busy || !readiness.canStart}
      >
        {isStarting ? "Starting sweep…" : "Start Autonomous Sweep"}
      </button>
    {/if}
  {:else if readiness.sweepActive}
    <button
      class="btn btn-secondary w-full"
      onclick={() => onPause?.()}
      disabled={!readiness.canPause || readiness.sweepPausing || isCancelling}
    >
      {readiness.sweepPausing ? "Pausing…" : "Pause Sweep"}
    </button>
    <button
      class="btn btn-danger w-full mt-2"
      onclick={() => onCancel?.()}
      disabled={!readiness.canCancel || isCancelling}
    >
      {isCancelling ? "Cancelling…" : "Cancel Sweep"}
    </button>
  {:else if readiness.sweepPausing}
    <button class="btn btn-secondary w-full" disabled>Pausing sweep…</button>
    <button
      class="btn btn-danger w-full mt-2"
      onclick={() => onCancel?.()}
      disabled={!readiness.canCancel || isCancelling}
    >
      {isCancelling ? "Cancelling…" : "Cancel Sweep"}
    </button>
  {:else if readiness.fullyPaused || readiness.sweepInterrupted}
    {#if showCreateCollection}
      <div class="setup-notice">
        <p class="hint warning-hint text-center">
          Qdrant collection
          <strong class="collection-highlight">'{config?.collection || "genomics_evidence"}'</strong>
          is missing. Create it before resuming.
        </p>
        <button
          class="btn btn-primary w-full mt-2"
          onclick={handleCreateCollection}
          disabled={isCreatingCollection || readiness.isConnectionPending}
        >
          {isCreatingCollection ? "Creating Collection..." : "Create Collection"}
        </button>
      </div>
    {:else}
      <div class="btn-group w-full">
        <button
          class="btn btn-primary"
          onclick={() => onResume?.()}
          disabled={busy || !readiness.canResume}
        >
          {isStarting ? "..." : "Resume Sweep"}
        </button>
        <Tooltip interactiveChildren label="Force re-enrich" description="Rebuilds enrichment for already-indexed vectors. Prefer Resume for multi-day runs.">
          <button
            class="btn btn-secondary"
            onclick={() => onStart?.({ forceReenrich: true })}
            disabled={busy || !readiness.canStart}
          >
            Force re-enrich
          </button>
        </Tooltip>
      </div>
      <p class="hint sweep-hint">
        <strong>Resume</strong> continues from the checkpoint and skips vectors already indexed in Qdrant.
        <strong>Expand queue</strong> is available after Cancel or Complete — it starts a new job row.
      </p>
      {#if readiness.sweepInterrupted || readiness.fullyPaused}
        <button
          class="btn btn-danger w-full mt-2"
          onclick={() => onCancel?.()}
          disabled={!readiness.canCancel || isCancelling}
        >
          {isCancelling ? "Cancelling…" : "Cancel Sweep"}
        </button>
      {/if}
    {/if}
  {:else if job.status === "complete"}
    <div class="completion-banner">Enrichment complete — markers indexed in Qdrant.</div>
    <p class="hint sweep-hint">
      <strong>Expand queue</strong> enriches newly capped rsIDs (skips already indexed).
      <strong>Force re-enrich</strong> rebuilds the full queue with the latest pipeline.
    </p>
    <div class="btn-group w-full mt-2">
      <button
        class="btn btn-primary"
        onclick={() => onStart?.()}
        disabled={busy || !readiness.canStart}
      >
        {isStarting ? "..." : "Expand queue"}
      </button>
      <button
        class="btn btn-secondary"
        onclick={() => onStart?.({ forceReenrich: true })}
        disabled={busy || !readiness.canStart}
      >
        Force re-enrich
      </button>
    </div>
  {:else if job.status === "error"}
    <div class="error-banner">
      Run failed: {job.error_message || "An error occurred during the sweep."}
    </div>
    <button
      class="btn btn-primary w-full mt-2"
      onclick={() => onStart?.()}
      disabled={busy || !readiness.canStart}
    >
      {isStarting ? "..." : "Retry Sweep"}
    </button>
  {/if}

  {#if readiness.primaryHint}
    <p class="hint status-hint">{readiness.primaryHint}</p>
  {:else if readiness.secondaryHint}
    <p class="hint warning-hint">{readiness.secondaryHint}</p>
  {/if}
</div>
