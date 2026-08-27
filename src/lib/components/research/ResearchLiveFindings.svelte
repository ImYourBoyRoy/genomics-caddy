<!-- ./src/lib/components/research/ResearchLiveFindings.svelte -->
<!--
  Live user-facing finding feed for vector research sweeps.
  Shows newly indexed variants as plain-English, triaged cards while the
  backend continues enrichment. Inputs are transient Tauri event previews;
  no mutations or medical claims are made here.
-->
<script lang="ts">
  import type { ResearchFindingPreview } from "../../types/research";
  import Tooltip from "../common/Tooltip.svelte";

  interface Props {
    findings?: ResearchFindingPreview[];
    sweepRunning?: boolean;
    qdrantSampleCount?: number | null;
  }

  let {
    findings = [],
    sweepRunning = false,
    qdrantSampleCount = null,
  }: Props = $props();

  function pct(v: number): string {
    return `${Math.round((v ?? 0) * 100)}%`;
  }

  function bucketLabel(bucket: string): string {
    switch (bucket) {
      case "clinical_review":
        return "Clinical review queue";
      case "wellness_relevant":
        return "Wellness research lead";
      case "interpretable_research":
        return "Interpretable research";
      case "low_confidence":
        return "Needs better evidence";
      default:
        return "Research context";
    }
  }

  function bucketHint(bucket: string): string {
    switch (bucket) {
      case "clinical_review":
        return "Potentially clinically relevant evidence surfaced; treat as a review queue item, not a diagnosis.";
      case "wellness_relevant":
        return "Likely useful for personal research or habit/lifestyle context after source review.";
      case "interpretable_research":
        return "Direction is available, but actionability is limited or source coverage is still developing.";
      case "low_confidence":
        return "Important metadata is missing; use this as a lead for follow-up, not a conclusion.";
      default:
        return "Indexed as background research context.";
    }
  }
</script>

<section class="glass-card live-findings-card">
  <div class="live-findings-header">
    <div>
      <h2 class="card-title">Live Finding Feed</h2>
      <p>
        Newly indexed variants translated into reviewable leads while the sweep runs.
        Open the Evidence Library after the scan for full provenance and saved packets.
      </p>
    </div>
    {#if sweepRunning}
      <span class="live-pill">Listening</span>
    {:else}
      <span class="live-pill idle">Idle</span>
    {/if}
  </div>

  {#if findings.length === 0}
    <div class="live-findings-empty">
      {#if sweepRunning}
        <strong>Working on the current batch.</strong>
        {#if qdrantSampleCount != null && qdrantSampleCount > 0}
          <span>
            {qdrantSampleCount.toLocaleString()} variants for this sample are already in Qdrant —
            new live cards appear here as each batch finishes embedding and upsert.
          </span>
        {:else}
          <span>Findings appear here after a batch is enriched, embedded, and saved to Qdrant.</span>
        {/if}
        <span>Prefetch and embedding can take several minutes per batch on remote Ollama/Qdrant hosts.</span>
      {:else if qdrantSampleCount != null && qdrantSampleCount > 0}
        <strong>No live session findings yet.</strong>
        <span>
          {qdrantSampleCount.toLocaleString()} indexed variants are in Qdrant — start or resume a sweep
          to stream new previews, or open the Evidence Library for the full catalog.
        </span>
      {:else}
        <strong>No live findings yet.</strong>
        <span>Start or resume a sweep to see plain-English finding previews here.</span>
      {/if}
    </div>
  {:else}
    <div class="live-findings-grid">
      {#each findings.slice(0, 12) as finding (finding.rsid)}
        <article class="live-finding-item bucket-{finding.impact_bucket}">
          <div class="live-finding-topline">
            <span class="live-finding-rsid">{finding.rsid}</span>
            {#if finding.gene_symbol}
              <span class="live-finding-gene">{finding.gene_symbol}</span>
            {/if}
            {#if finding.genotype}
              <span class="live-finding-genotype">{finding.genotype}</span>
            {/if}
          </div>

          <h3>{finding.trait_name ?? finding.trait_category?.replace(/_/g, " ") ?? "Indexed association"}</h3>
          <p class="live-finding-summary">{finding.summary}</p>

          <div class="live-finding-meta">
            <Tooltip label={bucketLabel(finding.impact_bucket)} description={bucketHint(finding.impact_bucket)}>
              <span>{bucketLabel(finding.impact_bucket)}</span>
            </Tooltip>
            <span>Data quality {pct(finding.data_quality_score)}</span>
            <span>Wellness relevance {pct(finding.wellness_actionability_score)}</span>
            {#if finding.clinical_actionability_score > 0}
              <span>Clinical review signal {pct(finding.clinical_actionability_score)}</span>
            {/if}
            <span>{finding.source_count} evidence sources</span>
          </div>
          {#if finding.source_names.length > 0}
            <p class="live-finding-sources">
              Sources: {finding.source_names.slice(0, 3).join(", ")}
              {#if finding.source_names.length > 3}
                +{finding.source_names.length - 3} more
              {/if}
            </p>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>
