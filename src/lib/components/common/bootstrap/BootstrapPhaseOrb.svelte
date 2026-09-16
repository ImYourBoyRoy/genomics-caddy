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
    width: 96px;
    height: 96px;
    display: grid;
    place-items: center;
    margin: 0.35rem auto 0.55rem;
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
    inset: -8px;
    opacity: 0.55;
    animation: bootstrap-orb-pulse 2.4s ease-in-out infinite 0.35s;
  }

  .ring-3 {
    inset: -16px;
    opacity: 0.3;
    border-style: dashed;
    animation: bootstrap-orb-spin 14s linear infinite;
  }

  .logo-shell {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    background: rgba(12, 18, 32, 0.96);
    border: 1px solid var(--border-color);
    display: grid;
    place-items: center;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.35),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
  }

  .logo {
    width: 44px;
    height: 44px;
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
