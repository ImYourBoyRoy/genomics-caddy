<!-- ./src/lib/components/agent/AutonomousScanSection.svelte -->
<!--
Module Docstring:
Purpose: Autonomous scanning controls and save path configurations for the Genomics Research Agent.
Responsibilities:
- Configure Curated Catalog and Deep Gene Region scans.
- Browse and configure a custom incremental findings JSON save path.
- Execute batched, real-time variant inquiries (size 5).
- Renders progress bars and delegates list selection to DiscoveredVariantsList.
Key Inputs: Selected sample, NCBI API key.
Key Outputs: Discovered VariantEvidence arrays.
Operational Notes: Stays well under the 500-line limit. Scoped styles. Svelte 5 runes.
-->
<script lang="ts">
  import { getVariantEvidence, queryRegion, fetchExternalApi, selectSavePath } from "../../api/tauri";
  import { asApiJson } from "../../types/api";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import type { GenomeSample } from "../../types/genomics";
  import type { VariantEvidence } from "../../types/agent";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import {
    buildCatalogCategories,
    defaultCatalogCategorySelection,
    selectedCatalogCategoryIds
  } from "../../utils/catalogCategoryRouting";
  import catalogData from "../../marker-packs/discovery_catalog.json";
  import DiscoveredVariantsList from "./DiscoveredVariantsList.svelte";

  type DiscoveredVariant = VariantEvidence & { checked?: boolean };

  interface Props {
    selectedSample: GenomeSample | null;
    ncbiApiKey: string;
    discoveredVariants: DiscoveredVariant[];
    isScanningDb: boolean;
    scanProgressMsg: string;
    currentProgress: number;
    totalToScan: number;
  }

  let {
    selectedSample,
    ncbiApiKey,
    discoveredVariants = $bindable([]),
    isScanningDb = $bindable(false),
    scanProgressMsg = $bindable(""),
    currentProgress = $bindable(0),
    totalToScan = $bindable(0)
  }: Props = $props();

  let discoveryMode = $state<"catalog" | "gene_region">("catalog");
  let geneSymbolInput = $state("MTHFR");
  let customExportPath = $state("");

  const categories = buildCatalogCategories(catalogData.markers);
  let selectedCategories = $state<Record<string, boolean>>(
    defaultCatalogCategorySelection(categories)
  );

  async function selectExportFile() {
    try {
      const defaultFilename = selectedSample 
        ? `${selectedSample.name.replace(/[^a-zA-Z0-9]/g, "_")}_findings_stack.json`
        : "dna_findings_stack.json";
      const path = await selectSavePath(defaultFilename);
      if (path) {
        customExportPath = path;
      }
    } catch (e: any) {
      dialogStore.alert("Failed to select path: " + e.message);
    }
  }

  // Mode 1: Curated Catalog Scan (batched)
  async function scanCatalogMarkers() {
    if (!selectedSample) return;
    isScanningDb = true;
    scanProgressMsg = "Loading active categories...";
    discoveredVariants = [];
    currentProgress = 0;
    totalToScan = 0;

    try {
      const activeCatalogCategories = selectedCatalogCategoryIds(categories, selectedCategories);
      const catalogMarkers = catalogData.markers.filter(m => activeCatalogCategories.has(m.category));
      const rsids = catalogMarkers.map(m => m.rsid);
      totalToScan = rsids.length;

      if (totalToScan === 0) {
        isScanningDb = false;
        return;
      }

      const results: VariantEvidence[] = [];
      const batchSize = 5;

      for (let i = 0; i < rsids.length; i += batchSize) {
        const batch = rsids.slice(i, i + batchSize);
        scanProgressMsg = `Evaluating catalog batch ${Math.floor(i / batchSize) + 1} of ${Math.ceil(rsids.length / batchSize)} (${batch.join(", ")})...`;

        const promises = batch.map(rsid =>
          getVariantEvidence(selectedSample.id, rsid, ncbiApiKey || undefined, customExportPath || undefined)
            .catch(err => {
              console.error(`Error querying ${rsid}`, err);
              return null;
            })
        );

        const batchResults = await Promise.all(promises);
        for (const res of batchResults) {
          if (res) {
            const castRes = { ...res } as any;
            castRes.checked = res.interpretation_status === "active_clinical" || 
                              res.interpretation_status === "active_research";
            results.push(castRes);
          }
        }
        discoveredVariants = [...results];
        currentProgress = Math.min(i + batchSize, totalToScan);
      }
    } catch (e: any) {
      console.error("Catalog scan failed", e);
    } finally {
      isScanningDb = false;
    }
  }

  // Mode 2: Deep Gene Region Scan (batched)
  async function scanGeneRegion() {
    if (!selectedSample) {
      dialogStore.alert("Please import and select a genome sample first.");
      return;
    }
    const gene = geneSymbolInput.trim().toUpperCase();
    if (!gene) {
      dialogStore.alert("Please enter a valid Gene symbol.");
      return;
    }

    isScanningDb = true;
    discoveredVariants = [];
    scanProgressMsg = `Resolving coordinates for gene ${gene} via Ensembl...`;
    currentProgress = 0;
    totalToScan = 0;

    try {
      const lookupUrl = `https://rest.ensembl.org/lookup/symbol/homo_sapiens/${gene}?content-type=application/json`;
      const geneLookup = asApiJson(await fetchExternalApi(lookupUrl, undefined, 86400).catch(() => null));

      if (!geneLookup || typeof geneLookup.seq_region_name !== "string") {
        dialogStore.alert(`Could not resolve coordinates for gene symbol "${gene}" via Ensembl.`);
        isScanningDb = false;
        return;
      }

      const chrom = geneLookup.seq_region_name;
      const start = Number(geneLookup.start);
      const end = Number(geneLookup.end);
      scanProgressMsg = `Gene ${gene} resolved to Chr ${chrom}:${start}-${end}. Querying user genotypes...`;

      const localSnps = await queryRegion(selectedSample.id, chrom, start, end).catch(() => []);
      if (localSnps.length === 0) {
        dialogStore.alert(`No imported SNPs found in the ${gene} gene region (Chr ${chrom}:${start}-${end}) in your DNA file.`);
        isScanningDb = false;
        return;
      }

      const snpsToQuery = localSnps.filter(s => s.rsid && s.rsid.startsWith("rs")).slice(0, 100);
      const rsids = snpsToQuery.map(s => s.rsid);
      totalToScan = rsids.length;

      const results: VariantEvidence[] = [];
      const batchSize = 5;

      for (let i = 0; i < rsids.length; i += batchSize) {
        const batch = rsids.slice(i, i + batchSize);
        scanProgressMsg = `Evaluating region batch ${Math.floor(i / batchSize) + 1} of ${Math.ceil(rsids.length / batchSize)} (${batch.join(", ")})...`;

        const promises = batch.map(rsid =>
          getVariantEvidence(selectedSample.id, rsid, ncbiApiKey || undefined, customExportPath || undefined)
            .catch(err => {
              console.error(`Error querying ${rsid}`, err);
              return null;
            })
        );

        const batchResults = await Promise.all(promises);
        for (const res of batchResults) {
          if (res) {
            const castRes = { ...res } as any;
            castRes.checked = res.interpretation_status === "active_clinical" || 
                              res.interpretation_status === "active_research";
            results.push(castRes);
          }
        }
        discoveredVariants = [...results];
        currentProgress = Math.min(i + batchSize, totalToScan);
      }
    } catch (err: any) {
      console.error("Deep region scan failed", err);
      dialogStore.alert("Failed region scan: " + (err.message || err));
    } finally {
      isScanningDb = false;
    }
  }

  // Reactive scan trigger removed — use explicit Run Catalog Scan button.

  function toggleCategory(catId: string) {
    selectedCategories[catId] = !selectedCategories[catId];
  }

  function setAllCategories(selected: boolean) {
    for (const category of categories) {
      selectedCategories[category.id] = selected;
    }
  }
