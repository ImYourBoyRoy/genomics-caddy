<!-- ./src/lib/components/search/LocalReferenceBrowser.svelte -->
<script lang="ts">
  import { queryLocalReferenceDb } from "../../api/tauri";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";

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
    loadData(true);
  }

  // Load initial data on mount
  $effect(() => {
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
        <label for="db-search-input">Search Keywords (e.g. rsID, Gene, Drug, Condition)</label>
        <div class="input-with-button">
          <input
            id="db-search-input"
            type="text"
            placeholder={
              selectedTable === 'clinvar' ? "rs4680, Breast Cancer, Pathogenic" :
              selectedTable === 'pharmgkb' ? "rs1801133, MTHFR, Warfarin" :
              selectedTable === 'clingen' ? "BRCA1, Definite, Arrhythmogenic" :
              selectedTable === 'gwas' ? "rs1042778, Obesity, Heart rate" :
              "APOE, Select, chr19"
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
        accent="#38bdf8"
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
          This table might be empty. Go to the <strong>Research Agent</strong> tab to download reference databases under <strong>Offline reference data</strong>.
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
          {#each rows as r}
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
        onclick={() => { page--; void loadData(); }}
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
        onclick={() => { page++; void loadData(); }}
      >
        Next ▶
      </button>
    </div>
  {/if}
</div>

<style>
  .local-db-browser {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 0;
  }

  .browser-header-summary {
    background: rgba(255, 255, 255, 0.03);
    border-left: 3px solid var(--accent, #38bdf8);
    padding: 12px 16px;
    border-radius: 4px;
  }

  .summary-text {
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--text-secondary, #94a3b8);
    margin: 0;
  }

  .search-controls-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: flex-end;
  }

  .select-group {
    width: 280px;
  }

  .query-form {
    flex: 1;
    min-width: 320px;
  }

  .query-input-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-with-button {
    display: flex;
    gap: 8px;
    width: 100%;
  }

  .input-with-button input {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.95rem;
    outline: none;
    transition: border-color 0.2s;
  }

  .input-with-button input:focus {
    border-color: var(--accent, #38bdf8);
  }

  .results-metadata-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.9rem;
    color: var(--text-secondary, #94a3b8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    padding-bottom: 8px;
  }

  .limit-selector {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .limit-selector select {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .table-scroll-container {
    width: 100%;
    overflow-x: auto;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    background: rgba(15, 23, 42, 0.2);
  }

  .db-results-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 0.9rem;
  }

  .db-results-table th {
    background: rgba(15, 23, 42, 0.4);
    padding: 12px 16px;
    font-weight: 600;
    color: var(--text-secondary, #94a3b8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    white-space: nowrap;
  }

  .db-results-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    vertical-align: top;
  }

  .db-results-table tr:hover td {
    background: rgba(255, 255, 255, 0.02);
  }

  .text-accent {
    color: var(--accent-light, #38bdf8);
  }

  .text-secondary {
    color: #f472b6;
  }

  .text-muted {
    color: #64748b;
    font-size: 0.85rem;
  }

  .bold {
    font-weight: 600;
  }

  .bold.text-accent {
    font-weight: 700;
  }

  /* Badge colors */
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 500;
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }

  .badge.pathogenic {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .badge.benign {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .level-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 600;
    background: rgba(255, 255, 255, 0.1);
  }

  .level-badge.level-1 {
    background: rgba(234, 179, 8, 0.15);
    color: #facc15;
    border: 1px solid rgba(234, 179, 8, 0.2);
  }

  .level-badge.level-2 {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .validity-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
    font-weight: 600;
    background: rgba(148, 163, 184, 0.1);
    color: #cbd5e1;
  }

  .validity-badge.definite {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.2);
  }

  .validity-badge.strong {
    background: rgba(34, 197, 94, 0.15);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .status-pill {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.8rem;
    background: rgba(255, 255, 255, 0.1);
  }

  .status-pill.select-status {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }

  .link-btn {
    display: inline-block;
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    color: #38bdf8;
    text-decoration: none;
    font-size: 0.8rem;
    transition: background-color 0.2s;
  }

  .link-btn:hover {
    background: rgba(56, 189, 248, 0.1);
  }

  .pmid-link {
    display: block;
    margin-top: 4px;
    color: #38bdf8;
    text-decoration: none;
    font-size: 0.8rem;
  }

  .pmid-link:hover {
    text-decoration: underline;
  }

  /* Specific cell restrictions */
  .desc-cell, .phenotype-cell {
    min-width: 200px;
    max-width: 350px;
    word-break: break-word;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .title-cell {
    min-width: 250px;
    max-width: 400px;
    word-break: break-word;
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .journal-cell {
    min-width: 150px;
  }

  .pagination-bar {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
    margin-top: 8px;
  }

  .page-indicator {
    font-size: 0.9rem;
    color: var(--text-secondary, #94a3b8);
  }

  .loading-container {
    padding: 32px 0;
  }

  .alert-box {
    padding: 12px 16px;
    border-radius: 6px;
    font-size: 0.9rem;
  }

  .error-alert {
    background: rgba(239, 68, 68, 0.1);
    border-left: 3px solid #ef4444;
    color: #f87171;
  }

  .empty-results-box {
    text-align: center;
    padding: 40px 16px;
    background: rgba(255, 255, 255, 0.01);
    border: 1px dashed rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: var(--text-secondary, #94a3b8);
  }

  .empty-results-box h4 {
    margin: 0 0 8px 0;
    color: white;
  }

  .empty-results-box p {
    margin: 0;
    font-size: 0.9rem;
    max-width: 450px;
    margin-left: auto;
    margin-right: auto;
    line-height: 1.5;
  }
</style>
