<!-- ./src/lib/components/search/VariantSearchPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Premium raw genotype + local reference database browser.
  Responsibilities:
  - Search imported genotypes by rsID or GRCh38 region.
  - Browse offline ClinVar / PharmGKB / ClinGen / GWAS / MANE tables.
  - Cross-link findings to Report and Chromosome Map.
  Key Inputs: sample, search fields, results, highlightRsid, navigation callbacks.
  Key Outputs: Interactive browser UI.
  */

  import type { DbSnpRecord, GenomeSample } from "../../types/genomics";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";
  import LocalReferenceBrowser from "./LocalReferenceBrowser.svelte";
  import "$lib/styles/components/variant-browser.css";

  interface Props {
    selectedSample: GenomeSample | null;
    searchRsid: string;
    browseChr: string;
    browseStart: number;
    browseEnd: number;
    browserResults: DbSnpRecord[];
    isBrowsing: boolean;
    highlightRsid?: string;
    onSearch: (e: Event) => void;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let {
    selectedSample,
    searchRsid = $bindable(),
    browseChr = $bindable(),
    browseStart = $bindable(),
    browseEnd = $bindable(),
    browserResults,
    isBrowsing,
    highlightRsid = "",
    onSearch,
    onNavigateToVariant,
  }: Props = $props();

  let activeSubTab = $state<"genotypes" | "reference">("genotypes");
  let userSearched = $state(false);
  const chromosomes = Array(22)
    .fill(0)
    .map((_, i) => (i + 1).toString())
    .concat(["X", "Y", "MT"]);

  const displayLimit = 100;
  let visibleResults = $derived(browserResults.slice(0, displayLimit));
  let truncated = $derived(browserResults.length > displayLimit);
  let hasSearched = $derived(userSearched || browserResults.length > 0);

  function handleSubmit(e: Event) {
    userSearched = true;
    onSearch(e);
  }

  function runExample(rsid: string) {
    searchRsid = rsid;
    userSearched = true;
    const fake = new Event("submit", { bubbles: true, cancelable: true });
    onSearch(fake);
  }
</script>

<section class="variant-browser">
  <header class="vb-hero">
    <div class="vb-hero-copy">
      <span class="vb-kicker">Raw genotypes · on-device</span>
      <h3>Variant browser</h3>
      <p class="vb-lead">
        Look up alleles from
        {#if selectedSample}
          <strong>{selectedSample.name}</strong>
        {:else}
          your imported genome
        {/if}
        by rsID or GRCh38 region, or inspect offline reference catalogs. This is a lookup tool — not a
        clinical report.
      </p>
    </div>
    <span class="vb-privacy">Local SQLite · no upload</span>
  </header>

  <aside class="vb-insight" aria-labelledby="vb-insight-title">
    <strong id="vb-insight-title">When to use this</strong>
    <p>
      Prefer <em>Trait Report</em> for curated pack findings and <em>Discovery</em> for ranked
      catalog associations. Use this browser when you need the raw genotype row or to page through a
      downloaded reference table.
    </p>
  </aside>

  <div class="vb-subtabs" role="tablist" aria-label="Browser mode">
    <button
      type="button"
      role="tab"
      class="vb-subtab"
      class:active={activeSubTab === "genotypes"}
      aria-selected={activeSubTab === "genotypes"}
      id="vb-tab-genotypes"
      onclick={() => (activeSubTab = "genotypes")}
    >
      Your genotypes
    </button>
    <button
      type="button"
      role="tab"
      class="vb-subtab"
      class:active={activeSubTab === "reference"}
      aria-selected={activeSubTab === "reference"}
      id="vb-tab-reference"
      onclick={() => (activeSubTab = "reference")}
    >
      Reference catalogs
    </button>
  </div>

  {#if activeSubTab === "genotypes"}
    <div class="vb-panel" role="tabpanel" aria-labelledby="vb-tab-genotypes">
      {#if !selectedSample}
        <div class="vb-empty" role="status">
          <strong>Import a genome first</strong>
          <p>Select or import a profile in the sidebar, then search by rsID or chromosome region.</p>
        </div>
      {:else}
        <form class="vb-form" onsubmit={handleSubmit}>
          <label class="vb-field vb-field-grow">
            <span>rsID (overrides region)</span>
            <input type="text" placeholder="e.g. rs4680" bind:value={searchRsid} />
          </label>
          <label class="vb-field">
            <span>Chromosome</span>
            <select bind:value={browseChr}>
              {#each chromosomes as chr}
                <option value={chr}>{chr}</option>
              {/each}
            </select>
          </label>
          <label class="vb-field">
            <span>Start (GRCh38)</span>
            <input type="number" bind:value={browseStart} />
          </label>
          <label class="vb-field">
            <span>End (GRCh38)</span>
            <input type="number" bind:value={browseEnd} />
          </label>
          <button type="submit" class="btn btn-accent vb-submit" disabled={isBrowsing}>
            {isBrowsing ? "Searching…" : "Search"}
          </button>
        </form>

        <div class="vb-examples" aria-label="Example queries">
          <span class="vb-examples-label">Try</span>
          {#each ["rs4680", "rs1801133", "rs429358"] as ex (ex)}
            <button type="button" class="vb-example" onclick={() => runExample(ex)}>{ex}</button>
          {/each}
        </div>

        <div class="vb-results">
          <div class="vb-results-head">
            <h4>Results</h4>
            {#if !isBrowsing && hasSearched}
              <span class="vb-results-count">{browserResults.length.toLocaleString()} row{browserResults.length === 1 ? "" : "s"}</span>
            {/if}
          </div>

          {#if isBrowsing}
            <PanelLoadingState
              message="Searching genotypes in your local database…"
              submessage={searchRsid.trim()
                ? `Looking up ${searchRsid.trim()}`
                : `Region chr${browseChr}:${browseStart.toLocaleString()}–${browseEnd.toLocaleString()} (GRCh38)`}
              accent="#2dd4bf"
              compact
            />
          {:else if !hasSearched && browserResults.length === 0}
            <div class="vb-empty quiet">
              <strong>No query yet</strong>
              <p>Enter an rsID or GRCh38 range, or use an example above.</p>
            </div>
          {:else if browserResults.length === 0}
            <div class="vb-empty" role="status">
              <strong>No genotypes matched</strong>
              <p>
                Nothing in {selectedSample.name} matched this query. Try another rsID or a narrower
                region.
              </p>
            </div>
          {:else}
            <div class="vb-table-wrap">
              <table class="vb-table">
                <thead>
                  <tr>
                    <th scope="col">rsID</th>
                    <th scope="col">Chr</th>
                    <th scope="col">GRCh38</th>
                    <th scope="col">GRCh37</th>
                    <th scope="col">Genotype</th>
                    <th scope="col" class="vb-actions-col">Open</th>
                  </tr>
                </thead>
                <tbody>
                  {#each visibleResults as r (r.rsid + String(r.position_grch38 ?? r.position_grch37))}
                    <tr
                      class:vb-row-highlight={highlightRsid &&
                        r.rsid.toLowerCase() === highlightRsid.toLowerCase()}
                    >
                      <td class="font-mono"><strong>{r.rsid}</strong></td>
                      <td>{r.chromosome}</td>
                      <td class="font-mono">
                        {r.position_grch38 ? r.position_grch38.toLocaleString() : "Unmapped"}
                      </td>
                      <td class="font-mono muted">{r.position_grch37.toLocaleString()}</td>
                      <td><span class="vb-gt">{r.allele1}{r.allele2}</span></td>
                      <td class="vb-actions">
                        {#if onNavigateToVariant}
                          <button
                            type="button"
                            class="btn btn-link btn-xs"
                            onclick={() => onNavigateToVariant?.(r.rsid, "report")}
                          >
                            Report
                          </button>
                          <button
                            type="button"
                            class="btn btn-link btn-xs"
                            onclick={() => onNavigateToVariant?.(r.rsid, "map")}
                          >
                            Map
                          </button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
            {#if truncated}
              <p class="vb-truncate">
                Showing first {displayLimit} of {browserResults.length.toLocaleString()} rows. Narrow
                the region or query a single rsID for a full match set.
              </p>
            {/if}
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <div class="vb-panel" role="tabpanel" aria-labelledby="vb-tab-reference">
      <LocalReferenceBrowser />
    </div>
  {/if}
</section>
