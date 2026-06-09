<!-- ./src/lib/components/ai/AiAssistantPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { scanOllamaModels, streamOllamaChat, showOllamaModel, saveReportJson } from "../../api/tauri";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import { LAYPERSON_MAP } from "../../utils/layperson";
  import { normalizeImage } from "../../utils/image";
  import { buildStandardMarkdown, buildClinicalHandoffMarkdown } from "../../utils/aiExport";
  import { stripThinkingTokens } from "../../utils/chatParser";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import {
    loadSessionsFromLocalStorage, saveSessionsToLocalStorage, createNewSession,
    type ChatSession
  } from "../../utils/chatSession";
  import {
    type UserBiohackingProfile, type ContextStats, type ActiveCategories,
    DEFAULT_INSTRUCTIONS, buildSystemPrompt, calculateContextStats,
    getActiveCategories, getDynamicQuestions, isReasoningModel as checkReasoningModel,
    isModelVisionCapable, getContextWindow, filterChatModels, autoSelectModel,
  } from "../../utils/aiPrompt";

  // Static Marker Packs & Manifest
  import manifest from "../../marker-packs/manifest.json";
  import core from "../../marker-packs/core.json";
  import pgx from "../../marker-packs/pgx.json";
  import metabolic from "../../marker-packs/metabolic.json";
  import nutrients from "../../marker-packs/nutrients.json";
  import neuropsych from "../../marker-packs/neuropsych.json";
  import sleep from "../../marker-packs/sleep.json";
  import connectiveTissue from "../../marker-packs/connective_tissue.json";
  import thyroidAutoimmune from "../../marker-packs/thyroid_autoimmune.json";
  import cardiovascular from "../../marker-packs/cardiovascular.json";
  import cancerConfirmationOnly from "../../marker-packs/cancer_confirmation_only.json";

  import ChatSidebar from "./ChatSidebar.svelte";
  import ChatWindow from "./ChatWindow.svelte";
  import ChatSettingsDrawer from "./ChatSettingsDrawer.svelte";
  import ChatModals from "./ChatModals.svelte";

  const PACKS_MAP: Record<string, any> = {
    core, pgx, metabolic, nutrients, neuropsych, sleep,
    connective_tissue: connectiveTissue, thyroid_autoimmune: thyroidAutoimmune,
    cardiovascular, cancer_confirmation_only: cancerConfirmationOnly,
  };

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
  let sessions = $state<ChatSession[]>([]);
  let filteredSessions = $derived(sessions.filter(s => s.sampleId === (selectedSample ? selectedSample.id : null)));
  let currentSessionId = $state<string | null>(null);

  let maxTokens = $state(2048);
  let extendedThinking = $state(false);
  let showSettingsDrawer = $state(true);
  let showHistorySidebar = $state(true);

  let showThinkingProcess = $state(true);
  let autoCollapseThinking = $state(true);
  let userCollapsedThinkingMap = $state(new Map<any, boolean>());
  let copiedMsgId = $state<number | null>(null);

  let isScanning = $state(false);
  let isChatting = $state(false);
  let scanError = $state("");
  let promptText = $state("");
  let models = $state<string[]>([]);
  let showPromptInspector = $state(false);
  let showExportModal = $state(false);
  let chatBox = $state<HTMLElement | null>(null);
  let userHasScrolledUp = $state(false);
  let isDragging = $state(false);
  let imageInput = $state<HTMLInputElement | null>(null);

  let unlistenChunk: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;

  // Token tracking
  let sessionPromptTokens = $state(0);
  let sessionResponseTokens = $state(0);
  let sessionTotalTokens = $derived(sessionPromptTokens + sessionResponseTokens);

  // Model details
  let modelDetails = $state<any>(null);
  let contextWindow = $state<number>(4096);
  let isVisionCapable = $state<boolean>(false);

  // Attachments
  let attachedImages = $state<{ name: string; base64: string; previewUrl: string }[]>([]);

  // Biohacking Profile
  let userProfile = $state<UserBiohackingProfile>({
    goals: "", challenges: "", diet: "", supplements: "", medications: "",
    bloodwork: "", diagnoses: "", supportiveTests: "", injectProfile: true,
  });

  // Custom system instructions
  let systemInstructions = $state("");

  // ── Scroll Handling ───────────────────────────────────────────────────
  function handleScroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    const threshold = 100;
    userHasScrolledUp = el.scrollHeight - el.scrollTop - el.clientHeight > threshold;
  }

  // Auto-scroll effect: only when user hasn't scrolled up
  $effect(() => {
    if (messages.length > 0 && chatBox && !userHasScrolledUp) {
      requestAnimationFrame(() => {
        if (chatBox) chatBox.scrollTop = chatBox.scrollHeight;
      });
    }
  });

  // ── Clipboard / Copy ──────────────────────────────────────────────────
  function copyToClipboard(text: string, index: number) {
    const cleanText = stripThinkingTokens(text);
    navigator.clipboard.writeText(cleanText)
      .then(() => { copiedMsgId = index; setTimeout(() => { if (copiedMsgId === index) copiedMsgId = null; }, 2000); })
      .catch(err => console.error("Failed to copy text: ", err));
  }

  // ── Export ────────────────────────────────────────────────────────────
  async function exportConversation(type: "standard" | "clinical") {
    if (messages.length === 0) { dialogStore.alert("No conversation history to export."); return; }
    const md = type === "clinical"
      ? buildClinicalHandoffMarkdown(messages, selectedSample, selectedModel, userProfile, currentSystemPrompt, generatedReport)
      : buildStandardMarkdown(messages, selectedSample, selectedModel);
    const sampleNameClean = (selectedSample ? selectedSample.name : "genome").replace(/[^a-zA-Z0-9]/g, "_");
    try {
      const saved = await saveReportJson(md, `${sampleNameClean}_consultation_${type}_${Date.now()}.md`);
      if (saved) dialogStore.alert("Chat conversation exported and saved successfully!");
    } catch (e: any) { dialogStore.alert("Failed to save chat export: " + e.message); }
  }

  // ── Image Handling ────────────────────────────────────────────────────
  async function handleImageFile(file: File) {
    if (!file.type.startsWith("image/")) { dialogStore.alert("Only image files are supported."); return; }
    try {
      const base64Str = await normalizeImage(file);
      attachedImages = [...attachedImages, { name: file.name, base64: base64Str, previewUrl: URL.createObjectURL(file) }];
    } catch (e: any) { dialogStore.alert("Failed to process image: " + e.message); }
  }
  function removeAttachedImage(index: number) {
    const img = attachedImages[index];
    if (img.previewUrl.startsWith("blob:")) URL.revokeObjectURL(img.previewUrl);
    attachedImages = attachedImages.filter((_, i) => i !== index);
  }
  function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files) { for (let i = 0; i < input.files.length; i++) handleImageFile(input.files[i]); input.value = ""; }
  }
  function handlePaste(e: ClipboardEvent) {
    if (!isVisionCapable) return;
    const items = e.clipboardData?.items;
    if (items) { for (let i = 0; i < items.length; i++) { if (items[i].type.startsWith("image/")) { const file = items[i].getAsFile(); if (file) { e.preventDefault(); handleImageFile(file); } } } }
  }
  function handleDragOver(e: DragEvent) { if (isVisionCapable) { e.preventDefault(); isDragging = true; } }
  function handleDragLeave() { isDragging = false; }
  function handleDrop(e: DragEvent) {
    if (!isVisionCapable) return;
    e.preventDefault(); isDragging = false;
    const files = e.dataTransfer?.files;
    if (files) { for (let i = 0; i < files.length; i++) handleImageFile(files[i]); }
  }

  // ── Model Details Loading ─────────────────────────────────────────────
  async function loadModelDetails() {
    if (!selectedModel) { modelDetails = null; isVisionCapable = false; contextWindow = 4096; return; }
    try {
      const details = await showOllamaModel(ollamaUrl, ollamaToken || undefined, selectedModel);
      modelDetails = details;
      isVisionCapable = isModelVisionCapable(selectedModel, details);
      contextWindow = getContextWindow(details.model_info);
    } catch (e) {
      console.error("Failed to load model details", e);
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
      const rawModels = await scanOllamaModels(ollamaUrl, ollamaToken || undefined);
      models = filterChatModels(rawModels);
      selectedModel = autoSelectModel(models, selectedModel);
    } catch (e: any) {
      let msg = `Failed to connect: ${e.message || e}`;
      if (!ollamaUrl.includes("localhost") && !ollamaUrl.includes("127.0.0.1")) {
        msg += "\n\n💡 Remote Connection Tips:\n1. Ensure Ollama is running on the remote host.\n2. Set OLLAMA_HOST=0.0.0.0 before starting Ollama.\n3. Verify port 11434 is open in the firewall.";
      }
      scanError = msg;
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

    userHasScrolledUp = false;
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
          stopListeners(); isChatting = false;
        }),
      ]);
      unlistenChunk = unChunk; unlistenDone = unDone;

      const limit = extendedThinking ? 8192 : maxTokens;
      await streamOllamaChat(ollamaUrl, ollamaToken || undefined, selectedModel, payloadMessages, temperature, limit);
    } catch (e: any) {
      stopListeners(); isChatting = false;
      messages[assistantIndex].content = `Error connecting to AI: ${e.message || e}`;
      messages = [...messages];
    }
  }

  function stopGeneration() {
    stopListeners(); isChatting = false;
    const last = messages.length - 1;
    if (last >= 0 && messages[last].role === "assistant") {
      messages[last].content += "\n\n*[Consultation response stopped by user]*";
      messages = [...messages];
    }
  }
  function clearHistory() {
    dialogStore.confirm("Are you sure you want to clear this consultation's chat history?", () => {
      messages = []; sessionPromptTokens = 0; sessionResponseTokens = 0;
    }, "Clear Chat History");
  }
  function deleteMessage(index: number) {
    dialogStore.confirm("Delete this message? Deleting a question also removes its AI answer.", () => {
      const msg = messages[index];
      if (msg.role === "user" && messages[index + 1]?.role === "assistant") {
        messages = messages.filter((_, i) => i !== index && i !== index + 1);
      } else { messages = messages.filter((_, i) => i !== index); }
    }, "Delete Message");
  }
  function editMessage(index: number) {
    if (messages[index]?.role !== "user") return;
    dialogStore.confirm("Edit this question? This will branch the conversation from this point.", () => {
      promptText = messages[index].content;
      messages = messages.slice(0, index);
    }, "Edit & Branch");
  }
  function stopListeners() {
    if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
    if (unlistenDone) { unlistenDone(); unlistenDone = null; }
  }

  // ── Session Management ────────────────────────────────────────────────
  function loadSessions() {
    sessions = loadSessionsFromLocalStorage();
    const pid = selectedSample ? selectedSample.id : null;
    const activeId = localStorage.getItem(`genomics_active_session_id_${pid}`);
    const found = activeId ? sessions.find(s => s.id === activeId && s.sampleId === pid) : null;
    if (found) loadSession(found.id);
    else { const ps = sessions.filter(s => s.sampleId === pid); if (ps.length > 0) loadSession(ps[0].id); else startNewSession(); }
  }
  function loadSession(id: string) {
    const session = sessions.find(s => s.id === id);
    if (!session) return;
    currentSessionId = id;
    const pid = selectedSample ? selectedSample.id : null;
    localStorage.setItem(`genomics_active_session_id_${pid}`, id);
    localStorage.setItem("genomics_active_session_id", id);
    messages = session.messages || [];
    selectedPacks = { ...session.selectedPacks }; onlyActiveFindings = session.onlyActiveFindings;
    temperature = session.temperature; selectedModel = session.selectedModel;
    maxTokens = session.maxTokens || 2048; extendedThinking = session.extendedThinking || false;
    sessionPromptTokens = 0; sessionResponseTokens = 0;
  }
  function startNewSession() {
    const name = selectedSample ? selectedSample.name : "Guest";
    const pid = selectedSample ? selectedSample.id : null;
    const ns = createNewSession({
      sampleName: name,
      sampleId: pid,
      selectedModel,
      models,
      manifestPacks: manifest.packs
    });
    sessions = [ns, ...sessions]; currentSessionId = ns.id;
    localStorage.setItem(`genomics_active_session_id_${pid}`, ns.id);
    localStorage.setItem("genomics_active_session_id", ns.id);
    messages = []; selectedPacks = ns.selectedPacks; onlyActiveFindings = true; temperature = 0.0; maxTokens = 2048; extendedThinking = false;
    sessionPromptTokens = 0; sessionResponseTokens = 0;
    saveSessionsToLocalStorage(sessions);
  }
  function saveSessionTitle(session: any) {
    if (session.title.trim()) { sessions = [...sessions]; saveSessionsToLocalStorage(sessions); }
  }
  function deleteSession(id: string) {
    dialogStore.confirm("Delete this consultation history? This cannot be undone.", () => {
      sessions = sessions.filter(s => s.id !== id);
      saveSessionsToLocalStorage(sessions);
      if (currentSessionId === id) { if (sessions.length > 0) loadSession(sessions[0].id); else startNewSession(); }
    }, "Delete Consultation");
  }

  // ── Profile Watcher ───────────────────────────────────────────────────
  $effect(() => {
    if (selectedSample) {
      const active = sessions.find(s => s.id === currentSessionId);
      if (!active || active.sampleId !== selectedSample.id) {
        const pid = selectedSample.id;
        const activeId = localStorage.getItem(`genomics_active_session_id_${pid}`);
        const found = activeId ? sessions.find(s => s.id === activeId && s.sampleId === pid) : null;
        if (found) loadSession(found.id);
        else { const ps = sessions.filter(s => s.sampleId === pid); if (ps.length > 0) loadSession(ps[0].id); else startNewSession(); }
      }
    }
  });

  // ── Session Auto-Save ─────────────────────────────────────────────────
  $effect(() => {
    if (!currentSessionId || sessions.length === 0) return;
    const idx = sessions.findIndex(s => s.id === currentSessionId);
    if (idx === -1) return;
    let changed = false;
    if (sessions[idx].messages !== messages) { sessions[idx].messages = messages; changed = true; }
    if (JSON.stringify(sessions[idx].selectedPacks) !== JSON.stringify(selectedPacks)) { sessions[idx].selectedPacks = { ...selectedPacks }; changed = true; }
    if (sessions[idx].onlyActiveFindings !== onlyActiveFindings) { sessions[idx].onlyActiveFindings = onlyActiveFindings; changed = true; }
    if (sessions[idx].temperature !== temperature) { sessions[idx].temperature = temperature; changed = true; }
    if (sessions[idx].selectedModel !== selectedModel) { sessions[idx].selectedModel = selectedModel; changed = true; }
    if (sessions[idx].maxTokens !== maxTokens) { sessions[idx].maxTokens = maxTokens; changed = true; }
    if (sessions[idx].extendedThinking !== extendedThinking) { sessions[idx].extendedThinking = extendedThinking; changed = true; }
    if (changed) {
      sessions[idx].timestamp = Date.now();
      sessions.sort((a, b) => b.timestamp - a.timestamp);
      saveSessionsToLocalStorage(sessions);
    }
  });

  // ── Reasoning Model Auto-Toggle ───────────────────────────────────────
  let isReasoning = $derived(checkReasoningModel(selectedModel));
  $effect(() => { if (isReasoning && !extendedThinking) extendedThinking = true; });

  // ── Derived: System Prompt (uses utility) ─────────────────────────────
  let currentSystemPrompt = $derived.by(() => {
    if (!selectedSample || !generatedReport) return "No sample or report loaded.";
    return buildSystemPrompt({
      selectedSample, generatedReport, selectedPacks, onlyActiveFindings,
      userProfile, systemInstructions: systemInstructions || DEFAULT_INSTRUCTIONS,
      manifestPacks: manifest.packs, packsMap: PACKS_MAP, laypersonMap: LAYPERSON_MAP,
    });
  });

  // ── Derived: Context Stats ────────────────────────────────────────────
  let contextStats = $derived.by((): ContextStats => {
    if (!generatedReport) return { included: 0, total: 0 };
    return calculateContextStats(generatedReport, selectedPacks, onlyActiveFindings, manifest.packs, PACKS_MAP);
  });

  // ── Derived: Active Categories & Dynamic Questions ────────────────────
  let activeCategories = $derived.by((): ActiveCategories => {
    if (!generatedReport) return { metabolicMethylation: false, histamineCaffeine: false, pgxDrug: false, clinicalConfirmation: false };
    return getActiveCategories(generatedReport, selectedPacks, manifest.packs, PACKS_MAP);
  });
  let dynamicCuratedQuestions = $derived(getDynamicQuestions(activeCategories));

  // ── Context Update Notifications ──────────────────────────────────────
  let lastContextSignature = $state("");
  $effect(() => {
    const activePacksStr = Object.entries(selectedPacks).filter(([_, on]) => on).map(([id]) => id).sort().join(",");
    const sig = `${activePacksStr}|${onlyActiveFindings}|${selectedSample?.id}`;
    if (lastContextSignature === "") { lastContextSignature = sig; return; }
    if (sig !== lastContextSignature) {
      lastContextSignature = sig;
      if (messages.length > 0 && selectedSample && generatedReport) {
        const packNames = manifest.packs.filter(p => selectedPacks[p.id]).map(p => p.label).join(", ");
        const shortMsg = `Genomic context updated. Active packs: [${packNames || "None"}]. Findings sent: ${contextStats.included} variants.`;
        messages = [...messages, { role: "system", content: shortMsg, fullContent: shortMsg }];
      }
    }
  });

  // ── Lifecycle ─────────────────────────────────────────────────────────
  onMount(() => {
    const savedUrl = localStorage.getItem("genomics_ollama_url");
    if (savedUrl && (!ollamaUrl || ollamaUrl === "http://localhost:11434")) ollamaUrl = savedUrl;
    const savedToken = localStorage.getItem("genomics_ollama_token");
    if (savedToken && !ollamaToken) ollamaToken = savedToken;
    if (Object.keys(selectedPacks).length === 0) {
      const packs: Record<string, boolean> = {};
      for (const pack of manifest.packs) packs[pack.id] = true;
      selectedPacks = packs;
    }
    const st = localStorage.getItem("genomics_show_thinking_process");
    if (st !== null) showThinkingProcess = st === "true";
    const ac = localStorage.getItem("genomics_auto_collapse_thinking");
    if (ac !== null) autoCollapseThinking = ac === "true";
    const sp = localStorage.getItem("genomics_user_biohacking_profile");
    if (sp) { try { userProfile = { ...userProfile, ...JSON.parse(sp) }; } catch { /* ignore */ } }
    const si = localStorage.getItem("genomics_system_instructions");
    if (si !== null) systemInstructions = si; else systemInstructions = DEFAULT_INSTRUCTIONS;
    loadSessions(); scanModels();
    return () => stopListeners();
  });
