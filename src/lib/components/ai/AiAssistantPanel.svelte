<!-- ./src/lib/components/ai/AiAssistantPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { scanOllamaModels, streamOllamaChat, showOllamaModel, saveReportJson } from "../../api/tauri";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import { LAYPERSON_MAP } from "../../utils/layperson";
  import { buildStandardMarkdown, buildClinicalHandoffMarkdown } from "../../utils/aiExport";
  import { stripThinkingTokens } from "../../utils/chatParser";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import { type ChatSession } from "../../utils/chatSession";
  import { ConsultationSessionStore } from "../../utils/consultationSession.svelte";
  import {
    type UserBiohackingProfile, type ContextStats, type ActiveCategories,
    DEFAULT_INSTRUCTIONS, buildSystemPrompt, calculateContextStats,
    getActiveCategories, getDynamicQuestions, isReasoningModel as checkReasoningModel,
    isModelVisionCapable, getContextWindow, filterChatModels, autoSelectModel,
    type AiContextMode, type ConsultationMode
  } from "../../utils/aiPrompt";
  import manifest from "../../marker-packs/manifest.json";
  import ChatSidebar from "./ChatSidebar.svelte";
  import ChatWindow from "./ChatWindow.svelte";
  import ChatSettingsDrawer from "./ChatSettingsDrawer.svelte";
  import ChatModals from "./ChatModals.svelte";
  import EvidenceLibraryPanel from "./EvidenceLibraryPanel.svelte";

  // ── Props ──────────────────────────────────────────────────────────────
  interface Props {
    selectedSample: GenomeSample | null;
    generatedReport: GeneratedReport | null;
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
    messages: any[];
    selectedPacks: Record<string, boolean>;
    onlyActiveFindings: boolean;
    temperature: number;
  }
  let {
    selectedSample, generatedReport,
    ollamaUrl = $bindable("http://localhost:11434"),
    ollamaToken = $bindable(""),
    selectedModel = $bindable(""),
    messages = $bindable([]),
    selectedPacks = $bindable({}),
    onlyActiveFindings = $bindable(true),
    temperature = $bindable(0.0),
  }: Props = $props();

  // ── Core UI State ──────────────────────────────────────────────────────
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
  let activeView = $state<"chat" | "evidence">("chat");

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
    if (unlistenReviewDone) { unlistenReviewDone(); unlistenReviewDone = null; }
  }

  let sessionPromptTokens = $state(0), sessionResponseTokens = $state(0);
  let sessionTotalTokens = $derived(sessionPromptTokens + sessionResponseTokens);
  let modelDetails = $state<any>(null), contextWindow = $state<number>(4096), isVisionCapable = $state<boolean>(false);
  let attachedImages = $state<{ name: string; base64: string; previewUrl: string }[]>([]);
  let userProfile = $state<UserBiohackingProfile>({ goals: "", challenges: "", diet: "", supplements: "", medications: "", bloodwork: "", diagnoses: "", supportiveTests: "", injectProfile: true });
  let systemInstructions = $state("");

  // ── Clipboard / Copy ──────────────────────────────────────────────────
  function copyToClipboard(text: string, index: number) {
    navigator.clipboard.writeText(includeTraceInExport ? text : stripThinkingTokens(text))
      .then(() => { copiedMsgId = index; setTimeout(() => { if (copiedMsgId === index) copiedMsgId = null; }, 2000); })
      .catch(err => console.error("Failed to copy text: ", err));
  }

  // ── Export ────────────────────────────────────────────────────────────
  async function exportConversation(type: "standard" | "clinical") {
    if (!messages.length) { dialogStore.alert("No conversation history to export."); return; }
    const md = type === "clinical"
      ? buildClinicalHandoffMarkdown(messages, selectedSample, selectedModel, userProfile, currentSystemPrompt, generatedReport, includeTraceInExport)
      : buildStandardMarkdown(messages, selectedSample, selectedModel, includeTraceInExport);
    const cleanName = (selectedSample ? selectedSample.name : "genome").replace(/[^a-zA-Z0-9]/g, "_");
    try {
      if (await saveReportJson(md, `${cleanName}_consultation_${type}_${Date.now()}.md`))
        dialogStore.alert("Chat conversation exported and saved successfully!");
    } catch (e: any) { dialogStore.alert("Failed to save chat export: " + e.message); }
  }

  // ── Model Details Loading ─────────────────────────────────────────────
  async function loadModelDetails() {
    if (!selectedModel) { modelDetails = null; isVisionCapable = false; contextWindow = 4096; return; }
    try {
      const details = await showOllamaModel(ollamaUrl, ollamaToken || undefined, selectedModel);
      modelDetails = details;
      isVisionCapable = isModelVisionCapable(selectedModel, details);
      contextWindow = getContextWindow(details.model_info);
    } catch {
      isVisionCapable = isModelVisionCapable(selectedModel, null);
      contextWindow = 4096;
    }
  }
  $effect(() => { if (selectedModel) loadModelDetails(); });

  // ── Model Scanning ────────────────────────────────────────────────────
  async function scanModels() {
    isScanning = true; scanError = ""; models = [];
    localStorage.setItem("genomics_ollama_url", ollamaUrl);
    localStorage.setItem("genomics_ollama_token", ollamaToken);
    try {
      models = filterChatModels(await scanOllamaModels(ollamaUrl, ollamaToken || undefined));
      selectedModel = autoSelectModel(models, selectedModel);
    } catch (e: any) {
      scanError = `Failed to connect: ${e.message || e}`;
      if (!ollamaUrl.includes("localhost") && !ollamaUrl.includes("127.0.0.1")) {
        scanError += "\n\n💡 Remote Connection Tips:\n1. Ensure Ollama is running on the remote host.\n2. Set OLLAMA_HOST=0.0.0.0 before starting Ollama.\n3. Verify port 11434 is open in the firewall.";
      }
    } finally { isScanning = false; }
  }

  // ── Chat Sending ──────────────────────────────────────────────────────
  async function sendPrompt(customPrompt?: string) {
    if (customPrompt === "TRIGGER_EXPORT_MODAL") { showExportModal = true; return; }
    if (customPrompt === "TRIGGER_CONTEXT_INSPECTOR") { showPromptInspector = true; return; }
    const text = customPrompt || promptText.trim();
    if (!text || isChatting || !selectedSample) return;
    if (!selectedModel) { dialogStore.alert("Please configure a connection and select an LLM model."); return; }
    if (!generatedReport) { dialogStore.alert("Report calculations are still loading. Please wait a moment."); return; }

    const userMsg: any = { role: "user", content: text };
    if (attachedImages.length > 0) userMsg.images = attachedImages.map(img => img.base64);
    messages = [...messages, userMsg];
    for (const img of attachedImages) { if (img.previewUrl.startsWith("blob:")) URL.revokeObjectURL(img.previewUrl); }
    attachedImages = [];
    if (!customPrompt) promptText = "";
    isChatting = true;
    messages = [...messages, { role: "assistant", content: "" }];
    const assistantIndex = messages.length - 1;

    try {
      const chatHistory = messages.slice(0, -1).map(m => {
        const msgObj: any = { role: m.role, content: m.fullContent || m.content };
        if (m.images?.length > 0) msgObj.images = m.images;
        return msgObj;
      });
      const payloadMessages = [{ role: "system", content: currentSystemPrompt }, ...chatHistory];
      stopListeners();

      const [unChunk, unDone] = await Promise.all([
        listen("ollama-chunk", (event) => {
          messages[assistantIndex].content += event.payload as string;
          messages = [...messages];
        }),
        listen("ollama-done", (event) => {
          const payload = event.payload as { prompt_eval_count?: number; eval_count?: number } | null;
          if (payload?.prompt_eval_count) sessionPromptTokens += payload.prompt_eval_count;
          if (payload?.eval_count) sessionResponseTokens += payload.eval_count;
          stopListeners();
        }),
      ]);
      unlistenChunk = unChunk; unlistenDone = unDone;

      const limit = extendedThinking ? 8192 : maxTokens;
      await streamOllamaChat(ollamaUrl, ollamaToken || undefined, selectedModel, payloadMessages, temperature, limit);

      // --- Dual-Model Safety Review ---
      if (twoModelReview && reviewModel) {
        messages[assistantIndex].safetyReview = "Reviewing response safety...";
        messages = [...messages];

        try {
          const reviewPrompt = `You are a medical safety auditor. Review the following genomic consultation draft for any clinical overclaiming, dosing advice, or diagnosing assertions. Output your safety corrections, warnings, or notes to the patient.

Draft Response to Review:
"""
${stripThinkingTokens(messages[assistantIndex].content)}
"""`;

          stopReviewListeners();

          const [unReviewChunk, unReviewDone] = await Promise.all([
            listen("ollama-chunk", (event) => {
              if (messages[assistantIndex].safetyReview === "Reviewing response safety...") {
                messages[assistantIndex].safetyReview = "";
              }
              messages[assistantIndex].safetyReview += event.payload as string;
              messages = [...messages];
            }),
            listen("ollama-done", (event) => {
              const payload = event.payload as { prompt_eval_count?: number; eval_count?: number } | null;
              if (payload?.prompt_eval_count) sessionPromptTokens += payload.prompt_eval_count;
              if (payload?.eval_count) sessionResponseTokens += payload.eval_count;
              stopReviewListeners();
              isChatting = false;
            }),
          ]);
          unlistenReviewChunk = unReviewChunk;
          unlistenReviewDone = unReviewDone;

          await streamOllamaChat(ollamaUrl, ollamaToken || undefined, reviewModel, [{ role: "user", content: reviewPrompt }], 0.0, 2048);
        } catch (revError: any) {
          stopReviewListeners();
          messages[assistantIndex].safetyReview = `⚠️ Safety Review Failed: ${revError.message || revError}`;
          messages = [...messages];
          isChatting = false;
        }
      } else {
        isChatting = false;
      }
    } catch (e: any) {
      stopListeners();
      stopReviewListeners();
      isChatting = false;
      if (messages[assistantIndex].content === "") {
        messages[assistantIndex].content = `Error connecting to AI: ${e.message || e}`;
      } else {
        messages[assistantIndex].content += `\n\n*[Error during stream: ${e.message || e}]*`;
      }
      messages = [...messages];
    }
  }

  function stopGeneration() {
    stopListeners();
    stopReviewListeners();
    isChatting = false;
    const last = messages.length - 1;
    if (last >= 0 && messages[last].role === "assistant") {
      if (messages[last].safetyReview === "Reviewing response safety...") {
        messages[last].safetyReview = "*[Review stopped by user]*";
      }
      messages[last].content += "\n\n*[Consultation response stopped by user]*";
      messages = [...messages];
    }
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
    if (unlistenDone) { unlistenDone(); unlistenDone = null; }
  }

  // ── Session Management ────────────────────────────────────────────────
  function loadSession(id: string) {
    isLoadingSession = true;
    const s = sessionStore.sessions.find(x => x.id === id);
    if (!s) {
      isLoadingSession = false;
      return;
    }
    sessionStore.currentSessionId = id;
    const pid = selectedSample ? selectedSample.id : null;
    localStorage.setItem(`genomics_active_session_id_${pid}`, id);
    localStorage.setItem("genomics_active_session_id", id);
    messages = s.messages || [];
    selectedPacks = { ...s.selectedPacks }; onlyActiveFindings = s.onlyActiveFindings;
    temperature = s.temperature; selectedModel = s.selectedModel;
    maxTokens = s.maxTokens || 2048; extendedThinking = s.extendedThinking || false;
    consultationMode = (s.consultationMode || "general") as ConsultationMode;
    sessionPromptTokens = 0; sessionResponseTokens = 0;
    setTimeout(() => {
      isLoadingSession = false;
    }, 0);
  }
  async function startNewSession() {
    isLoadingSession = true;
    await sessionStore.startNew(selectedSample, selectedModel, models, manifest.packs);
    if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
  }
  async function saveSessionTitle(session: any) {
    if (session.title.trim()) await sessionStore.saveTitle();
  }
  async function deleteSession(id: string) {
    dialogStore.confirm("Delete this consultation history? This cannot be undone.", async () => {
      isLoadingSession = true;
      await sessionStore.delete(id, selectedSample, selectedModel, models, manifest.packs);
      if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
    }, "Delete Consultation");
  }

  // ── Profile Watcher ───────────────────────────────────────────────────
  $effect(() => {
    if (selectedSample) {
      const active = sessionStore.sessions.find(s => s.id === sessionStore.currentSessionId);
      if (!active || active.sampleId !== selectedSample.id) {
        isLoadingSession = true;
        sessionStore.load(selectedSample, selectedModel, models, manifest.packs).then(() => {
          if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
          else isLoadingSession = false;
        });
      }
    }
  });

  // ── Session Auto-Save ─────────────────────────────────────────────────
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

  // ── Derived State Calculations ────────────────────────────────────────
  let currentSystemPrompt = $derived((selectedSample && generatedReport) ? buildSystemPrompt({ selectedSample, generatedReport, selectedPacks, onlyActiveFindings, contextMode, consultationMode, userProfile, systemInstructions: systemInstructions || DEFAULT_INSTRUCTIONS, laypersonMap: LAYPERSON_MAP }) : "No sample or report loaded.");
  let contextStats = $derived(generatedReport ? calculateContextStats(generatedReport, selectedPacks, contextMode) : { included: 0, total: 0 });
  let activeCategories = $derived(generatedReport ? getActiveCategories(generatedReport, selectedPacks) : { metabolicMethylation: false, histamineCaffeine: false, pgxDrug: false, clinicalConfirmation: false });
  let dynamicCuratedQuestions = $derived(getDynamicQuestions(activeCategories));

  // ── Context Update Notifications ──────────────────────────────────────
  let lastContextSignature = $state("");
  $effect(() => {
    const sig = `${Object.keys(selectedPacks).filter(k => selectedPacks[k]).sort().join(",")}|${onlyActiveFindings}|${selectedSample?.id}`;
    if (lastContextSignature && sig !== lastContextSignature && messages.length > 0 && selectedSample && generatedReport) {
      const names = manifest.packs.filter(p => selectedPacks[p.id]).map(p => p.label).join(", ") || "None";
      const shortMsg = `Genomic context updated. Active packs: [${names}]. Findings sent: ${contextStats.included} variants.`;
      messages = [...messages, { role: "system", content: shortMsg, fullContent: shortMsg }];
    }
    lastContextSignature = sig;
  });

  // ── Lifecycle ─────────────────────────────────────────────────────────
  onMount(() => {
    ollamaUrl = localStorage.getItem("genomics_ollama_url") || ollamaUrl;
    ollamaToken = localStorage.getItem("genomics_ollama_token") || ollamaToken;
    if (Object.keys(selectedPacks).length === 0) {
      selectedPacks = Object.fromEntries(manifest.packs.map(p => [p.id, true]));
    }
    showThinkingProcess = localStorage.getItem("genomics_show_thinking_process") !== "false";
    autoCollapseThinking = localStorage.getItem("genomics_auto_collapse_thinking") !== "false";
    includeTraceInExport = localStorage.getItem("genomics_include_trace_in_export") === "true";
    contextMode = (localStorage.getItem("genomics_context_mode") || "active_findings") as AiContextMode;
    consultationMode = (localStorage.getItem("genomics_consultation_mode") || "general") as ConsultationMode;
    reviewModel = localStorage.getItem("genomics_review_model") || "";
    twoModelReview = localStorage.getItem("genomics_two_model_review") === "true";
    try { userProfile = { ...userProfile, ...JSON.parse(localStorage.getItem("genomics_user_biohacking_profile") || "{}") }; } catch {}
    systemInstructions = localStorage.getItem("genomics_system_instructions") || DEFAULT_INSTRUCTIONS;
    sessionStore.load(selectedSample, selectedModel, models, manifest.packs);
    if (sessionStore.currentSessionId) loadSession(sessionStore.currentSessionId);
    scanModels();
    return () => {
      stopListeners();
      stopReviewListeners();
    };
  });

  $effect(() => {
    const sets = { show_thinking_process: showThinkingProcess, auto_collapse_thinking: autoCollapseThinking, include_trace_in_export: includeTraceInExport, context_mode: contextMode, consultation_mode: consultationMode, review_model: reviewModel, two_model_review: twoModelReview };
    for (const [k, v] of Object.entries(sets)) localStorage.setItem(`genomics_${k}`, String(v));
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
    />
  {:else}
    <EvidenceLibraryPanel
      ollamaUrl={ollamaUrl}
      ollamaToken={ollamaToken}
      bind:showHistorySidebar={showHistorySidebar}
      bind:showSettingsDrawer={showSettingsDrawer}
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
    />
  {/if}
</div>

<ChatModals
  bind:showPromptInspector={showPromptInspector}
  currentSystemPrompt={currentSystemPrompt}
  bind:showExportModal={showExportModal}
  exportConversation={exportConversation}
/>

<style>
  .ai-consultation-container { display: flex; gap: 0; height: calc(100vh - 140px); width: 100%; box-sizing: border-box; border-radius: 12px; overflow: hidden; border: 1px solid var(--border-color); background: rgba(10, 11, 20, 0.3); backdrop-filter: blur(16px); }
</style>
