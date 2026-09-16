<!-- ./src/lib/components/common/bootstrap/BootstrapStatReveal.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus } from "../../../types/genomics";
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { animateCountUp } from "./bootstrapCountUp";

  interface Props {
    status: AppBootstrapStatus;
    phase: BootstrapPhase;
  }

  let { status, phase }: Props = $props();

  let visible = $derived(
    phase === "stats" ||
      phase === "profile" ||
      phase === "report" ||
      phase === "ready"
  );

  let sampleCount = $state(0);
  let genotypeCount = $state(0);
  let gwasCount = $state(0);
  let findingsCount = $state(0);

  const stats = $derived([
    { label: "Profiles", value: sampleCount, raw: status.sample_count },
    { label: "Genotypes", value: genotypeCount, raw: status.genotype_count },
    { label: "GWAS refs", value: gwasCount, raw: status.gwas_reference_count },
    { label: "Agent findings", value: findingsCount, raw: status.discovered_findings_count },
  ]);

  let animateKey = $state("");

  $effect(() => {
    if (!visible) {
      sampleCount = 0;
      genotypeCount = 0;
      gwasCount = 0;
      findingsCount = 0;
      animateKey = "";
      return;
    }

    const key = `${status.sample_count}:${status.genotype_count}:${status.gwas_reference_count}:${status.discovered_findings_count}`;
    if (animateKey === key) return;
    animateKey = key;

    void animateCountUp(status.sample_count, 700, (v) => (sampleCount = v));
    void animateCountUp(status.genotype_count, 1100, (v) => (genotypeCount = v));
    void animateCountUp(status.gwas_reference_count, 900, (v) => (gwasCount = v));
    void animateCountUp(status.discovered_findings_count, 800, (v) => (findingsCount = v));
  });
</script>

{#if visible}
  <div class="stats-reveal">
    {#each stats as stat, index}
      <div
        class="stat-card bootstrap-motion"
        style="animation-delay: {index * 0.1}s"
      >
        <span class="stat-value">{stat.value.toLocaleString()}</span>
        <span class="stat-label">{stat.label}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .stats-reveal {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    max-width: 520px;
    width: 100%;
    margin-top: 0.5rem;
  }

  .stat-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    padding: 12px 8px;
    position: relative;
    overflow: hidden;
  }

  .stat-card::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(
      120deg,
      transparent 30%,
      rgba(255, 255, 255, 0.04) 50%,
      transparent 70%
    );
    transform: translateX(-120%);
    animation: bootstrap-shimmer-slide 2.8s ease-in-out infinite;
  }

  .stat-value {
    display: block;
    font-size: 0.98rem;
    font-weight: 700;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .stat-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  @media (max-width: 520px) {
    .stats-reveal {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
