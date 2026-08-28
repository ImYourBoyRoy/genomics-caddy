<!-- ./src/lib/components/report/VectorPromotedSection.svelte -->
<script lang="ts">
  import { getVectorPromotedFindings, getEvidenceCorpusSummary } from "../../api/tauri";
  import type { GenomeSample, VectorPromotedFinding } from "../../types/genomics";
  import type { ActionabilityPoint } from "../../types/research";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import Tooltip from "../common/Tooltip.svelte";

  interface Props {
    selectedSample: GenomeSample;
    presentationMode?: 'simple' | 'clinical' | 'compare';
    highlightRsid?: string;
    onNavigate?: (rsid: string, target: VariantNavTarget) => void;
    onExploreResearch?: (rsid: string) => void;
  }

  let { selectedSample, presentationMode = 'simple', highlightRsid = "", onNavigate, onExploreResearch }: Props = $props();

  let findings = $state<VectorPromotedFinding[]>([]);
  let corpusHighlights = $state<ActionabilityPoint[]>([]);
  let isLoading = $state(true);
  let expanded = $state(false);
  const promotedBodyId = 'vector-promoted-items';

  let displayItems = $derived.by(() => {
    const seen = new Set(findings.map((f) => f.rsid.toLowerCase()));
    const merged: VectorPromotedFinding[] = [...findings];
    for (const point of corpusHighlights) {
      if (!seen.has(point.rsid.toLowerCase())) {
        merged.push({
          rsid: point.rsid,
          gene: point.gene_symbol,
          user_genotype: undefined,
          trait_summary: point.trait_category?.replace(/_/g, " ") ?? "Indexed actionable variant",
          trait_categories: point.trait_category ? [point.trait_category] : [],
          significance_score: Math.max(point.wellness_actionability_score, point.data_quality_score),
          enrichment_version: "corpus_index",
          promoted_at: 0,
        });
      }
    }
    return merged;
  });

  $effect(() => {
    const sampleId = selectedSample.id;
    isLoading = true;
    Promise.all([
      getVectorPromotedFindings(sampleId).catch(() => [] as VectorPromotedFinding[]),
      getEvidenceCorpusSummary(sampleId)
        .then((s) => s.top_actionable.slice(0, 16))
        .catch(() => [] as ActionabilityPoint[]),
    ])
      .then(([rows, highlights]) => {
        findings = rows;
        corpusHighlights = highlights;
      })
      .finally(() => {
        isLoading = false;
      });
  });
</script>

