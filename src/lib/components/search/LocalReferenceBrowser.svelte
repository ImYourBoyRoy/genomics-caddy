<!-- ./src/lib/components/search/LocalReferenceBrowser.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { queryLocalReferenceDb } from "../../api/tauri";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";
  import "$lib/styles/components/local-reference-browser.css";

  let selectedTable = $state<"clinvar" | "pharmgkb" | "clingen" | "gwas" | "mane">("clinvar");
  let searchQuery = $state("");
  let limit = $state(20);
  let page = $state(0);
  let rows = $state<any[]>([]);
  let total = $state(0);
  let isLoading = $state(false);
  let errorMsg = $state("");

  // Keep track of search execution
  let searchTriggered = $state(false);

  let pageCount = $derived(Math.max(1, Math.ceil(total / limit)));
  let pageStart = $derived(total === 0 ? 0 : page * limit + 1);
  let pageEnd = $derived(Math.min(total, (page + 1) * limit));

  async function loadData(resetPage = false) {
    if (resetPage) page = 0;
    isLoading = true;
    errorMsg = "";
    searchTriggered = true;
    try {
      const result = await queryLocalReferenceDb(
        selectedTable,
        searchQuery.trim() || undefined,
        limit,
        page * limit
      );
      rows = result.rows;
      total = result.total;
    } catch (e: any) {
      errorMsg = e instanceof Error ? e.message : String(e);
      rows = [];
      total = 0;
    } finally {
      isLoading = false;
    }
  }

  function handleTableChange() {
    searchQuery = "";
    void loadData(true);
  }

  function goPrev() {
    if (page <= 0) return;
    page -= 1;
    void loadData(false);
  }

  function goNext() {
    if (page + 1 >= pageCount) return;
    page += 1;
    void loadData(false);
  }

  // Initial load only — do not $effect on `page` (that reset pagination).
  onMount(() => {
    void loadData(true);
  });
</script>

