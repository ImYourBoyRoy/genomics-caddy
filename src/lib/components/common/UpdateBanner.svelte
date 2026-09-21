<!-- ./src/lib/components/common/UpdateBanner.svelte -->
<script lang="ts">
  import { formatUpdateBannerCopy, type UpdateInstallKind } from '../../utils/updater';
  import '$lib/styles/components/update-banner.css';

  /*
  Purpose: Presentational signed-update strip.
  Responsibilities: Show the available version, install progress, and dismiss.
  Key Inputs: version label, installing/progress, install kind, callbacks.
  Key Outputs: Accessible status banner; no updater I/O of its own.
  */

  interface Props {
    version: string;
    installing?: boolean;
    progress?: number;
    kind?: UpdateInstallKind;
    onInstall: () => void;
    onDismiss: () => void;
  }

  let {
    version,
    installing = false,
    progress = 0,
    kind = 'installer',
    onInstall,
    onDismiss,
  }: Props = $props();

  let statusText = $derived(formatUpdateBannerCopy({ version, installing, progress, kind }));
  let actionLabel = $derived(
    installing ? (kind === 'portable' ? 'Replacing…' : 'Installing…') : kind === 'portable' ? 'Replace…' : 'Install…',
  );
</script>

<div class="app-update-banner no-print" role="status" aria-live="polite">
  <p class="app-update-banner-copy">{statusText}</p>
  <div class="app-update-banner-actions">
    <button type="button" class="btn btn-primary btn-sm" onclick={onInstall} disabled={installing}>
      {actionLabel}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={onDismiss} disabled={installing}>
      Not now
    </button>
  </div>
</div>
