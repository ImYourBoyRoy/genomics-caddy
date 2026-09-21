<!-- ./src/lib/components/common/AppUpdateHost.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import type { Update as TauriUpdate } from '@tauri-apps/plugin-updater';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import {
    checkForAppUpdate,
    closeUpdateResource,
    formatVersionLabel,
    installAppUpdate,
    resolveUpdateInstallKind,
    setDismissedUpdateVersion,
    shouldShowUpdateBanner,
    type UpdateInstallKind,
    type UpdateUiState,
  } from '../../utils/updater';
  import UpdateBanner from './UpdateBanner.svelte';

  /*
  Purpose: Quiet desktop update check and confirm-to-apply flow.
  Responsibilities: Probe GitHub on launch, show a dismissible banner, confirm
  before downloading a signed artifact, then replace this copy or run NSIS.
  Key Inputs: Tauri updater/process plugins; dialogStore for confirm/alert.
  Key Outputs: Banner visibility and install progress. No genotype I/O.
  */

  let updateCheckState = $state<UpdateUiState>('idle');
  let pendingUpdate = $state.raw<TauriUpdate | undefined>(undefined);
  let liveUpdate: TauriUpdate | undefined;
  let updateProgress = $state(0);
  let showBanner = $state(false);
  let installKind = $state<UpdateInstallKind>('installer');

  function setPending(update: TauriUpdate | undefined) {
    liveUpdate = update;
    pendingUpdate = update;
  }

  onMount(() => {
    void bootstrap();
    return () => {
      void closeUpdateResource(liveUpdate);
    };
  });

  async function bootstrap() {
    if (!isTauri()) return;
    installKind = await resolveUpdateInstallKind();
    await checkQuietly();
  }

  async function checkQuietly() {
    if (!isTauri()) return;
    if (updateCheckState === 'checking' || updateCheckState === 'installing') return;
    updateCheckState = 'checking';
    updateProgress = 0;
    const result = await checkForAppUpdate({ quiet: true, previous: pendingUpdate });
    setPending(result.update);
    updateCheckState = result.state;
    showBanner =
      result.state === 'available' && result.update
        ? shouldShowUpdateBanner(result.update.version)
        : false;
  }

  function dismiss() {
    if (pendingUpdate) setDismissedUpdateVersion(pendingUpdate.version);
    void closeUpdateResource(liveUpdate);
    setPending(undefined);
    showBanner = false;
  }

  function confirmCopy(version: string): string {
    if (installKind === 'portable') {
      return `Download Genomics Caddy ${version} and replace this portable copy? Your Data folder stays. The app restarts after the signed zip is verified. This only fetches the update — it does not upload your DNA or reports.`;
    }
    return `Download and install Genomics Caddy ${version} from GitHub? The app will restart after the signed installer finishes. This only fetches the update — it does not upload your DNA or reports.`;
  }

  function promptInstall() {
    const update = pendingUpdate;
    if (!update || updateCheckState === 'installing') return;
    const version = formatVersionLabel(update.version);
    dialogStore.confirm(
      confirmCopy(version),
      () => runInstall(update),
      installKind === 'portable' ? 'Replace this copy' : 'Install update',
    );
  }

  async function runInstall(update: TauriUpdate) {
    updateCheckState = 'installing';
    updateProgress = 0;
    try {
      const installResult = await installAppUpdate(update, {
        confirmed: true,
        kind: installKind,
        onProgress: (percent) => {
          updateProgress = percent;
        },
      });
      setPending(undefined);
      showBanner = false;
      updateCheckState = 'current';
      if (!installResult.relaunched) {
        dialogStore.alert(
          installKind === 'portable'
            ? 'The signed portable copy is ready. Start Genomics Caddy again from this folder.'
            : 'The signed update is installed. Restart Genomics Caddy to start it.',
          'Update ready',
        );
      }
    } catch (error) {
      updateCheckState = 'error';
      setPending(undefined);
      showBanner = false;
      const detail = error instanceof Error ? error.message : String(error);
      dialogStore.alert(
        detail ? `Could not apply the signed update. ${detail}` : 'Could not apply the signed update.',
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
    kind={installKind}
    onInstall={promptInstall}
    onDismiss={dismiss}
  />
{/if}
