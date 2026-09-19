<!-- ./src/lib/components/common/GlobalDialogs.svelte -->
<script lang="ts">
  import { dialogStore } from '../../utils/dialogState.svelte';
  import '$lib/styles/components/global-dialogs.css';

  /*
  Module Docstring:
  Purpose: Global dialog backdrop rendering alerts, confirms, and choices.
  Responsibilities:
  - Read from the reactive global dialogStore.
  - Render a glassmorphic modal containing title, message, and action buttons.
  - Support OK / Cancel / multi-choice callbacks.
  Key Inputs: dialogStore.state.
  Key Outputs: Custom UI overlay.
  Operational Notes: Escape, X, and backdrop abort a choice without picking Move or Use empty.
  */

  function dismissDialog(cancelled = false) {
    if (dialogStore.state.busy) return;
    dialogStore.close({ cancelled });
  }

  function handleDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      dismissDialog(dialogStore.state.type !== 'alert');
    }
    event.stopPropagation();
  }

  function choiceClass(variant: string | undefined): string {
    if (variant === 'danger') return 'btn btn-danger';
    if (variant === 'secondary') return 'btn btn-secondary';
    return 'btn btn-accent';
  }
</script>

{#if dialogStore.state.show}
  <div class="modal-backdrop dialog-backdrop" onclick={() => dismissDialog(dialogStore.state.type !== 'alert')} role="presentation">
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
          onclick={() => dismissDialog(dialogStore.state.type !== 'alert')}
        >&times;</button>
      </div>
      <div class="modal-body dialog-body">
        <p id="global-dialog-message">{dialogStore.state.message}</p>
      </div>
      <div class="modal-footer dialog-footer" class:dialog-choice-footer={dialogStore.state.type === 'choice'}>
        {#if dialogStore.state.type === "confirm"}
          <button type="button" class="btn btn-secondary" disabled={dialogStore.state.busy} onclick={() => dismissDialog(true)}>Cancel</button>
          <button type="button" class="btn btn-accent" disabled={dialogStore.state.busy} aria-busy={dialogStore.state.busy ? 'true' : undefined} onclick={() => dialogStore.handleConfirm()}>
            {dialogStore.state.busy ? 'Working…' : 'Confirm'}
          </button>
        {:else if dialogStore.state.type === "choice"}
          {#each dialogStore.state.choices ?? [] as choice (choice.id)}
            <button
              type="button"
              class={choiceClass(choice.variant)}
              disabled={dialogStore.state.busy}
              aria-busy={dialogStore.state.busy ? 'true' : undefined}
              onclick={() => dialogStore.handleChoice(choice.id)}
            >
              {dialogStore.state.busy ? 'Working…' : choice.label}
            </button>
          {/each}
        {:else}
          <button type="button" class="btn btn-accent" onclick={() => dismissDialog(false)}>OK</button>
        {/if}
      </div>
    </div>
  </div>
{/if}
