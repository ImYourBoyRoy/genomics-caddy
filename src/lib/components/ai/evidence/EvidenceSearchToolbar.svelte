<!-- ./src/lib/components/ai/evidence/EvidenceSearchToolbar.svelte -->
<script lang="ts">
  import { TRAIT_CATEGORIES } from "../../../constants/traitCategories";
  import Tooltip from "../../common/Tooltip.svelte";

  interface Props {
    searchQuery: string;
    searchSource: "sqlite" | "qdrant" | "workbench";
    discoveryMode: "semantic" | "trait_index";
    traitCategory: string;
    workbenchView: "search" | "quality" | "candidates" | "clusters" | "atlas" | "matrix" | "pathways";
    isSearching: boolean;
    isSemanticSearchAvailable: boolean;
    exportMsg?: string;
    wbDirectionFilter: string;
    wbMinDq: number;
    wbEvidenceTier: string;
    onSearch: () => void;
    onBrowseTrait: (categoryId: string) => void;
    onClearTrait: () => void;
  }

  let {
    searchQuery = $bindable(),
    searchSource = $bindable(),
    discoveryMode = $bindable(),
    traitCategory = $bindable(),
    workbenchView = $bindable(),
    isSearching,
    isSemanticSearchAvailable,
    exportMsg = "",
    wbDirectionFilter = $bindable(),
    wbMinDq = $bindable(),
    wbEvidenceTier = $bindable(),
    onSearch,
    onBrowseTrait,
    onClearTrait,
  }: Props = $props();

  const canSubmit = $derived(
    !isSearching &&
      (!!searchQuery.trim() ||
        (searchSource === "qdrant" && discoveryMode === "trait_index" && !!traitCategory))
  );
</script>

<div class="search-section-card">
  <div class="search-source-selector">
    <button
      type="button"
      class="source-tab"
      class:active={searchSource === "sqlite"}
      onclick={() => {
        searchSource = "sqlite";
        if (searchQuery) onSearch();
      }}
    >
      📚 SQLite Guidelines
    </button>
    <button
      type="button"
      class="source-tab"
      class:active={searchSource === "qdrant"}
      onclick={() => {
        searchSource = "qdrant";
        if (searchQuery) onSearch();
      }}
    >
      🤖 Qdrant Research Vector
    </button>
    <button
      type="button"
      class="source-tab"
      class:active={searchSource === "workbench"}
      onclick={() => {
        searchSource = "workbench";
        workbenchView = "search";
        if (searchQuery) onSearch();
      }}
    >
      🔬 Evidence Workbench
    </button>
  </div>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      onSearch();
    }}
    class="search-form"
  >
    <div class="search-input-wrapper">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder={searchSource === "workbench"
          ? "Hybrid semantic search with evidence cards (e.g. bone density, COMT, caffeine metabolism)..."
          : searchSource === "qdrant"
            ? "Search deep vector research (e.g. coffee, methylation, cardiovascular)..."
            : "Search by gene name, rsID, or disease keywords (e.g. DPYD, rs55886062, warfarin)..."}
        class="search-input"
        disabled={isSearching}
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-input-btn"
          onclick={() => (searchQuery = "")}
          disabled={isSearching}
        >
          ✕
        </button>
      {/if}
    </div>
    <button type="submit" class="btn btn-primary search-btn" disabled={!canSubmit}>
      {#if isSearching}
        <span class="spinner-small"></span> Searching...
      {:else}
        🔍 Search
      {/if}
    </button>
  </form>

  {#if searchSource === "workbench"}
    <div class="workbench-nav">
      <button type="button" class="mode-chip" class:active={workbenchView === "search"} onclick={() => (workbenchView = "search")}>Search</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "quality"} onclick={() => (workbenchView = "quality")}>Quality</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "candidates"} onclick={() => (workbenchView = "candidates")}>Candidates</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "clusters"} onclick={() => (workbenchView = "clusters")}>Clusters</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "atlas"} onclick={() => (workbenchView = "atlas")}>Atlas</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "matrix"} onclick={() => (workbenchView = "matrix")}>Matrix</button>
      <button type="button" class="mode-chip" class:active={workbenchView === "pathways"} onclick={() => (workbenchView = "pathways")}>Pathways</button>
    </div>
    {#if exportMsg}
      <p class="export-banner">{exportMsg}</p>
    {/if}
    <div class="workbench-filters">
      <label>
        Min data quality
        <input type="range" min="0" max="1" step="0.05" bind:value={wbMinDq} />
        {(wbMinDq * 100).toFixed(0)}%
      </label>
      <label>
        Direction
        <select bind:value={wbDirectionFilter}>
          <option value="">Any</option>
          <option value="known">Known direction only</option>
          <option value="unknown">Unknown direction only</option>
        </select>
      </label>
      <label>
        Evidence tier
        <input type="text" bind:value={wbEvidenceTier} placeholder="e.g. curated_gwas" />
      </label>
    </div>
  {/if}

  {#if searchSource === "qdrant"}
    <div class="trait-discovery-bar">
      <div class="discovery-mode-row">
        <span class="discovery-label">Discovery mode</span>
        <button
          type="button"
          class="mode-chip"
          class:active={discoveryMode === "semantic"}
          onclick={() => {
            discoveryMode = "semantic";
          }}
        >
          Semantic search
        </button>
        <button
          type="button"
          class="mode-chip"
          class:active={discoveryMode === "trait_index"}
          onclick={() => {
            discoveryMode = "trait_index";
            searchSource = "qdrant";
          }}
        >
          Trait index browse
        </button>
      </div>
      <div class="trait-category-row">
        <span class="discovery-label">Trait categories</span>
        {#each TRAIT_CATEGORIES as cat}
          <Tooltip interactiveChildren interactiveClickBehavior="dismiss" label={cat.label} description={cat.queryHint} placement="bottom">
            <button
              type="button"
              class="trait-chip"
              class:active={traitCategory === cat.id}
              onclick={() => onBrowseTrait(cat.id)}
            >
              {cat.label}
            </button>
          </Tooltip>
        {/each}
        {#if traitCategory}
          <button type="button" class="trait-chip clear" onclick={onClearTrait}>Clear filter</button>
        {/if}
      </div>
    </div>
  {/if}

  <div class="search-status-bar">
    {#if searchSource === "qdrant"}
      <div class="status-indicator active">
        <span class="dot pulse"></span>
        <span class="status-label">Qdrant Vector Database Mode</span>
        <span class="status-sub">Querying autonomous PubMed, GWAS & ClinVar crawl context</span>
      </div>
    {:else if searchSource === "workbench"}
      <div class="status-indicator active">
        <span class="dot pulse"></span>
        <span class="status-label">Evidence Workbench</span>
        <span class="status-sub">Hybrid Qdrant search + SQLite association facts with inspectable provenance</span>
      </div>
    {:else if isSemanticSearchAvailable}
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
  {#if searchSource === "qdrant"}
    <p class="mode-hint">
      Raw <code>[VARIANT]</code> blocks are embedding text for the vector index — useful for search, not bedside reading.
      Switch to <strong>Evidence Workbench</strong> for structured cards, or use <strong>AI Consultation</strong> to ask what a hit means for you.
    </p>
  {/if}
</div>
