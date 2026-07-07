<!-- ./src/lib/components/ai/settings/GenomicContextSection.svelte -->
<script lang="ts">
  import { markerPacksStore } from "../../../utils/markerPacksState.svelte";

  interface Props {
    selectedPacks: Record<string, boolean>;
    onlyActiveFindings: boolean;
    contextStats: { included: number; total: number };
  }

  let {
    selectedPacks = $bindable(),
    onlyActiveFindings = $bindable(),
    contextStats
  }: Props = $props();
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">🧬 Genomic Data Context</summary>
  <div class="settings-details-content">
    <p class="card-hint">Select which marker packs the model is allowed to read:</p>
    <div class="packs-list scrollable">
      {#each markerPacksStore.manifest.packs as pack}
        <label class="pack-checkbox-label">
          <input type="checkbox" bind:checked={selectedPacks[pack.id]} />
          <span>{pack.label}</span>
        </label>
      {/each}
    </div>

    <label class="filter-toggle-label mt-2">
      <input type="checkbox" bind:checked={onlyActiveFindings} />
      <span class="highlight-text">Only active findings (effect &gt; 0)</span>
    </label>
    
    <div class="context-pill mt-2">
      📊 Sending <strong>{contextStats.included}</strong> of <strong>{contextStats.total}</strong> found variants to AI
    </div>
  </div>
</details>

<style>
  .packs-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 180px;
    overflow-y: auto;
    border: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.15);
    padding: 10px;
    border-radius: 6px;
  }

  .pack-checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8rem;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
    min-width: 0;
  }

  .pack-checkbox-label input {
    cursor: pointer;
  }

  .pack-checkbox-label span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .filter-toggle-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .context-pill {
    background: rgba(88, 80, 236, 0.1);
    border: 1px solid rgba(88, 80, 236, 0.25);
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 0.72rem;
    color: var(--text-secondary);
    display: block;
    line-height: 1.4;
  }

  .context-pill strong {
    color: var(--text-primary);
  }

  .card-hint {
    font-size: 0.72rem;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
</style>
