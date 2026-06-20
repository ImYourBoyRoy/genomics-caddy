<!-- ./src/lib/components/ai/evidence/EvidenceSearchForm.svelte -->
<script lang="ts">
  interface Props {
    searchQuery: string;
    isSearching: boolean;
    isSemanticSearchAvailable: boolean;
    onSearch: () => void;
  }

  let {
    searchQuery = $bindable(),
    isSearching,
    isSemanticSearchAvailable,
    onSearch
  }: Props = $props();
</script>

<div class="search-section-card">
  <form onsubmit={(e) => { e.preventDefault(); onSearch(); }} class="search-form">
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

<style>
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
    background: var(--accent);
    color: var(--text-primary);
    border: 1px solid rgba(255, 255, 255, 0.1);
    cursor: pointer;
    transition: background-color 0.2s, transform 0.1s;
  }

  .search-btn:hover:not(:disabled) {
    background: var(--accent-hover, #6c63ff);
    transform: translateY(-1px);
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
</style>
