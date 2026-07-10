<!-- ./src/lib/components/ai/evidence/VectorAtlasPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { buildVectorAtlas, getVectorAtlasCached, enableNamedVectorsCollection } from "../../../api/tauri";
  import type { AtlasPoint } from "../../../types/research";

  interface Props {
    sampleId: number;
    ollamaUrl: string;
    onSelectRsid?: (rsid: string) => void;
  }

  let { sampleId, ollamaUrl, onSelectRsid }: Props = $props();

  let points = $state<AtlasPoint[]>([]);
  let loading = $state(false);
  let error = $state("");
  let warning = $state("");
  let method = $state("");
  let namedMsg = $state("");

  const pad = 24;
  let width = $state(640);
  let height = $state(420);

  function scaleX(x: number) {
    return pad + ((x + 1) / 2) * (width - pad * 2);
  }
  function scaleY(y: number) {
    return pad + ((1 - y) / 2) * (height - pad * 2);
  }

  async function loadCached() {
    loading = true;
    error = "";
    try {
      points = await getVectorAtlasCached(sampleId, 1500);
      if (points.length === 0) {
        warning = "No cached atlas yet. Build the atlas from your enriched vectors.";
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function buildAtlas() {
    loading = true;
    error = "";
    try {
      const result = await buildVectorAtlas(sampleId, 1500);
      points = result.points;
      warning = result.warning;
      method = result.projection_method;
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function enableNamedVectors() {
    namedMsg = "Enabling named vectors on Qdrant collection…";
    try {
      namedMsg = await enableNamedVectorsCollection(ollamaUrl);
    } catch (e: unknown) {
      namedMsg = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(loadCached);
  $effect(() => {
    if (sampleId) loadCached();
  });
</script>

<section class="atlas-panel">
  <header class="atlas-header">
    <div>
      <h4>Vector atlas</h4>
      <p class="hint">2D semantic layout of enriched variants ({method || "umap"} projection).</p>
    </div>
    <div class="atlas-actions">
      <button type="button" class="btn btn-secondary btn-xs" onclick={loadCached} disabled={loading}>Reload cache</button>
      <button type="button" class="btn btn-primary btn-xs" onclick={buildAtlas} disabled={loading}>Build atlas</button>
      <button type="button" class="btn btn-secondary btn-xs" onclick={enableNamedVectors} disabled={!ollamaUrl}>Enable named vectors</button>
    </div>
  </header>

  {#if warning}
    <p class="warning">{warning}</p>
  {/if}
  {#if namedMsg}
    <p class="info">{namedMsg}</p>
  {/if}
  {#if error}
    <p class="error">{error}</p>
  {:else if loading}
    <p>Projecting vectors…</p>
  {:else if points.length === 0}
    <p>No atlas points yet.</p>
  {:else}
    <svg class="atlas-svg" viewBox="0 0 {width} {height}" role="img" aria-label="Vector atlas scatter plot">
      <rect x="0" y="0" {width} {height} class="atlas-bg" />
      {#each points as p (p.rsid)}
        <circle
          cx={scaleX(p.x)}
          cy={scaleY(p.y)}
          r={4}
          class="atlas-dot"
          style:opacity={0.35 + p.data_quality_score * 0.55}
          tabindex="0"
          role="button"
          aria-label="{p.rsid} {p.trait_category || ''}"
          onclick={() => onSelectRsid?.(p.rsid)}
          onkeydown={(e) => e.key === "Enter" && onSelectRsid?.(p.rsid)}
        >
          <title>{p.rsid} · {p.gene_symbol || "—"} · {p.trait_category || "uncategorized"} · DQ {(p.data_quality_score * 100).toFixed(0)}%</title>
        </circle>
      {/each}
    </svg>
    <p class="footnote">{points.length.toLocaleString()} points · click a dot to inspect rsID</p>
  {/if}
</section>

<style>
  .atlas-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .atlas-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }
  .atlas-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .hint, .footnote {
    font-size: 0.85rem;
    color: var(--text-muted, #888);
    margin: 0;
  }
  .warning {
    color: var(--warning, #c90);
    margin: 0;
  }
  .info {
    color: var(--text-muted, #888);
    margin: 0;
    font-size: 0.85rem;
  }
  .error {
    color: var(--danger, #c33);
  }
  .atlas-svg {
    width: 100%;
    max-width: 100%;
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--surface-2, #111);
  }
  .atlas-bg {
    fill: transparent;
  }
  .atlas-dot {
    fill: var(--accent, #5b9bd5);
    stroke: rgba(255, 255, 255, 0.35);
    stroke-width: 0.5;
    cursor: pointer;
  }
  .atlas-dot:hover {
    fill: var(--accent-strong, #8ec5ff);
  }
</style>
