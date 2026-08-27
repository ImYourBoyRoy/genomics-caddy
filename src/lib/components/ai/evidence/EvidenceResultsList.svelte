<!-- ./src/lib/components/ai/evidence/EvidenceResultsList.svelte -->
<script lang="ts">
  import type { EvidenceRecord } from "../../../api/tauri";
  import PanelLoadingState from "../../common/loading/PanelLoadingState.svelte";
  import Tooltip from "../../common/Tooltip.svelte";
  import { parseCitationUrl } from "../../../utils/urlSafety";

  interface Props {
    results: EvidenceRecord[];
    isSearching: boolean;
    searchError: string;
    hasSearched: boolean;
    onQuickSearch: (term: string) => void;
  }

  let {
    results,
    isSearching,
    searchError,
    hasSearched,
    onQuickSearch
  }: Props = $props();
</script>

<div class="results-column">
  {#if isSearching}
    <PanelLoadingState
      message="Searching evidence library…"
      submessage="Querying local SQLite indices and semantic embeddings when available."
      accent="#38bdf8"
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
      <h4>Found {results.length} matched guidelines</h4>
    </div>
    <div class="results-list">
      {#each results as rec, idx}
        <div class="result-card" style="animation-delay: {idx * 50}ms">
          <div class="result-meta">
            <div class="marker-badge">
              <span class="gene-name">{rec.gene}</span>
              <span class="rsid">{rec.rsid}</span>
            </div>
            
            <div class="right-meta">
              {#if rec.similarity !== null}
                <Tooltip label="Concept match" description="Calculated cosine similarity of the embedded evidence text; it helps organize results and is not a clinical probability.">
                  <span class="match-badge vector">
                    🤖 {(rec.similarity * 100).toFixed(0)}% concept match
                  </span>
                </Tooltip>
              {:else}
                <Tooltip label="Text match" description="This result matched the exact words in your search rather than a vector similarity score.">
                  <span class="match-badge keyword">
                    📝 Text match
                  </span>
                </Tooltip>
              {/if}
              
              {#if rec.has_embedding}
                <Tooltip label="Vectorized" description="A vector embedding for this evidence record is generated and cached in the local SQLite index.">
                  <span class="embedding-indicator active">
                    Vectorized
                  </span>
                </Tooltip>
              {/if}
            </div>
          </div>

          <div class="evidence-text">
            {rec.evidence_text}
          </div>

          <div class="source-citation-footer">
            <span class="citation-label">Source Citation:</span>
            {#if rec.source_citation && rec.source_citation.includes("http")}
              {@const citation = parseCitationUrl(rec.source_citation)}
              {#if citation.url}
                <a href={citation.url} target="_blank" rel="noopener noreferrer" class="citation-link">
                  🔗 {citation.title}
                </a>
              {:else}
                <span class="citation-static">🔗 {citation.title || rec.source_citation}</span>
              {/if}
            {:else}
              <span class="citation-static">🔗 {rec.source_citation || "Unknown Source"}</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {:else if hasSearched}
    <div class="empty-results-state">
      <div class="empty-icon">🔍</div>
      <h4>No matched evidence records</h4>
      <p>No guidelines or marker definitions matched your search term in the local SQLite database.</p>
      <button class="btn btn-secondary btn-sm" onclick={() => onQuickSearch("DPYD")}>Try searching "DPYD"</button>
    </div>
  {:else}
    <div class="intro-state">
      <div class="intro-logo">🧬</div>
      <h4>Local Guideline Evidence Base</h4>
      <p>Directly search our local SQLite evidence index. This table is seeded on application startup with curated annotations from all active marker packs.</p>
      
      <div class="quick-searches-box">
        <h5>Quick Sample Queries:</h5>
        <div class="quick-buttons">
          <button class="btn btn-secondary btn-sm" onclick={() => onQuickSearch("DPYD")}>rs55886062 (DPYD)</button>
          <button class="btn btn-secondary btn-sm" onclick={() => onQuickSearch("warfarin")}>Warfarin Dosing</button>
          <button class="btn btn-secondary btn-sm" onclick={() => onQuickSearch("MTHFR")}>Methylation (MTHFR)</button>
          <button class="btn btn-secondary btn-sm" onclick={() => onQuickSearch("COMT")}>Dopamine (COMT)</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .results-column {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .results-header h4 {
    font-size: 0.88rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0;
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .result-card {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid var(--border-color);
    padding: 18px;
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: slide-up-anim 0.3s ease-out forwards;
    opacity: 0;
    transform: translateY(10px);
    transition: all 0.2s;
  }

  @keyframes slide-up-anim {
    to { opacity: 1; transform: translateY(0); }
  }

  .result-card:hover {
    border-color: rgba(88, 80, 236, 0.3);
    background: rgba(255, 255, 255, 0.03);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.2);
  }

  .result-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .marker-badge {
    display: inline-flex;
    background: rgba(88, 80, 236, 0.12);
    border: 1px solid rgba(88, 80, 236, 0.25);
    border-radius: 6px;
    overflow: hidden;
    font-family: monospace;
    font-size: 0.8rem;
  }

  .marker-badge .gene-name {
    background: rgba(88, 80, 236, 0.15);
    color: #a5b4fc;
    padding: 3px 8px;
    font-weight: 600;
  }

  .marker-badge .rsid {
    color: #c7d2fe;
    padding: 3px 8px;
  }

  .right-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .match-badge {
    font-size: 0.72rem;
    padding: 3px 8px;
    border-radius: 4px;
    font-weight: 500;
  }

  .match-badge.vector {
    background: rgba(16, 185, 129, 0.1);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.2);
  }

  .match-badge.keyword {
    background: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .embedding-indicator {
    font-size: 0.7rem;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 4px;
    opacity: 0.8;
  }

  .evidence-text {
    font-size: 0.88rem;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .source-citation-footer {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.74rem;
    color: var(--text-secondary);
    border-top: 1px solid rgba(255, 255, 255, 0.03);
    padding-top: 8px;
    margin-top: 4px;
  }

  .citation-link {
    color: #818cf8;
    text-decoration: none;
    transition: color 0.15s;
    font-weight: 500;
  }

  .citation-link:hover {
    color: #a5b4fc;
    text-decoration: underline;
  }

  .error-state-card {
    background: rgba(239, 68, 68, 0.04);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #fca5a5;
    padding: 20px;
    border-radius: 10px;
    text-align: left;
  }

  .error-state-card h4 {
    margin: 0 0 8px 0;
    color: #ef4444;
  }

  .error-state-card p {
    margin: 0 0 8px 0;
    font-size: 0.85rem;
    line-height: 1.5;
  }

  .error-state-card .suggestion {
    margin: 8px 0 0 0;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .empty-results-state, .intro-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 50px 20px;
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid var(--border-color);
    border-radius: 12px;
  }

  .empty-icon {
    font-size: 2.5rem;
    margin-bottom: 12px;
  }

  .empty-results-state h4, .intro-state h4 {
    font-size: 1.05rem;
    color: var(--text-primary);
    margin: 0 0 8px 0;
  }

  .empty-results-state p, .intro-state p {
    font-size: 0.86rem;
    color: var(--text-secondary);
    line-height: 1.5;
    max-width: 460px;
    margin: 0 0 16px 0;
  }

  .intro-logo {
    font-size: 3rem;
    margin-bottom: 16px;
    background: radial-gradient(circle, rgba(88, 80, 236, 0.2), transparent 70%);
    padding: 10px;
  }

  .quick-searches-box {
    margin-top: 10px;
    background: rgba(0, 0, 0, 0.1);
    padding: 14px 18px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.03);
    width: 100%;
    max-width: 440px;
  }

  .quick-searches-box h5 {
    font-size: 0.76rem;
    text-transform: uppercase;
    color: var(--text-secondary);
    margin: 0 0 10px 0;
    letter-spacing: 0.05em;
  }

  .quick-buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.2s, transform 0.1s;
  }

  .btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }

  .btn-sm {
    padding: 6px 12px;
    font-size: 0.78rem;
    border-radius: 6px;
    white-space: nowrap;
  }
</style>
