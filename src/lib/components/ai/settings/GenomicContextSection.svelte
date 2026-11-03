<!-- ./src/lib/components/ai/settings/GenomicContextSection.svelte -->
<script lang="ts">
  import { markerPacksStore } from "../../../utils/markerPacksState.svelte";
  import type { ContextDropReason } from "../../../utils/consultationContextPack";

  interface Props {
    selectedPacks: Record<string, boolean>;
    onlyActiveFindings: boolean;
    contextStats: {
      included: number;
      total: number;
      estimatedTokens?: number;
      notices?: string[];
      dropCounts?: Partial<Record<ContextDropReason, number>>;
      budgetNotice?: string;
    };
  }

  let {
    selectedPacks = $bindable(),
    onlyActiveFindings = $bindable(),
    contextStats
  }: Props = $props();

  let dropSummary = $derived.by(() => {
    const drops = contextStats.dropCounts;
    if (!drops) return "";
    const parts: string[] = [];
    if (drops.mode_filter) parts.push(`${drops.mode_filter} mode-filtered`);
    if (drops.budget) parts.push(`${drops.budget} budget`);
    if (drops.low_rank) parts.push(`${drops.low_rank} lower-rank`);
    if (drops.history_trim) parts.push(`${drops.history_trim} history`);
    if (drops.mode_remap) parts.push("mode remapped");
    return parts.join(" · ");
  });
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">🧬 Genomic Data Context</summary>
  <div class="settings-details-content">
    <p class="card-hint">Select which marker packs the model is allowed to read:</p>
    <div class="packs-list scrollable">
      {#each markerPacksStore.manifest.packs as pack (pack.id)}
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
      📊 Pool <strong>{contextStats.total}</strong> · sending <strong>{contextStats.included}</strong>
      {#if contextStats.estimatedTokens != null}
        · ~<strong>{contextStats.estimatedTokens.toLocaleString()}</strong> tokens
      {/if}
    </div>
    {#if contextStats.budgetNotice}
      <p class="context-budget-line" role="status">{contextStats.budgetNotice}</p>
    {/if}
    {#if dropSummary}
      <p class="context-drop-line">Drops: {dropSummary}</p>
    {/if}
    {#if contextStats.notices?.length}
      <ul class="context-notice-list">
        {#each contextStats.notices as notice, i (i)}
          <li>{notice}</li>
        {/each}
      </ul>
    {/if}
    <p class="raw-data-note" role="note">
      🔒 Raw genotype calls are always included for the findings sent to AI. Context mode sets the candidate pool; ask-scoped ranking and provider budgets choose what is actually sent. Review the endpoint disclosure in Connection &amp; Model before connecting to a remote host.
    </p>
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
    background: rgba(20, 184, 166, 0.1);
    border: 1px solid rgba(20, 184, 166, 0.25);
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

  .context-budget-line,
  .context-drop-line {
    margin: 6px 0 0;
    font-size: 0.7rem;
    color: var(--text-muted);
    line-height: 1.35;
  }

  .context-notice-list {
    margin: 6px 0 0;
    padding-left: 1.1rem;
    font-size: 0.7rem;
    color: var(--warning, #d4a017);
    line-height: 1.35;
  }

  .card-hint {
    font-size: 0.72rem;
    color: var(--text-muted);
    margin-bottom: 6px;
  }

  .raw-data-note {
    margin: 8px 0 0;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.4;
  }
</style>
