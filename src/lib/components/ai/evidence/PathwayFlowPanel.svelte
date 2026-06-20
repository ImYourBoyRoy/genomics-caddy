<!-- ./src/lib/components/ai/evidence/PathwayFlowPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getPathwayFlowRows } from "../../../api/tauri";
  import type { PathwayFlowRow } from "../../../types/research";

  interface Props {
    sampleId: number;
    onSelectRsid?: (rsid: string) => void;
  }

  let { sampleId, onSelectRsid }: Props = $props();

  let rows = $state<PathwayFlowRow[]>([]);
  let loading = $state(false);
  let error = $state("");

  async function load() {
    loading = true;
    error = "";
    try {
      rows = await getPathwayFlowRows(sampleId, 120);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
      rows = [];
    } finally {
      loading = false;
    }
  }

  onMount(load);

  const grouped = $derived.by(() => {
    const map = new Map<string, PathwayFlowRow[]>();
    for (const row of rows) {
      const key = row.gene_symbol;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(row);
    }
    return [...map.entries()].slice(0, 24);
  });
</script>

<section class="pathway-panel">
  <header class="pp-header">
    <h4>Gene → pathway context</h4>
    <button type="button" class="btn btn-secondary btn-xs" onclick={load} disabled={loading}>
      {loading ? "Loading…" : "Reload"}
    </button>
  </header>
  <p class="hint">Reactome pathway associations from enriched evidence (context only, not causal claims).</p>

  {#if error}
    <p class="error">{error}</p>
  {:else if rows.length === 0 && !loading}
    <p class="muted">No Reactome pathway rows in association facts yet.</p>
  {:else}
    <div class="flow-list">
      {#each grouped as [gene, items]}
        <article class="flow-row">
          <div class="gene-col">{gene}</div>
          <div class="arrow">→</div>
          <ul class="pathway-col">
            {#each items.slice(0, 4) as item}
              <li>
                {item.pathway_name}
                {#if item.rsid}
                  <button type="button" class="rsid-link" onclick={() => onSelectRsid?.(item.rsid!)}>
                    {item.rsid}
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .pathway-panel { padding: 4px 0; }
  .pp-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .pp-header h4 { margin: 0; }
  .hint, .muted { font-size: 0.82rem; opacity: 0.75; }
  .error { color: #f08080; }
  .flow-list { display: flex; flex-direction: column; gap: 10px; }
  .flow-row {
    display: grid;
    grid-template-columns: 100px 24px 1fr;
    gap: 8px;
    align-items: start;
    padding: 8px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(0, 0, 0, 0.12);
  }
  .gene-col { font-weight: 600; font-size: 0.85rem; }
  .arrow { opacity: 0.5; padding-top: 2px; }
  .pathway-col { margin: 0; padding-left: 16px; font-size: 0.8rem; }
  .rsid-link {
    margin-left: 6px;
    background: none;
    border: none;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
    font-size: 0.75rem;
    text-decoration: underline;
  }
</style>
