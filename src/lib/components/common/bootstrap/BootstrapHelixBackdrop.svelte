<!-- ./src/lib/components/common/bootstrap/BootstrapHelixBackdrop.svelte -->
<script lang="ts">
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { PHASE_ACCENT, PHASE_ORDER } from "./bootstrapPhases";
  import SequencingHelix from "../loading/SequencingHelix.svelte";

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

  // Startup phases light the strand from the top; an error leaves it dark.
  let helixPercent = $derived.by(() => {
    if (phase === "error") return 0;
    const index = PHASE_ORDER.indexOf(phase);
    return Math.round(((index + (phase === "ready" ? 0 : 0.5)) / (PHASE_ORDER.length - 1)) * 100);
  });
</script>

<div
  class="helix-backdrop bootstrap-motion"
  style="--phase-accent: {PHASE_ACCENT[phase]}"
  aria-hidden="true"
>
  <div class="mesh-glow"></div>

  <div class="helix-anchor">
    <SequencingHelix percentage={helixPercent} />
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
    opacity: 0.75;
  }

  .helix-anchor :global(.sequencing-helix) {
    --helix-width: clamp(9rem, 22vw, 15rem);
    --helix-amp: calc(var(--helix-width) * 0.42);
    --helix-node: clamp(0.6rem, 1.3vw, 0.95rem);
    --helix-period: 6s;
    --accent: var(--phase-accent);
    width: var(--helix-width);
    height: 100%;
    margin-inline: auto;
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
