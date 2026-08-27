<!-- ./src/lib/components/sidebar/ProgressTrack.svelte -->
<script lang="ts">
  interface Props {
    percent?: number;
    variant?: 'download' | 'indexing' | 'preparing';
    spaced?: boolean;
    label: string;
  }

  let {
    percent = -1,
    variant = 'download',
    spaced = false,
    label,
  }: Props = $props();

  let normalizedPercent = $derived(
    Number.isFinite(percent) && percent >= 0 ? Math.min(100, Math.max(0, percent)) : null,
  );
  let isIndeterminate = $derived(normalizedPercent === null || variant === 'preparing');
  let trackClass = $derived([
    'progress-track',
    spaced ? 'progress-track-spaced' : '',
    variant === 'indexing' ? 'progress-track-indexing' : '',
    variant === 'preparing' ? 'progress-track-preparing' : '',
  ].filter(Boolean).join(' '));
  let fillClass = $derived([
    'progress-fill',
    variant === 'indexing' ? 'progress-fill-indexing' : '',
    isIndeterminate ? 'progress-fill-indeterminate' : '',
  ].filter(Boolean).join(' '));
  let fillStyle = $derived(`--progress-width: ${normalizedPercent ?? 100}%;`);
  let valueText = $derived(normalizedPercent === null ? 'In progress' : `${Math.round(normalizedPercent)}%`);
</script>

<div
  class={trackClass}
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={normalizedPercent ?? undefined}
  aria-valuetext={valueText}
>
  <div class={fillClass} style={fillStyle}></div>
</div>