</script>

<div class="ai-consultation-container">
  <ChatSidebar
    filteredSessions={filteredSessions}
    bind:currentSessionId={currentSessionId}
    startNewSession={startNewSession}
    loadSession={loadSession}
    deleteSession={deleteSession}
    saveSessionTitle={saveSessionTitle}
    showHistorySidebar={showHistorySidebar}
  />
  
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
    userHasScrolledUp={userHasScrolledUp}
    bind:chatBox={chatBox}
    bind:isDragging={isDragging}
    copiedMsgId={copiedMsgId}
    bind:imageInput={imageInput}
    sendPrompt={sendPrompt}
    stopGeneration={stopGeneration}
    clearHistory={clearHistory}
    copyToClipboard={copyToClipboard}
    editMessage={editMessage}
    deleteMessage={deleteMessage}
    handleScroll={handleScroll}
    handlePaste={handlePaste}
    handleDragOver={handleDragOver}
    handleDragLeave={handleDragLeave}
    handleDrop={handleDrop}
    handleFileChange={handleFileChange}
    removeAttachedImage={removeAttachedImage}
  />

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
  .ai-consultation-container {
    display: flex;
    gap: 0;
    height: calc(100vh - 140px);
    width: 100%;
    box-sizing: border-box;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--border-color);
    background: rgba(10, 11, 20, 0.3);
    backdrop-filter: blur(16px);
  }
</style>
