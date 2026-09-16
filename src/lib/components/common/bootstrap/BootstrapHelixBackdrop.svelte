<!-- ./src/lib/components/common/bootstrap/BootstrapHelixBackdrop.svelte -->
<script lang="ts">
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { PHASE_ACCENT } from "./bootstrapPhases";

  interface Props {
    phase: BootstrapPhase;
  }

  let { phase }: Props = $props();

  const particles = Array.from({ length: 12 }, (_, i) => ({
    id: i,
    left: `${12 + ((i * 19) % 76)}%`,
    top: `${10 + ((i * 23) % 78)}%`,
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

  <div class="helix-anchor">
    <div class="helix-stage">
      <svg class="helix-svg" viewBox="0 0 400 520" preserveAspectRatio="xMidYMid meet">
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
        stroke-width="16"
        opacity="0.15"
      />
      <path
        class="strand strand-a"
        d="M 120 0 Q 220 65 120 130 Q 20 195 120 260 Q 220 325 120 390 Q 20 455 120 520"
        fill="none"
        stroke="url(#strand-a)"
        stroke-width="7"
        opacity="0.45"
      />
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
        stroke-width="16"
        opacity="0.12"
      />
      <path
        class="strand strand-b"
        d="M 280 0 Q 180 65 280 130 Q 380 195 280 260 Q 180 325 280 390 Q 380 455 280 520"
        fill="none"
        stroke="url(#strand-b)"
        stroke-width="7"
        opacity="0.35"
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
    </div>
  </div>

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
  /* Full-screen background layer; the helix itself is anchored quietly on the right. */
  .helix-backdrop {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .mesh-glow {
    position: absolute;
    inset: -10%;
    background:
      radial-gradient(ellipse at 84% 48%, var(--bootstrap-glow-soft), transparent 50%),
      radial-gradient(ellipse at 78% 78%, rgba(16, 185, 129, 0.1), transparent 44%);
  }

  .helix-anchor {
    position: absolute;
    top: 50%;
    right: clamp(1.5rem, 7vw, 8rem);
    left: auto;
    width: min(34vw, 28rem);
    height: min(70vh, 34rem);
    transform: translateY(-50%);
    opacity: 0.42;
  }

  .helix-stage {
    width: 100%;
    height: 100%;
    animation: bootstrap-helix-drift 9s ease-in-out infinite;
  }

  .helix-svg {
    width: 100%;
    height: 100%;
  }

  .strand {
    stroke-linecap: round;
  }

  .strand-a {
    animation: bootstrap-helix-drift 7s ease-in-out infinite reverse;
  }

  .rung {
    opacity: 0.35;
  }

  .particle {
    position: absolute;
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--phase-accent);
    box-shadow: 0 0 8px var(--phase-accent);
    animation: bootstrap-particle-float 5s ease-in-out infinite;
  }

  @media (max-width: 720px) {
    .helix-anchor {
      right: 4%;
      width: min(52vw, 16rem);
      height: min(62vh, 28rem);
      opacity: 0.28;
    }
  }
</style>
