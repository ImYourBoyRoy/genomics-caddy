<!-- ./src/lib/components/ai/evidence/TraitClusterPanel.svelte -->
<script lang="ts">
  import { buildTraitClusters, exportEvidencePacket } from "../../../api/tauri";
  import type { TraitClusterSummary } from "../../../types/research";

  interface Props {
    sampleId: number;
    traitCategory?: string;
  }

  let { sampleId, traitCategory = "" }: Props = $props();

  let clusters = $state<TraitClusterSummary[]>([]);
  let loading = $state(false);
  let error = $state("");
  let exportMsg = $state("");

  async function load() {
    loading = true;
    error = "";
    try {
      clusters = await buildTraitClusters(
        sampleId,
        traitCategory || undefined,
        0.35,
        20
      );
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function exportCluster(clusterId: string) {
    exportMsg = "Exporting…";
    try {
      const packet = await exportEvidencePacket(sampleId, undefined, clusterId);
      exportMsg = `Packet ${packet.packet_id} ready (${packet.association_facts.length} facts).`;
    } catch (e: unknown) {
      exportMsg = e instanceof Error ? e.message : String(e);
    }
  }

  $effect(() => {
    if (sampleId) load();
  });
</script>

<section class="cluster-panel">
  <header class="cluster-header">
    <h4>Trait / gene clusters</h4>
    <button type="button" class="btn btn-secondary btn-xs" onclick={load} disabled={loading}>Rebuild</button>
  </header>

  {#if loading}
    <p>Building clusters…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if clusters.length === 0}
    <p class="muted">No clusters yet — association facts populate after enrichment sweeps.</p>
  {:else}
    {#each clusters as cluster (cluster.cluster_id)}
      <article class="cluster-card">
        <h5>{cluster.cluster_title}</h5>
        <p class="cluster-meta">
          {cluster.association_count} associations · DQ {(cluster.data_quality_score * 100).toFixed(0)}% ·
          {cluster.known_direction_count} known direction · {cluster.unknown_direction_count} unknown
        </p>
        {#if cluster.top_traits.length > 0}
          <p><strong>Traits:</strong> {cluster.top_traits.join(", ")}</p>
        {/if}
        {#if cluster.top_genes.length > 0}
          <p><strong>Genes:</strong> {cluster.top_genes.join(", ")}</p>
        {/if}
        {#if cluster.top_rsids.length > 0}
          <p><strong>Variants:</strong> {cluster.top_rsids.slice(0, 8).join(", ")}</p>
        {/if}
        <button type="button" class="btn btn-secondary btn-xs" onclick={() => exportCluster(cluster.cluster_id)}>
          Export packet
        </button>
      </article>
    {/each}
  {/if}
  {#if exportMsg}<p class="export-msg">{exportMsg}</p>{/if}
</section>

<style>
  .cluster-panel { padding: 4px 0; }
  .cluster-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
  .cluster-header h4 { margin: 0; }
  .cluster-card {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 10px;
    background: rgba(0, 0, 0, 0.12);
  }
  .cluster-card h5 { margin: 0 0 6px; }
  .cluster-meta { font-size: 0.82rem; opacity: 0.85; margin-bottom: 8px; }
  .cluster-card p { margin: 4px 0; font-size: 0.85rem; }
  .error { color: #f08080; }
  .muted, .export-msg { font-size: 0.82rem; opacity: 0.8; }
</style>
