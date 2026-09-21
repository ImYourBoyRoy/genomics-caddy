<!-- ./src/lib/components/ai/evidence/QdrantResultsList.svelte -->
<script lang="ts">
  import type { QdrantHit } from "../../../types/research";
  import type { VariantNavTarget } from "../../../constants/traitCategories";
  import PanelLoadingState from "../../common/loading/PanelLoadingState.svelte";
  import Tooltip from "../../common/Tooltip.svelte";

  interface Props {
    results: QdrantHit[];
    isSearching: boolean;
    searchError: string;
    hasSearched: boolean;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let {
    results,
    isSearching,
    searchError,
    hasSearched,
    onNavigateToVariant,
  }: Props = $props();
</script>

<div class="results-column">
  {#if isSearching}
    <PanelLoadingState
      message="Searching Qdrant vector index…"
      submessage="Embedding query via Ollama and matching cosine similarities inside Qdrant collection."
      accent="var(--status-accent-text)"
      compact
    />
  {:else if searchError}
    <div class="error-state-card">
      <h4>Search Error</h4>
      <p>{searchError}</p>
      {#if searchError.includes("Ollama")}
        <p class="suggestion">💡 Ensure your remote Ollama instance is online and the model tag is correct in the Settings drawer.</p>
      {/if}
    </div>
  {:else if results.length > 0}
    <div class="results-header">
      <h4>Found {results.length} matched research findings</h4>
    </div>
    <div class="results-list">
      {#each results as rec, idx}
        <div class="result-card" style="animation-delay: {idx * 50}ms">
          <div class="result-meta">
            <div class="marker-badge">
              {#if rec.gene}
                <span class="gene-name">{rec.gene}</span>
              {/if}
              <span class="rsid">{rec.rsid}</span>
            </div>

            <div class="right-meta">
              <Tooltip label="Concept match" description="Calculated cosine similarity score from Qdrant vector search; it helps organize research results and is not a clinical probability.">
                <span class="match-badge vector">
                  🤖 {(rec.score * 100).toFixed(0)}% concept match
                </span>
              </Tooltip>
              {#if rec.significance_score > 0}
                <Tooltip label="Research significance" description="A local organization signal based on available ClinVar and GWAS counts; it is not a disease probability.">
                  <span class="match-badge keyword">
                    ⭐ Significance: {rec.significance_score.toFixed(2)}
                  </span>
                </Tooltip>
              {/if}
            </div>
          </div>

          <div class="evidence-text pre-wrap">{rec.text}</div>

          {#if rec.trait_categories && rec.trait_categories.length > 0}
            <div class="trait-cat-tags">
              {#each rec.trait_categories as cat}
                <span class="trait-cat-tag">{cat.replace(/_/g, " ")}</span>
              {/each}
            </div>
          {/if}

          {#if rec.gwas_trait || rec.gnomad_af !== null || rec.category}
            <div class="qdrant-payload-footer">
              {#if rec.category}
                <span class="payload-tag">Category: <strong>{rec.category}</strong></span>
              {/if}
              {#if rec.gnomad_af !== null && rec.gnomad_af !== undefined}
                <span class="payload-tag">gnomAD AF: <strong>{rec.gnomad_af.toFixed(6)}</strong></span>
              {/if}
              {#if rec.gwas_trait}
                <span class="payload-tag">Traits: <strong>{rec.gwas_trait}</strong></span>
              {/if}
            </div>
          {/if}

          {#if rec.rsid && onNavigateToVariant}
            <div class="variant-nav-row">
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(rec.rsid, "report")}>📋 Report</button>
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(rec.rsid, "map")}>🗺️ Map</button>
              <button type="button" class="btn btn-secondary btn-xs" onclick={() => onNavigateToVariant?.(rec.rsid, "browser")}>🔍 Browser</button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {:else if hasSearched}
    <div class="empty-results-state">
      <div class="empty-icon">🔍</div>
      <h4>No matched records</h4>
      <p>No vectorized research findings matched your search query in the database.</p>
    </div>
  {:else}
    <div class="intro-state">
      <div class="intro-logo">🧬</div>
      <h4>Qdrant Research Vector Index</h4>
      <p>Search deep vector embeddings from autonomous PubMed, GWAS, and ClinVar enrichment sweeps.</p>
    </div>
  {/if}
</div>

<style>
  .pre-wrap {
    white-space: pre-wrap;
  }
</style>