{#if isLoading}
  <div class="vector-promoted no-print">
    <ActivityPulse message="Loading vector research discoveries…" accent="var(--status-success-text)" maxWidth="100%" />
  </div>
{:else if displayItems.length > 0}
  <section class="vector-promoted no-print">
    <div class="header">
      <span class="icon">🧬</span>
      <div>
        <strong>{displayItems.length} vector-indexed highlight{displayItems.length === 1 ? "" : "s"}</strong>
        <p class="hint">
          Sweep-promoted GWAS hits plus top actionable variants from your {findings.length > 0 ? "enrichment run" : "corpus index"}.
        </p>
      </div>
      <button
        type="button"
        class="toggle"
        aria-expanded={expanded}
        aria-controls={expanded ? promotedBodyId : undefined}
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? "Hide" : "Show"}
      </button>
    </div>

    {#if expanded}
      <div class="grid" id={promotedBodyId}>
        {#each displayItems.slice(0, 32) as item (item.rsid)}
          <article
            class="card"
            class:highlighted={highlightRsid && item.rsid.toLowerCase() === highlightRsid.toLowerCase()}
            id="vector-promoted-{item.rsid.toLowerCase()}"
          >
            {#if presentationMode === 'simple'}
              <div class="card-top simple-card-top">
                <strong class="simple-research-label">Research context</strong>
              </div>
              <p class="trait">{item.trait_summary}</p>
              <details class="technical-details">
                <summary>Technical data</summary>
                <dl>
                  <div><dt>rsID</dt><dd>{item.rsid}</dd></div>
                  {#if item.gene}<div><dt>Gene</dt><dd>{item.gene}</dd></div>{/if}
                  {#if item.user_genotype}<div><dt>DNA call</dt><dd>{item.user_genotype}</dd></div>{/if}
                  <div><dt>Research index</dt><dd>{Math.round(item.significance_score * 100)}%</dd></div>
                  {#if item.trait_categories.length > 0}
                    <div><dt>Categories</dt><dd>{item.trait_categories.map((cat) => cat.replace(/_/g, " ")).join(', ')}</dd></div>
                  {/if}
                </dl>
              </details>
            {:else}
              <div class="card-top">
                <code class="rsid">{item.rsid}</code>
                {#if item.gene}<span class="gene">{item.gene}</span>{/if}
                {#if item.user_genotype}<span class="gt">{item.user_genotype}</span>{/if}
                <Tooltip label="Research index score" description="This local ranking signal helps organize research items. It is not a disease probability or a health score.">
                  <span class="score">Rank {Math.round(item.significance_score * 100)}%</span>
                </Tooltip>
              </div>
              <p class="trait">{item.trait_summary}</p>
              {#if item.trait_categories.length > 0}
                <div class="tags">
                  {#each item.trait_categories as cat}
                    <span class="tag">{cat.replace(/_/g, " ")}</span>
                  {/each}
                </div>
              {/if}
            {/if}
            <div class="actions">
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigate?.(item.rsid, "report")}>📋 Report</button>
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigate?.(item.rsid, "map")}>🗺️ Map</button>
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigate?.(item.rsid, "browser")}>🔍 Browser</button>
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onExploreResearch?.(item.rsid)}>🔬 Evidence</button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .vector-promoted {
    margin-bottom: 16px;
    padding: 12px 14px;
    border-radius: 10px;
    border: 1px solid var(--status-success-border);
    background: var(--status-success-bg);
  }
  .header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .icon { font-size: 1.2rem; }
  .hint {
    margin: 2px 0 0;
    font-size: 0.78rem;
    opacity: 0.8;
  }
  .toggle {
    margin-left: auto;
    background: none;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: inherit;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 0.75rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-top: 12px;
  }
  .card {
    padding: 10px;
    border-radius: 8px;
    background: var(--surface-card);
    border: 1px solid var(--border-color);
  }
  .simple-card-top {
    margin-bottom: 6px;
  }
  .simple-research-label {
    color: var(--text-primary);
    font-size: 0.78rem;
  }
  .technical-details {
    margin: 0.55rem 0 0.7rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
  }
  .technical-details summary {
    cursor: pointer;
    font-weight: 700;
  }
  .technical-details dl {
    display: grid;
    gap: 0.35rem;
    margin: 0.55rem 0 0;
  }
  .technical-details dl > div {
    display: grid;
    grid-template-columns: minmax(4.5rem, auto) minmax(0, 1fr);
    gap: 0.5rem;
  }
  .technical-details dt {
    font-weight: 700;
  }
  .technical-details dd {
    min-width: 0;
    margin: 0;
    overflow-wrap: anywhere;
  }
  .card.highlighted {
    border-color: var(--status-success-border);
    box-shadow: 0 0 0 1px var(--status-success-border);
  }
  .card-top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
  }
  .rsid { color: var(--status-success-text); font-size: 0.85rem; }
  .gene {
    font-size: 0.72rem;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--status-accent-bg);
  }
  .gt { font-family: monospace; font-size: 0.72rem; opacity: 0.85; }
  .score { margin-left: auto; font-size: 0.7rem; opacity: 0.75; }
  .trait {
    margin: 0 0 6px;
    font-size: 0.8rem;
    line-height: 1.35;
  }
  .tags { margin-bottom: 8px; }
  .tag {
    display: inline-block;
    margin: 0 4px 4px 0;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.65rem;
    text-transform: capitalize;
    background: var(--status-info-bg);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .btn-xs { font-size: 0.68rem; padding: 2px 6px; }

  @media (max-width: 720px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
