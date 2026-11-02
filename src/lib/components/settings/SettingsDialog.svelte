<!-- ./src/lib/components/settings/SettingsDialog.svelte -->
<script lang="ts">
  /*
  Purpose: One home for app-level preferences that used to crowd the sidebar:
  appearance, genome assembly (liftover), reference data and updates, connections,
  and the library folder.
  Inputs: open (bindable), onOpenConnections, and `liftover` / `referenceData`
  snippets rendered by the sidebar because it owns the catalog-sync state.
  Outputs: A native modal <dialog> (focus trap, Esc, inert backdrop for free).
  Styling lives in styles/components/settings-dialog.css (static sheet for WebKitGTK).
  */
  import type { Snippet } from 'svelte';
  import ThemeToggle from '../common/ThemeToggle.svelte';
  import LibraryPanel from '../sidebar/LibraryPanel.svelte';
  import '$lib/styles/components/settings-dialog.css';
  import '$lib/styles/components/library-panel.css';

  interface Props {
    open: boolean;
    runtimeAvailable?: boolean;
    version?: string;
    liftover?: Snippet;
    referenceData?: Snippet;
    onOpenConnections?: () => void;
  }

  let { open = $bindable(false), runtimeAvailable = true, version = '', liftover, referenceData, onOpenConnections }: Props = $props();

  let dialogEl = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    const el = dialogEl;
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  });

  function close() {
    open = false;
  }

  function openConnections() {
    close();
    onOpenConnections?.();
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === dialogEl) close();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="settings-dialog"
  aria-labelledby="settings-dialog-title"
  onclose={() => (open = false)}
  onclick={onBackdropClick}
>
  {#if open}
    <div class="settings-dialog-surface">
      <header class="settings-dialog-header">
        <h2 id="settings-dialog-title">Settings</h2>
        <button type="button" class="settings-dialog-close" aria-label="Close settings" onclick={close}>✕</button>
      </header>

      <div class="settings-dialog-body">
        <section class="settings-section" aria-labelledby="settings-appearance">
          <h3 id="settings-appearance">Appearance</h3>
          <ThemeToggle />
        </section>

        {#if liftover}
          <section class="settings-section" aria-labelledby="settings-assembly">
            <h3 id="settings-assembly">Genome assembly</h3>
            {@render liftover()}
          </section>
        {/if}

        {#if referenceData}
          <section class="settings-section" aria-labelledby="settings-data">
            <h3 id="settings-data">Data &amp; updates</h3>
            {@render referenceData()}
          </section>
        {/if}

        {#if onOpenConnections}
          <section class="settings-section" aria-labelledby="settings-connections">
            <h3 id="settings-connections">Connections</h3>
            <div class="settings-row">
              <p>Ollama and vector database endpoints, health, and model settings.</p>
              <button type="button" class="btn btn-secondary btn-sm" onclick={openConnections} disabled={!runtimeAvailable}>
                Open Connections
              </button>
            </div>
          </section>
        {/if}

        <section class="settings-section" aria-labelledby="settings-library">
          <h3 id="settings-library">Library folder</h3>
          <LibraryPanel />
        </section>
      </div>

      {#if version}
        <footer class="settings-dialog-footer">Genomics Caddy {version}</footer>
      {/if}
    </div>
  {/if}
</dialog>
