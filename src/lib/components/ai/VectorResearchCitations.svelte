<!-- ./src/lib/components/ai/VectorResearchCitations.svelte -->
<script lang="ts">
  import type { EvidenceCard, QdrantHit } from "../../types/research";
  import type { VariantNavTarget } from "../../constants/traitCategories";

  interface Props {
    hits: QdrantHit[];
    evidenceCards?: EvidenceCard[];
    indexBrief?: string;
    query?: string;
    error?: string;
    enabled?: boolean;
    onOpenEvidence?: (query: string) => void;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let {
    hits = [],
    evidenceCards = [],
    indexBrief = "",
    query = "",
    error = "",
    enabled = true,
    onOpenEvidence,
    onNavigateToVariant,
  }: Props = $props();

  let filterText = $state("");
  let expanded = $state(true);

  let cardByRsid = $derived.by(() => {
    const map = new Map<string, EvidenceCard>();
    for (const card of evidenceCards) map.set(card.rsid, card);
    return map;
  });

  let filteredHits = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    if (!q) return hits;
    return hits.filter((h) => {
      const blob = [
        h.rsid,
        h.gene,
        h.gwas_trait,
        h.text,
        h.genotype,
        h.clinvar_significance,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return blob.includes(q);
    });
  });
</script>

{#if !enabled}
  <div class="vector-citations vector-citations--muted">
    <span>Vector research retrieval is off for consultation.</span>
  </div>
{:else if error}
  <div class="vector-citations vector-citations--error">
    <strong>Vector search failed:</strong> {error}
  </div>
{:else if hits.length > 0}
  <div class="vector-citations">
    <div class="vector-citations-header">
      <button type="button" class="collapse-btn" onclick={() => (expanded = !expanded)}>
        {expanded ? "▾" : "▸"} Vector research ({hits.length} hits)
      </button>
      {#if query}
        <span class="query-pill" title="Semantic query">“{query.slice(0, 80)}{query.length > 80 ? "…" : ""}”</span>
      {/if}
      {#if onOpenEvidence}
        <button type="button" class="btn btn-secondary btn-xs" onclick={() => onOpenEvidence?.(query || hits[0]?.rsid || "")}>
          Open in Evidence Library
        </button>
      {/if}
    </div>

    {#if expanded}
      {#if indexBrief}
        <p class="index-brief">{indexBrief}</p>
      {/if}
      <input
        type="search"
        class="filter-input"
        placeholder="Filter hits by rsID, gene, trait…"
        bind:value={filterText}
      />
      <div class="hits-grid">
        {#each filteredHits as hit (hit.rsid + hit.score)}
          {@const card = cardByRsid.get(hit.rsid)}
          <article class="hit-card">
            <header>
              <strong class="rsid">{hit.rsid}</strong>
              {#if hit.gene}
                <span class="gene">{hit.gene}</span>
              {/if}
              {#if hit.genotype}
                <span class="genotype">{hit.genotype}</span>
              {/if}
              <span class="score" title="Semantic similarity">{Math.round(hit.score * 100)}% match</span>
            </header>
            {#if card}
              <div class="card-scores">
                <span class="score-chip">DQ {Math.round(card.data_quality_score * 100)}%</span>
                <span class="score-chip">Wellness {Math.round(card.wellness_actionability_score * 100)}%</span>
                {#if card.personal_direction}
                  <span class="score-chip direction">{card.personal_direction.replace(/_/g, " ")}</span>
                {/if}
              </div>
              {#if card.synthesis}
                <p class="synthesis">{card.synthesis.slice(0, 220)}{card.synthesis.length > 220 ? "…" : ""}</p>
              {/if}
            {/if}
            {#if hit.trait_categories && hit.trait_categories.length > 0}
              <div class="category-tags">
                {#each hit.trait_categories as cat}
                  <span class="cat-tag">{cat.replace(/_/g, " ")}</span>
                {/each}
              </div>
            {/if}
            {#if hit.gwas_trait}
              <p class="traits">{hit.gwas_trait}</p>
            {/if}
            {#if hit.gwas_associations && hit.gwas_associations.length > 0}
              <ul class="assoc-list">
                {#each hit.gwas_associations.slice(0, 3) as assoc}
                  <li>
                    {#if assoc.trait}{assoc.trait}{/if}
                    {#if assoc.reported_gene || assoc.mapped_gene}
                      <span class="assoc-gene"> → {assoc.reported_gene || assoc.mapped_gene}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
            {#if hit.gnomad_af != null}
              <div class="meta gnomad-context">
                <strong>Population frequency context</strong> (not clinical significance)
                <span>AF: {(hit.gnomad_af * 100).toFixed(4)}%</span>
                {#if hit.gnomad_lookup_status}
                  <span class="muted">· {hit.gnomad_lookup_status}</span>
                {/if}
              </div>
            {/if}
            {#if hit.clinvar_significance}
              <span class="meta clinvar">{hit.clinvar_significance}</span>
            {/if}
            {#if hit.rsid && onNavigateToVariant}
              <div class="nav-row">
                <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(hit.rsid, "report")}>📋 Report</button>
                <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(hit.rsid, "map")}>🗺️ Map</button>
                <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(hit.rsid, "browser")}>🔍 Browser</button>
                <button type="button" class="btn btn-secondary btn-xs" onclick={() => onOpenEvidence?.(hit.rsid)}>🔬 Evidence</button>
              </div>
            {/if}
          </article>
        {/each}
        {#if filteredHits.length === 0}
          <p class="no-filter">No hits match filter.</p>
        {/if}
      </div>
    {/if}
  </div>
{:else if query}
  <div class="vector-citations vector-citations--muted">
    No vector research hits matched this query for your sample.
  </div>
{/if}

<style>
  .vector-citations {
    margin: 0 12px 8px;
    padding: 10px 12px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: rgba(56, 189, 248, 0.06);
    font-size: 0.82rem;
  }
  .vector-citations--muted {
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-muted, #94a3b8);
  }
  .vector-citations--error {
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.35);
  }
  .vector-citations-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .collapse-btn {
    background: none;
    border: none;
    color: inherit;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }
  .query-pill {
    font-size: 0.75rem;
    opacity: 0.85;
    font-style: italic;
  }
  .filter-input {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: rgba(0, 0, 0, 0.2);
    color: inherit;
  }
  .index-brief {
    margin: 0 0 8px;
    font-size: 0.76rem;
    line-height: 1.4;
    opacity: 0.88;
  }
  .card-scores {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 4px;
  }
  .score-chip {
    font-size: 0.68rem;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(56, 189, 248, 0.15);
  }
  .score-chip.direction {
    background: rgba(34, 197, 94, 0.15);
    text-transform: capitalize;
  }
  .synthesis {
    margin: 0 0 4px;
    font-size: 0.76rem;
    line-height: 1.35;
    opacity: 0.9;
  }
  .hits-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 220px;
    overflow-y: auto;
  }
  .hit-card {
    padding: 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }
  .hit-card header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }
  .rsid {
    color: var(--accent-color, #38bdf8);
  }
  .gene {
    font-size: 0.78rem;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.2);
  }
  .genotype {
    font-family: monospace;
    font-size: 0.75rem;
    opacity: 0.9;
  }
  .score {
    margin-left: auto;
    font-size: 0.72rem;
    opacity: 0.75;
  }
  .cat-tag {
    display: inline-block;
    margin-right: 4px;
    margin-bottom: 4px;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 0.68rem;
    text-transform: capitalize;
    background: rgba(34, 197, 94, 0.15);
    color: #86efac;
  }
  .category-tags {
    margin-bottom: 4px;
  }
  .traits {
    margin: 0 0 4px;
    line-height: 1.35;
    opacity: 0.9;
  }
  .assoc-list {
    margin: 4px 0;
    padding-left: 1.1rem;
    font-size: 0.78rem;
    opacity: 0.85;
  }
  .assoc-gene {
    color: var(--accent-secondary, #a78bfa);
  }
  .meta {
    display: inline-block;
    margin-right: 8px;
    font-size: 0.72rem;
    opacity: 0.8;
  }
  .meta.clinvar {
    color: #fbbf24;
  }
  .no-filter {
    margin: 0;
    opacity: 0.7;
  }
  .btn-xs {
    font-size: 0.72rem;
    padding: 2px 8px;
  }
  .nav-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
</style>
