<!-- ./src/lib/components/report/ReportLoadingState.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { ReportGenerationProgress } from '$lib/types/genomics';
  import '$lib/styles/components/report-loading.css';

  interface Props {
    sampleName: string;
    progress: ReportGenerationProgress | null;
  }

  let { sampleName, progress }: Props = $props();
  let elapsedSeconds = $state(0);

  onMount(() => {
    const startedAt = Date.now() - Math.max(0, progress?.elapsedMs ?? 0);
    const updateElapsed = () => {
      elapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
    };
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 1000);
    return () => window.clearInterval(timer);
  });

  const percentage = $derived(
    progress && progress.total > 0 && (progress.phase === 'markers' || progress.phase === 'clinvar')
      ? Math.min(100, Math.floor((progress.current / progress.total) * 100))
      : null,
  );

  const countLabel = $derived(
    progress?.phase === 'markers'
      ? 'Curated marker checks'
      : progress?.phase === 'clinvar'
        ? 'Profile variants checked'
        : '',
  );

  const estimatedSecondsRemaining = $derived.by(() => {
    if (!progress || percentage === null || percentage >= 95) return null;
    if (progress.phaseElapsedMs < 3_000 || progress.current < 100) return null;
    const elapsed = progress.phaseElapsedMs / 1_000;
    const rate = progress.current / elapsed;
    if (!Number.isFinite(rate) || rate <= 0) return null;
    return Math.ceil((progress.total - progress.current) / rate);
  });

  function formatCount(value: number): string {
    return new Intl.NumberFormat().format(value);
  }

  function formatDuration(seconds: number): string {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return minutes > 0 ? `${minutes}m ${remainingSeconds}s` : `${remainingSeconds}s`;
  }
</script>

<section
  class="report-loading-state"
  aria-busy="true"
  aria-labelledby="report-loading-title"
>
  <p class="report-kicker">Trait report</p>
  <h3 id="report-loading-title">Preparing {sampleName}'s report</h3>
  <p class="report-loading-lead" role="status" aria-live="polite">
    {progress?.status ?? 'Preparing the local profile and report resources.'}
  </p>
  <div
    class="report-loading-progress"
    class:determinate={percentage !== null}
    role="progressbar"
    aria-label={countLabel || 'Report preparation'}
    aria-valuemin="0"
    aria-valuemax={percentage === null ? undefined : 100}
    aria-valuenow={percentage ?? undefined}
    aria-valuetext={percentage === null ? undefined : `${percentage}% complete`}
    style={`--report-progress-width: ${percentage ?? 0}%`}
  >
    <span></span>
  </div>
  {#if percentage !== null}
    <div class="report-loading-counts">
      <span>{countLabel}: <strong>{formatCount(progress?.current ?? 0)} / {formatCount(progress?.total ?? 0)}</strong></span>
      {#if progress && progress.matchesLabel}
        <span><strong>{formatCount(progress.matches)}</strong> {progress.matchesLabel}</span>
      {/if}
    </div>
  {/if}
  <p class="report-loading-timing">
    <span>Elapsed {formatDuration(elapsedSeconds)}</span>
    {#if estimatedSecondsRemaining !== null}
      <span>About {formatDuration(estimatedSecondsRemaining)} left in this step</span>
    {/if}
  </p>
  {#if estimatedSecondsRemaining !== null}
    <p class="report-loading-note">Estimate is based on the current scan rate; final report assembly follows.</p>
  {:else if percentage !== null && percentage < 95 && progress && progress.phaseElapsedMs >= 3000}
    <p class="report-loading-note">The count is current; an estimate will appear after enough scan progress has been measured.</p>
  {:else}
    <p class="report-loading-note">Your DNA stays on this computer.</p>
  {/if}
</section>
