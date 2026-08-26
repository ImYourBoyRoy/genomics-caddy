<!-- ./src/lib/components/ai/AiAssistantPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getOllamaToken, cancelOllamaStream } from "../../api/tauri";
  import type { ChatMessage } from "../../types/agent";
  import type { QdrantHit, VectorResearchDiagnostics } from "../../types/research";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import { LAYPERSON_MAP } from "../../utils/layperson";
  import { copyMessageToClipboard, exportConsultationMarkdown } from "../../utils/aiAssistantExportActions";
  import {
    readConsultationSession,
    startNewConsultationSession,
    saveConsultationSessionTitle,
    deleteConsultationSession,
  } from "../../utils/aiAssistantSessionActions";
  import { loadOllamaModelDetails, scanChatModels } from "../../utils/aiAssistantModelActions";
  import { loadAiAssistantPreferences, persistAiAssistantPreferences } from "../../utils/aiAssistantPreferences";
  import { fetchVectorResearchDiagnostics } from "../../utils/aiAssistantVectorDiagnostics";
  import { sendConsultationPrompt, stopConsultationGeneration } from "../../utils/aiAssistantSendActions";
  import { saveOllamaUrl, loadOllamaUrl } from "../../utils/ollamaSettings";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import { ConsultationSessionStore } from "../../utils/consultationSession.svelte";
  import {
    type UserBiohackingProfile, type ContextStats, type ActiveCategories,
    DEFAULT_INSTRUCTIONS, buildSystemPrompt, calculateContextStats,
    getActiveCategories, getDynamicQuestions, isReasoningModel as checkReasoningModel,
    type AiContextMode, type ConsultationMode
  } from "../../utils/aiPrompt";
  import { markerPacksStore } from "../../utils/markerPacksState.svelte";
  import "$lib/styles/components/ai-assistant-panel.css";
  import ChatSidebar from "./ChatSidebar.svelte";
  import ChatWindow from "./ChatWindow.svelte";
  import ChatSettingsDrawer from "./ChatSettingsDrawer.svelte";
  import ChatModals from "./ChatModals.svelte";
  import EvidenceLibraryPanel from "./EvidenceLibraryPanel.svelte";

  interface Props {
    selectedSample: GenomeSample | null;
    generatedReport: GeneratedReport | null;
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
    messages: ChatMessage[];
    selectedPacks: Record<string, boolean>;
    onlyActiveFindings: boolean;
    temperature: number;
    initialSearchQuery?: string;
    activeView?: "chat" | "evidence";
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }
  let {
    selectedSample, generatedReport,
    ollamaUrl = $bindable(""),
    ollamaToken = $bindable(""),
    selectedModel = $bindable(""),
    messages = $bindable([]),
    selectedPacks = $bindable({}),
    onlyActiveFindings = $bindable(true),
    temperature = $bindable(0.0),
    initialSearchQuery = $bindable(""),
    activeView = $bindable("chat"),
    onNavigateToVariant,
  }: Props = $props();

  const sessionStore = new ConsultationSessionStore();
  let filteredSessions = $derived(sessionStore.sessions.filter(s => s.sampleId === (selectedSample ? selectedSample.id : null)));

  let maxTokens = $state(2048);
  let extendedThinking = $state(false);
  let showSettingsDrawer = $state(true);
  let showHistorySidebar = $state(true);

  let showThinkingProcess = $state(true);
  let autoCollapseThinking = $state(true);
  let includeTraceInExport = $state(false);
  let contextMode = $state<AiContextMode>("active_findings");
  let consultationMode = $state<ConsultationMode>("general");
  let reviewModel = $state("");
  let twoModelReview = $state(false);
  let userCollapsedThinkingMap = $state(new Map<any, boolean>());
  let copiedMsgId = $state<number | null>(null);

  // Link contextMode and onlyActiveFindings bi-directionally
  $effect(() => {
    if (contextMode === "full_selected" && onlyActiveFindings) onlyActiveFindings = false;
    else if (contextMode !== "full_selected" && !onlyActiveFindings && contextMode !== "developer_raw_json") contextMode = "full_selected";
    else if (onlyActiveFindings && contextMode === "full_selected") contextMode = "selected_pack_active";
  });

  let isScanning = $state(false);
  let isChatting = $state(false);
  let isLoadingSession = $state(false);
  let scanError = $state("");
  let promptText = $state("");
  let models = $state<string[]>([]);
  let showPromptInspector = $state(false);
  let showExportModal = $state(false);

  let unlistenChunk: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;
  let unlistenReviewChunk: (() => void) | null = null;
  let unlistenReviewDone: (() => void) | null = null;

  function stopReviewListeners() {
    if (unlistenReviewChunk) { unlistenReviewChunk(); unlistenReviewChunk = null; }
    unlistenReviewDone = null;
  }

  let sessionPromptTokens = $state(0), sessionResponseTokens = $state(0);
  let sessionTotalTokens = $derived(sessionPromptTokens + sessionResponseTokens);
  let modelDetails = $state<any>(null), contextWindow = $state<number>(4096), isVisionCapable = $state<boolean>(false);
  let attachedImages = $state<{ name: string; base64: string; previewUrl: string }[]>([]);
  let userProfile = $state<UserBiohackingProfile>({ goals: "", challenges: "", relevantBodySystems: "", reproductiveHormoneContext: "", diet: "", supplements: "", medications: "", bloodwork: "", diagnoses: "", supportiveTests: "", injectProfile: true });
  let systemInstructions = $state("");

  let useVectorResearch = $state(
    typeof localStorage !== "undefined" && localStorage.getItem("genomics_consultation_vector_rag") !== null
      ? localStorage.getItem("genomics_consultation_vector_rag") !== "false"
      : true
  );
  let lastVectorHits = $state<QdrantHit[]>([]);
  let lastVectorEvidenceCards = $state<import("../../types/research").EvidenceCard[]>([]);
  let lastVectorIndexBrief = $state("");
  let lastVectorQuery = $state("");
  let lastVectorError = $state("");
  let vectorDiagnostics = $state<VectorResearchDiagnostics | null>(null);
  let vectorDiagnosticsLoading = $state(false);

  async function refreshVectorDiagnostics() {
    if (!selectedSample) {
      vectorDiagnostics = null;
      return;
    }
    vectorDiagnosticsLoading = true;
    vectorDiagnostics = await fetchVectorResearchDiagnostics(selectedSample);
    vectorDiagnosticsLoading = false;
  }

  function openEvidenceFromCitation(query: string) {
    initialSearchQuery = query;
    activeView = "evidence";
  }

  function copyToClipboard(text: string, index: number) {
    copyMessageToClipboard({
      text,
      includeTraceInExport,
      onSuccess: () => {
        copiedMsgId = index;
        setTimeout(() => {
          if (copiedMsgId === index) copiedMsgId = null;
        }, 2000);
      },
    });
  }

  async function exportConversation(type: "standard" | "clinical") {
    await exportConsultationMarkdown({
      type,
      messages,
      selectedSample,
      selectedModel,
      userProfile,
      currentSystemPrompt,
      generatedReport,
      includeTraceInExport,
      alert: (message) => dialogStore.alert(message),
    });
  }

  async function scanModels() {
    isScanning = true;
    const result = await scanChatModels(ollamaUrl, ollamaToken, selectedModel);
    models = result.models;
    selectedModel = result.selectedModel;
    scanError = result.scanError;
    isScanning = false;
  }

  async function loadModelDetails() {
    const details = await loadOllamaModelDetails(ollamaUrl, ollamaToken, selectedModel);
    modelDetails = details.modelDetails;
    isVisionCapable = details.isVisionCapable;
    contextWindow = details.contextWindow;
  }
  $effect(() => { if (selectedModel) void loadModelDetails(); });

  async function sendPrompt(customPrompt: string | undefined = undefined) {
    await sendConsultationPrompt({
      customPrompt,
      promptText,
      isChatting,
      selectedSample,
      selectedModel,
      generatedReport,
      attachedImages,
      messages,
      ollamaUrl,
      ollamaToken,
      temperature,
      maxTokens,
      extendedThinking,
      contextWindow,
      useVectorResearch,
      vectorDiagnostics,
      twoModelReview,
      reviewModel,
      selectedPacks,
      onlyActiveFindings,
      contextMode,
      consultationMode,
      userProfile,
      systemInstructions,
      alert: (message) => dialogStore.alert(message),
      onExportModal: () => { showExportModal = true; },
      onPromptInspector: () => { showPromptInspector = true; },
      onState: (patch) => {
        if (patch.promptText !== undefined) promptText = patch.promptText;
        if (patch.attachedImages !== undefined) attachedImages = patch.attachedImages;
        if (patch.messages !== undefined) messages = patch.messages;
        if (patch.isChatting !== undefined) isChatting = patch.isChatting;
        if (patch.lastVectorQuery !== undefined) lastVectorQuery = patch.lastVectorQuery;
        if (patch.lastVectorHits !== undefined) lastVectorHits = patch.lastVectorHits;
        if (patch.lastVectorEvidenceCards !== undefined) lastVectorEvidenceCards = patch.lastVectorEvidenceCards;
        if (patch.lastVectorIndexBrief !== undefined) lastVectorIndexBrief = patch.lastVectorIndexBrief;
        if (patch.lastVectorError !== undefined) lastVectorError = patch.lastVectorError;
        if (patch.sessionPromptTokens !== undefined) sessionPromptTokens = patch.sessionPromptTokens;
        if (patch.sessionResponseTokens !== undefined) sessionResponseTokens = patch.sessionResponseTokens;
      },
      getSessionPromptTokens: () => sessionPromptTokens,
      getSessionResponseTokens: () => sessionResponseTokens,
      stopListeners,
      stopReviewListeners,
      registerMainListener: (stop) => { unlistenChunk = stop; unlistenDone = null; },
      registerReviewListener: (stop) => { unlistenReviewChunk = stop; unlistenReviewDone = null; },
    });
  }

  function stopGeneration() {
    stopConsultationGeneration(messages, stopListeners, stopReviewListeners, (patch) => {
      messages = patch.messages;
      isChatting = patch.isChatting;
    });
  }
  function clearHistory() {
    dialogStore.confirm("Are you sure you want to clear this consultation's chat history?", () => { messages = []; sessionPromptTokens = 0; sessionResponseTokens = 0; }, "Clear Chat History");
  }
  function deleteMessage(index: number) {
    dialogStore.confirm("Delete this message? Deleting a question also removes its AI answer.", () => { messages = messages.filter((_, i) => messages[index].role === "user" && messages[index + 1]?.role === "assistant" ? (i !== index && i !== index + 1) : i !== index); }, "Delete Message");
  }
  function editMessage(index: number) {
    if (messages[index]?.role === "user") { dialogStore.confirm("Edit this question? This will branch the conversation from this point.", () => { promptText = messages[index].content; messages = messages.slice(0, index); }, "Edit & Branch"); }
  }
  function stopListeners() {
    if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
    unlistenDone = null;
  }

  function loadSession(id: string) {
    isLoadingSession = true;
    const snapshot = readConsultationSession(id, sessionStore, selectedSample);
    if (!snapshot) {
      isLoadingSession = false;
      return;
    }
    messages = snapshot.messages;
    selectedPacks = snapshot.selectedPacks;
    onlyActiveFindings = snapshot.onlyActiveFindings;
    temperature = snapshot.temperature;
    selectedModel = snapshot.selectedModel;
    maxTokens = snapshot.maxTokens;
    extendedThinking = snapshot.extendedThinking;
    consultationMode = snapshot.consultationMode;
    sessionPromptTokens = 0;
    sessionResponseTokens = 0;
    setTimeout(() => { isLoadingSession = false; }, 0);
  }
  async function startNewSession() {
    isLoadingSession = true;
    await startNewConsultationSession(sessionStore, selectedSample, selectedModel, models, markerPacksStore.manifest.packs);
    if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
  }
  async function saveSessionTitle(session: { title: string }) {
    await saveConsultationSessionTitle(sessionStore, session);
  }
  async function deleteSession(id: string) {
    dialogStore.confirm("Delete this consultation history? This cannot be undone.", async () => {
      isLoadingSession = true;
      await deleteConsultationSession(id, sessionStore, selectedSample, selectedModel, models, markerPacksStore.manifest.packs);
      if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
    }, "Delete Consultation");
  }

  $effect(() => {
    if (selectedSample) {
      const active = sessionStore.sessions.find(s => s.id === sessionStore.currentSessionId);
      if (!active || active.sampleId !== selectedSample.id) {
        isLoadingSession = true;
        sessionStore.load(selectedSample, selectedModel, models, markerPacksStore.manifest.packs).then(() => {
          if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
          else isLoadingSession = false;
        });
      }
    }
  });

  $effect(() => {
    if (isLoadingSession) return;
    if (!sessionStore.currentSessionId || sessionStore.sessions.length === 0) return;
    const s = sessionStore.sessions.find(x => x.id === sessionStore.currentSessionId);
    if (!s) return;
    const packsStr = JSON.stringify(selectedPacks);
    if (s.messages !== messages || JSON.stringify(s.selectedPacks) !== packsStr || s.onlyActiveFindings !== onlyActiveFindings || s.temperature !== temperature || s.selectedModel !== selectedModel || s.maxTokens !== maxTokens || s.extendedThinking !== extendedThinking || s.consultationMode !== consultationMode) {
      Object.assign(s, { messages, selectedPacks: { ...selectedPacks }, onlyActiveFindings, temperature, selectedModel, maxTokens, extendedThinking, consultationMode, timestamp: Date.now() });
      sessionStore.sessions.sort((a, b) => b.timestamp - a.timestamp);
      sessionStore.saveTitle();
    }
  });

  $effect(() => { if (checkReasoningModel(selectedModel) && !extendedThinking) extendedThinking = true; });

  let currentSystemPrompt = $derived((selectedSample && generatedReport) ? buildSystemPrompt({ selectedSample, generatedReport, selectedPacks, onlyActiveFindings, contextMode, consultationMode, userProfile, systemInstructions: systemInstructions || DEFAULT_INSTRUCTIONS, laypersonMap: LAYPERSON_MAP }) : "No sample or report loaded.");
  let contextStats = $derived(generatedReport ? calculateContextStats(generatedReport, selectedPacks, contextMode) : { included: 0, total: 0 });
  let activeCategories = $derived(generatedReport ? getActiveCategories(generatedReport, selectedPacks) : { metabolicMethylation: false, histamineCaffeine: false, pgxDrug: false, clinicalConfirmation: false });
  let dynamicCuratedQuestions = $derived(getDynamicQuestions(activeCategories));

  let lastContextSignature = $state("");
  $effect(() => {
    const sig = `${Object.keys(selectedPacks).filter(k => selectedPacks[k]).sort().join(",")}|${onlyActiveFindings}|${selectedSample?.id}`;
    if (lastContextSignature && sig !== lastContextSignature && messages.length > 0 && selectedSample && generatedReport) {
      const names = markerPacksStore.manifest.packs.filter(p => selectedPacks[p.id]).map(p => p.label).join(", ") || "None";
      const shortMsg = `Genomic context updated. Active packs: [${names}]. Findings sent: ${contextStats.included} variants.`;
      messages = [...messages, { role: "system", content: shortMsg, fullContent: shortMsg }];
    }
    lastContextSignature = sig;
  });

  // ── Lifecycle ─────────────────────────────────────────────────────────
  onMount(() => {
    void (async () => {
      ollamaUrl = loadOllamaUrl(ollamaUrl);
      ollamaToken = (await getOllamaToken()) || ollamaToken;
    })();
    if (Object.keys(selectedPacks).length === 0) {
      selectedPacks = Object.fromEntries(
        markerPacksStore.manifest.packs.map((p) => [p.id, p.default_enabled !== false])
      );
    }
    const prefs = loadAiAssistantPreferences(userProfile);
    showThinkingProcess = prefs.showThinkingProcess;
    autoCollapseThinking = prefs.autoCollapseThinking;
    includeTraceInExport = prefs.includeTraceInExport;
    contextMode = prefs.contextMode;
    consultationMode = prefs.consultationMode;
    reviewModel = prefs.reviewModel;
    twoModelReview = prefs.twoModelReview;
    userProfile = prefs.userProfile as UserBiohackingProfile;
    systemInstructions = prefs.systemInstructions;
    sessionStore.load(selectedSample, selectedModel, models, markerPacksStore.manifest.packs);
    if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
    scanModels();
    refreshVectorDiagnostics();
    return () => {
      stopListeners();
      stopReviewListeners();
    };
  });

  $effect(() => {
    if (selectedSample?.id) refreshVectorDiagnostics();
  });

  $effect(() => {
    persistAiAssistantPreferences({
      showThinkingProcess,
      autoCollapseThinking,
      includeTraceInExport,
      contextMode,
      consultationMode,
      reviewModel,
      twoModelReview,
    });
  });
