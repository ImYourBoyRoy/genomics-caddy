<!-- ./src/lib/components/ai/ChatWindow.svelte -->
<script lang="ts">
  import WelcomeChat from './WelcomeChat.svelte';
  import ChatMessages from './ChatMessages.svelte';
  import ChatInput from './ChatInput.svelte';

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
    userHasScrolledUp: boolean;
    chatBox: HTMLElement | null;
    isDragging: boolean;
    copiedMsgId: number | null;
    imageInput: HTMLInputElement | null;
    
    // Actions/Handlers
    sendPrompt: (customPrompt?: string) => void;
    stopGeneration: () => void;
    clearHistory: () => void;
    copyToClipboard: (text: string, index: number) => void;
    editMessage: (index: number) => void;
    deleteMessage: (index: number) => void;
    handleScroll: (e: Event) => void;
    handlePaste: (e: ClipboardEvent) => void;
    handleDragOver: (e: DragEvent) => void;
    handleDragLeave: () => void;
    handleDrop: (e: DragEvent) => void;
    handleFileChange: (e: Event) => void;
    removeAttachedImage: (index: number) => void;
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
    userHasScrolledUp,
    chatBox = $bindable(),
    isDragging = $bindable(),
    copiedMsgId,
    imageInput = $bindable(),
    
    sendPrompt,
    stopGeneration,
    clearHistory,
    copyToClipboard,
    editMessage,
    deleteMessage,
    handleScroll,
    handlePaste,
    handleDragOver,
    handleDragLeave,
    handleDrop,
    handleFileChange,
    removeAttachedImage
  }: Props = $props();
</script>

<section class="ai-chat-area">
  <div class="chat-header">
    <div class="chat-title-info">
      <button 
        type="button"
        class="btn btn-secondary btn-sm toggle-sidebar-btn" 
        onclick={() => showHistorySidebar = !showHistorySidebar}
        class:drawer-open={showHistorySidebar}
        title="Toggle History Sidebar"
        style="margin-right: 8px;"
      >
        📂
      </button>
      <h3>Private AI Consultation</h3>
      <span class="badge success">🔒 Privacy Secured: Local Only</span>
    </div>
    
    <div class="chat-header-actions">
      {#if messages.length > 0}
        <button class="btn btn-secondary btn-sm" onclick={() => sendPrompt("TRIGGER_EXPORT_MODAL")} title="Export options">
          📥 Export Chat
        </button>
      {/if}
      <button 
        class="btn btn-secondary btn-sm"
        onclick={() => sendPrompt("TRIGGER_CONTEXT_INSPECTOR")}
        disabled={!selectedSample}
        title="Inspect System Prompt & DNA Context"
      >
        🔍 Context
      </button>
      <button 
        class="btn btn-secondary btn-sm toggle-settings-btn" 
        onclick={() => showSettingsDrawer = !showSettingsDrawer}
        class:drawer-open={showSettingsDrawer}
        title="Toggle Model Settings"
      >
        ⚙️ Settings
      </button>
    </div>
  </div>

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

    {#if messages.length === 0}
      <WelcomeChat />
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
      />
    {/if}
  </div>

  <!-- Curated Helpers -->
  {#if !isChatting}
    <div class="curated-questions-bar">
      {#each dynamicCuratedQuestions as q}
        <button class="curated-btn" onclick={() => sendPrompt(q.text)}>
          {q.label}
        </button>
      {/each}
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

<style>
  .ai-chat-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.01);
    min-width: 0;
    height: 100%;
    box-sizing: border-box;
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 18px;
    background: rgba(0, 0, 0, 0.15);
    border-bottom: 1px solid var(--border-color);
  }

  .chat-title-info {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .chat-title-info h3 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chat-header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .btn-sm {
    padding: 6px 12px;
    font-size: 0.78rem;
    border-radius: 6px;
    white-space: nowrap;
  }

  .toggle-settings-btn.drawer-open, .toggle-sidebar-btn.drawer-open {
    background: rgba(88, 80, 236, 0.2);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .chat-box {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    background: radial-gradient(circle at bottom, rgba(18, 20, 32, 0.4), transparent);
    position: relative;
  }

  .chat-box.dragging-active {
    border: 2px dashed var(--accent);
    background: rgba(88, 80, 236, 0.04) !important;
  }

  .drag-drop-overlay {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(10, 11, 18, 0.85);
    backdrop-filter: blur(4px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 10;
    pointer-events: none;
  }

  .drag-drop-overlay-box {
    border: 2px dashed var(--accent);
    background: rgba(88, 80, 236, 0.1);
    color: #a5b4fc;
    font-weight: 500;
    padding: 24px 32px;
    border-radius: 12px;
    font-size: 0.95rem;
    text-align: center;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
  }

  /* Curated Prompts Bar */
  .curated-questions-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px 20px;
    background: rgba(0, 0, 0, 0.1);
    border-top: 1px solid var(--border-color);
  }

  .curated-btn {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
    padding: 6px 12px;
    border-radius: 9999px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .curated-btn:hover {
    background: rgba(88, 80, 236, 0.1);
    border-color: rgba(88, 80, 236, 0.4);
    color: var(--text-primary);
  }

  .badge.success {
    /* inherits global badge styles */
  }
</style>
