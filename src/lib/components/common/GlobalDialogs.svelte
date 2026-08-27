<!-- ./src/lib/components/common/GlobalDialogs.svelte -->
<script lang="ts">
  import { dialogStore } from '../../utils/dialogState.svelte';

  /*
  Module Docstring:
  Purpose: Global dialog backdrop rendering alerts and confirms.
  Responsibilities:
  - Read from the reactive global dialogStore.
  - Render a glassmorphic modal containing title, message, and action buttons.
  - Support OK/Cancel action callbacks.
  Key Inputs: dialogStore.state.
  Key Outputs: Custom UI overlay.
  */

  function handleDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      dialogStore.close();
    }
    event.stopPropagation();
  }
</script>

{#if dialogStore.state.show}
  <div class="modal-backdrop dialog-backdrop" onclick={() => dialogStore.close()} role="presentation">
    <div
      class="modal-content dialog-content"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleDialogKeydown}
      role="alertdialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="global-dialog-title"
      aria-describedby="global-dialog-message"
    >
      <div class="modal-header">
        <h3 id="global-dialog-title">{dialogStore.state.title}</h3>
        <button
          type="button"
          class="modal-close"
          aria-label="Close dialog"
          onclick={() => dialogStore.close()}
        >&times;</button>
      </div>
      <div class="modal-body dialog-body">
        <p id="global-dialog-message">{dialogStore.state.message}</p>
      </div>
      <div class="modal-footer dialog-footer">
        {#if dialogStore.state.type === "confirm"}
          <button type="button" class="btn btn-secondary" onclick={() => dialogStore.close()}>Cancel</button>
          <button type="button" class="btn btn-accent" onclick={() => dialogStore.handleConfirm()}>Confirm</button>
        {:else}
          <button type="button" class="btn btn-accent" onclick={() => dialogStore.close()}>OK</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: var(--modal-backdrop-bg);
    backdrop-filter: blur(8px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1100; /* overlay above everything */
    animation: modalFadeIn 0.2s ease-out;
  }

  .modal-content {
    background: var(--surface-raised);
    border: 1px solid var(--border-color);
    box-shadow: 0 20px 40px var(--shadow-modal);
    border-radius: 12px;
    width: 90%;
    max-width: 440px;
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
    background: var(--surface-subtle);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .modal-close {
    min-width: 44px;
    min-height: 44px;
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

  .modal-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    background: var(--surface-subtle);
  }

  .dialog-backdrop {
    background: var(--modal-backdrop-bg);
    backdrop-filter: blur(12px);
    z-index: 1110;
  }

  .dialog-content {
    max-width: 440px;
    background: var(--surface-raised);
    border: 1px solid var(--border-color);
    box-shadow: 0 25px 50px var(--shadow-modal);
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
    background: var(--surface-subtle);
  }
</style>
