<!-- ./src/lib/components/ai/ChatInput.svelte -->
<script lang="ts">
  interface Props {
    promptText: string;
    attachedImages: { name: string; base64: string; previewUrl: string }[];
    isChatting: boolean;
    isVisionCapable: boolean;
    imageInput: HTMLInputElement | null;
    messages: any[];
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
        : "Ask a question about your genomic findings (e.g. 'Explain what my AOC1 gene copy variant means')...."}
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
          title="Attach Image (multimodal analysis)"
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
        <button type="submit" class="btn btn-accent" disabled={!promptText.trim() && attachedImages.length === 0}>
          📤 Ask Assistant
        </button>
      {/if}
    </div>
  </form>
</div>

<style>
  .chat-input-area {
    padding: 16px 20px;
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-color);
  }

  .chat-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .chat-form textarea {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    border-radius: 8px;
    padding: 12px 14px;
    font-size: 0.88rem;
    resize: none;
    height: 60px;
    font-family: inherit;
    line-height: 1.4;
  }

  .chat-form textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .chat-form textarea:disabled {
    opacity: 0.6;
  }

  .chat-controls {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  /* Image preview bar inside form */
  .attached-images-preview-bar {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    background: rgba(0, 0, 0, 0.15);
    padding: 8px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    margin-bottom: 6px;
  }
  .attached-thumbnail-container {
    position: relative;
    width: 60px;
    height: 60px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    overflow: hidden;
    background: #000;
  }
  .attached-thumbnail {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .btn-remove-attachment {
    position: absolute;
    top: 2px;
    right: 2px;
    background: rgba(239, 68, 68, 0.85);
    color: white;
    border: none;
    border-radius: 50%;
    width: 16px;
    height: 16px;
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-weight: bold;
    padding: 0;
    line-height: 1;
  }
  .btn-remove-attachment:hover {
    background: #ef4444;
  }

  .btn-icon {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
