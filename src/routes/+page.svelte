<!-- ./src/routes/+page.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  
  /*
  Module Docstring:
  Purpose: Main orchestrator page for the Genomics Caddy frontend.
  Responsibilities:
  - Coordinate Tauri API operations (sample management, file selection, report requests).
  - Manage application tab navigation, current profile, and file ingestion progress state.
  - Load and merge genetic marker packs statically before invoking Rust evaluation.
  - Mount high-level UI panels (Sidebar, ReportView, GenomeMap, VariantSearchPanel, McpPanel).
  Key Inputs: Tauri events and user selection inputs.
  Key Outputs: Full reactive dashboard views.
  Operational Notes: Uses Svelte 5 runes and snippets for clean separation of layout and logic.
  */

  // API wrappers
  import {
    selectFile,
    getAppPaths,
    importGenome as apiImportGenome,
    getSamples,
    queryRsids,
    queryRegion,
    generateReport,
    deleteSample as apiDeleteSample,
    checkChainStatus,
    downloadChain as apiDownloadChain
  } from "$lib/api/tauri";

  // Svelte 5 components
  import AppShell from "$lib/components/layout/AppShell.svelte";
  import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
  import EmptyState from "$lib/components/common/EmptyState.svelte";
  import ReportView from "$lib/components/report/ReportView.svelte";
  import GenomeMap from "$lib/components/genome/GenomeMap.svelte";
  import VariantSearchPanel from "$lib/components/search/VariantSearchPanel.svelte";
  import McpPanel from "$lib/components/mcp/McpPanel.svelte";
  import AiAssistantPanel from "$lib/components/ai/AiAssistantPanel.svelte";
  import AgentResearchPanel from "$lib/components/agent/AgentResearchPanel.svelte";

  // Static Marker Packs & Manifest
  import manifest from "$lib/marker-packs/manifest.json";
  import core from "$lib/marker-packs/core.json";
  import pgx from "$lib/marker-packs/pgx.json";
  import metabolic from "$lib/marker-packs/metabolic.json";
  import nutrients from "$lib/marker-packs/nutrients.json";
  import neuropsych from "$lib/marker-packs/neuropsych.json";
  import sleep from "$lib/marker-packs/sleep.json";
  import connectiveTissue from "$lib/marker-packs/connective_tissue.json";
  import thyroidAutoimmune from "$lib/marker-packs/thyroid_autoimmune.json";
  import cardiovascular from "$lib/marker-packs/cardiovascular.json";
  import cancerConfirmationOnly from "$lib/marker-packs/cancer_confirmation_only.json";

  // Stylesheet imports
  import "$lib/styles/theme.css";
  import "$lib/styles/print.css";

  // Map pack IDs to static content
  const PACKS_MAP: Record<string, { name: string; markers: any[] }> = {
    core,
    pgx,
    metabolic,
    nutrients,
    neuropsych,
    sleep,
    connective_tissue: connectiveTissue,
    thyroid_autoimmune: thyroidAutoimmune,
    cardiovascular,
    cancer_confirmation_only: cancerConfirmationOnly
  };

  import type { GenomeSample, AppPaths, GeneratedReport, DbSnpRecord, SectionDefinition, ReportTemplate, MarkerDefinition } from "$lib/types/genomics";

  // State Runes (Svelte 5)
  let samples = $state<GenomeSample[]>([]);
  let selectedSample = $state<GenomeSample | null>(null);
  let filePath = $state("");
  let sampleNameInput = $state("");
  let appPaths = $state<AppPaths | null>(null);
  
  let isChainDownloaded = $state(false);
  let isDownloadingChain = $state(false);
  
  let isImporting = $state(false);
  let progressPercent = $state(0);
  let progressStatus = $state("");
  let importError = $state("");
  let importSuccess = $state("");

  let activeTab = $state("report"); // "report", "map", "browser", "mcp", "agent", "ai"

  let generatedReport = $state<GeneratedReport | null>(null);
  let isGeneratingReport = $state(false);
  let reportError = $state("");

  let searchRsid = $state("");
  let browseChr = $state("1");
  let browseStart = $state(1);
  let browseEnd = $state(5000000);
  let browserResults = $state<DbSnpRecord[]>([]);
  let isBrowsing = $state(false);

  // Persistent AI Consultation State (Lifts state from AiAssistantPanel to survive tab unmounts)
  let aiOllamaUrl = $state("http://localhost:11434");
  let aiOllamaToken = $state("");
  let aiSelectedModel = $state("");
  let aiMessages = $state<{ role: "user" | "assistant" | "system"; content: string; fullContent?: string; images?: string[] }[]>([]);
  let aiSelectedPacks = $state<Record<string, boolean>>({});
  let aiOnlyActiveFindings = $state(true);
  let aiTemperature = $state(0.0);

  let unlistenProgress: () => void;

  onMount(() => {
    async function init() {
      await refreshChainStatus();
      await refreshSamples();
      await loadAppPaths();
    }
    init();

    listen<{ percentage: number; status: string }>("import-progress", (event) => {
      progressPercent = event.payload.percentage;
      progressStatus = event.payload.status;
    }).then(unlisten => {
      unlistenProgress = unlisten;
    });

    return () => {
      if (unlistenProgress) unlistenProgress();
    };
  });

  async function loadAppPaths() {
    try {
      appPaths = await getAppPaths();
    } catch (e) {
      console.error("Failed to load paths", e);
    }
  }

  async function refreshChainStatus() {
    try {
      isChainDownloaded = await checkChainStatus();
    } catch (e) {
      console.error(e);
    }
  }

  async function downloadChain() {
    isDownloadingChain = true;
    try {
      await apiDownloadChain();
      await refreshChainStatus();
    } catch (e: any) {
      alert("Failed to download chain file: " + e.toString());
    } finally {
      isDownloadingChain = false;
    }
  }

  async function refreshSamples() {
    try {
      samples = await getSamples();
      if (samples.length > 0 && selectedSample === null) {
        selectSample(samples[0]);
      }
    } catch (e) {
      console.error(e);
    }
  }

  function selectSample(sample: GenomeSample) {
    selectedSample = sample;
    triggerReport();
  }

  async function browseFile() {
    try {
      const selected = await selectFile();
      if (selected) {
        filePath = selected;
        const parts = selected.split(/[\\/]/);
        const fileName = parts[parts.length - 1];
        sampleNameInput = fileName.replace(/\.[^/.]+$/, "");
      }
    } catch (e) {
      console.error("File selector error", e);
    }
  }

  async function importGenome(e: Event) {
    e.preventDefault();
    if (!filePath || !sampleNameInput) {
      importError = "Please specify both file path and sample name.";
      return;
    }
    isImporting = true;
    importError = "";
    importSuccess = "";
    progressPercent = 0;
    progressStatus = "Initializing ingestion...";
    
    try {
      const sampleId = await apiImportGenome(filePath, sampleNameInput);
      importSuccess = `Successfully imported sample as ID: ${sampleId}!`;
      filePath = "";
      sampleNameInput = "";
      await refreshSamples();
      
      const newSample = samples.find(s => s.id === sampleId);
      if (newSample) selectSample(newSample);
    } catch (e: any) {
      importError = e.toString();
    } finally {
      isImporting = false;
    }
  }

  function buildMergedTemplate(): ReportTemplate {
    const sections: SectionDefinition[] = [];
    for (const pack of manifest.packs) {
      const packContent = PACKS_MAP[pack.id];
      if (packContent && packContent.markers) {
        sections.push({
          name: packContent.name || pack.label,
          markers: packContent.markers as MarkerDefinition[]
        });
      }
    }
    return {
      title: "DNA Analysis & Biohacker Profile Report",
      description: "Personal genomic profile matching candidate markers across multiple health systems.",
      sections
    };
  }

  async function triggerReport() {
    if (!selectedSample) return;
    isGeneratingReport = true;
    reportError = "";
    try {
      const mergedTemplate = buildMergedTemplate();
      const templateJson = JSON.stringify(mergedTemplate);
      generatedReport = await generateReport(selectedSample.id, templateJson);
    } catch (e: any) {
      reportError = "Report generation failed: " + e.toString();
      console.error(e);
    } finally {
      isGeneratingReport = false;
    }
  }

  async function deleteSample(id: number) {
    if (!confirm("Are you sure you want to delete this sample and all its genotypes?")) return;
    try {
      await apiDeleteSample(id);
      if (selectedSample && selectedSample.id === id) {
        selectedSample = null;
        generatedReport = null;
      }
      await refreshSamples();
    } catch (e: any) {
      alert("Delete failed: " + e.toString());
    }
  }

  async function searchVariant(e: Event) {
    e.preventDefault();
    if (!selectedSample) return;
    isBrowsing = true;
    try {
      if (searchRsid.trim()) {
        browserResults = await queryRsids(selectedSample.id, [searchRsid.trim()]);
      } else {
        browserResults = await queryRegion(
          selectedSample.id,
          browseChr,
          Number(browseStart),
          Number(browseEnd)
        );
      }
    } catch (e: any) {
      alert("Search failed: " + e.toString());
    } finally {
      isBrowsing = false;
    }
  }

  let totalMarkersChecked = $derived(
    generatedReport && generatedReport.sections
      ? generatedReport.sections.reduce((acc: number, sec) => acc + (sec.markers ? sec.markers.length : 0), 0)
      : 0
  );

  let foundMarkersCount = $derived(
    generatedReport && generatedReport.sections
      ? generatedReport.sections.reduce((acc: number, sec) => 
          acc + (sec.markers ? sec.markers.filter(m => m.user_genotype !== "--" && !m.user_genotype.includes('-')).length : 0), 0)
      : 0
  );