</script>

<div class="ai-consultation-container">
  <ChatSidebar
    filteredSessions={filteredSessions}
    bind:currentSessionId={sessionStore.currentSessionId}
    startNewSession={startNewSession}
    loadSession={loadSession}
    deleteSession={deleteSession}
    saveSessionTitle={saveSessionTitle}
    showHistorySidebar={showHistorySidebar}
    bind:activeView={activeView}
  />
  
  {#if activeView === "chat"}
    <ChatWindow
      bind:messages={messages}
      isChatting={isChatting}
      isVisionCapable={isVisionCapable}
      bind:promptText={promptText}
      bind:attachedImages={attachedImages}
      dynamicCuratedQuestions={dynamicCuratedQuestions}
      showThinkingProcess={showThinkingProcess}
      autoCollapseThinking={autoCollapseThinking}
      userCollapsedThinkingMap={userCollapsedThinkingMap}
      selectedSample={selectedSample}
      bind:showHistorySidebar={showHistorySidebar}
      bind:showSettingsDrawer={showSettingsDrawer}
      copiedMsgId={copiedMsgId}
      sendPrompt={sendPrompt}
      stopGeneration={stopGeneration}
      clearHistory={clearHistory}
      copyToClipboard={copyToClipboard}
      editMessage={editMessage}
      deleteMessage={deleteMessage}
      selectedModel={selectedModel}
      vectorHits={lastVectorHits}
      vectorEvidenceCards={lastVectorEvidenceCards}
      vectorIndexBrief={lastVectorIndexBrief}
      vectorQuery={lastVectorQuery}
      vectorError={lastVectorError}
      useVectorResearch={useVectorResearch}
      onOpenEvidence={openEvidenceFromCitation}
      onNavigateToVariant={onNavigateToVariant}
    />
  {:else}
    <EvidenceLibraryPanel
      ollamaUrl={ollamaUrl}
      ollamaToken={ollamaToken}
      bind:showHistorySidebar={showHistorySidebar}
      bind:showSettingsDrawer={showSettingsDrawer}
      selectedSample={selectedSample}
      bind:initialSearchQuery={initialSearchQuery}
      onNavigateToVariant={onNavigateToVariant}
    />
  {/if}

  {#if showSettingsDrawer}
    <ChatSettingsDrawer
      bind:ollamaUrl={ollamaUrl}
      bind:ollamaToken={ollamaToken}
      bind:selectedModel={selectedModel}
      models={models}
      isScanning={isScanning}
      scanError={scanError}
      scanModels={scanModels}
      modelDetails={modelDetails}
      isVisionCapable={isVisionCapable}
      contextWindow={contextWindow}
      bind:temperature={temperature}
      bind:maxTokens={maxTokens}
      bind:extendedThinking={extendedThinking}
      sessionPromptTokens={sessionPromptTokens}
      sessionResponseTokens={sessionResponseTokens}
      sessionTotalTokens={sessionTotalTokens}
      bind:selectedPacks={selectedPacks}
      bind:onlyActiveFindings={onlyActiveFindings}
      contextStats={contextStats}
      bind:userProfile={userProfile}
      bind:systemInstructions={systemInstructions}
      defaultInstructions={DEFAULT_INSTRUCTIONS}
      bind:showThinkingProcess={showThinkingProcess}
      bind:autoCollapseThinking={autoCollapseThinking}
      bind:includeTraceInExport={includeTraceInExport}
      bind:contextMode={contextMode}
      bind:consultationMode={consultationMode}
      bind:reviewModel={reviewModel}
      bind:twoModelReview={twoModelReview}
      bind:useVectorResearch={useVectorResearch}
      vectorDiagnostics={vectorDiagnostics}
      vectorDiagnosticsLoading={vectorDiagnosticsLoading}
      refreshVectorDiagnostics={refreshVectorDiagnostics}
    />
  {/if}
</div>

<ChatModals
  bind:showPromptInspector={showPromptInspector}
  currentSystemPrompt={currentSystemPrompt}
  bind:showExportModal={showExportModal}
  exportConversation={exportConversation}
/>
