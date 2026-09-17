<!-- ./src/lib/components/common/AppUpdateHost.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import type { Update as TauriUpdate } from '@tauri-apps/plugin-updater';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import {
    checkForAppUpdate,
    formatVersionLabel,
    installAppUpdate,
    setDismissedUpdateVersion,
    shouldShowUpdateBanner,
    type UpdateUiState,
  } from '../../utils/updater';
  import UpdateBanner from './UpdateBanner.svelte';

  /*
  Purpose: Quiet desktop update check and confirm-to-install flow.
  Responsibilities: Probe GitHub on launch, show a dismissible banner, confirm
  before downloading a signed installer, then relaunch.
  Key Inputs: Tauri updater/process plugins; dialogStore for confirm/alert.
  Key Outputs: Banner visibility and install progress. No genotype I/O.
  */

  let updateCheckState = $state<UpdateUiState>('idle');
  let pendingUpdate = $state.raw<TauriUpdate | undefined>(undefined);
  let updateProgress = $state(0);
  let showBanner = $state(false);

  onMount(() => {
    void checkQuietly();
  });

  async function checkQuietly() {
    if (!isTauri()) return;
    if (updateCheckState === 'checking' || updateCheckState === 'installing') return;
    updateCheckState = 'checking';
    updateProgress = 0;
    const result = await checkForAppUpdate({ quiet: true, previous: pendingUpdate });
    pendingUpdate = result.update;
    updateCheckState = result.state;
    showBanner =
      result.state === 'available' && result.update
        ? shouldShowUpdateBanner(result.update.version)
        : false;
  }

  function dismiss() {
    if (pendingUpdate) setDismissedUpdateVersion(pendingUpdate.version);
    showBanner = false;
  }

  function promptInstall() {
    const update = pendingUpdate;
    if (!update || updateCheckState === 'installing') return;
    const version = formatVersionLabel(update.version);
    dialogStore.confirm(
      `Download and install Genomics Caddy ${version} from GitHub? The app will restart after the signed installer finishes. This only fetches the update — it does not upload your DNA or reports.`,
      () => runInstall(update),
      'Install update',
    );
  }

  async function runInstall(update: TauriUpdate) {
    updateCheckState = 'installing';
    updateProgress = 0;
    try {
      const installResult = await installAppUpdate(update, {
        confirmed: true,
        onProgress: (percent) => {
          updateProgress = percent;
        },
      });
      pendingUpdate = undefined;
      showBanner = false;
      updateCheckState = 'current';
      if (!installResult.relaunched) {
        dialogStore.alert(
          'The signed update is installed. Restart Genomics Caddy to start it.',
          'Update ready',
        );
      }
    } catch (error) {
      updateCheckState = 'error';
      pendingUpdate = undefined;
      showBanner = false;
      const detail = error instanceof Error ? error.message : String(error);
      dialogStore.alert(
        detail ? `Could not install the signed update. ${detail}` : 'Could not install the signed update.',
        'Update failed',
      );
    }
  }
</script>

{#if showBanner && pendingUpdate && (updateCheckState === 'available' || updateCheckState === 'installing')}
  <UpdateBanner
    version={formatVersionLabel(pendingUpdate.version)}
    installing={updateCheckState === 'installing'}
    progress={updateProgress}
    onInstall={promptInstall}
    onDismiss={dismiss}
  />
{/if}
