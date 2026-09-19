<!-- ./src/lib/components/sidebar/LibraryPanel.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { exit, relaunch } from '@tauri-apps/plugin-process';
  import {
    eraseLibraryData,
    getLibraryStatus,
    openLibraryDir,
    resetLibraryDir,
    selectDirectory,
    setLibraryDir,
  } from '../../api/tauri';
  import type { LibraryStatus } from '../../types/genomics';
  import { dialogStore } from '../../utils/dialogState.svelte';

  /*
  Purpose: Compact sidebar control for the personal library folder.
  Responsibilities: Show where genomes live, open the folder, change it,
  reset to the launch default, or erase known library files.
  */

  let status = $state<LibraryStatus | null>(null);
  let busy = $state(false);
  let error = $state('');
  let runtime = $state(false);

  async function refresh() {
    if (!isTauri()) return;
    status = await getLibraryStatus();
  }

  onMount(() => {
    runtime = isTauri();
    if (!runtime) return;
    void refresh().catch((cause) => {
      error = cause instanceof Error ? cause.message : String(cause);
    });
  });

  async function restartApp(next: () => Promise<unknown>, goodbye: 'relaunch' | 'exit') {
    busy = true;
    error = '';
    try {
      await next();
      if (goodbye === 'relaunch') {
        try {
          await relaunch();
        } catch {
          dialogStore.alert(
            'The library folder is updated. Restart Genomics Caddy to use it.',
            'Restart required',
          );
        }
      } else {
        try {
          await exit(0);
        } catch {
          dialogStore.alert('Local library files were removed. You can close the app.', 'Library erased');
        }
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }

  function handleOpen() {
    if (!runtime || busy) return;
    void openLibraryDir().catch((cause) => {
      error = cause instanceof Error ? cause.message : String(cause);
    });
  }

  async function handleChange() {
    if (!runtime || busy) return;
    const dest = await selectDirectory();
    if (!dest) return;
    dialogStore.choice(
      `Use this folder for genomes and downloads?\n${dest}\n\nMove copies this library, then removes it from the old folder. Use empty folder leaves the old files where they are. Cancel keeps the current folder.`,
      [
        { id: 'move', label: 'Move library', variant: 'accent' },
        { id: 'use', label: 'Use empty folder', variant: 'secondary' },
      ],
      (id) => {
        if (id !== 'move' && id !== 'use') return;
        void restartApp(() => setLibraryDir(dest, id), 'relaunch');
      },
      'Change library folder',
    );
  }

  function handleReset() {
    if (!runtime || busy || !status?.is_custom) return;
    dialogStore.confirm(
      `Return to the default library folder?\n${status.default_data_dir}`,
      () => void restartApp(() => resetLibraryDir(), 'relaunch'),
      'Reset library folder',
    );
  }

  function handleErase() {
    if (!runtime || busy) return;
    dialogStore.confirm(
      'Erase genomes, downloads, and local caches for this library? The app will quit. This does not uninstall the program.',
      () => void restartApp(() => eraseLibraryData(), 'exit'),
      'Erase local data',
    );
  }
</script>

<section class="library-panel" aria-label="Library folder">
  <div class="library-panel-copy">
    <strong>Library</strong>
    {#if status}
      <span class="library-panel-path" title={status.data_dir}>{status.truncated_path}</span>
    {:else if !runtime}
      <span class="library-panel-path">Desktop app only</span>
    {:else}
      <span class="library-panel-path">Locating folder…</span>
    {/if}
  </div>
  <div class="library-panel-actions">
    <button type="button" class="btn btn-secondary btn-sm" disabled={!runtime || busy} onclick={handleOpen}>
      Open
    </button>
    <button type="button" class="btn btn-secondary btn-sm" disabled={!runtime || busy} onclick={handleChange}>
      Change
    </button>
  </div>
  {#if status}
    <div class="library-panel-meta">
      {#if !status.writable}
        <span class="library-panel-warn">This folder is not writable. Move the app to a writable disk or choose another folder.</span>
      {:else if status.mode === 'per_user'}
        <span>This account has its own library.</span>
      {:else if status.is_custom}
        <span>Custom folder for this launch.</span>
      {:else}
        <span>Genomes stay next to this app.</span>
      {/if}
      {#if status.is_custom}
        <button type="button" class="library-text-btn" disabled={busy} onclick={handleReset}>Reset</button>
      {/if}
      <button type="button" class="library-text-btn" disabled={busy} onclick={handleErase}>Erase</button>
    </div>
  {/if}
  {#if error}
    <p class="library-panel-error">{error}</p>
  {/if}
</section>
