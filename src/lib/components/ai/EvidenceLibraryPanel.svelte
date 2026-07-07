<!-- ./src/lib/components/ai/EvidenceLibraryPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import {
    listEvidenceSources,
    exportEvidencePacket,
    getVariantEvidenceCard,
    getEvidenceCorpusSummary,
    saveReportJson,
    type EvidenceRecord,
  } from "../../api/tauri";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import type { QdrantHit, EvidenceCard, EvidenceCorpusSummary } from "../../types/research";
  import type { GenomeSample } from "../../types/genomics";
  import VectorEvidenceCard from "./evidence/VectorEvidenceCard.svelte";
  import SimilarAssociationsPanel from "./evidence/SimilarAssociationsPanel.svelte";
  import EvidenceQualityDashboard from "./evidence/EvidenceQualityDashboard.svelte";
  import CandidateFindingsPanel from "./evidence/CandidateFindingsPanel.svelte";
  import TraitClusterPanel from "./evidence/TraitClusterPanel.svelte";
  import VectorAtlasPanel from "./evidence/VectorAtlasPanel.svelte";
  import ActionabilityMatrix from "./evidence/ActionabilityMatrix.svelte";
  import PathwayFlowPanel from "./evidence/PathwayFlowPanel.svelte";
  import EvidenceSearchToolbar from "./evidence/EvidenceSearchToolbar.svelte";
  import EvidenceResultsList from "./evidence/EvidenceResultsList.svelte";
  import QdrantResultsList from "./evidence/QdrantResultsList.svelte";
  import EvidenceSourcesSidebar from "./evidence/EvidenceSourcesSidebar.svelte";
  import EvidenceCorpusOverview from "./evidence/EvidenceCorpusOverview.svelte";
  import FindingsNavigator from "./evidence/FindingsNavigator.svelte";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";
  import { runEvidenceSearch, type BrowsePreset } from "../../utils/evidenceSearch";
  import { TRAIT_CATEGORIES } from "../../constants/traitCategories";
  import "$lib/styles/components/evidence-library-panel.css";

  interface Props {
    ollamaUrl: string;
    ollamaToken: string;
    showHistorySidebar: boolean;
    showSettingsDrawer: boolean;
    selectedSample: GenomeSample | null;
    initialSearchQuery?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let {
    ollamaUrl,
    ollamaToken,
    showHistorySidebar = $bindable(),
    showSettingsDrawer = $bindable(),
    selectedSample,
    initialSearchQuery = $bindable(""),
    onNavigateToVariant,
  }: Props = $props();

  let searchQuery = $state("");
  let traitCategory = $state("");
  let discoveryMode = $state<"semantic" | "trait_index">("semantic");
  let isSearching = $state(false);
  let results = $state<EvidenceRecord[]>([]);
  let qdrantResults = $state<QdrantHit[]>([]);
  let workbenchCards = $state<EvidenceCard[]>([]);
  let searchSource = $state<"sqlite" | "qdrant" | "workbench">("workbench");
  let workbenchView = $state<"search" | "quality" | "candidates" | "clusters" | "atlas" | "matrix" | "pathways">("search");
  let similarRsid = $state("");
  let exportMsg = $state("");
  let wbDirectionFilter = $state("");
  let wbMinDq = $state(0.3);
  let wbMinWellness = $state<number | undefined>(undefined);
  let wbEvidenceTier = $state("");
  let sources = $state<string[]>([]);
  let searchError = $state("");
  let hasSearched = $state(false);
  let corpusSummary = $state<EvidenceCorpusSummary | null>(null);
  let corpusLoading = $state(false);
  let activeBrowsePreset = $state<BrowsePreset | "">("");
  let libraryMode = $state<"browse" | "search">("browse");
  let showAdvancedSearch = $state(false);

  $effect(() => {
    if (initialSearchQuery) {
      searchQuery = initialSearchQuery;
      searchSource = "qdrant";
      performSearch();
      initialSearchQuery = "";
    }
  });

  let isSemanticSearchAvailable = $derived(!!ollamaUrl && ollamaUrl.trim() !== "");

  async function performSearch(browsePreset: BrowsePreset | undefined = undefined) {
    const q = searchQuery.trim();
    const traitBrowse =
      searchSource === "qdrant" && discoveryMode === "trait_index" && !!traitCategory;
    const preset = browsePreset ?? (activeBrowsePreset || undefined);
    if (!q && !traitBrowse && !preset) return;

    isSearching = true;
    searchError = "";
    hasSearched = true;
    if (browsePreset) {
      activeBrowsePreset = browsePreset;
    } else if (q) {
      activeBrowsePreset = "";
    }

    try {
      const out = await runEvidenceSearch({
        query: q,
        searchSource,
        discoveryMode,
        traitCategory,
        sampleId: selectedSample?.id ?? null,
        ollamaUrl,
        ollamaToken,
        wbDirectionFilter,
        wbMinDq,
        wbMinWellness,
        wbEvidenceTier,
        browsePreset: preset,
      });
      results = out.results;
      qdrantResults = out.qdrantResults;
      workbenchCards = out.workbenchCards;
      if (searchSource === "workbench") workbenchView = "search";
    } catch (e: unknown) {
      searchError = e instanceof Error ? e.message : String(e);
      results = [];
      qdrantResults = [];
      workbenchCards = [];
    } finally {
      isSearching = false;
    }
  }

  async function loadCorpusOverview(autoBrowse = false) {
    if (!selectedSample) {
      corpusSummary = null;
      return;
    }
    corpusLoading = true;
    try {
      corpusSummary = await getEvidenceCorpusSummary(selectedSample.id);
      const indexed = corpusSummary.dashboard.vectorized_variants ?? 0;
      if (autoBrowse && indexed > 0 && !initialSearchQuery && !hasSearched) {
        libraryMode = "browse";
      }
    } catch (e) {
      console.warn("Failed to load evidence corpus summary:", e);
      corpusSummary = null;
    } finally {
      corpusLoading = false;
    }
  }

  function handleBrowsePreset(preset: BrowsePreset) {
    libraryMode = "browse";
    activeBrowsePreset = preset;
  }

  function handleCorpusRsid(rsid: string) {
    searchQuery = rsid;
    searchSource = "workbench";
    workbenchView = "search";
    activeBrowsePreset = "";
    handleCandidateSelect(rsid);
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

  function browseTraitCategory(categoryId: string) {
    traitCategory = categoryId;
    searchSource = "qdrant";
    discoveryMode = "trait_index";
    const hint = TRAIT_CATEGORIES.find((c) => c.id === categoryId)?.queryHint;
    if (!searchQuery.trim() && hint) searchQuery = hint;
    performSearch();
  }

  function clearTraitCategory() {
    traitCategory = "";
    discoveryMode = "semantic";
  }

  async function handleExportPacket(rsid: string) {
    if (!selectedSample) return;
    exportMsg = "Exporting evidence packet…";
    try {
      const packet = await exportEvidencePacket(selectedSample.id, rsid);
      const saved = await saveReportJson(
        JSON.stringify(packet, null, 2),
        `${packet.packet_id}.json`
      );
      exportMsg = saved
        ? `Saved ${packet.packet_id}.json (${packet.association_facts.length} facts, ${packet.source_records?.length ?? 0} source records)`
        : `Packet ${packet.packet_id} ready (${packet.association_facts.length} facts) — save cancelled`;
    } catch (e: unknown) {
      exportMsg = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleCandidateSelect(rsid: string) {
    searchQuery = rsid;
    searchSource = "workbench";
    workbenchView = "search";
    if (selectedSample && /^rs\d/i.test(rsid)) {
      try {
        const card = await getVariantEvidenceCard(selectedSample.id, rsid);
        if (card) {
          workbenchCards = [card];
          hasSearched = true;
          searchError = "";
          return;
        }
      } catch {
        /* fall through */
      }
    }
    performSearch();
  }

  function handleShowSimilar(rsid: string) {
    similarRsid = rsid;
    workbenchView = "search";
    searchSource = "workbench";
  }

  function focusRsidInWorkbench(rsid: string) {
    searchQuery = rsid;
    similarRsid = rsid;
    workbenchView = "search";
    performSearch();
  }

  onMount(() => {
    loadSources();
    void loadCorpusOverview(true);
  });

  $effect(() => {
    const sampleId = selectedSample?.id;
    if (sampleId) {
      void loadCorpusOverview(!hasSearched && !initialSearchQuery);
    } else {
      corpusSummary = null;
    }
  });
</script>

<section class="evidence-panel-area">
  <div class="panel-header">
    <div class="panel-title-info">
      <button
        type="button"
        class="btn btn-secondary btn-sm toggle-sidebar-btn"
        onclick={() => (showHistorySidebar = !showHistorySidebar)}
        class:drawer-open={showHistorySidebar}
        title="Toggle History Sidebar"
        style="margin-right: 8px;"
      >
        📂
      </button>
      <h3>Local Evidence Library</h3>
      <span class="badge info">Browse · Filter · Export</span>
    </div>

    <div class="panel-header-actions">
      <div class="library-mode-toggle">
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          class:drawer-open={libraryMode === "browse"}
          onclick={() => (libraryMode = "browse")}
        >
          Findings Navigator
        </button>
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          class:drawer-open={libraryMode === "search"}
          onclick={() => (libraryMode = "search")}
        >
          Search &amp; tools
        </button>
      </div>
      <button
        class="btn btn-secondary btn-sm toggle-settings-btn"
        onclick={() => (showSettingsDrawer = !showSettingsDrawer)}
        class:drawer-open={showSettingsDrawer}
        title="Toggle Model Settings"
      >
        ⚙️ Settings
      </button>
    </div>
  </div>

  <div class="panel-content-scroll">
    {#if selectedSample && libraryMode === "browse"}
      <FindingsNavigator
        sampleId={selectedSample.id}
        summary={corpusSummary}
        loadingSummary={corpusLoading}
        initialPreset={activeBrowsePreset || undefined}
        initialTraitCategory={traitCategory}
        {onNavigateToVariant}
        onExportPacket={handleExportPacket}
      />
    {:else if selectedSample}
      {#if corpusSummary && (corpusSummary.dashboard.vectorized_variants ?? 0) > 0}
        <EvidenceCorpusOverview
          summary={corpusSummary}
          loading={corpusLoading}
          activePreset={activeBrowsePreset}
          onBrowsePreset={handleBrowsePreset}
          onSelectRsid={handleCorpusRsid}
          onBrowseTrait={(cat) => {
            traitCategory = cat;
            activeBrowsePreset = "";
            libraryMode = "browse";
          }}
        />
      {/if}

      <EvidenceSearchToolbar
      bind:searchQuery
      bind:searchSource
      bind:discoveryMode
      bind:traitCategory
      bind:workbenchView
      {isSearching}
      {isSemanticSearchAvailable}
      {exportMsg}
      bind:wbDirectionFilter
      bind:wbMinDq
      bind:wbEvidenceTier
      onSearch={performSearch}
      onBrowseTrait={browseTraitCategory}
      onClearTrait={clearTraitCategory}
    />

    <div class="evidence-grid">
      {#if searchSource === "sqlite"}
        <EvidenceResultsList
          {results}
          {isSearching}
          {searchError}
          {hasSearched}
          onQuickSearch={handleQuickSearch}
        />
      {:else if searchSource === "qdrant"}
        <QdrantResultsList
          results={qdrantResults}
          {isSearching}
          {searchError}
          {hasSearched}
          {onNavigateToVariant}
        />
      {:else}
        <div class="results-column">
          {#if isSearching}
            <PanelLoadingState
              message="Running hybrid vector search…"
              submessage="Payload filters and evidence card normalization in progress."
              accent="#38bdf8"
              compact
            />
          {:else if searchError}
            <div class="error-state-card">
              <h4>Search Error</h4>
              <p>{searchError}</p>
            </div>
          {:else if workbenchView === "quality" && selectedSample}
            <EvidenceQualityDashboard sampleId={selectedSample.id} {ollamaUrl} />
          {:else if workbenchView === "candidates"}
            <CandidateFindingsPanel onSelectRsid={handleCandidateSelect} />
          {:else if workbenchView === "clusters" && selectedSample}
            <TraitClusterPanel sampleId={selectedSample.id} {traitCategory} />
          {:else if workbenchView === "atlas" && selectedSample}
            <VectorAtlasPanel
              sampleId={selectedSample.id}
              {ollamaUrl}
              onSelectRsid={focusRsidInWorkbench}
            />
          {:else if workbenchView === "matrix" && selectedSample}
            <ActionabilityMatrix sampleId={selectedSample.id} />
          {:else if workbenchView === "pathways" && selectedSample}
            <PathwayFlowPanel sampleId={selectedSample.id} onSelectRsid={focusRsidInWorkbench} />
          {:else if workbenchCards.length > 0}
            <div class="results-header">
              <h4>{workbenchCards.length} structured evidence hits</h4>
            </div>
            {#if similarRsid && selectedSample}
              <SimilarAssociationsPanel
                sampleId={selectedSample.id}
                rsid={similarRsid}
                {onNavigateToVariant}
                onExportPacket={handleExportPacket}
              />
            {/if}
            <div class="results-list">
              {#each workbenchCards as card, idx (card.rsid + (card.qdrant_point_id || String(idx)))}
                <VectorEvidenceCard
                  {card}
                  {onNavigateToVariant}
                  onExportPacket={handleExportPacket}
                  onShowSimilar={handleShowSimilar}
                />
              {/each}
            </div>
          {:else if hasSearched}
            <div class="empty-results-state">
              <div class="empty-icon">🔍</div>
              <h4>No matched records</h4>
              <p>No structured evidence hits matched your search query in the database.</p>
            </div>
          {:else}
            <div class="intro-state">
              <div class="intro-logo">🔬</div>
              <h4>Evidence Workbench</h4>
              <p>Hybrid Qdrant search with SQLite association facts and inspectable provenance cards.</p>
            </div>
          {/if}
        </div>
      {/if}

      <EvidenceSourcesSidebar {sources} />
    </div>
    {/if}
  </div>
</section>

