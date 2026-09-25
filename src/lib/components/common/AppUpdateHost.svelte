<!-- ./src/lib/components/common/AppUpdateHost.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import type { Update as TauriUpdate } from '@tauri-apps/plugin-updater';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import { appUpdateState, type AppUpdateState } from '../../utils/appUpdateState';
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

  function publishSharedState(patch: Partial<AppUpdateState>) {
    appUpdateState.update((current) => ({ ...current, ...patch }));
  }

  onMount(() => {
    publishSharedState({
      isDesktop: isTauri(),
      checkNow: undefined,
      installAvailable: undefined,
    });
    appUpdateState.update((current) => ({
      ...current,
      checkNow: () => { void checkManually(); },
      installAvailable: () => promptInstall(),
    }));
    void bootstrap();
    return () => {
      void closeUpdateResource(liveUpdate);
      appUpdateState.update((current) => ({
        ...current,
        checkNow: null,
        installAvailable: null,
      }));
    };
  });

  async function bootstrap() {
    if (!isTauri()) return;
    try {
      publishSharedState({ currentVersion: formatVersionLabel(await getVersion()) });
    } catch {
      publishSharedState({ currentVersion: 'Version unavailable' });
    }
    installKind = await resolveUpdateInstallKind();
    await checkQuietly();
  }

  async function checkQuietly() {
    await runUpdateCheck(true);
  }

  async function checkManually() {
    await runUpdateCheck(false);
  }

  async function runUpdateCheck(quiet: boolean) {
    if (!isTauri()) return;
    if (updateCheckState === 'checking' || updateCheckState === 'installing') return;
    updateCheckState = 'checking';
    updateProgress = 0;
    publishSharedState({ status: 'checking', message: '' });
    const result = await checkForAppUpdate({ quiet, previous: pendingUpdate });
    setPending(result.update);
    updateCheckState = result.state;
    const message = result.state === 'current'
      ? 'You are using the latest signed app release.'
      : result.state === 'available' && quiet
        ? 'A signed app update is available to review.'
        : result.message;
    publishSharedState({
      status: result.state,
      availableVersion: result.update?.version ?? null,
      message,
      checkedAt: Date.now(),
    });
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
    publishSharedState({
      status: 'idle',
      availableVersion: null,
      message: 'The update was deferred. You can check again whenever you are ready.',
    });
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
    publishSharedState({ status: 'installing', message: 'Preparing the signed update…' });
    try {
      const installResult = await installAppUpdate(update, {
        confirmed: true,
        kind: installKind,
        onProgress: (percent) => {
          updateProgress = percent;
          publishSharedState({ status: 'installing', message: `Update download ${percent}% complete.` });
        },
      });
      setPending(undefined);
      showBanner = false;
      updateCheckState = 'current';
      publishSharedState({
        status: 'current',
        availableVersion: null,
        message: installResult.relaunched
          ? 'The signed update was applied and the app is restarting.'
          : 'The signed update is ready. Restart Genomics Caddy to use it.',
      });
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
      publishSharedState({
        status: 'error',
        availableVersion: null,
        message: detail ? `Could not apply the signed update. ${detail}` : 'Could not apply the signed update.',
      });
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
