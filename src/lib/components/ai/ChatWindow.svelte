<!-- ./src/lib/components/ai/ChatWindow.svelte -->
<script lang="ts">
  import WelcomeChat from './WelcomeChat.svelte';
  import ChatMessages from './ChatMessages.svelte';
  import ChatInput from './ChatInput.svelte';
  import VectorResearchCitations from './VectorResearchCitations.svelte';
  import { normalizeImage } from '../../utils/image';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import type { QdrantHit, EvidenceCard } from '../../types/research';
  import type { VariantNavTarget } from '../../constants/traitCategories';
  import { classifyOllamaEndpoint } from '../../utils/ollamaSettings';
  import type { ChatProvider } from '../../utils/chatProvider';
  import { chatgptOpenUsage } from '../../api/chatgpt';
  import '$lib/styles/components/chat-window.css';

  interface Props {
    messages: { role: "user" | "assistant" | "system"; content: string; fullContent?: string; images?: string[] }[];
    isChatting: boolean;
    isVisionCapable: boolean;
    promptText: string;
    attachedImages: { name: string; base64: string; previewUrl: string }[];
    dynamicCuratedQuestions: { label: string; text: string }[];
    showThinkingProcess: boolean;
    autoCollapseThinking: boolean;
    userCollapsedThinkingMap: Map<any, boolean>;
    selectedSample: any;
    showHistorySidebar: boolean;
    showSettingsDrawer: boolean;
    vectorHits?: QdrantHit[];
    vectorEvidenceCards?: EvidenceCard[];
    vectorIndexBrief?: string;
    vectorQuery?: string;
    vectorError?: string;
    useVectorResearch?: boolean;
    onOpenEvidence?: (query: string) => void;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;

    // Actions/Handlers
    sendPrompt: (customPrompt?: string) => void;
    stopGeneration: () => void;
    clearHistory: () => void;
    editMessage: (index: number) => void;
    deleteMessage: (index: number) => void;

    copiedMsgId: number | null;
    copyToClipboard: (text: string, index: number) => void;
    selectedModel: string;
    ollamaUrl?: string;
    chatProvider?: ChatProvider;
    /** One-line honesty about pool vs sent / remap / history trim for the last turn. */
    contextBudgetNotice?: string;
  }

  let {
    messages = $bindable(),
    isChatting,
    isVisionCapable,
    promptText = $bindable(),
    attachedImages = $bindable(),
    dynamicCuratedQuestions,
    showThinkingProcess,
    autoCollapseThinking,
    userCollapsedThinkingMap,
    selectedSample,
    showHistorySidebar = $bindable(),
    showSettingsDrawer = $bindable(),

    sendPrompt,
    stopGeneration,
    clearHistory,
    editMessage,
    deleteMessage,

    copiedMsgId = $bindable(null),
    copyToClipboard,
    selectedModel,
    ollamaUrl = "",
    chatProvider = "ollama",
    contextBudgetNotice = "",
    vectorHits = [],
    vectorEvidenceCards = [],
    vectorIndexBrief = "",
    vectorQuery = "",
    vectorError = "",
    useVectorResearch = true,
    onOpenEvidence,
    onNavigateToVariant,
  }: Props = $props();

  let usingChatGpt = $derived(chatProvider === 'chatgpt');
  let endpointScope = $derived(usingChatGpt ? 'remote' : classifyOllamaEndpoint(ollamaUrl));
  let endpointHost = $derived.by(() => {
    if (usingChatGpt) return 'OpenAI (ChatGPT plan)';
    try { return new URL(ollamaUrl.trim()).host; } catch { return ""; }
  });
  let scopeLabel = $derived(
    usingChatGpt ? 'ChatGPT plan · cloud'
      : endpointScope === 'local' ? 'On this device'
      : endpointScope === 'remote' ? `Remote · ${endpointHost || 'custom host'}`
      : 'Not connected'
  );

  // Internal local state
  let chatBox = $state<HTMLElement | null>(null);
  let userHasScrolledUp = $state(false);
  let isDragging = $state(false);
  let imageInput = $state<HTMLInputElement | null>(null);

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
</script>

