<!-- ./src/lib/components/ai/ChatModals.svelte -->
<script lang="ts">
  import { dialogStore } from '../../utils/dialogState.svelte';

  interface Props {
    showPromptInspector: boolean;
    currentSystemPrompt: string;
    showExportModal: boolean;
    exportConversation: (type: "standard" | "clinical") => void;
  }

  let {
    showPromptInspector = $bindable(false),
    currentSystemPrompt,
    showExportModal = $bindable(false),
    exportConversation
  }: Props = $props();

  function triggerExport(type: "standard" | "clinical") {
    showExportModal = false;
    exportConversation(type);
  }

  function triggerAlert(msg: string) {
    navigator.clipboard.writeText(currentSystemPrompt);
    dialogStore.alert(msg);
  }
</script>

<!-- 1. System Prompt Inspector Modal -->
{#if showPromptInspector}
  <div class="modal-backdrop" onclick={() => showPromptInspector = false} role="presentation">
    <div class="modal-content" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="modal-header">
        <h3>System Prompt &amp; Context Inspector</h3>
        <button class="modal-close" onclick={() => showPromptInspector = false}>&times;</button>
      </div>
      <div class="modal-body">
        <p class="modal-description">
          Below is the exact system prompt and genomic context payload (including the raw JSON findings) that is sent to the LLM. It updates dynamically as you change your selected packs and active findings filters.
        </p>
        <pre class="code-block-inspect"><code>{currentSystemPrompt}</code></pre>
      </div>
      <div class="modal-footer">
        <button 
          class="btn btn-secondary" 
          onclick={() => triggerAlert("System prompt copied to clipboard!")}
        >
          📋 Copy System Prompt
        </button>
        <button class="btn btn-accent" onclick={() => showPromptInspector = false}>
          Close
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- 2. Export Selector Modal -->
{#if showExportModal}
  <div class="modal-backdrop dialog-backdrop" onclick={() => showExportModal = false} role="presentation">
    <div class="modal-content dialog-content" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="modal-header">
        <h3>Select Export Format</h3>
        <button class="modal-close" onclick={() => showExportModal = false}>&times;</button>
      </div>
      <div class="modal-body dialog-body" style="text-align: left; display: flex; flex-direction: column; gap: 12px;">
        <p style="margin: 0; font-size: 0.88rem;">Select how you want to export this AI Consultation:</p>
        
        <button 
          type="button" 
          class="btn-export-option"
          onclick={() => triggerExport("standard")}
        >
          <div class="export-option-icon">📄</div>
          <div class="export-option-details">
            <strong>Standard Conversation Log</strong>
            <span>Markdown file containing only the chat questions and AI responses.</span>
          </div>
        </button>
        
        <button 
          type="button" 
          class="btn-export-option"
          onclick={() => triggerExport("clinical")}
        >
          <div class="export-option-icon">🏥</div>
          <div class="export-option-details">
            <strong>Clinical Handoff &amp; Frontier LLM Bundle</strong>
            <span>Includes user health profile (goals, diet, medications), clinical summary of active variants, copy-pasteable JSON prompt context for external models (Claude/GPT-4o), and full chat transcript.</span>
          </div>
        </button>
      </div>
      <div class="modal-footer dialog-footer">
        <button class="btn btn-secondary" onclick={() => showExportModal = false}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

<!-- 3. Custom Glassmorphic Dialogue Modal (Replaces browser alert/confirm popups) -->
{#if dialogStore.state.show}
  <div class="modal-backdrop dialog-backdrop" onclick={() => dialogStore.close()} role="presentation">
    <div class="modal-content dialog-content" onclick={(e) => e.stopPropagation()} role="presentation">
      <div class="modal-header">
        <h3>{dialogStore.state.title}</h3>
        <button class="modal-close" onclick={() => dialogStore.close()}>&times;</button>
      </div>
      <div class="modal-body dialog-body">
        <p>{dialogStore.state.message}</p>
      </div>
      <div class="modal-footer dialog-footer">
        {#if dialogStore.state.type === "confirm"}
          <button class="btn btn-secondary" onclick={() => dialogStore.close()}>Cancel</button>
          <button class="btn btn-accent" onclick={() => dialogStore.handleConfirm()}>Confirm</button>
        {:else}
          <button class="btn btn-accent" onclick={() => dialogStore.close()}>OK</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  /* Modal backdrop & content for inspecting payload / dialogs */
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
    animation: modalFadeIn 0.2s ease-out;
  }

  .modal-content {
    background: rgba(20, 22, 37, 0.95);
    border: 1px solid var(--border-color);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    border-radius: 12px;
    width: 90%;
    max-width: 800px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: modalSlideUp 0.25s ease-out;
  }

  @keyframes modalFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes modalSlideUp {
    from { opacity: 0; transform: translateY(20px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.2);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .modal-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 1.5rem;
    cursor: pointer;
    line-height: 1;
    padding: 0;
    transition: color 0.2s;
  }

  .modal-close:hover {
    color: var(--danger);
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .modal-description {
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.4;
    margin: 0;
  }

  .code-block-inspect {
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid var(--border-color);
    padding: 14px;
    border-radius: 6px;
    overflow-y: auto;
    font-family: monospace;
    font-size: 0.8rem;
    color: #a5b4fc;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-all;
    flex: 1;
    max-height: 50vh;
  }

  .modal-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    background: rgba(0, 0, 0, 0.2);
  }

  /* Dialogue custom popup styles */
  .dialog-backdrop {
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(12px);
    z-index: 1010;
  }

  .dialog-content {
    max-width: 440px;
    background: rgba(15, 17, 28, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 25px 50px rgba(0, 0, 0, 0.7);
  }

  .dialog-body {
    padding: 24px 20px;
    text-align: center;
    font-size: 0.95rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .dialog-footer {
    justify-content: center;
    gap: 12px;
    background: rgba(0, 0, 0, 0.1);
  }

  /* Export Button Options */
  .btn-export-option {
    display: flex;
    gap: 14px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    padding: 12px 14px;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s;
    color: var(--text-primary);
  }

  .btn-export-option:hover {
    background: rgba(88, 80, 236, 0.08);
    border-color: rgba(88, 80, 236, 0.35);
  }

  .export-option-icon {
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .export-option-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .export-option-details strong {
    font-size: 0.85rem;
    color: #ffffff;
  }

  .export-option-details span {
    font-size: 0.74rem;
    color: var(--text-secondary);
    line-height: 1.3;
  }
</style>
