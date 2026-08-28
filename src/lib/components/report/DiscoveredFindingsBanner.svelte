<!-- ./src/lib/components/report/DiscoveredFindingsBanner.svelte -->
<script lang="ts">
  import { getDiscoveredFindingsSummary } from "../../api/tauri";
  import type { DiscoveredFindingSummary, GenomeSample } from "../../types/genomics";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";

  import type { VariantNavTarget } from "../../constants/traitCategories";

  interface Props {
    selectedSample: GenomeSample;
    presentationMode?: 'simple' | 'clinical' | 'compare';
    onExploreResearch?: (rsid: string) => void;
    onNavigate?: (rsid: string, target: VariantNavTarget) => void;
  }

  let { selectedSample, presentationMode = 'simple', onExploreResearch, onNavigate }: Props = $props();

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
    <ActivityPulse message="Loading agent discoveries…" accent="var(--status-info-text)" maxWidth="100%" />
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
      <button
        type="button"
        class="toggle-btn"
        aria-expanded={expanded}
        aria-controls={expanded ? "discovered-findings-list" : undefined}
        aria-label={expanded ? "Hide discovered findings" : "Show discovered findings"}
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? "Hide" : "Show"}
      </button>
    </div>
    {#if expanded}
      <ul id="discovered-findings-list" class="findings-list">
        {#each activeFindings.slice(0, 24) as item}
          <li>
            {#if presentationMode === 'simple'}
              <div class="simple-finding-summary">
                <button
                  type="button"
                  class="finding-link"
                  onclick={() => onExploreResearch?.(item.rsid)}
                >
                  View research finding
                </button>
                <details class="technical-details">
                  <summary>Technical data</summary>
                  <dl>
                    <div><dt>rsID</dt><dd>{item.rsid}</dd></div>
                    {#if item.gene}<div><dt>Gene</dt><dd>{item.gene}</dd></div>{/if}
                    {#if item.user_genotype}<div><dt>DNA call</dt><dd>{item.user_genotype}</dd></div>{/if}
                    {#if item.clinvar_clinical_significance}<div><dt>Clinical catalog</dt><dd>{item.clinvar_clinical_significance}</dd></div>{/if}
                  </dl>
                </details>
              </div>
            {:else}
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
            {/if}
            <span class="mini-nav">
              <button type="button" class="mini-btn" aria-label="Open on genome map" onclick={() => onNavigate?.(item.rsid, "map")}>🗺️</button>
              <button type="button" class="mini-btn" aria-label="Open in raw browser" onclick={() => onNavigate?.(item.rsid, "browser")}>🔍</button>
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
    background: var(--report-claim-bg);
    border: 1px solid var(--report-claim-border);
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
    border: 1px solid var(--border-color);
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

  .findings-list li {
    display: flex;
    min-width: 0;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 0.35rem 0.6rem;
  }

  .finding-link {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    min-width: 0;
    max-width: 100%;
    padding: 0;
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    overflow-wrap: anywhere;
  }

  .simple-finding-summary {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.2rem;
  }

  .technical-details {
    color: var(--text-secondary);
    font-size: 0.7rem;
  }

  .technical-details summary {
    cursor: pointer;
    font-weight: 700;
  }

  .technical-details dl {
    display: grid;
    gap: 0.35rem;
    margin: 0.4rem 0 0;
  }

  .technical-details dl > div {
    display: grid;
    grid-template-columns: minmax(4.5rem, auto) minmax(0, 1fr);
    gap: 0.5rem;
  }

  .technical-details dt {
    font-weight: 700;
  }

  .technical-details dd {
    min-width: 0;
    margin: 0;
    overflow-wrap: anywhere;
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

  .mini-nav {
    display: inline-flex;
    flex: 0 0 auto;
  }
</style>
