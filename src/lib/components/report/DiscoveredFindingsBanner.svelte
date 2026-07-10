<!-- ./src/lib/components/report/DiscoveredFindingsBanner.svelte -->
<script lang="ts">
  import { getDiscoveredFindingsSummary } from "../../api/tauri";
  import type { DiscoveredFindingSummary, GenomeSample } from "../../types/genomics";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";

  import type { VariantNavTarget } from "../../constants/traitCategories";

  interface Props {
    selectedSample: GenomeSample;
    onExploreResearch?: (rsid: string) => void;
    onNavigate?: (rsid: string, target: VariantNavTarget) => void;
  }

  let { selectedSample, onExploreResearch, onNavigate }: Props = $props();

  let findings = $state<DiscoveredFindingSummary[]>([]);
  let isLoading = $state(true);
  let loadError = $state("");
  let expanded = $state(false);

  const activeFindings = $derived(
    findings.filter(
      (f) =>
        f.interpretation_status === "active_clinical" ||
        f.interpretation_status === "active_research"
    )
  );

  $effect(() => {
    const sampleId = selectedSample.id;
    isLoading = true;
    loadError = "";
    getDiscoveredFindingsSummary(sampleId)
      .then((rows) => {
        findings = rows;
      })
      .catch((e) => {
        loadError = String(e);
        findings = [];
      })
      .finally(() => {
        isLoading = false;
      });
  });
</script>

{#if isLoading}
  <div class="discoveries-banner loading no-print">
    <ActivityPulse message="Loading agent discoveries…" accent="#60a5fa" maxWidth="100%" />
  </div>
{:else if loadError}
  <!-- silent fail -->
{:else if activeFindings.length > 0}
  <div class="discoveries-banner no-print">
    <div class="banner-header">
      <span class="banner-icon">🕵️</span>
      <div>
        <strong>{activeFindings.length} active variant{activeFindings.length === 1 ? "" : "s"}</strong>
        from Research Agent scans not in the curated trait report.
      </div>
      <button type="button" class="toggle-btn" onclick={() => (expanded = !expanded)}>
        {expanded ? "Hide" : "Show"}
      </button>
    </div>
    {#if expanded}
      <ul class="findings-list">
        {#each activeFindings.slice(0, 24) as item}
          <li>
            <button
              type="button"
              class="finding-link"
              onclick={() => onExploreResearch?.(item.rsid)}
            >
              <code>{item.rsid}</code>
              {#if item.gene}<span class="gene">{item.gene}</span>{/if}
              {#if item.user_genotype}<span class="gt">{item.user_genotype}</span>{/if}
            </button>
            {#if item.clinvar_clinical_significance}
              <span class="clinvar">{item.clinvar_clinical_significance}</span>
            {/if}
            <span class="mini-nav">
              <button type="button" class="mini-btn" onclick={() => onNavigate?.(item.rsid, "map")} title="Map">🗺️</button>
              <button type="button" class="mini-btn" onclick={() => onNavigate?.(item.rsid, "browser")} title="Browser">🔍</button>
            </span>
          </li>
        {/each}
      </ul>
      {#if activeFindings.length > 24}
        <p class="more-note">+ {activeFindings.length - 24} more in Research Agent tab</p>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .discoveries-banner {
    background: rgba(59, 130, 246, 0.08);
    border: 1px solid rgba(59, 130, 246, 0.22);
    border-radius: 10px;
    padding: 12px 14px;
    margin-bottom: 12px;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .discoveries-banner.loading {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .banner-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .banner-icon {
    font-size: 1.1rem;
  }

  .toggle-btn {
    margin-left: auto;
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .findings-list {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .finding-link {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .finding-link code {
    color: var(--accent);
  }

  .gene {
    font-weight: 600;
    color: var(--text-primary);
  }

  .gt {
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
  }

  .clinvar {
    display: block;
    margin-left: 0.5rem;
    font-size: 0.72rem;
    opacity: 0.85;
  }

  .more-note {
    margin: 8px 0 0;
    font-size: 0.72rem;
  }
</style>
