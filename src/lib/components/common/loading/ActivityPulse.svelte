<!-- ./src/lib/components/common/loading/ActivityPulse.svelte -->
<script lang="ts">
  import "../../../styles/bootstrap-animations.css";

  interface Props {
    message: string;
    accent?: string;
    isError?: boolean;
    maxWidth?: string;
  }

  let {
    message,
    accent = "var(--accent)",
    isError = false,
    maxWidth = "100%",
  }: Props = $props();
</script>

<div
  class="activity-pulse bootstrap-motion"
  class:error={isError}
  style="--phase-accent: {isError ? 'var(--danger)' : accent}; max-width: {maxWidth};"
  aria-live="polite"
>
  <div class="pulse-border bootstrap-motion"></div>
  <div class="pulse-inner">
    {#if !isError}
      <span class="pulse-dot bootstrap-motion"></span>
    {:else}
      <span class="error-mark">!</span>
    {/if}
    {#key message}
      <span class="pulse-message">{message}</span>
    {/key}
  </div>
</div>

<style>
  .activity-pulse {
    position: relative;
    width: 100%;
    box-sizing: border-box;
    min-width: 0;
    border-radius: 10px;
    padding: 1px;
    overflow: hidden;
  }

  .pulse-border {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: conic-gradient(
      from var(--bootstrap-border-angle, 0deg),
      transparent 0deg,
      var(--phase-accent) 80deg,
      transparent 160deg,
      color-mix(in srgb, var(--phase-accent) 40%, var(--success)) 240deg,
      transparent 320deg
    );
    animation: bootstrap-orb-spin 4.5s linear infinite;
    opacity: 0.65;
  }

  .pulse-inner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 14px;
    border-radius: 9px;
    background: var(--surface-control);
    border: 1px solid color-mix(in srgb, var(--phase-accent) 22%, var(--border-color));
    font-size: 0.78rem;
    color: var(--text-primary);
  }

  .pulse-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--phase-accent);
    box-shadow: 0 0 12px var(--phase-accent);
    flex-shrink: 0;
    animation: bootstrap-orb-pulse 1.2s ease-in-out infinite;
  }

  .pulse-message {
    line-height: 1.35;
    color: var(--text-primary);
    opacity: 1;
    animation: none;
  }

  .activity-pulse.error .pulse-border {
    background: conic-gradient(from 0deg, transparent, var(--danger), transparent);
    opacity: 0.5;
  }

  .activity-pulse.error .pulse-inner {
    color: var(--status-danger-strong-text);
    border-color: var(--status-danger-border);
    background: var(--status-danger-bg);
    animation: none;
  }

  .error-mark {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--status-danger-bg);
    color: var(--status-danger-strong-text);
    display: grid;
    place-items: center;
    font-size: 0.72rem;
    font-weight: 700;
    flex-shrink: 0;
  }
</style>
