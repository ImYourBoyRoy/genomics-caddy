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
      <span class="pulse-message bootstrap-motion">{message}</span>
    {/key}
  </div>
</div>

<style>
  .activity-pulse {
    position: relative;
    width: 100%;
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
    background:
      linear-gradient(
        110deg,
        rgba(255, 255, 255, 0.02) 0%,
        var(--bootstrap-shimmer) 45%,
        rgba(255, 255, 255, 0.02) 90%
      ),
      rgba(18, 19, 26, 0.88);
    background-size: 220% 100%;
    animation: bootstrap-shimmer-slide 3.2s linear infinite;
    border: 1px solid color-mix(in srgb, var(--phase-accent) 22%, var(--border-color));
    font-size: 0.78rem;
    color: var(--text-secondary);
    backdrop-filter: blur(14px);
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
    animation: bootstrap-message-in 0.45s ease both;
    line-height: 1.35;
  }

  .activity-pulse.error .pulse-border {
    background: conic-gradient(from 0deg, transparent, var(--danger), transparent);
    opacity: 0.5;
  }

  .activity-pulse.error .pulse-inner {
    color: #fca5a5;
    border-color: rgba(239, 68, 68, 0.25);
    background: rgba(239, 68, 68, 0.08);
    animation: none;
  }

  .error-mark {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(239, 68, 68, 0.2);
    color: #fca5a5;
    display: grid;
    place-items: center;
    font-size: 0.72rem;
    font-weight: 700;
    flex-shrink: 0;
  }
</style>