<section class="ai-chat-area">
  <header class="chat-header">
    <div class="chat-title-info">
      <button
        type="button"
        class="chat-icon-btn toggle-sidebar-btn"
        onclick={() => showHistorySidebar = !showHistorySidebar}
        class:drawer-open={showHistorySidebar}
        aria-label="Toggle history sidebar"
        aria-pressed={showHistorySidebar}
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">
          <rect x="2" y="3" width="12" height="10" rx="2" />
          <path d="M6 3v10" />
        </svg>
      </button>
      <div class="chat-title-stack">
        <h3>Ask your DNA</h3>
        <button
          type="button"
          class="chat-scope-pill"
          data-scope={endpointScope}
          aria-label={`AI endpoint: ${scopeLabel}. Open connection settings`}
          onclick={() => showSettingsDrawer = true}
        >
          <span class="chat-scope-dot" aria-hidden="true"></span>{scopeLabel}
        </button>
      </div>
    </div>

    <div class="chat-header-actions">
      {#if messages.length > 0}
        <button type="button" class="btn btn-secondary chat-header-btn" onclick={() => sendPrompt("TRIGGER_EXPORT_MODAL")}>
          📥 Export
        </button>
      {/if}
      <button
        type="button"
        class="btn btn-secondary chat-header-btn"
        onclick={() => sendPrompt("TRIGGER_CONTEXT_INSPECTOR")}
        disabled={!selectedSample}
      >
        🔍 Context
      </button>
      <button
        type="button"
        class="btn btn-secondary chat-header-btn toggle-settings-btn"
        onclick={() => showSettingsDrawer = !showSettingsDrawer}
        class:drawer-open={showSettingsDrawer}
        aria-pressed={showSettingsDrawer}
      >
        ⚙️ Model
      </button>
    </div>
  </header>

  <!-- Chat Display Scrollable -->
  <div
    class="chat-box"
    bind:this={chatBox}
    class:dragging-active={isDragging}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
    onscroll={handleScroll}
    role="presentation"
  >
    {#if isDragging}
      <div class="drag-drop-overlay">
        <div class="drag-drop-overlay-box">
          <span>📥 Drop image files here to attach (normalized dynamically)</span>
        </div>
      </div>
    {/if}

    {#if contextBudgetNotice}
      <p class="context-budget-notice" role="status">{contextBudgetNotice}</p>
    {/if}

    {#if messages.length === 0}
      <WelcomeChat
        questions={dynamicCuratedQuestions}
        scope={endpointScope}
        host={endpointHost}
        {sendPrompt}
        onOpenConnection={() => showSettingsDrawer = true}
      />
    {:else}
      <ChatMessages
        {messages}
        {isChatting}
        {copiedMsgId}
        {showThinkingProcess}
        {autoCollapseThinking}
        {userCollapsedThinkingMap}
        {copyToClipboard}
        {editMessage}
        {deleteMessage}
        {selectedModel}
      />
    {/if}
  </div>

  <VectorResearchCitations
    hits={vectorHits}
    evidenceCards={vectorEvidenceCards}
    indexBrief={vectorIndexBrief}
    query={vectorQuery}
    error={vectorError}
    enabled={useVectorResearch}
    onOpenEvidence={onOpenEvidence}
    onNavigateToVariant={onNavigateToVariant}
  />

  <!-- Curated Helpers -->
  {#if !isChatting && messages.length > 0}
    <div class="curated-questions-bar">
      {#each dynamicCuratedQuestions as q (q.label)}
        <button class="curated-btn" onclick={() => sendPrompt(q.text)}>
          {q.label}
        </button>
      {/each}
    </div>
  {/if}

  {#if usingChatGpt}
    <div class="chat-plan-strip" role="note">
      <span class="chat-plan-strip-dot" aria-hidden="true"></span>
      <span>Using your ChatGPT plan</span>
      <button type="button" class="chat-plan-strip-link" onclick={() => void chatgptOpenUsage()}>Manage usage</button>
    </div>
  {/if}

  <!-- Chat Action Input Area -->
  <ChatInput
    bind:promptText
    bind:attachedImages
    bind:imageInput
    {isChatting}
    {isVisionCapable}
    {messages}
    {sendPrompt}
    {stopGeneration}
    {clearHistory}
    {handlePaste}
    {handleFileChange}
    {removeAttachedImage}
  />
</section>
