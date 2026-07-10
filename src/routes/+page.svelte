<!-- ./src/routes/+page.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  // API wrappers
  import {
    selectFile,
    checkChainStatus,
    downloadChain as apiDownloadChain,
    getResearchJobStatus,
    getOllamaToken,
    logJsError,
  } from "$lib/api/tauri";
  import type { ChatMessage } from "$lib/types/agent";
  import type { ResearchJob } from "$lib/types/research";
  import {
    startResearchEventListeners,
    stopResearchEventListeners,
  } from "$lib/research/researchEvents";
  import {
    jobRefs,
    researchEventContext,
    applyLiveFieldsFromJob,
  } from "$lib/research/liveProgress.svelte";

  // Svelte 5 components
  import AppShell from "$lib/components/layout/AppShell.svelte";
  import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
  import EmptyState from "$lib/components/common/EmptyState.svelte";
  import BootstrapOverlay from "$lib/components/common/bootstrap/BootstrapOverlay.svelte";
  import {
    DB_TICKER_MESSAGES,
    type BootstrapPhase,
  } from "$lib/components/common/bootstrap/bootstrapPhases";
  import ReportView from "$lib/components/report/ReportView.svelte";
  import GenomeMap from "$lib/components/genome/GenomeMap.svelte";
  import VariantSearchPanel from "$lib/components/search/VariantSearchPanel.svelte";
  import McpPanel from "$lib/components/mcp/McpPanel.svelte";
  import AiAssistantPanel from "$lib/components/ai/AiAssistantPanel.svelte";
  import AgentResearchPanel from "$lib/components/agent/AgentResearchPanel.svelte";
  import ResearchPanel from "$lib/components/research/ResearchPanel.svelte";
  import { dialogStore } from "$lib/utils/dialogState.svelte";
  import GlobalDialogs from "$lib/components/common/GlobalDialogs.svelte";
  import { runPageBootstrap } from "$lib/utils/pageBootstrap";
  import {
    runImportGenome,
    warmReport as runWarmReport,
    triggerReport as runTriggerReport,
    deleteSampleWithConfirm,
    fetchSamples,
    searchVariants,
    computeReportMarkerCounts,
    browseGenomeFile,
    downloadReferenceChain,
  } from "$lib/utils/pageSampleHandlers";
  import { navigateToVariant as goToVariant } from "$lib/utils/variantNavigation";
  import { resolveInitialOllamaUrl } from "$lib/utils/ollamaSettings";
  import { installAgentUiBridge } from "$lib/utils/agentUiBridge";

  // Stylesheet imports
  import "$lib/styles/theme.css";
  import "$lib/styles/print.css";

  import type { GenomeSample, AppPaths, AppBootstrapStatus, GeneratedReport, NormalizedReport, DbSnpRecord } from "$lib/types/genomics";
  import type { VariantNavTarget } from "$lib/constants/traitCategories";

  // State Runes (Svelte 5)
  let samples = $state<GenomeSample[]>([]);
  let selectedSample = $state<GenomeSample | null>(null);
  let filePath = $state("");
  let sampleNameInput = $state("");
  let appPaths = $state<AppPaths | null>(null);

  let isBootstrapping = $state(true);
  let bootstrapPhase = $state<BootstrapPhase>("db");
  let bootstrapMessage = $state("Opening local database…");
  let bootstrapError = $state("");
  let bootstrapStatus = $state<AppBootstrapStatus | null>(null);
  let showBootstrapOverlay = $derived(
    isBootstrapping || (bootstrapPhase === "error" && samples.length === 0)
  );
  let isChainDownloaded = $state(false);
  let isDownloadingChain = $state(false);
  
  let isImporting = $state(false);
  let progressPercent = $state(0);
  let progressStatus = $state("");
  let importError = $state("");
  let importSuccess = $state("");

  let activeTab = $state("report"); // "report", "map", "browser", "mcp", "agent", "research", "ai"

  let generatedReport = $state<GeneratedReport | null>(null);
  let rawReport = $state<NormalizedReport | null>(null);
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
  let aiMessages = $state<ChatMessage[]>([]);
  let aiSelectedPacks = $state<Record<string, boolean>>({});
  let aiOnlyActiveFindings = $state(true);
  let aiTemperature = $state(0.0);
  let researchJob = $state<import("$lib/types/research").ResearchJob | null>(null);
  
  let aiInitialSearchQuery = $state("");
  let aiActiveView = $state<"chat" | "evidence">("chat");

  let highlightRsid = $state("");
  let mapFocusRsid = $state("");
  let activeSampleId = $state<number | null>(null);
  let expandDatabases = $state(false);
  let offlineStatusForWelcome = $state<import("$lib/types/research").OfflineUpdateCheck | null>(null);
  let sidebarApi = $state<{
    syncAllMissing: () => void;
    expandDatabases: () => void;
  } | null>(null);

  $effect(() => {
    activeSampleId = selectedSample?.id ?? null;
  });

  function handleWelcomeImport() {
    void browseFile();
  }

  function handleWelcomeDownloadDatabases() {
    expandDatabases = true;
    sidebarApi?.expandDatabases();
    sidebarApi?.syncAllMissing();
  }

  $effect(() => {
    if (expandDatabases) {
      const t = setTimeout(() => {
        expandDatabases = false;
      }, 500);
      return () => clearTimeout(t);
    }
  });

  function handleExploreResearch(rsid: string) {
    aiInitialSearchQuery = rsid;
    aiActiveView = "evidence";
    activeTab = "ai";
  }

  function navigateToVariant(rsid: string, target: VariantNavTarget) {
    goToVariant(rsid, target, {
      getSelectedSample: () => selectedSample,
      setState: (patch) => {
        if (patch.activeTab !== undefined) activeTab = patch.activeTab;
        if (patch.highlightRsid !== undefined) highlightRsid = patch.highlightRsid;
        if (patch.mapFocusRsid !== undefined) mapFocusRsid = patch.mapFocusRsid;
        if (patch.searchRsid !== undefined) searchRsid = patch.searchRsid;
        if (patch.browserResults !== undefined) browserResults = patch.browserResults;
        if (patch.isBrowsing !== undefined) isBrowsing = patch.isBrowsing;
        if (patch.aiInitialSearchQuery !== undefined) aiInitialSearchQuery = patch.aiInitialSearchQuery;
        if (patch.aiActiveView !== undefined) aiActiveView = patch.aiActiveView;
      },
      onExploreResearch: handleExploreResearch,
      onSearchError: (message) => dialogStore.alert("Search failed: " + message),
    });
  }

  let unlistenProgress: () => void;

  $effect(() => {
    const sampleId = selectedSample?.id;
    if (!sampleId) {
      researchJob = null;
      return;
    }
    void getResearchJobStatus(sampleId)
      .then((job) => { researchJob = job; })
      .catch((e) => {
        console.warn("Research job status load failed:", e);
        researchJob = null;
      });
  });

  $effect(() => {
    const sampleId = selectedSample?.id;
    const shouldPoll =
      sampleId != null &&
      activeTab !== "research" &&
      (researchJob?.status === "running" ||
        (researchJob?.status === "paused" && researchJob?.loop_active === true));

    if (!shouldPoll || sampleId == null) {
      return;
    }

    const interval = setInterval(() => {
      void getResearchJobStatus(sampleId)
        .then((job) => {
          if (job) {
            researchJob = job;
            applyLiveFieldsFromJob(job);
          }
        })
        .catch((e) => {
          console.warn("Research job poll failed:", e);
        });
    }, 2000);

    return () => clearInterval(interval);
  });

  $effect(() => {
    researchEventContext.sampleId = selectedSample?.id ?? null;
    jobRefs.getJob = () => researchJob;
    jobRefs.setJob = (j) => {
      researchJob = j;
    };
  });

  onMount(() => {
    // Intercept and route global webview JS errors to the persistent file log
    window.onerror = (message, source, lineno, colno, error) => {
      const msg = typeof message === "string" ? message : (message as any).message || "Unknown error";
      const stack = error?.stack || "";
      void logJsError(msg, source || "unknown", lineno || null, colno || null, stack || null);
    };

    window.onunhandledrejection = (event) => {
      const reason = event.reason;
      const msg = reason?.message || String(reason) || "Unhandled promise rejection";
      const stack = reason?.stack || "";
      void logJsError(msg, "unhandled_rejection", null, null, stack || null);
    };

    const uninstallAgentUi = installAgentUiBridge({
      getActiveTab: () => activeTab,
      setActiveTab: (tab) => {
        activeTab = tab;
      },
      getSelectedSample: () =>
        selectedSample
          ? {
              id: selectedSample.id,
              name: selectedSample.name,
              genetic_sex: selectedSample.genetic_sex,
            }
          : null,
      getReport: () => generatedReport,
    });

    async function init() {
      if (typeof localStorage !== "undefined") {
        aiOllamaUrl = await resolveInitialOllamaUrl();
      }
      aiOllamaToken = (await getOllamaToken()) || "";
      await runBootstrap();
    }
    init();

    listen<{ percentage: number; status: string }>("import-progress", (event) => {
      progressPercent = event.payload.percentage;
      progressStatus = event.payload.status;
    }).then(unlisten => {
      unlistenProgress = unlisten;
    });

    let unlistenBootstrap: (() => void) | null = null;
    listen<string>("bootstrap-progress", (event) => {
      bootstrapMessage = event.payload;
    }).then(unlisten => {
      unlistenBootstrap = unlisten;
    });

    void startResearchEventListeners();

    return () => {
      uninstallAgentUi();
      if (unlistenProgress) unlistenProgress();
      if (unlistenBootstrap) unlistenBootstrap();
      stopResearchEventListeners();
    };
  });

  async function runBootstrap() {
    isBootstrapping = true;
    bootstrapPhase = "db";
    bootstrapMessage = DB_TICKER_MESSAGES[0];
    bootstrapError = "";

    const { pendingSample } = await runPageBootstrap({
      onPhase: (phase, message) => {
        bootstrapPhase = phase;
        bootstrapMessage = message;
      },
      onStatus: (status) => {
        bootstrapStatus = status;
      },
      onPaths: (paths) => {
        appPaths = paths;
      },
      onChainPresent: (present) => {
        isChainDownloaded = present;
      },
      onSamples: (loaded) => {
        samples = loaded;
      },
      onError: (message) => {
        bootstrapError = message;
      },
      warmReport,
    });

    if (pendingSample) {
      selectedSample = pendingSample;
      // warmReport was already called inside bootstrap — only re-trigger if it
      // failed to produce a report (e.g. due to an error path that cleared the state).
      if (!generatedReport && !isGeneratingReport) {
        void triggerReport();
      }
    }
    isBootstrapping = false;
  }

  async function refreshChainStatus() {
    try {
      isChainDownloaded = await checkChainStatus();
    } catch (e) {
      console.error(e);
    }
  }

  async function downloadChain() {
    await downloadReferenceChain(
      apiDownloadChain,
      checkChainStatus,
      (patch) => {
        if (patch.isDownloadingChain !== undefined) isDownloadingChain = patch.isDownloadingChain;
        if (patch.isChainDownloaded !== undefined) isChainDownloaded = patch.isChainDownloaded;
      },
      (message) => dialogStore.alert(message),
    );
  }

  async function refreshSamples() {
    try {
      samples = await fetchSamples();
      if (samples.length > 0 && selectedSample === null) {
        selectSample(samples[0]);
      }
    } catch (e) {
      console.error(e);
    }
  }

  function selectSample(sample: GenomeSample) {
    selectedSample = sample;
    generatedReport = null;
    void runTriggerReport({
      selectedSample,
      generatedReport,
      warmReportFn: (id) => warmReport(id),
    });
  }

  async function browseFile() {
    await browseGenomeFile(selectFile, (patch) => {
      if (patch.filePath !== undefined) filePath = patch.filePath;
      if (patch.sampleNameInput !== undefined) sampleNameInput = patch.sampleNameInput;
    });
  }

  async function importGenome(e: Event) {
    await runImportGenome(e, {
      filePath,
      sampleNameInput,
      onState: (patch) => {
        if (patch.isImporting !== undefined) isImporting = patch.isImporting;
        if (patch.importError !== undefined) importError = patch.importError;
        if (patch.importSuccess !== undefined) importSuccess = patch.importSuccess;
        if (patch.progressPercent !== undefined) progressPercent = patch.progressPercent;
        if (patch.progressStatus !== undefined) progressStatus = patch.progressStatus;
        if (patch.filePath !== undefined) filePath = patch.filePath;
        if (patch.sampleNameInput !== undefined) sampleNameInput = patch.sampleNameInput;
      },
      refreshSamples: async () => {
        await refreshSamples();
        return samples;
      },
      selectSample,
    });
  }

  async function warmReport(sampleId: number) {
    await runWarmReport({
      sampleId,
      onState: (patch) => {
        if (patch.isGeneratingReport !== undefined) isGeneratingReport = patch.isGeneratingReport;
        if (patch.reportError !== undefined) reportError = patch.reportError;
        if (patch.generatedReport !== undefined) generatedReport = patch.generatedReport;
        if (patch.rawReport !== undefined) rawReport = patch.rawReport;
      },
    });
  }

  async function triggerReport() {
    await runTriggerReport({ selectedSample, generatedReport, warmReportFn: warmReport });
  }

  async function deleteSample(id: number) {
    await deleteSampleWithConfirm({
      id,
      selectedSample,
      confirm: (message, onConfirm, title) => dialogStore.confirm(message, onConfirm, title),
      alert: (message) => dialogStore.alert(message),
      onState: (patch) => {
        if (patch.selectedSample !== undefined) selectedSample = patch.selectedSample;
        if (patch.generatedReport !== undefined) generatedReport = patch.generatedReport;
      },
      refreshSamples,
    });
  }

  async function searchVariant(e: Event) {
    await searchVariants(e, {
      selectedSample,
      searchRsid,
      browseChr,
      browseStart,
      browseEnd,
      onState: (patch) => {
        if (patch.isBrowsing !== undefined) isBrowsing = patch.isBrowsing;
        if (patch.browserResults !== undefined) browserResults = patch.browserResults;
      },
      alert: (message) => dialogStore.alert(message),
    });
  }

  let reportMarkerCounts = $derived(computeReportMarkerCounts(generatedReport));
  let totalMarkersChecked = $derived(reportMarkerCounts.totalMarkersChecked);
  let foundMarkersCount = $derived(reportMarkerCounts.foundMarkersCount);
