<!-- ./src/lib/components/ai/settings/VectorResearchSettingsSection.svelte -->
<script lang="ts">
  import type { VectorResearchDiagnostics } from "../../../types/research";

  interface Props {
    useVectorResearch: boolean;
    vectorDiagnostics: VectorResearchDiagnostics | null;
    diagnosticsLoading?: boolean;
    refreshDiagnostics: () => void;
  }

  let {
    useVectorResearch = $bindable(true),
    vectorDiagnostics = null,
    diagnosticsLoading = false,
    refreshDiagnostics,
  }: Props = $props();

  $effect(() => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("genomics_consultation_vector_rag", String(useVectorResearch));
    }
  });

  function pct(enriched: number | undefined = undefined, total: number | undefined = undefined): string {
    if (enriched == null || !total) return "—";
    return `${Math.round((enriched / total) * 100)}%`;
  }
</script>

<details class="settings-details-group" open>
  <summary class="settings-details-summary">🔬 Vector Research (Consultation RAG)</summary>
  <div class="settings-details-content">
    <p class="card-hint">
      Uses hybrid workbench search (filtered + reranked) on each message and injects enriched hits into the AI context.
    </p>

    <label class="filter-toggle-label">
      <input type="checkbox" bind:checked={useVectorResearch} />
      <span class="highlight-text">Include vector research in AI consultation</span>
    </label>

    <div class="diag-panel">
      <div class="diag-header">
        <span>Index status</span>
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          onclick={refreshDiagnostics}
          disabled={diagnosticsLoading}
        >
          {diagnosticsLoading ? "Checking…" : "Refresh"}
        </button>
      </div>

      {#if vectorDiagnostics}
        <ul class="diag-list">
          <li>
            <span>Qdrant</span>
            <strong class:ok={vectorDiagnostics.connected} class:bad={!vectorDiagnostics.connected}>
              {vectorDiagnostics.connected ? "Connected" : "Offline"}
            </strong>
          </li>
          <li>
            <span>Collection</span>
            <strong>{vectorDiagnostics.collection}</strong>
          </li>
          <li>
            <span>Total vectors</span>
            <strong>{vectorDiagnostics.total_vectors?.toLocaleString() ?? "—"}</strong>
          </li>
          <li>
            <span>This sample</span>
            <strong>{vectorDiagnostics.sample_vectors?.toLocaleString() ?? "—"} indexed</strong>
          </li>
          {#if vectorDiagnostics.enrichment_total}
            <li>
              <span>Enrichment sweep</span>
              <strong>
                {vectorDiagnostics.enrichment_enriched?.toLocaleString()} / {vectorDiagnostics.enrichment_total.toLocaleString()}
                ({pct(vectorDiagnostics.enrichment_enriched, vectorDiagnostics.enrichment_total)})
                {#if vectorDiagnostics.enrichment_status}
                  · {vectorDiagnostics.enrichment_status}
                {/if}
              </strong>
            </li>
          {/if}
          <li>
            <span>Sweep quality</span>
            <strong class:bad={vectorDiagnostics.sweep_quality === "fast"}>
              {vectorDiagnostics.sweep_quality === "fast" ? "Fast (reduced sources)" : "Full"}
            </strong>
          </li>
          <li>
            <span>Named vectors</span>
            <strong>{vectorDiagnostics.named_vectors_enabled ? "Enabled" : "Default only"}</strong>
          </li>
          {#if vectorDiagnostics.stale_vector_count != null && vectorDiagnostics.stale_vector_count > 0}
            <li>
              <span>Stale vectors</span>
              <strong class="bad">{vectorDiagnostics.stale_vector_count.toLocaleString()}</strong>
            </li>
          {/if}
          {#if vectorDiagnostics.index_embedding_model}
            <li>
              <span>Index embed model</span>
              <strong class:bad={vectorDiagnostics.embedding_model_mismatch}>
                {vectorDiagnostics.index_embedding_model}
              </strong>
            </li>
          {/if}
        </ul>
        {#if vectorDiagnostics.embedding_model_mismatch}
          <p class="diag-warn">
            Embedding model mismatch — consultation RAG is blocked until you re-embed or re-sweep with
            <strong>{vectorDiagnostics.embedding_model}</strong>.
          </p>
        {/if}
        {#if vectorDiagnostics.sweep_quality === "fast"}
          <p class="diag-warn">
            Last sweep used fast mode (gnomAD/named vectors/secondary sources may be skipped). Run a full sweep for richer retrieval.
          </p>
        {/if}
        {#if vectorDiagnostics.error}
          <p class="diag-error">{vectorDiagnostics.error}</p>
        {/if}
        {#if vectorDiagnostics.sample_vectors === 0 && vectorDiagnostics.connected}
          <p class="diag-warn">
            No vectors indexed for this sample yet. Run Vector Research enrichment or check sample filter.
          </p>
        {/if}
      {:else}
        <p class="card-hint">Load a sample to check vector index coverage.</p>
      {/if}
    </div>
  </div>
</details>

<style>
  .diag-panel {
    margin-top: 12px;
    padding: 10px;
    border-radius: 8px;
    border: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.15);
  }
  .diag-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .diag-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 0.8rem;
  }
  .diag-list li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .diag-list span {
    opacity: 0.75;
  }
  .ok {
    color: #4ade80;
  }
  .bad {
    color: #f87171;
  }
  .diag-error {
    margin: 8px 0 0;
    font-size: 0.78rem;
    color: #f87171;
  }
  .diag-warn {
    margin: 8px 0 0;
    font-size: 0.78rem;
    color: #fbbf24;
  }
</style>
