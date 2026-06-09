<!-- ./src/lib/components/ai/EvidenceLibraryPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { searchEvidence, listEvidenceSources, type EvidenceRecord } from "../../api/tauri";

  interface Props {
    ollamaUrl: string;
    ollamaToken: string;
    showHistorySidebar: boolean;
    showSettingsDrawer: boolean;
  }

  let {
    ollamaUrl,
    ollamaToken,
    showHistorySidebar = $bindable(),
    showSettingsDrawer = $bindable()
  }: Props = $props();

  let searchQuery = $state("");
  let isSearching = $state(false);
  let results = $state<EvidenceRecord[]>([]);
  let sources = $state<string[]>([]);
  let searchError = $state("");
  let hasSearched = $state(false);

  // Check if semantic search is available (Ollama is configured)
  let isSemanticSearchAvailable = $derived(!!ollamaUrl && ollamaUrl.trim() !== "");

  async function performSearch() {
    const q = searchQuery.trim();
    if (!q) return;

    isSearching = true;
    searchError = "";
    hasSearched = true;

    try {
      results = await searchEvidence(
        q,
        ollamaUrl || undefined,
        ollamaToken || undefined
      );
    } catch (e: any) {
      searchError = e.message || String(e);
      results = [];
    } finally {
      isSearching = false;
    }
  }

  async function loadSources() {
    try {
      sources = await listEvidenceSources();
    } catch (e) {
      console.error("Failed to load evidence sources:", e);
    }
  }

  function handleQuickSearch(term: string) {
    searchQuery = term;
    performSearch();
  }

  onMount(() => {
    loadSources();
  });
</script>

