<!-- ./src/lib/components/search/VariantSearchPanel.svelte -->
<script lang="ts">
  import type { DbSnpRecord } from '../../types/genomics';
  import PanelLoadingState from '../common/loading/PanelLoadingState.svelte';
  import LocalReferenceBrowser from './LocalReferenceBrowser.svelte';

  /*
  Module Docstring:
  Purpose: Raw genomic variant and local reference database search panel.
  Responsibilities:
  - Allow switching between querying raw genotypes and browsing reference databases.
  - Provide fields to search genotypes by rsID or coordinate ranges.
  - Integrate LocalReferenceBrowser to browse offline ClinVar, PharmGKB, etc.
  Key Inputs: searchRsid (string), browseChr (string), browseStart (number), browseEnd (number), browserResults (DbSnpRecord[]), isBrowsing (boolean), onSearch (callback).
  Key Outputs: Tabs and sub-panel interfaces.
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

  let activeSubTab = $state<"genotypes" | "reference">("genotypes");
  const chromosomes = Array(22).fill(0).map((_, i) => (i + 1).toString()).concat(["X", "Y", "MT"]);
</script>

<div class="card browser-container">
  <div class="browser-tab-header">
    <div class="browser-title-area">
      <h3>Local Database Browser</h3>
      <span class="badge secondary">SQL Query Engine</span>
    </div>
    <div class="browser-subtabs">
      <button
        type="button"
        class="tab-btn subtab-btn"
        class:active={activeSubTab === "genotypes"}
        onclick={() => activeSubTab = "genotypes"}
      >
        🧬 Raw Genotypes
      </button>
      <button
        type="button"
        class="tab-btn subtab-btn"
        class:active={activeSubTab === "reference"}
        onclick={() => activeSubTab = "reference"}
      >
        📚 Reference Databases
      </button>
    </div>
  </div>

  {#if activeSubTab === "genotypes"}
    <div class="genotypes-browser-area">
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
          <p class="empty-hint">No query results. Enter an rsID or chromosome range above to view your raw file genotypes.</p>
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
  {:else}
    <div class="reference-browser-area">
      <LocalReferenceBrowser />
    </div>
  {/if}
</div>

<style>
  .browser-tab-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 14px;
    margin-bottom: 20px;
    flex-wrap: wrap;
    gap: 16px;
  }

  .browser-title-area {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .browser-title-area h3 {
    margin: 0;
  }

  .browser-subtabs {
    display: flex;
    background: rgba(255, 255, 255, 0.03);
    padding: 3px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .subtab-btn {
    border: none;
    background: transparent;
    padding: 6px 14px;
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: 6px;
    color: var(--text-secondary, #94a3b8);
    cursor: pointer;
    transition: all 0.2s;
  }

  .subtab-btn:hover {
    color: white;
    background: rgba(255, 255, 255, 0.02);
  }

  .subtab-btn.active {
    background: rgba(255, 255, 255, 0.08);
    color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .badge.secondary {
    background: rgba(148, 163, 184, 0.1);
    color: #94a3b8;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }
</style>