<div class="local-db-browser">
  <div class="browser-header-summary">
    <p class="summary-text">
      Query and inspect the non-vectorized reference databases downloaded within Genomics Caddy.
      This lets you browse ClinVar annotations, PharmGKB drug interactions, ClinGen gene validity classifications, and GWAS findings directly in your local SQLite store.
    </p>
  </div>

  <div class="search-controls-bar">
    <div class="form-group select-group">
      <label for="db-table-select">Select Reference DB</label>
      <select id="db-table-select" bind:value={selectedTable} onchange={handleTableChange}>
        <option value="clinvar">ClinVar Variants ({selectedTable === 'clinvar' ? total : '...'})</option>
        <option value="pharmgkb">PharmGKB Clinical Annotations ({selectedTable === 'pharmgkb' ? total : '...'})</option>
        <option value="clingen">ClinGen Gene Validity ({selectedTable === 'clingen' ? total : '...'})</option>
        <option value="gwas">GWAS Catalog Associations ({selectedTable === 'gwas' ? total : '...'})</option>
        <option value="mane">MANE Select Transcripts ({selectedTable === 'mane' ? total : '...'})</option>
      </select>
    </div>

    <form onsubmit={(e) => { e.preventDefault(); loadData(true); }} class="query-form">
      <div class="form-group query-input-group">
        <label for="db-search-input">Keyword search (space-separated tokens = AND; substring match, not typo-fuzzy)</label>
        <div class="input-with-button">
          <input
            id="db-search-input"
            type="text"
            placeholder={
              selectedTable === 'clinvar' ? "rs4680 breast pathogenic — or gene BRCA1" :
              selectedTable === 'pharmgkb' ? "rs1801133 MTHFR Warfarin" :
              selectedTable === 'clingen' ? "BRCA1 Definite Arrhythmogenic" :
              selectedTable === 'gwas' ? "rs1042778 Obesity" :
              "APOE Select chr19"
            }
            bind:value={searchQuery}
          />
          <button type="submit" class="btn btn-accent" disabled={isLoading}>
            {isLoading ? "Searching..." : "Search"}
          </button>
        </div>
      </div>
    </form>
  </div>

  {#if isLoading}
    <div class="loading-container">
      <PanelLoadingState
        message={`Querying local ${selectedTable.toUpperCase()} reference database...`}
        submessage={searchQuery.trim() ? `Matching "${searchQuery}"` : "Retrieving all records"}
        accent="var(--status-info-text)"
        compact
      />
    </div>
  {:else if errorMsg}
    <div class="alert-box error-alert">
      <strong>Query Failed:</strong> {errorMsg}. Make sure you have downloaded reference data for this tier in the settings.
    </div>
  {:else if rows.length === 0}
    <div class="empty-results-box">
      <h4>No Records Found</h4>
      <p>
        No matches for "{searchQuery}" in {selectedTable.toUpperCase()}.
        {#if !searchQuery.trim()}
          This table might be empty. Download reference databases from the sidebar
          <strong>Reference Databases</strong> section, then search again.
        {:else}
          Try clearing your search query or using a different keyword.
        {/if}
      </p>
    </div>
  {:else}
    <div class="results-metadata-row">
      <div class="pagination-info">
        Showing <strong>{pageStart}–{pageEnd}</strong> of <strong>{total.toLocaleString()}</strong> records
      </div>
      <div class="limit-selector">
        <label for="limit-select">Rows:</label>
        <select id="limit-select" bind:value={limit} onchange={() => loadData(true)}>
          <option value={10}>10</option>
          <option value={20}>20</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
        </select>
      </div>
    </div>

    <div class="table-scroll-container">
      <table class="db-results-table">
        <thead>
          <tr>
            {#if selectedTable === 'clinvar'}
              <th>rsID</th>
              <th>Variation ID</th>
              <th>Clinical Significance</th>
              <th>Review Status</th>
              <th>Phenotypes / Conditions</th>
              <th>Last Evaluated</th>
            {:else if selectedTable === 'pharmgkb'}
              <th>rsID</th>
              <th>Gene</th>
              <th>Drug</th>
              <th>Phenotypic Effect / Outcome</th>
              <th>Evidence Level</th>
            {:else if selectedTable === 'clingen'}
              <th>Gene</th>
              <th>Disease Label</th>
              <th>Validity Classification</th>
              <th>Mode of Inheritance</th>
              <th>Link</th>
            {:else if selectedTable === 'gwas'}
              <th>rsID</th>
              <th>Mapped Trait</th>
              <th>P-Value</th>
              <th>OR / Beta</th>
              <th>Journal / Citation</th>
              <th>Study Title</th>
            {:else if selectedTable === 'mane'}
              <th>Gene</th>
              <th>Ensembl Transcript</th>
              <th>RefSeq Transcript</th>
              <th>Status</th>
              <th>GRCh38 Coordinates</th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each rows as r, i (selectedTable + ':' + page + ':' + i + ':' + (r.rsid ?? r.gene_symbol ?? r.variation_id ?? ''))}
            <tr>
              {#if selectedTable === 'clinvar'}
                <td class="font-mono bold text-accent">{r.rsid}</td>
                <td class="font-mono">{r.variation_id}</td>
                <td>
                  <span class="badge" class:pathogenic={r.clinical_significance.toLowerCase().includes('pathogenic')} class:benign={r.clinical_significance.toLowerCase().includes('benign')}>
                    {r.clinical_significance}
                  </span>
                </td>
                <td class="review-status text-muted">{r.review_status}</td>
                <td class="phenotype-cell">{r.phenotype_names}</td>
                <td class="date-cell font-mono text-muted">{r.last_evaluated || 'N/A'}</td>
              {:else if selectedTable === 'pharmgkb'}
                <td class="font-mono bold text-accent">{r.rsid}</td>
                <td class="bold">{r.gene || 'N/A'}</td>
                <td class="text-secondary bold">{r.drug || 'N/A'}</td>
                <td class="desc-cell">{r.phenotype || 'N/A'}</td>
                <td>
                  <span class="level-badge" class:level-1={r.evidence_level?.startsWith('1')} class:level-2={r.evidence_level?.startsWith('2')}>
                    Level {r.evidence_level || 'N/A'}
                  </span>
                </td>
              {:else if selectedTable === 'clingen'}
                <td class="bold text-accent">{r.gene_symbol}</td>
                <td>{r.disease_label}</td>
                <td>
                  <span class="validity-badge" class:definite={r.classification?.toLowerCase() === 'definite'} class:strong={r.classification?.toLowerCase() === 'strong'}>
                    {r.classification || 'N/A'}
                  </span>
                </td>
                <td class="font-mono text-muted">{r.moi || 'N/A'}</td>
                <td>
                  {#if r.report_url}
                    <a href={r.report_url} target="_blank" rel="noopener noreferrer" class="link-btn">Report ↗</a>
                  {:else}
                    <span class="text-muted">—</span>
                  {/if}
                </td>
              {:else if selectedTable === 'gwas'}
                <td class="font-mono bold text-accent">{r.rsid}</td>
                <td class="bold">{r.trait_name}</td>
                <td class="font-mono">{r.p_value.toExponential(3)}</td>
                <td class="font-mono text-muted">{r.or_or_beta || 'N/A'}</td>
                <td class="journal-cell">
                  <span class="bold">{r.journal || 'N/A'}</span>
                  {#if r.pubmed_id}
                    <a href={`https://pubmed.ncbi.nlm.nih.gov/${r.pubmed_id}`} target="_blank" rel="noopener noreferrer" class="pmid-link font-mono">PMID:{r.pubmed_id}</a>
                  {/if}
                </td>
                <td class="title-cell text-muted">{r.study_title || 'N/A'}</td>
              {:else if selectedTable === 'mane'}
                <td class="bold text-accent">{r.gene_symbol}</td>
                <td class="font-mono">{r.ensembl_transcript || 'N/A'}</td>
                <td class="font-mono">{r.refseq_transcript || 'N/A'}</td>
                <td>
                  <span class="status-pill select-status">{r.mane_status}</span>
                </td>
                <td class="font-mono text-muted">{r.grch38_coordinates || 'N/A'}</td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination Controls -->
    <div class="pagination-bar">
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={page === 0}
        onclick={goPrev}
      >
        ◀ Previous
      </button>
      <span class="page-indicator">
        Page <strong>{page + 1}</strong> of <strong>{pageCount}</strong>
      </span>
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={page >= pageCount - 1}
        onclick={goNext}
      >
        Next ▶
      </button>
    </div>
  {/if}
</div>