</script>

{#if showBootstrapOverlay}
  <BootstrapOverlay
    phase={bootstrapPhase}
    message={bootstrapMessage}
    status={bootstrapStatus}
    error={bootstrapError}
  />
{/if}

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
      report={generatedReport}
      sweepRunning={researchJob?.status === "running" && researchJob.loop_active !== false}
      {expandDatabases}
      onReady={(api) => {
        sidebarApi = api;
      }}
      onOfflineStatusChange={(status) => {
        offlineStatusForWelcome = status;
      }}
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
        <EmptyState
          {appPaths}
          offlineStatus={offlineStatusForWelcome}
          {isChainDownloaded}
          onImportGenome={handleWelcomeImport}
          onDownloadDatabases={handleWelcomeDownloadDatabases}
          onDownloadChain={downloadChain}
        />
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
            <button class="tab-btn" class:active={activeTab === "research"} onclick={() => activeTab = "research"}>
              🔬 Vector Research
              {#if researchJob?.status === "running" && researchJob.loop_active !== false}
                <span class="tab-status-pill running" title="Enrichment sweep in progress">{researchJob.enriched_count}/{researchJob.total_markers}</span>
              {:else if researchJob?.status === "running"}
                <span class="tab-status-pill paused" title="Sweep interrupted — resume on Vector Research tab">interrupted</span>
              {:else if researchJob?.status === "paused"}
                <span class="tab-status-pill paused" title="Sweep paused">paused</span>
              {/if}
            </button>
            <button class="tab-btn" class:active={activeTab === "ai"} onclick={() => activeTab = "ai"}>💬 AI Consultation</button>
          </nav>
        </header>

        <div class="tab-content">
          {#if activeTab === "report"}
            <ReportView
              {generatedReport}
              {rawReport}
              {isGeneratingReport}
              {selectedSample}
              {foundMarkersCount}
              {totalMarkersChecked}
              {reportError}
              {highlightRsid}
              onExploreResearch={handleExploreResearch}
              onNavigateToVariant={navigateToVariant}
            />
          {:else if activeTab === "map"}
            <GenomeMap {selectedSample} {generatedReport} focusRsid={mapFocusRsid} onNavigateToVariant={navigateToVariant} />
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
          {:else if activeTab === "research"}
            <ResearchPanel
              {selectedSample}
              bind:ollamaUrl={aiOllamaUrl}
              bind:ollamaToken={aiOllamaToken}
              bind:job={researchJob}
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
              bind:initialSearchQuery={aiInitialSearchQuery}
              bind:activeView={aiActiveView}
              onNavigateToVariant={navigateToVariant}
            />
          {/if}
        </div>
      {/if}
    </main>
  {/snippet}
</AppShell>

<GlobalDialogs />