<section class="evidence-panel-area">
  <div class="panel-header">
    <div class="panel-title-info">
      <button 
        type="button"
        class="btn btn-secondary btn-sm toggle-sidebar-btn" 
        onclick={() => showHistorySidebar = !showHistorySidebar}
        class:drawer-open={showHistorySidebar}
        title="Toggle History Sidebar"
        style="margin-right: 8px;"
      >
        📂
      </button>
      <h3>Local Evidence Library</h3>
      <span class="badge info">📚 Guidelines & Citations</span>
    </div>
    
    <div class="panel-header-actions">
      <button 
        class="btn btn-secondary btn-sm toggle-settings-btn" 
        onclick={() => showSettingsDrawer = !showSettingsDrawer}
        class:drawer-open={showSettingsDrawer}
        title="Toggle Model Settings"
      >
        ⚙️ Settings
      </button>
    </div>
  </div>

  <div class="panel-content-scroll">
    <!-- Search Bar Section -->
    <div class="search-section-card">
      <form onsubmit={(e) => { e.preventDefault(); performSearch(); }} class="search-form">
        <div class="search-input-wrapper">
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search by gene name, rsID, or disease keywords (e.g. DPYD, rs55886062, warfarin)..."
            class="search-input"
            disabled={isSearching}
          />
          {#if searchQuery}
            <button 
              type="button" 
              class="clear-input-btn"
              onclick={() => searchQuery = ""}
              disabled={isSearching}
            >
              ✕
            </button>
          {/if}
        </div>
        <button type="submit" class="btn btn-primary search-btn" disabled={isSearching || !searchQuery.trim()}>
          {#if isSearching}
            <span class="spinner-small"></span> Searching...
          {:else}
            🔍 Search
          {/if}
        </button>
      </form>

      <div class="search-status-bar">
        {#if isSemanticSearchAvailable}
          <div class="status-indicator active">
            <span class="dot pulse"></span>
            <span class="status-label">Semantic AI Vector Search Active</span>
            <span class="status-sub">Matches concepts using local embeddings via Ollama</span>
          </div>
        {:else}
          <div class="status-indicator warning">
            <span class="dot"></span>
            <span class="status-label">Keyword-Only Matching Active</span>
            <span class="status-sub">Configure Ollama URL in settings for semantic vector search</span>
          </div>
        {/if}
      </div>
    </div>

    <!-- Main Content Grid -->
    <div class="evidence-grid">
      <!-- Results Column -->
      <div class="results-column">
        {#if isSearching}
          <div class="searching-state">
            <div class="glow-spinner"></div>
            <p>Querying local SQLite guideline indices and calculating cosine similarity weights...</p>
          </div>
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
                      <span class="match-badge vector" title="Calculated cosine similarity of embedded text">
                        🤖 {(rec.similarity * 100).toFixed(0)}% concept match
                      </span>
                    {:else}
                      <span class="match-badge keyword" title="Exact text search hit">
                        📝 Text match
                      </span>
                    {/if}
                    
                    {#if rec.has_embedding}
                      <span class="embedding-indicator active" title="Vector embedding generated and cached in SQLite">
                        Vectorized
                      </span>
                    {/if}
                  </div>
                </div>

                <div class="evidence-text">
                  {rec.evidence_text}
                </div>

                <div class="source-citation-footer">
                  <span class="citation-label">Source Citation:</span>
                  {#if rec.source_citation.includes("http")}
                    {@const parts = rec.source_citation.split(" (")}
                    {@const title = parts[0]}
                    {@const url = parts[1] ? parts[1].replace(")", "") : ""}
                    <a href={url} target="_blank" rel="noopener noreferrer" class="citation-link">
                      🔗 {title}
                    </a>
                  {:else}
                    <span class="citation-static">🔗 {rec.source_citation}</span>
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
            <button class="btn btn-secondary btn-sm" onclick={() => handleQuickSearch("DPYD")}>Try searching "DPYD"</button>
          </div>
        {:else}
          <div class="intro-state">
            <div class="intro-logo">🧬</div>
            <h4>Local Guideline Evidence Base</h4>
            <p>Directly search our local SQLite evidence index. This table is seeded on application startup with curated annotations from all active marker packs.</p>
            
            <div class="quick-searches-box">
              <h5>Quick Sample Queries:</h5>
              <div class="quick-buttons">
                <button class="btn btn-secondary btn-sm" onclick={() => handleQuickSearch("DPYD")}>rs55886062 (DPYD)</button>
                <button class="btn btn-secondary btn-sm" onclick={() => handleQuickSearch("warfarin")}>Warfarin Dosing</button>
                <button class="btn btn-secondary btn-sm" onclick={() => handleQuickSearch("MTHFR")}>Methylation (MTHFR)</button>
                <button class="btn btn-secondary btn-sm" onclick={() => handleQuickSearch("COMT")}>Dopamine (COMT)</button>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Sources Sidebar Column -->
      <div class="sources-sidebar-column">
        <div class="sidebar-widget">
          <h4>Loaded Guideline Packs</h4>
          <p class="widget-desc">Citations and references currently loaded in your local database:</p>
          <div class="sources-list">
            {#each sources as src}
              <div class="source-item">
                <span class="source-icon">📄</span>
                <span class="source-name" title={src}>{src}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .evidence-panel-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.01);
    min-width: 0;
    height: 100%;
    box-sizing: border-box;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 18px;
    background: rgba(0, 0, 0, 0.15);
    border-bottom: 1px solid var(--border-color);
  }

  .panel-title-info {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .panel-title-info h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .panel-header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .btn-sm {
    padding: 6px 12px;
    font-size: 0.78rem;
    border-radius: 6px;
    white-space: nowrap;
  }

  .toggle-sidebar-btn.drawer-open, .toggle-settings-btn.drawer-open {
    background: rgba(88, 80, 236, 0.2);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .panel-content-scroll {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
    background: radial-gradient(circle at bottom, rgba(18, 20, 32, 0.4), transparent);
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .search-section-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    padding: 16px;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .search-form {
    display: flex;
    gap: 10px;
  }

  .search-input-wrapper {
    position: relative;
    flex: 1;
  }

  .search-input {
    width: 100%;
    background: rgba(10, 11, 20, 0.45);
    border: 1px solid var(--border-color);
    padding: 12px 36px 12px 14px;
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
    transition: all 0.2s;
  }

  .search-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(88, 80, 236, 0.15);
  }

  .clear-input-btn {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 2px;
  }

  .clear-input-btn:hover {
    color: var(--text-primary);
  }

  .search-btn {
    padding: 0 20px;
    border-radius: 8px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 6px;
    box-shadow: var(--btn-shadow);
  }

  .spinner-small {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top: 2px solid white;
    border-radius: 50%;
    animation: spin-anim 0.8s linear infinite;
  }

  @keyframes spin-anim {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }

  .search-status-bar {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.76rem;
  }

  .status-indicator .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .status-indicator.active .dot {
    background: #10b981;
    box-shadow: 0 0 8px #10b981;
  }

  .status-indicator.active .dot.pulse {
    animation: pulse-anim 1.5s infinite alternate;
  }

  @keyframes pulse-anim {
    0% { transform: scale(1); opacity: 0.5; }
    100% { transform: scale(1.4); opacity: 1; }
  }

  .status-indicator.warning .dot {
    background: #f59e0b;
    box-shadow: 0 0 8px #f59e0b;
  }

  .status-indicator .status-label {
    font-weight: 500;
    color: var(--text-primary);
  }

  .status-indicator .status-sub {
    color: var(--text-secondary);
    margin-left: auto;
    opacity: 0.8;
  }

  /* Main Grid */
  .evidence-grid {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: 20px;
    align-items: start;
  }

  @media (max-width: 1024px) {
    .evidence-grid {
      grid-template-columns: 1fr;
    }
  }

  /* Results Column */
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

  /* Searching and Error States */
  .searching-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    text-align: center;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.01);
    border: 1px dashed var(--border-color);
    border-radius: 12px;
  }

  .glow-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(88, 80, 236, 0.1);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin-anim 0.8s cubic-bezier(0.5, 0, 0.5, 1) infinite;
    box-shadow: 0 0 10px rgba(88, 80, 236, 0.2);
    margin-bottom: 16px;
  }

  .searching-state p {
    font-size: 0.88rem;
    max-width: 400px;
    margin: 0;
  }

  .error-state-card {
    background: rgba(239, 68, 68, 0.04);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #fca5a5;
    padding: 20px;
    border-radius: 10px;
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

  /* Sidebar Widget */
  .sources-sidebar-column {
    display: flex;
    flex-direction: column;
  }

  .sidebar-widget {
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid var(--border-color);
    padding: 16px;
    border-radius: 10px;
  }

  .sidebar-widget h4 {
    font-size: 0.85rem;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: var(--text-primary);
  }

  .widget-desc {
    font-size: 0.74rem;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
    line-height: 1.4;
  }

  .sources-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 400px;
    overflow-y: auto;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(10, 11, 20, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.03);
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 0.74rem;
  }

  .source-item .source-icon {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .source-item .source-name {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
</style>
