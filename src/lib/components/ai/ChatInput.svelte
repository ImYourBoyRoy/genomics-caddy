<!-- ./src/lib/components/ai/ChatInput.svelte -->
<script lang="ts">
  import type { ChatMessage } from "../../types/agent";

  interface Props {
    promptText: string;
    attachedImages: { name: string; base64: string; previewUrl: string }[];
    isChatting: boolean;
    isVisionCapable: boolean;
    imageInput: HTMLInputElement | null;
    messages: ChatMessage[];
    sendPrompt: (customPrompt?: string) => void;
    stopGeneration: () => void;
    clearHistory: () => void;
    handlePaste: (e: ClipboardEvent) => void;
    handleFileChange: (e: Event) => void;
    removeAttachedImage: (index: number) => void;
  }

  let {
    promptText = $bindable(),
    attachedImages = $bindable(),
    isChatting,
    isVisionCapable,
    imageInput = $bindable(),
    messages,
    sendPrompt,
    stopGeneration,
    clearHistory,
    handlePaste,
    handleFileChange,
    removeAttachedImage,
  }: Props = $props();
</script>

<div class="chat-input-area">
  <form onsubmit={(e) => { e.preventDefault(); sendPrompt(); }} class="chat-form">
    {#if attachedImages.length > 0}
      <div class="attached-images-preview-bar">
        {#each attachedImages as img, idx}
          <div class="attached-thumbnail-container">
            <img class="attached-thumbnail" src={img.previewUrl} alt="Thumbnail preview" />
            <button type="button" class="btn-remove-attachment" onclick={() => removeAttachedImage(idx)} aria-label="Remove image">&times;</button>
          </div>
        {/each}
      </div>
    {/if}

    <textarea
      bind:value={promptText}
      placeholder={isVisionCapable
        ? "Ask a question (paste or drop images here to analyze them)..."
        : "Ask about a result, a symptom pattern, a supplement or a test (Enter to send, Shift+Enter for a new line)"}
      disabled={isChatting}
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          sendPrompt();
        }
      }}
      onpaste={handlePaste}
    ></textarea>

    <div class="chat-controls">
      {#if isVisionCapable}
        <input
          type="file"
          accept="image/*"
          multiple
          style="display: none"
          bind:this={imageInput}
          onchange={handleFileChange}
        />
        <button
          type="button"
          class="btn btn-secondary btn-icon"
          onclick={() => imageInput?.click()}
          disabled={isChatting}
        >
          📷 Attach Image
        </button>
      {/if}

      {#if isChatting}
        <button type="button" class="btn btn-danger" onclick={stopGeneration}>
          ⏹ Stop Generation
        </button>
      {:else}
        <button type="button" class="btn btn-secondary" onclick={clearHistory} disabled={messages.length === 0}>
          🗑 Clear Chat
        </button>
        <button type="submit" class="btn chat-send-btn" disabled={!promptText.trim() && attachedImages.length === 0}>
          Ask ➤
        </button>
      {/if}
    </div>
  </form>
</div>
