<!-- ./src/lib/components/common/bootstrap/BootstrapPhaseOrb.svelte -->
<script lang="ts">
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { PHASE_ACCENT } from "./bootstrapPhases";

  interface Props {
    phase: BootstrapPhase;
  }

  let { phase }: Props = $props();

  let isReady = $derived(phase === "ready");
  let isError = $derived(phase === "error");
</script>

<div
  class="phase-orb bootstrap-motion"
  class:ready={isReady}
  class:error={isError}
  style="--phase-accent: {PHASE_ACCENT[phase]}"
>
  <span class="ring ring-1 bootstrap-motion"></span>
  <span class="ring ring-2 bootstrap-motion"></span>
  <span class="ring ring-3 bootstrap-motion"></span>
  <div class="logo-shell">
    <img src="/logo.png" alt="Genomics Caddy Logo" class="logo" />
  </div>
</div>

<style>
  .phase-orb {
    position: relative;
    width: 112px;
    height: 112px;
    display: grid;
    place-items: center;
    margin-bottom: 0.75rem;
  }

  .ring {
    position: absolute;
    border-radius: 50%;
    border: 1px solid color-mix(in srgb, var(--phase-accent) 35%, transparent);
    box-shadow: 0 0 20px color-mix(in srgb, var(--phase-accent) 25%, transparent);
  }

  .ring-1 {
    inset: 0;
    animation: bootstrap-orb-pulse 2.4s ease-in-out infinite;
  }

  .ring-2 {
    inset: -10px;
    opacity: 0.55;
    animation: bootstrap-orb-pulse 2.4s ease-in-out infinite 0.35s;
  }

  .ring-3 {
    inset: -22px;
    opacity: 0.3;
    border-style: dashed;
    animation: bootstrap-orb-spin 14s linear infinite;
  }

  .logo-shell {
    width: 72px;
    height: 72px;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-color);
    display: grid;
    place-items: center;
    backdrop-filter: blur(12px);
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.35),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    animation: bootstrap-fade-up 0.7s ease both;
  }

  .logo {
    width: 52px;
    height: 52px;
    object-fit: contain;
  }

  .phase-orb.ready .logo-shell {
    animation: bootstrap-ready-glow 1.6s ease-in-out infinite;
    border-color: color-mix(in srgb, var(--success) 45%, var(--border-color));
  }

  .phase-orb.error .ring {
    border-color: color-mix(in srgb, var(--danger) 50%, transparent);
    box-shadow: 0 0 18px var(--bootstrap-glow-error);
  }
</style>
