<!-- ./src/lib/components/ai/settings/SystemPromptSection.svelte -->
<script lang="ts">
  interface Props {
    systemInstructions: string;
    defaultInstructions: string;
  }

  let {
    systemInstructions = $bindable(),
    defaultInstructions
  }: Props = $props();

  function resetInstructions() {
    systemInstructions = defaultInstructions;
  }
</script>

<details class="settings-details-group">
  <summary class="settings-details-summary">✍️ Custom System Prompt</summary>
  <div class="settings-details-content">
    <label for="system-instructions-editor">Custom Instructions Template</label>
    <textarea 
      id="system-instructions-editor" 
      bind:value={systemInstructions} 
      placeholder="Edit system prompt rules here..." 
      class="instructions-textarea"
    ></textarea>
    <button class="btn btn-secondary w-full mt-1" onclick={resetInstructions}>
      🔄 Restore Default Prompt
    </button>
  </div>
</details>

<style>
  .instructions-textarea {
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border-color);
    color: #a5b4fc;
    padding: 8px;
    border-radius: 6px;
    font-size: 0.76rem;
    min-height: 120px;
    resize: vertical;
    font-family: monospace;
    line-height: 1.4;
  }

  .instructions-textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .w-full { width: 100%; }
  .mt-1 { margin-top: 4px; }
</style>
