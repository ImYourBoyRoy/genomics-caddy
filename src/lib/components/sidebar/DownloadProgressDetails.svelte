<script lang="ts">
  import type { DownloadProgressSample } from '../../utils/offlineDownloadProgress';
  import { formatTransferEta, formatTransferSize } from '../../utils/offlineDownloadProgress';
  import ProgressTrack from './ProgressTrack.svelte';

  interface Props {
    progress: DownloadProgressSample;
    label: string;
    compact?: boolean;
  }

  let { progress, label, compact = false }: Props = $props();

  const transferRate = $derived(
    progress.speedBytesPerSecond == null
      ? 'Measuring transfer speed…'
      : `${(progress.speedBytesPerSecond / 1_000_000).toFixed(1)} MB/s`,
  );

  const remaining = $derived(
    progress.totalBytes > 0 && progress.bytesDone >= progress.totalBytes
      ? 'Transfer complete · verifying file'
      : progress.etaSeconds == null
      ? progress.totalBytes > 0 ? 'Estimating time…' : 'Total size unavailable · ETA unavailable'
      : formatTransferEta(progress.etaSeconds),
  );
</script>

<div class="download-progress-details" class:compact role="group" aria-label={label}>
  <ProgressTrack percent={progress.percent} label={`${label} progress`} spaced={compact} />
  <div class="download-progress-line">
    <span>
      {#if progress.totalBytes > 0}
        {formatTransferSize(progress.bytesDone)} of {formatTransferSize(progress.totalBytes)}
      {:else}
        {formatTransferSize(progress.bytesDone)} received
      {/if}
    </span>
    <span>{progress.percent >= 0 ? `${progress.percent}%` : 'Size unknown'}</span>
  </div>
  <div class="download-progress-line download-progress-estimate">
    <span>{transferRate}</span>
    <span>{remaining}</span>
  </div>
  {#if progress.resumedBytes > 0}
    <div class="download-progress-resume">
      Resumed · {formatTransferSize(progress.resumedBytes)} retained
    </div>
  {/if}
</div>

<style>
  .download-progress-details {
    display: grid;
    gap: 0.22rem;
    min-width: 0;
    margin-top: 0.28rem;
    padding: 0.42rem 0.5rem;
    border: 1px solid var(--border-subtle, var(--border-strong));
    border-radius: 0.55rem;
    background: var(--surface-inset, var(--surface-subtle));
  }

  .download-progress-details.compact {
    gap: 0.16rem;
    padding: 0.32rem 0.42rem;
  }

  .download-progress-line {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.35;
    font-variant-numeric: tabular-nums;
  }

  .download-progress-line span:first-child {
    overflow-wrap: anywhere;
  }

  .download-progress-line span:last-child {
    flex: 0 0 auto;
    text-align: right;
  }

  .download-progress-estimate {
    color: var(--text-muted, var(--text-secondary));
  }

  .download-progress-resume {
    width: fit-content;
    max-width: 100%;
    margin-top: 0.05rem;
    padding: 0.12rem 0.34rem;
    border-radius: 999px;
    color: var(--status-accent-text);
    background: var(--status-accent-bg);
    font-size: 0.66rem;
    line-height: 1.3;
  }
</style>
