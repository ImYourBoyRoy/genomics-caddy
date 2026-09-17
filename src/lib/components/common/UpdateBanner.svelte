<!-- ./src/lib/components/common/UpdateBanner.svelte -->
<script lang="ts">
  import { formatUpdateBannerCopy } from '../../utils/updater';
  import '$lib/styles/components/update-banner.css';

  /*
  Purpose: Presentational signed-update strip.
  Responsibilities: Show the available version, install progress, and dismiss.
  Key Inputs: version label, installing/progress, install and dismiss callbacks.
  Key Outputs: Accessible status banner; no updater I/O of its own.
  */

  interface Props {
    version: string;
    installing?: boolean;
    progress?: number;
    onInstall: () => void;
    onDismiss: () => void;
  }

  let { version, installing = false, progress = 0, onInstall, onDismiss }: Props = $props();

  let statusText = $derived(formatUpdateBannerCopy({ version, installing, progress }));
</script>

<div class="app-update-banner no-print" role="status" aria-live="polite">
  <p class="app-update-banner-copy">{statusText}</p>
  <div class="app-update-banner-actions">
    <button type="button" class="btn btn-primary btn-sm" onclick={onInstall} disabled={installing}>
      {installing ? 'Installing…' : 'Install…'}
    </button>
    <button type="button" class="btn btn-secondary btn-sm" onclick={onDismiss} disabled={installing}>
      Not now
    </button>
  </div>
</div>