</script>

<AppShell>
  {#snippet sidebar()}
    <Sidebar
      {isChainDownloaded}
      {isDownloadingChain}
      bind:filePath
      bind:sampleNameInput
      {isImporting}
      {progressPercent}
      {progressStatus}
      {importError}
      {importSuccess}
      {samples}
      {selectedSample}
      onDownloadChain={downloadChain}
      onBrowseFile={browseFile}
      onImportGenome={importGenome}
      onSelectSample={selectSample}
      onDeleteSample={deleteSample}
    />
  {/snippet}

  {#snippet children()}
    <main class="main-content">
      {#if selectedSample === null}
        <EmptyState {appPaths} />
      {:else}
        <header class="content-header">
          <div class="profile-summary">
            <h2>Profile: {selectedSample.name}</h2>
            <span class="pill font-mono">Sex: {selectedSample.genetic_sex}</span>
            <span class="pill">Sample ID: {selectedSample.id}</span>
          </div>
          <nav class="tabs">
            <button class="tab-btn" class:active={activeTab === "report"} onclick={() => activeTab = "report"}>📊 Trait Report</button>
            <button class="tab-btn" class:active={activeTab === "map"} onclick={() => activeTab = "map"}>🎨 Chromosome Map</button>
            <button class="tab-btn" class:active={activeTab === "browser"} onclick={() => activeTab = "browser"}>🔍 Raw Browser</button>
            <button class="tab-btn" class:active={activeTab === "mcp"} onclick={() => activeTab = "mcp"}>🤖 MCP Integration</button>
            <button class="tab-btn" class:active={activeTab === "agent"} onclick={() => activeTab = "agent"}>🕵️ Research Agent</button>
            <button class="tab-btn" class:active={activeTab === "ai"} onclick={() => activeTab = "ai"}>💬 AI Consultation</button>
          </nav>
        </header>

        <div class="tab-content">
          {#if activeTab === "report"}
            <ReportView
              {generatedReport}
              {isGeneratingReport}
              {selectedSample}
              {foundMarkersCount}
              {totalMarkersChecked}
              {reportError}
            />
          {:else if activeTab === "map"}
            <GenomeMap {selectedSample} {generatedReport} />
          {:else if activeTab === "browser"}
            <VariantSearchPanel
              bind:searchRsid
              bind:browseChr
              bind:browseStart
              bind:browseEnd
              {browserResults}
              {isBrowsing}
              onSearch={searchVariant}
            />
          {:else if activeTab === "mcp"}
            <McpPanel {appPaths} />
          {:else if activeTab === "agent"}
            <AgentResearchPanel
              {selectedSample}
              {generatedReport}
              bind:ollamaUrl={aiOllamaUrl}
              bind:ollamaToken={aiOllamaToken}
              bind:selectedModel={aiSelectedModel}
            />
          {:else if activeTab === "ai"}
            <AiAssistantPanel
              {selectedSample}
              {generatedReport}
              bind:ollamaUrl={aiOllamaUrl}
              bind:ollamaToken={aiOllamaToken}
              bind:selectedModel={aiSelectedModel}
              bind:messages={aiMessages}
              bind:selectedPacks={aiSelectedPacks}
              bind:onlyActiveFindings={aiOnlyActiveFindings}
              bind:temperature={aiTemperature}
            />
          {/if}
        </div>
      {/if}
    </main>
  {/snippet}
</AppShell>
