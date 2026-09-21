<!-- ./src/routes/+page.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { isTauri } from "@tauri-apps/api/core";

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
  import ImportWorkspaceState from "$lib/components/common/ImportWorkspaceState.svelte";
  import BootstrapOverlay from "$lib/components/common/bootstrap/BootstrapOverlay.svelte";
  import {
    DB_TICKER_MESSAGES,
    type BootstrapPhase,
  } from "$lib/components/common/bootstrap/bootstrapPhases";
  import { dialogStore } from "$lib/utils/dialogState.svelte";
  import GlobalDialogs from "$lib/components/common/GlobalDialogs.svelte";
  import { runPageBootstrap } from "$lib/utils/pageBootstrap";
  import {
    runImportGenome,
    warmReport as runWarmReport,
    triggerReport as runTriggerReport,
    reloadReport as runReloadReport,
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
  import { installDesktopContextMenu } from "$lib/utils/desktopContextMenu";
  import { formatGeneticSexLabel } from "$lib/utils/uiLabels";
  import type { Component } from "svelte";

  // Stylesheet imports
  import "$lib/styles/theme.css";
  import "$lib/styles/print.css";
  import "$lib/styles/components/context-panel.css";

  import type { GenomeSample, AppPaths, AppBootstrapStatus, GeneratedReport, NormalizedReport, DbSnpRecord, GenomeImportPreview } from "$lib/types/genomics";
  import type { VariantNavTarget } from "$lib/constants/traitCategories";
  import {
    EMPTY_PROFILE_CONTEXT,
    loadProfileContext,
    type ProfileContext,
  } from "$lib/utils/profileContext";
  import {
    importPhaseForProgress,
    importStepForPhase,
    type ImportPhase,
    type ImportStepId,
  } from "$lib/utils/importProgress";

  // State Runes (Svelte 5)
  let samples = $state<GenomeSample[]>([]);
  let selectedSample = $state<GenomeSample | null>(null);
  let profileContext = $state<ProfileContext>({
    ...EMPTY_PROFILE_CONTEXT,
    notes: { ...EMPTY_PROFILE_CONTEXT.notes },
    safety: { ...EMPTY_PROFILE_CONTEXT.safety },
    exportPreferences: { ...EMPTY_PROFILE_CONTEXT.exportPreferences },
  });
  let loadedProfileContextKey = $state("");
  let filePath = $state("");
  let sampleNameInput = $state("");
  let appPaths = $state<AppPaths | null>(null);

  let isBootstrapping = $state(true);
  let bootstrapPhase = $state<BootstrapPhase>("db");
  let bootstrapMessage = $state("Opening local database…");
  let bootstrapError = $state("");
  let bootstrapStatus = $state<AppBootstrapStatus | null>(null);
  let runtimeAvailable = $state(true);
  let isResourceSyncing = $state(false);
  let resourceSyncOverlayDismissed = $state(false);
  let isImportPreparing = $state(false);
  let importPhase = $state<ImportPhase>("idle");
  let importPreview = $state<GenomeImportPreview | null>(null);
  let importProfileName = $state("");
  let importReplacingExisting = $state(false);
  let pendingImportConfirmation = $state<(() => void | Promise<void>) | null>(null);
  let importFailedStep = $state<ImportStepId | null>(null);
  let importOverlayDismissed = $state(false);
  let isChainDownloaded = $state(false);
  let isDownloadingChain = $state(false);
  
  let isImporting = $state(false);
  let progressPercent = $state(0);
  let progressStatus = $state("");
  let importError = $state("");
  let importSuccess = $state("");
  const NATIVE_IMPORT_PHASES = new Set<ImportPhase>([
    "reading",
    "parsing",
    "liftover",
    "ingesting",
    "indexing",
  ]);

  let showImportOverlay = $derived(
    // Full-screen import overlay only for failures that need an explicit dismiss.
    // Live import progress belongs in the sidebar card (right of the dock / left panel).
    importPhase === "error" && !importOverlayDismissed,
  );
  let showBootstrapOverlay = $derived(
    isBootstrapping ||
      (isResourceSyncing && !resourceSyncOverlayDismissed) ||
      (bootstrapPhase === "error" && samples.length === 0),
  );

  function bootstrapPhaseForImport(phase: ImportPhase): BootstrapPhase {
    if (phase === "error") return "error";
    if (phase === "ready") return "ready";
    if (phase === "report") return "report";
    if (phase === "profile") return "profile";
    if (phase === "liftover" || phase === "ingesting" || phase === "indexing") return "profile";
    return "stats";
  }

  let importOverlayPhase = $derived(bootstrapPhaseForImport(importPhase));
  let importOverlayProgress = $derived({ percentage: progressPercent, status: progressStatus });

  let activeTab = $state("report"); // report | context | diary | map | discovery | legal | browser | mcp | agent | research | ai | connections
  const ADVANCED_TAB_KEY = "genomics_caddy_last_advanced_tab";

  const PRIMARY_TABS = [
    { id: "report", label: "Trait Report" },
    { id: "context", label: "Context" },
    { id: "diary", label: "Diary" },
    { id: "map", label: "Chromosome Map" },
    { id: "discovery", label: "Discovery" },
  ] as const;

  const ADVANCED_TABS = [
    { id: "connections", label: "Connections" },
    { id: "browser", label: "Raw Browser" },
    { id: "mcp", label: "MCP Integration" },
    { id: "agent", label: "Research Agent" },
    { id: "research", label: "Vector Research" },
    { id: "ai", label: "AI Consultation" },
    { id: "legal", label: "Legal & Privacy" },
  ] as const;

  let advancedActive = $derived(ADVANCED_TABS.some((t) => t.id === activeTab));

  type DeferredPanel = Component<Record<string, unknown>>;
  let deferredPanels = $state<Record<string, DeferredPanel>>({});
  let deferredPanelLoading = $state<string | null>(null);
  let deferredPanelErrors = $state<Record<string, string>>({});

  async function loadDeferredPanel(tab: string): Promise<void> {
    if (deferredPanels[tab] || deferredPanelLoading === tab) return;
    const deferredTabs = new Set([
      "report",
      "context",
      "diary",
      "map",
      "discovery",
      "connections",
      "browser",
      "mcp",
      "agent",
      "research",
      "ai",
      "legal",
    ]);
    if (!deferredTabs.has(tab)) return;

    deferredPanelLoading = tab;
    delete deferredPanelErrors[tab];
    try {
      const module = await (async () => {
        switch (tab) {
          case "report": return import("$lib/components/report/ReportView.svelte");
          case "context": return import("$lib/components/context/ContextPanel.svelte");
          case "diary": return import("$lib/components/context/DiaryPanel.svelte");
          case "map": return import("$lib/components/genome/GenomeMap.svelte");
          case "discovery": return import("$lib/components/discovery/DiscoveryPanel.svelte");
          case "connections": return import("$lib/components/settings/ConnectionsPanel.svelte");
          case "browser": return import("$lib/components/search/VariantSearchPanel.svelte");
          case "mcp": return import("$lib/components/mcp/McpPanel.svelte");
          case "agent": return import("$lib/components/agent/AgentResearchPanel.svelte");
          case "research": return import("$lib/components/research/ResearchPanel.svelte");
          case "ai": return import("$lib/components/ai/AiAssistantPanel.svelte");
          case "legal": return import("$lib/components/legal/LegalPrivacyPanel.svelte");
          default: return null;
        }
      })();
      if (module) deferredPanels[tab] = module.default as DeferredPanel;
    } catch (error) {
      deferredPanelErrors[tab] = error instanceof Error ? error.message : String(error);
    } finally {
      if (deferredPanelLoading === tab) deferredPanelLoading = null;
    }
  }

  $effect(() => {
    void loadDeferredPanel(activeTab);
  });

  function deferredPanelStatus(tab: string, label: string): string {
    if (deferredPanelErrors[tab] && deferredPanelLoading === null) {
      return `Could not load ${label}. Select it again to retry.`;
    }
    return `Loading ${label}…`;
  }

  $effect(() => {
    const contextKey = selectedSample?.id == null ? "none" : String(selectedSample.id);
    if (loadedProfileContextKey === contextKey) return;
    loadedProfileContextKey = contextKey;
    profileContext = loadProfileContext(selectedSample?.id);
  });

  function selectTab(tab: string) {
    activeTab = tab;
    if (ADVANCED_TABS.some((t) => t.id === tab)) {
      try {
        localStorage.setItem(ADVANCED_TAB_KEY, tab);
      } catch {
        /* ignore */
      }
    }
  }

  function onTabKeydown(e: KeyboardEvent, tabIds: string[]) {
    const i = tabIds.indexOf(activeTab);
    if (i < 0) return;
    if (e.key === "ArrowRight") {
      e.preventDefault();
      selectTab(tabIds[(i + 1) % tabIds.length]);
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      selectTab(tabIds[(i - 1 + tabIds.length) % tabIds.length]);
    }
  }

  function openAdvancedArea() {
    if (advancedActive) return;
    try {
      const last = localStorage.getItem(ADVANCED_TAB_KEY);
      if (last && ADVANCED_TABS.some((t) => t.id === last)) {
        selectTab(last);
        return;
      }
    } catch {
      /* ignore */
    }
    selectTab(ADVANCED_TABS[0].id);
  }

  let generatedReport = $state<GeneratedReport | null>(null);
  let rawReport = $state<NormalizedReport | null>(null);
  let isGeneratingReport = $state(false);
  let reportError = $state("");
  let reportRequestGeneration = 0;

  let searchRsid = $state("");
  let browseChr = $state("1");
  let browseStart = $state(1);
  let browseEnd = $state(5000000);
  let browserResults = $state<DbSnpRecord[]>([]);
  let isBrowsing = $state(false);

  // Persistent AI Consultation State (Lifts state from AiAssistantPanel to survive tab unmounts)
  let aiOllamaUrl = $state("");
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
  let offlineStatusFreshForWelcome = $state(false);
  let sidebarApi = $state<{
    syncAllMissing: () => Promise<void>;
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
    runtimeAvailable = isTauri();
    if (!runtimeAvailable) {
      isBootstrapping = false;
      bootstrapPhase = "ready";
      bootstrapMessage = "Web preview mode — the desktop database is unavailable.";
      return;
    }

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

    const uninstallDesktopContextMenu = installDesktopContextMenu({
      hasReport: () => generatedReport !== null,
      onReloadReport: reloadReport,
      onOpenDiscovery: () => selectTab("discovery"),
      onOpenAiConsultation: () => selectTab("ai"),
      onImportGenome: handleWelcomeImport,
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
      if (isImporting && NATIVE_IMPORT_PHASES.has(importPhase)) {
        importPhase = importPhaseForProgress(event.payload.status, event.payload.percentage);
        importFailedStep = null;
      }
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
      uninstallDesktopContextMenu();
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
    const loaded = await fetchSamples();
    samples = loaded;
    return loaded;
  }

  async function selectSample(sample: GenomeSample): Promise<void> {
    selectedSample = sample;
    generatedReport = null;
    await runTriggerReport({
      selectedSample: sample,
      generatedReport: null,
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
    pendingImportConfirmation = null;
    importReplacingExisting = false;
    await runImportGenome(e, {
      filePath,
      sampleNameInput,
      existingSamples: samples,
      selectedSampleId: selectedSample?.id ?? null,
      onConfirmationRequired: (onConfirm, replacingExisting) => {
        pendingImportConfirmation = onConfirm;
        importReplacingExisting = replacingExisting;
      },
      onState: (patch) => {
        if (patch.importPhase === "error") {
          importFailedStep = importStepForPhase(importPhase) ?? "validate";
          importOverlayDismissed = false;
        } else if (patch.importPhase !== undefined) {
          importFailedStep = null;
          if (patch.importPhase !== "awaiting-confirmation") importOverlayDismissed = false;
        }
        if (patch.isImportPreparing !== undefined) isImportPreparing = patch.isImportPreparing;
        if (patch.isImporting !== undefined) {
          isImporting = patch.isImporting;
          if (patch.isImporting) importOverlayDismissed = false;
        }
        if (patch.importError !== undefined) importError = patch.importError;
        if (patch.importSuccess !== undefined) importSuccess = patch.importSuccess;
        if (patch.progressPercent !== undefined) progressPercent = patch.progressPercent;
        if (patch.progressStatus !== undefined) progressStatus = patch.progressStatus;
        if (patch.importPhase !== undefined) importPhase = patch.importPhase;
        if (patch.importPreview !== undefined) importPreview = patch.importPreview;
        if (patch.importProfileName !== undefined) importProfileName = patch.importProfileName;
        if (patch.filePath !== undefined) filePath = patch.filePath;
        if (patch.sampleNameInput !== undefined) sampleNameInput = patch.sampleNameInput;
      },
      refreshSamples: async () => {
        // Refresh without selecting the first row implicitly. The import
        // waterfall selects and warms the exact returned profile next.
        const loaded = await fetchSamples();
        samples = loaded;
        return loaded;
      },
      selectSample,
    });
  }

  function confirmImportReview() {
    const confirm = pendingImportConfirmation;
    pendingImportConfirmation = null;
    importReplacingExisting = false;
    if (confirm) void confirm();
  }

  function cancelImportReview() {
    pendingImportConfirmation = null;
    importReplacingExisting = false;
    isImportPreparing = false;
    isImporting = false;
    importPhase = "idle";
    importPreview = null;
    importFailedStep = null;
    importOverlayDismissed = true;
    importError = "";
    progressPercent = 0;
    progressStatus = "";
  }

  async function warmReport(sampleId: number) {
    const generation = ++reportRequestGeneration;
    await runWarmReport({
      sampleId,
      onState: (patch) => {
        if (generation !== reportRequestGeneration) return;
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

  async function reloadReport() {
    await runReloadReport({ selectedSample, warmReportFn: warmReport });
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
      selectSample,
      removeSampleFromList: (sampleId) => {
        samples = samples.filter((profile) => profile.id !== sampleId);
      },
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

{#if showImportOverlay}
  <BootstrapOverlay
    phase={importOverlayPhase}
    message={progressStatus || "Preparing DNA import…"}
    status={null}
    error={importError}
    mode="import"
    importPhase={importPhase}
    importProgress={importOverlayProgress}
    importPreview={importPreview}
    importProfileName={importProfileName}
    importFailedStep={importFailedStep}
    showWorkspaceAction={importPhase === "error"}
    workspaceActionLabel="Return to workspace"
    onContinue={() => {
      importOverlayDismissed = true;
    }}
  />
{:else if showBootstrapOverlay}
  <BootstrapOverlay
    phase={bootstrapPhase}
    message={bootstrapMessage}
    status={bootstrapStatus}
    error={bootstrapError}
    mode={isResourceSyncing ? "resources" : "startup"}
    showWorkspaceAction={isResourceSyncing}
    onContinue={() => {
      resourceSyncOverlayDismissed = true;
    }}
  />
{/if}

<AppShell>
  {#snippet sidebar()}
    <Sidebar
      {isChainDownloaded}
      {isDownloadingChain}
      bind:filePath
      bind:sampleNameInput
      {isImportPreparing}
      {isImporting}
      {progressPercent}
      {progressStatus}
      {importError}
      {importSuccess}
      {samples}
      {selectedSample}
      report={generatedReport}
      {runtimeAvailable}
      sweepRunning={researchJob?.status === "running" && researchJob.loop_active !== false}
      {expandDatabases}
      onReady={(api) => {
        sidebarApi = api;
      }}
      onOfflineStatusChange={(status, fresh) => {
        offlineStatusForWelcome = status;
        offlineStatusFreshForWelcome = fresh;
      }}
      onDownloadChain={downloadChain}
      onBrowseFile={browseFile}
      onImportGenome={importGenome}
      onSelectSample={selectSample}
      onDeleteSample={deleteSample}
      onOpenConnections={() => selectTab("connections")}
      onResourcesUpdated={async () => {
        await reloadReport();
      }}
      onResourceSyncStateChange={({ active, message }) => {
        isResourceSyncing = active;
        if (active) {
          resourceSyncOverlayDismissed = false;
          bootstrapPhase = "stats";
          bootstrapMessage = message;
          bootstrapError = "";
        }
      }}
    />
  {/snippet}
  {#snippet children()}
    <main class="main-content">
      {#if isImportPreparing || isImporting || importPhase === "awaiting-confirmation"}
        <ImportWorkspaceState
          phase={importPhase}
          percentage={progressPercent}
          status={progressStatus}
          profileName={importProfileName || sampleNameInput}
          preview={importPreview}
          failedStep={importFailedStep}
          replacingExisting={importReplacingExisting}
          onConfirmImport={confirmImportReview}
          onCancelImport={cancelImportReview}
        />
      {:else if selectedSample === null}
        <EmptyState
          {appPaths}
          offlineStatus={offlineStatusForWelcome}
          offlineStatusFresh={offlineStatusFreshForWelcome}
          {isChainDownloaded}
          {runtimeAvailable}
          onImportGenome={handleWelcomeImport}
          onDownloadDatabases={handleWelcomeDownloadDatabases}
          onDownloadChain={downloadChain}
        />
      {:else}
        <header class="content-header">
          <div class="content-header-top">
            <div class="profile-summary">
              <h2>Profile: {selectedSample.name}</h2>
              <span class="pill">Sex: {formatGeneticSexLabel(selectedSample.genetic_sex)}</span>
            </div>
            <div class="tabs" role="tablist" aria-label="Primary views">
              {#each PRIMARY_TABS as tab (tab.id)}
                <button
                  type="button"
                  class="tab-btn"
                  class:active={activeTab === tab.id}
                  role="tab"
                  id="tab-{tab.id}"
                  aria-selected={activeTab === tab.id}
                  aria-controls="panel-{tab.id}"
                  tabindex={activeTab === tab.id ? 0 : -1}
                  onclick={() => selectTab(tab.id)}
                  onkeydown={(e) => onTabKeydown(e, PRIMARY_TABS.map((t) => t.id))}
                >
                  {tab.label}
                </button>
              {/each}

              <div class="tabs-advanced">
                <button
                  type="button"
                  class="tab-btn tab-btn-advanced"
                  class:active={advancedActive}
                  aria-expanded={advancedActive}
                  aria-controls={advancedActive ? "advanced-subtabs" : undefined}
                  onclick={openAdvancedArea}
                >
                  Advanced
                  <span class="tabs-advanced-caret" aria-hidden="true">{advancedActive ? '▴' : '▾'}</span>
                </button>
              </div>
            </div>
          </div>

          {#if advancedActive}
            <div
              class="tabs-advanced-strip"
              id="advanced-subtabs"
              role="tablist"
              aria-label="Advanced views"
            >
              {#each ADVANCED_TABS as tab (tab.id)}
                <button
                  type="button"
                  class="tabs-advanced-chip"
                  class:active={activeTab === tab.id}
                  role="tab"
                  id="tab-{tab.id}"
                  aria-selected={activeTab === tab.id}
                  aria-controls="panel-{tab.id}"
                  tabindex={activeTab === tab.id ? 0 : -1}
                  onclick={() => selectTab(tab.id)}
                  onkeydown={(e) => onTabKeydown(e, ADVANCED_TABS.map((t) => t.id))}
                >
                  {tab.label}
                  {#if tab.id === "research"}
                    {#if researchJob?.status === "running" && researchJob.loop_active !== false}
                      <span class="tab-status-pill running">{researchJob.enriched_count}/{researchJob.total_markers}</span>
                    {:else if researchJob?.status === "running"}
                      <span class="tab-status-pill paused">interrupted</span>
                    {:else if researchJob?.status === "paused"}
                      <span class="tab-status-pill paused">paused</span>
                    {/if}
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </header>

        <div class="tab-content" role="tabpanel" id="panel-{activeTab}" aria-labelledby="tab-{activeTab}">
          {#if activeTab === "report"}
            {#if deferredPanels.report}
              {@const ReportPanel = deferredPanels.report}
              <ReportPanel
                {generatedReport}
                {rawReport}
                {isGeneratingReport}
                {selectedSample}
                {foundMarkersCount}
                {totalMarkersChecked}
                {reportError}
                {profileContext}
                {highlightRsid}
                onExploreResearch={handleExploreResearch}
                onNavigateToVariant={navigateToVariant}
                onOpenDiscovery={() => selectTab("discovery")}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("report", "Trait Report")}</div>
            {/if}
          {:else if activeTab === "context"}
            {#if deferredPanels.context}
              {@const ContextPanel = deferredPanels.context}
              <ContextPanel
                {selectedSample}
                bind:profileContext
                onOpenDiary={() => selectTab("diary")}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("context", "Context")}</div>
            {/if}
          {:else if activeTab === "diary"}
            {#if deferredPanels.diary}
              {@const DiaryPanel = deferredPanels.diary}
              <DiaryPanel
                {selectedSample}
                bind:profileContext
                onOpenContext={() => selectTab("context")}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("diary", "Diary")}</div>
            {/if}
          {:else if activeTab === "map"}
            {#if deferredPanels.map}
              {@const GenomeMapPanel = deferredPanels.map}
              <GenomeMapPanel {selectedSample} {generatedReport} focusRsid={mapFocusRsid} onNavigateToVariant={navigateToVariant} />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("map", "Chromosome Map")}</div>
            {/if}
          {:else if activeTab === "discovery"}
            {#if deferredPanels.discovery}
              {@const DiscoveryPanel = deferredPanels.discovery}
              <DiscoveryPanel
                {selectedSample}
                onNavigate={(rsid: string) => navigateToVariant(rsid, "report")}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("discovery", "Discovery")}</div>
            {/if}
          {:else if activeTab === "legal"}
            {#if deferredPanels.legal}
              {@const LegalPrivacyPanel = deferredPanels.legal}
              <LegalPrivacyPanel {selectedSample} />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("legal", "Legal & Privacy")}</div>
            {/if}
          {:else if activeTab === "connections"}
            {#if deferredPanels.connections}
              {@const ConnectionsPanel = deferredPanels.connections}
              <ConnectionsPanel
                bind:ollamaUrl={aiOllamaUrl}
                bind:ollamaToken={aiOllamaToken}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("connections", "Connections")}</div>
            {/if}
          {:else if activeTab === "browser"}
            {#if deferredPanels.browser}
              {@const VariantSearchPanel = deferredPanels.browser}
              <VariantSearchPanel
                {selectedSample}
                bind:searchRsid
                bind:browseChr
                bind:browseStart
                bind:browseEnd
                {browserResults}
                {isBrowsing}
                {highlightRsid}
                onSearch={searchVariant}
                onNavigateToVariant={navigateToVariant}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("browser", "Variant browser")}</div>
            {/if}
          {:else if activeTab === "mcp"}
            {#if deferredPanels.mcp}
              {@const McpPanel = deferredPanels.mcp}
              <McpPanel {appPaths} />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("mcp", "MCP Integration")}</div>
            {/if}
          {:else if activeTab === "agent"}
            {#if deferredPanels.agent}
              {@const AgentResearchPanel = deferredPanels.agent}
              <AgentResearchPanel
                {selectedSample}
                {generatedReport}
                bind:ollamaUrl={aiOllamaUrl}
                bind:ollamaToken={aiOllamaToken}
                bind:selectedModel={aiSelectedModel}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("agent", "Research Agent")}</div>
            {/if}
          {:else if activeTab === "research"}
            {#if deferredPanels.research}
              {@const ResearchPanel = deferredPanels.research}
              <ResearchPanel
                {selectedSample}
                bind:ollamaUrl={aiOllamaUrl}
                bind:ollamaToken={aiOllamaToken}
                bind:job={researchJob}
              />
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("research", "Vector Research")}</div>
            {/if}
          {:else if activeTab === "ai"}
            {#if deferredPanels.ai}
              {@const AiAssistantPanel = deferredPanels.ai}
              <AiAssistantPanel
                {selectedSample}
                {generatedReport}
                bind:profileContext
                onOpenContext={() => selectTab("context")}
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
            {:else}
              <div class="tab-loading-state" role="status">{deferredPanelStatus("ai", "AI Consultation")}</div>
            {/if}
          {/if}
        </div>
      {/if}
      {#if selectedSample !== null}
        <footer class="app-reference-note" role="note">
          For reference only. Discuss findings and health decisions with a qualified healthcare professional.
        </footer>
      {/if}
    </main>
  {/snippet}
</AppShell>

<GlobalDialogs />
