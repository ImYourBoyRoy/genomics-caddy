<!-- ./src/lib/components/common/bootstrap/BootstrapHelixBackdrop.svelte -->
<script lang="ts">
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { PHASE_ACCENT } from "./bootstrapPhases";

  interface Props {
    phase: BootstrapPhase;
  }

  let { phase }: Props = $props();

  const particles = Array.from({ length: 18 }, (_, i) => ({
    id: i,
    left: `${8 + ((i * 17) % 84)}%`,
    top: `${12 + ((i * 23) % 76)}%`,
    delay: `${(i % 7) * 0.35}s`,
    duration: `${4 + (i % 5) * 0.6}s`,
    dx: `${-12 + (i % 24)}px`,
    dy: `${-28 - (i % 16)}px`,
  }));

  const rungs = Array.from({ length: 14 }, (_, i) => ({
    i,
    x1: 120 + Math.sin(i * 0.9) * 80,
    y1: 20 + i * 36,
    x2: 280 - Math.sin(i * 0.9) * 80,
    y2: 20 + i * 36,
    delay: i * 0.12,
  }));
</script>

<div
  class="helix-backdrop bootstrap-motion"
  style="--phase-accent: {PHASE_ACCENT[phase]}"
  aria-hidden="true"
>
  <div class="mesh-glow"></div>

  <svg class="helix-svg" viewBox="0 0 400 520" preserveAspectRatio="xMidYMid slice">
    <defs>
      <linearGradient id="strand-a" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="var(--phase-accent)" stop-opacity="0.05" />
        <stop offset="50%" stop-color="var(--phase-accent)" stop-opacity="0.55" />
        <stop offset="100%" stop-color="var(--phase-accent)" stop-opacity="0.05" />
      </linearGradient>
      <linearGradient id="strand-b" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="var(--success)" stop-opacity="0.04" />
        <stop offset="50%" stop-color="var(--success)" stop-opacity="0.4" />
        <stop offset="100%" stop-color="var(--success)" stop-opacity="0.04" />
      </linearGradient>
    </defs>

    <path
      class="strand strand-a"
      d="M 120 0 Q 220 65 120 130 Q 20 195 120 260 Q 220 325 120 390 Q 20 455 120 520"
      fill="none"
      stroke="url(#strand-a)"
      stroke-width="2"
    />
    <path
      class="strand strand-b"
      d="M 280 0 Q 180 65 280 130 Q 380 195 280 260 Q 180 325 280 390 Q 380 455 280 520"
      fill="none"
      stroke="url(#strand-b)"
      stroke-width="2"
    />

    {#each rungs as rung}
      <line
        class="rung"
        style="animation-delay: {rung.delay}s"
        x1={rung.x1}
        y1={rung.y1}
        x2={rung.x2}
        y2={rung.y2}
        stroke="var(--phase-accent)"
        stroke-opacity="0.22"
        stroke-width="1"
      />
    {/each}
  </svg>

  {#each particles as p}
    <span
      class="particle bootstrap-motion"
      style="
        left: {p.left};
        top: {p.top};
        animation-delay: {p.delay};
        animation-duration: {p.duration};
        --dx: {p.dx};
        --dy: {p.dy};
      "
    ></span>
  {/each}
</div>

<style>
  .helix-backdrop {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    z-index: 0;
  }

  .mesh-glow {
    position: absolute;
    inset: -20%;
    background:
      radial-gradient(circle at 20% 15%, var(--bootstrap-glow-soft), transparent 42%),
      radial-gradient(circle at 80% 70%, rgba(16, 185, 129, 0.08), transparent 45%),
      radial-gradient(circle at 50% 50%, rgba(88, 80, 236, 0.06), transparent 55%);
    transition: opacity 0.6s ease;
  }

  .helix-svg {
    position: absolute;
    left: 50%;
    top: 50%;
    width: min(920px, 110vw);
    height: min(720px, 90vh);
    transform: translate(-50%, -50%);
    animation: bootstrap-helix-drift 9s ease-in-out infinite;
    filter: drop-shadow(0 0 32px var(--bootstrap-glow-soft));
  }

  .strand {
    stroke-linecap: round;
  }

  .strand-a {
    animation: bootstrap-helix-drift 7s ease-in-out infinite reverse;
  }

  .rung {
    animation: bootstrap-fade-up 0.8s ease both;
  }

  .particle {
    position: absolute;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--phase-accent);
    box-shadow: 0 0 8px var(--phase-accent);
    animation: bootstrap-particle-float 5s ease-in-out infinite;
  }
</style>
