<!-- ./src/lib/components/search/VariantSearchPanel.svelte -->
<script lang="ts">
  import type { DbSnpRecord } from '../../types/genomics';
  import PanelLoadingState from '../common/loading/PanelLoadingState.svelte';

  /*
  Module Docstring:
  Purpose: Raw genomic variant search panel.
  Responsibilities:
  - Provide fields to search by rsID or chromosomal coordinate ranges (GRCh38).
  - Render a data table listing matching positions and user genotypes.
  Key Inputs: searchRsid (string), browseChr (string), browseStart (number), browseEnd (number), browserResults (DbSnpRecord[]), isBrowsing (boolean), onSearch (callback).
  Key Outputs: Search form and results table.
  Operational Notes: Limits table display to first 100 rows for performance.
  */

  interface Props {
    searchRsid: string;
    browseChr: string;
    browseStart: number;
    browseEnd: number;
    browserResults: DbSnpRecord[];
    isBrowsing: boolean;
    onSearch: (e: Event) => void;
  }

  let {
    searchRsid = $bindable(),
    browseChr = $bindable(),
    browseStart = $bindable(),
    browseEnd = $bindable(),
    browserResults,
    isBrowsing,
    onSearch
  }: Props = $props();

  const chromosomes = Array(22).fill(0).map((_, i) => (i + 1).toString()).concat(["X", "Y", "MT"]);
</script>

<div class="card browser-container">
  <h3>Query Raw Genotypes</h3>
  <form onsubmit={onSearch} class="browser-form">
    <div class="row">
      <div class="form-group flex-1">
        <label for="search-rsid">Query by rsID (Overrides Region)</label>
        <input id="search-rsid" type="text" placeholder="rs4680" bind:value={searchRsid} />
      </div>
      <div class="form-group flex-1">
        <label for="browse-chr">Chromosome</label>
        <select id="browse-chr" bind:value={browseChr}>
          {#each chromosomes as chr}
            <option value={chr}>{chr}</option>
          {/each}
        </select>
      </div>
    </div>
    <div class="row">
      <div class="form-group flex-1">
        <label for="browse-start">Start Position (GRCh38)</label>
        <input id="browse-start" type="number" bind:value={browseStart} />
      </div>
      <div class="form-group flex-1">
        <label for="browse-end">End Position (GRCh38)</label>
        <input id="browse-end" type="number" bind:value={browseEnd} />
      </div>
    </div>
    <button type="submit" class="btn btn-accent" disabled={isBrowsing}>
      {isBrowsing ? "Searching..." : "Search"}
    </button>
  </form>

  <div class="browser-results">
    <h4>Results ({browserResults.length})</h4>
    {#if isBrowsing}
      <PanelLoadingState
        message="Searching genotypes in SQLite…"
        submessage={searchRsid.trim() ? `Looking up ${searchRsid.trim()}` : `Region chr${browseChr}:${browseStart.toLocaleString()}-${browseEnd.toLocaleString()}`}
        accent="#818cf8"
        compact
      />
    {:else if browserResults.length === 0}
      <p class="empty-hint">No query results. Enter an rsID or chromosome range above.</p>
    {:else}
      <table class="results-table">
        <thead>
          <tr>
            <th>rsID</th>
            <th>Chromosome</th>
            <th>GRCh37 Position</th>
            <th>GRCh38 Position</th>
            <th>Genotype</th>
          </tr>
        </thead>
        <tbody>
          {#each browserResults.slice(0, 100) as r}
            <tr>
              <td><strong>{r.rsid}</strong></td>
              <td>Chr {r.chromosome}</td>
              <td>{r.position_grch37.toLocaleString()}</td>
              <td>{r.position_grch38 ? r.position_grch38.toLocaleString() : "Unmapped"}</td>
              <td><span class="genotype-tag">{r.allele1}{r.allele2}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if browserResults.length > 100}
        <div class="more-hint">Showing first 100 rows. Use narrower bounds to filter.</div>
      {/if}
    {/if}
  </div>
</div>
