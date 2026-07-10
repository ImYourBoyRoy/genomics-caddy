<!-- ./src/lib/components/ai/evidence/EvidenceCorpusOverview.svelte -->
<script lang="ts">
  import type { EvidenceCorpusSummary } from "../../../types/research";
  import type { BrowsePreset } from "../../../utils/evidenceSearch";
  import "$lib/styles/components/evidence-corpus-overview.css";

  interface Props {
    summary: EvidenceCorpusSummary | null;
    loading?: boolean;
    activePreset?: BrowsePreset | "";
    onBrowsePreset?: (preset: BrowsePreset) => void;
    onSelectRsid?: (rsid: string) => void;
    onBrowseTrait?: (traitCategory: string) => void;
  }

  let {
    summary = null,
    loading = false,
    activePreset = "",
    onBrowsePreset,
    onSelectRsid,
    onBrowseTrait,
  }: Props = $props();

  const presets: { id: BrowsePreset; label: string }[] = [
    { id: "actionable", label: "Top actionable" },
    { id: "clinical", label: "Clinical signals" },
    { id: "gwas", label: "GWAS hits" },
    { id: "unknown", label: "Unknown direction" },
  ];

  let indexed = $derived(summary?.dashboard.vectorized_variants ?? 0);
</script>

<section class="evidence-corpus-overview">
  {#if loading}
    <p class="eco-loading">Loading corpus overview from vector index…</p>
  {:else if !summary || indexed === 0}
    <p class="eco-empty">
      No enriched variants in Qdrant yet. Run Vector Research to index your genome, then browse
      actionable associations here without typing a query.
    </p>
  {:else}
    <div class="eco-header">
      <h4>Indexed corpus ({indexed.toLocaleString()} variants)</h4>
      <div class="eco-stats">
        <span class="eco-stat">{summary.dashboard.association_fact_count.toLocaleString()} facts</span>
        <span class="eco-stat">avg DQ {Math.round(summary.dashboard.avg_data_quality_score * 100)}%</span>
        {#if summary.enrichment_progress_pct != null}
          <span class="eco-stat">{Math.round(summary.enrichment_progress_pct)}% enriched</span>
        {/if}
      </div>
    </div>

    <p class="eco-brief">{summary.index_brief}</p>

    {#if summary.trait_buckets.length > 0}
      <p class="eco-section-title">Browse by trait category</p>
      <div class="eco-buckets">
        {#each summary.trait_buckets as bucket (bucket.trait_category)}
          <button
            type="button"
            class="eco-bucket eco-bucket-btn"
            onclick={() => onBrowseTrait?.(bucket.trait_category)}
          >
            {bucket.trait_category.replace(/_/g, " ")} · {bucket.variant_count.toLocaleString()}
          </button>
        {/each}
      </div>
    {/if}

    <p class="eco-section-title">Quick filters</p>
    <div class="eco-presets">
      {#each presets as p (p.id)}
        <button
          type="button"
          class="btn btn-secondary btn-sm eco-preset-btn"
          class:active={activePreset === p.id}
          onclick={() => onBrowsePreset?.(p.id)}
        >
          {p.label}
        </button>
      {/each}
    </div>

    {#if summary.top_actionable.length > 0}
      <p class="eco-section-title">Highest actionability</p>
      <div class="eco-top-rsids">
        {#each summary.top_actionable.slice(0, 12) as point (point.rsid)}
          <button type="button" class="eco-rsid-chip" onclick={() => onSelectRsid?.(point.rsid)}>
            {point.rsid}
            {#if point.gene_symbol}
              · {point.gene_symbol}
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</section>