</script>

<!-- Discovery Mode Tabs -->
<div class="discovery-tabs">
  <button
    type="button"
    class="tab-btn"
    class:active={discoveryMode === "catalog"}
    onclick={() => discoveryMode = "catalog"}
  >
    📁 Curated Catalog
  </button>
  <button
    type="button"
    class="tab-btn"
    class:active={discoveryMode === "gene_region"}
    onclick={() => {
      discoveryMode = "gene_region";
      discoveredVariants = [];
    }}
  >
    🧬 Deep Gene Region
  </button>
</div>

<!-- Custom Save Path Option -->
<div class="incremental-path-selector">
  <div class="input-row">
    <label for="incremental-export-path">Incremental Discovery Save Path (Optional)</label>
    <div class="input-with-button">
      <input
        id="incremental-export-path"
        type="text"
        placeholder="e.g. /path/to/discoveries.json (or blank for default)"
        bind:value={customExportPath}
      />
      <button
        type="button"
        class="btn btn-secondary"
        onclick={selectExportFile}
      >
        📁 Browse
      </button>
    </div>
    <p class="input-hint">
      💾 Active discoveries will be appended directly to this file in real-time batches of 5.
    </p>
  </div>
</div>

{#if discoveryMode === "catalog"}
  <!-- Category Filter Checklist -->
  <div class="category-filters-container">
    <span class="label-header">Filter Catalog Categories</span>
    <p class="input-hint">
      {categories.length} catalog domains · {categories.reduce((total, category) => total + category.count, 0)} research entries. Select all or focus the scan to a smaller area.
    </p>
    <div class="categories-grid">
      {#each categories as cat}
        <button
          type="button"
          class="category-tag-btn"
          class:active={selectedCategories[cat.id]}
          onclick={() => toggleCategory(cat.id)}
        >
          <span class="status-dot">{selectedCategories[cat.id] ? "●" : "○"}</span>
          <span>{cat.label}</span>
          <span class="category-count">{cat.count}</span>
        </button>
      {/each}
    </div>
    <div class="catalog-selection-actions">
      <button type="button" class="btn btn-secondary" onclick={() => setAllCategories(true)} disabled={isScanningDb}>
        Select all
      </button>
      <button type="button" class="btn btn-secondary" onclick={() => setAllCategories(false)} disabled={isScanningDb}>
        Clear all
      </button>
    </div>
    <div class="catalog-actions">
      <button
        type="button"
        class="btn btn-primary"
        disabled={isScanningDb || !selectedSample}
        onclick={scanCatalogMarkers}
      >
        {isScanningDb ? "Scanning…" : "🔍 Run Catalog Scan"}
      </button>
      <p class="input-hint">
        Scans selected categories against your genome and persists active findings to the local database.
      </p>
    </div>
  </div>
{:else}
  <!-- Deep Gene Scan Inputs -->
  <div class="gene-scan-input-area">
    <div class="input-row">
      <label for="gene-region-symbol">Enter Gene Symbol</label>
      <div class="input-with-button">
        <input
          id="gene-region-symbol"
          type="text"
          bind:value={geneSymbolInput}
          placeholder="e.g., MTHFR, BRCA1, APOE, CYP2C19"
        />
        <button
          type="button"
          class="btn btn-secondary"
          disabled={isScanningDb}
          onclick={scanGeneRegion}
        >
          🔍 Scan
        </button>
      </div>
      <p class="input-hint">
        🧬 Resolves genomic region dynamically, queries SQLite, and fetches batch ClinVar annotations.
      </p>
    </div>
  </div>
{/if}

<!-- Progress Indicator -->
{#if isScanningDb && totalToScan > 0}
  <div class="scan-progress-bar-container">
    <div class="progress-bar-label font-mono">
      <ActivityPulse message={scanProgressMsg} accent="#60a5fa" maxWidth="100%" />
      <span class="progress-count">{currentProgress}/{totalToScan}</span>
    </div>
    <div class="progress-track">
      <div class="progress-fill" style="width: {(currentProgress / totalToScan) * 100}%"></div>
    </div>
  </div>
{/if}

<!-- Discovered Active Variants -->
<DiscoveredVariantsList
  {selectedSample}
  bind:discoveredVariants
  {isScanningDb}
  {scanProgressMsg}
/>

<style>
  /* Tabs style */
  .discovery-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 12px;
  }
  .tab-btn {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    padding: 8px 16px;
    font-size: 0.76rem;
    cursor: pointer;
    transition: all 0.2s;
  }
  .tab-btn:hover {
    color: var(--text-primary);
  }
  .tab-btn.active {
    border-bottom-color: var(--accent);
    color: var(--text-primary);
    font-weight: 600;
  }

  .input-with-button {
    display: flex;
    gap: 8px;
  }
  .input-with-button input {
    flex: 1;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.88rem;
    box-sizing: border-box;
  }
  .input-with-button input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .input-with-button button {
    flex-shrink: 0;
  }

  .incremental-path-selector {
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px dashed var(--border-color);
  }

  /* Autonomous Scan styles */
  .category-filters-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }
  .categories-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .catalog-actions {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .catalog-selection-actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }
  .catalog-selection-actions button {
    padding: 5px 9px;
    font-size: 0.68rem;
  }
  .category-tag-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    padding: 8px;
    font-size: 0.72rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
  }
  .category-tag-btn:hover {
    border-color: var(--accent);
    background: rgba(255, 255, 255, 0.04);
  }
  .category-tag-btn.active {
    background: rgba(88, 80, 236, 0.15);
    border-color: var(--accent);
    color: var(--text-primary);
  }
  .status-dot {
    flex: 0 0 auto;
    font-size: 0.85rem;
    transition: color 0.2s;
  }
  .category-count {
    margin-left: auto;
    color: var(--text-secondary);
    font-size: 0.64rem;
    font-variant-numeric: tabular-nums;
  }
  .category-tag-btn.active .status-dot {
    color: var(--success);
  }
  .category-tag-btn:not(.active) .status-dot {
    color: rgba(255, 255, 255, 0.2);
  }

  .gene-scan-input-area {
    margin-bottom: 12px;
  }
  .input-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .input-row label, .label-header {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-weight: 500;
  }
  .input-hint {
    font-size: 0.65rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.35;
  }

  /* Progress Bar */
  .scan-progress-bar-container {
    margin-bottom: 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    padding: 10px;
    border-radius: 6px;
  }
  .progress-bar-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 0.65rem;
    color: var(--accent);
    margin-bottom: 6px;
  }
  .progress-count {
    flex-shrink: 0;
    font-size: 0.65rem;
    color: var(--text-secondary);
  }
  .progress-track {
    background: rgba(0, 0, 0, 0.4);
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
  }
  .progress-fill {
    background: var(--accent);
    height: 100%;
    border-radius: 3px;
    transition: width 0.2s ease-out;
  }

  .font-mono { font-family: monospace; }
</style>
