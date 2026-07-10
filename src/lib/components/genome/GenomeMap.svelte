<!-- ./src/lib/components/genome/GenomeMap.svelte -->
<script lang="ts">
  import { getChromosomeCounts, queryRsids, getVectorPromotedFindings, getChromosomeTraitOverlay, getSamples } from "../../api/tauri";
  import type { ChromosomeTraitBand } from "../../types/research";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import { CHR_LENGTHS, CHR_ORDER } from "../../constants/chromosomeLayout";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";

  /*
  Module Docstring:
  Purpose: Live database-backed chromosome visualization with interactive zoom, overlays, and comparison views.
  Responsibilities:
  - Display all 24 chromosomes with relative SNP density heatbars.
  - Query the local database for SNP density and position coordinates.
  - Map moderate and high-risk variants as pins on their respective chromosomes.
  - Support horizontal zooming and panning across chromosomes.
  - Display gene labels directly above pins when zoomed.
  - Compare risk variant genotypes across all imported samples in a matrix view.
  Key Inputs: selectedSample, generatedReport.
  Key Outputs: Interactive, fully realized chromosome heat map and comparison grid.
  Operational Notes: Normalizes variant density based on the highest count chromosome.
  */

  interface Props {
    selectedSample: GenomeSample | null;
    generatedReport: GeneratedReport | null;
    focusRsid?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let { selectedSample, generatedReport, focusRsid = "", onNavigateToVariant }: Props = $props();

  let chromosomeCounts = $state<Record<string, number>>({});
  interface RiskPin {
    rsid: string;
    gene: string;
    severity: string;
    chr: string;
    pos: number;
    promoted?: boolean;
  }
  let riskPins = $state<RiskPin[]>([]);
  let traitBands = $state<ChromosomeTraitBand[]>([]);
  let isLoading = $state(false);
  let error = $state("");

  // Zooming
  let zoomScale = $state(1);

  // Multi-sample comparison
  let allSamples = $state<GenomeSample[]>([]);
  interface ComparisonRow {
    rsid: string;
    gene: string;
    severity: string;
    genotypes: Record<number, string>;
  }
  let comparisonData = $state<ComparisonRow[]>([]);
  let isComparing = $state(false);

  interface HoveredPin {
    rsid: string;
    gene: string;
    severity: string;
    x: number;
    y: number;
  }
  let hoveredPin = $state<HoveredPin | null>(null);

  let maxCount = $derived(
    Object.values(chromosomeCounts).reduce((max, val) => Math.max(max, val), 1)
  );

  async function loadComparison() {
    if (riskPins.length === 0) return;
    try {
      allSamples = await getSamples();
      if (allSamples.length <= 1) return;

      const rsids = riskPins.map(p => p.rsid);
      const tempRows: ComparisonRow[] = riskPins.map(p => ({
        rsid: p.rsid,
        gene: p.gene,
        severity: p.severity,
        genotypes: {}
      }));

      for (const sample of allSamples) {
        const records = await queryRsids(sample.id, rsids);
        for (const rec of records) {
          const row = tempRows.find(r => r.rsid.toLowerCase() === rec.rsid.toLowerCase());
          if (row) {
            const clean_allele1 = rec.allele1 || "-";
            const clean_allele2 = rec.allele2 || "-";
            row.genotypes[sample.id] = `${clean_allele1}${clean_allele2}`;
          }
        }
      }
      comparisonData = tempRows;
      isComparing = true;
    } catch (e) {
      console.error("Failed to load sample comparisons:", e);
    }
  }

  function closeComparison() {
    isComparing = false;
  }

  async function loadData() {
    if (!selectedSample) return;
    isLoading = true;
    error = "";
    try {
      const counts = await getChromosomeCounts(selectedSample.id);
      chromosomeCounts = counts;
      try {
        traitBands = await getChromosomeTraitOverlay(selectedSample.id);
      } catch {
        traitBands = [];
      }

      if (generatedReport) {
        const riskMarkers = [];
        for (const sec of generatedReport.sections) {
          for (const m of sec.markers) {
            if (
              m.severity_class === "high_risk" ||
              m.severity_class === "moderate_risk" ||
              m.severity_class === "confirmation_required"
            ) {
              riskMarkers.push(m);
            }
          }
        }

        if (riskMarkers.length > 0) {
          const rsids = riskMarkers.map(m => m.rsid);
          const records = await queryRsids(selectedSample.id, rsids);

          const pins: RiskPin[] = [];
          for (const rec of records) {
            if (rec.position_grch38) {
              const marker = riskMarkers.find(m => m.rsid === rec.rsid);
              if (marker) {
                pins.push({
                  rsid: rec.rsid,
                  gene: marker.gene,
                  severity: marker.severity_class,
                  chr: rec.chromosome,
                  pos: rec.position_grch38
                });
              }
            }
          }
          riskPins = pins;
        } else {
          riskPins = [];
        }
      }

      const promoted = await getVectorPromotedFindings(selectedSample.id);
      if (promoted.length > 0) {
        const existing = new Set(riskPins.map((p) => p.rsid.toLowerCase()));
        const promotedRsids = promoted
          .map((p) => p.rsid)
          .filter((r) => !existing.has(r.toLowerCase()));
        if (promotedRsids.length > 0) {
          const records = await queryRsids(selectedSample.id, promotedRsids);
          for (const rec of records) {
            if (!rec.position_grch38) continue;
            const meta = promoted.find((p) => p.rsid.toLowerCase() === rec.rsid.toLowerCase());
            riskPins = [
              ...riskPins,
              {
                rsid: rec.rsid,
                gene: meta?.gene || "Vector",
                severity: "vector_promoted",
                chr: rec.chromosome,
                pos: rec.position_grch38,
                promoted: true,
              },
            ];
          }
        }
      }
    } catch (e: any) {
      error = e.toString();
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    if (selectedSample) {
      loadData();
    }
  });

  $effect(() => {
    if (!focusRsid || riskPins.length === 0) return;
    const pin = riskPins.find((p) => p.rsid.toLowerCase() === focusRsid.toLowerCase());
    if (!pin) return;
    const el = document.querySelector(`[data-rsid="${pin.rsid.toLowerCase()}"]`);
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
  });

  function showTooltip(pin: RiskPin, e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const mapContainer = document.querySelector(".map-container");
    if (mapContainer) {
      const parentRect = mapContainer.getBoundingClientRect();
      hoveredPin = {
        rsid: pin.rsid,
        gene: pin.gene,
        severity: pin.severity,
        x: rect.left - parentRect.left + rect.width / 2,
        y: rect.top - parentRect.top - 45
      };
    }
  }

  function hideTooltip() {
    hoveredPin = null;
  }
</script>

<div class="card map-container">
  <div class="map-header">
    <h3>🧬 Chromosome Density & Variant Map</h3>
    <span class="privacy-pill">Local Database Live Query</span>
  </div>

  <p class="map-intro">
    This heat map visualizes your imported genetic data. The background fill of each chromosome represents the relative density of parsed SNPs. The highlighted points show the exact positions of risk variants identified in your report.
  </p>

  <div class="map-legend" aria-label="Map legend">
    <span class="legend-title">Pin colors</span>
    <span class="legend-item"><span class="legend-dot high_risk"></span> High risk (curated report)</span>
    <span class="legend-item"><span class="legend-dot moderate_risk"></span> Moderate risk</span>
    <span class="legend-item"><span class="legend-dot confirmation_required"></span> Needs confirmation</span>
    <span class="legend-item"><span class="legend-dot vector_promoted"></span> Vector research discovery</span>
    <span class="legend-item legend-note">⚠️ count = report + vector variants on that chromosome (hover a pin for gene &amp; rsID)</span>
  </div>

  <!-- Interactive Controls (Zoom + Pan + Multi-Sample Comparison) -->
  <div class="map-controls-row" style="display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-bottom: 1rem; padding: 0.75rem; background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 8px;">
    <div style="display: flex; align-items: center; gap: 0.75rem;">
      <span style="font-size: 0.78rem; font-weight: 600; color: var(--text-secondary);">🔍 Zoom Scale: {zoomScale}x</span>
      <input
        type="range"
        min="1"
        max="8"
        step="0.5"
        bind:value={zoomScale}
        style="width: 130px; cursor: col-resize; accent-color: #34d399;"
      />
      {#if zoomScale > 1}
        <button
          class="btn btn-secondary btn-xs"
          onclick={() => zoomScale = 1}
          style="padding: 3px 6px; font-size: 0.65rem;"
        >
          Reset
        </button>
      {/if}
    </div>
    
    <button
      class="btn btn-secondary btn-xs"
      onclick={loadComparison}
      disabled={isLoading}
      style="display: flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; padding: 5px 10px;"
    >
      👥 Compare Genotypes
    </button>
  </div>

  {#if isComparing}
    <div class="comparison-overlay" style="margin-bottom: 1.5rem; padding: 1rem; background: rgba(0,0,0,0.5); border: 1px solid var(--border-color); border-radius: 8px; animation: fadeIn 0.2s ease;">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem;">
        <h4 style="margin: 0; color: #a78bfa; font-size: 0.85rem;">👥 Multi-Sample Variant Comparison</h4>
        <button class="btn btn-secondary btn-xs" onclick={closeComparison} style="padding: 2px 6px; font-size: 0.65rem;">Close</button>
      </div>
      {#if allSamples.length <= 1}
        <p style="font-size: 0.72rem; opacity: 0.8; margin: 0;">Import multiple samples to compare genotypes across family members.</p>
      {:else}
        <div style="overflow-x: auto;">
          <table style="width: 100%; border-collapse: collapse; font-size: 0.72rem; text-align: left;">
            <thead>
              <tr style="border-bottom: 1px solid rgba(255,255,255,0.15);">
                <th style="padding: 6px;">Variant / rsID</th>
                <th style="padding: 6px;">Gene</th>
                {#each allSamples as sample}
                  <th style="padding: 6px; text-align: center;" class:active-sample={selectedSample && sample.id === selectedSample.id}>
                    {sample.name} {selectedSample && sample.id === selectedSample.id ? '👤' : ''}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each comparisonData as row}
                <tr style="border-bottom: 1px solid rgba(255,255,255,0.06);" class="comparison-tr">
                  <td style="padding: 6px;" class="font-mono">{row.rsid}</td>
                  <td style="padding: 6px; font-weight: 600;">{row.gene}</td>
                  {#each allSamples as sample}
                    {@const gt = row.genotypes[sample.id] || '--'}
                    <td style="padding: 6px; text-align: center; font-family: var(--font-mono), monospace;" class="genotype-cell {row.severity}">
                      {gt}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}

  {#if isLoading}
    <PanelLoadingState
      message="Mapping chromosome density from your local database…"
      submessage="Counting SNPs per chromosome and placing report variant pins."
      accent="#34d399"
      compact
    />
  {:else if error}
    <div class="map-error">Failed to query database: {error}</div>
    <button class="btn btn-secondary btn-xs" onclick={loadData}>Retry</button>
  {:else}
    <div class="chromosome-list">
      {#each CHR_ORDER as chr}
        {@const count = chromosomeCounts[chr] || 0}
        {@const pct = (count / maxCount) * 100}
        {@const chrPins = riskPins.filter(p => p.chr === chr)}
        {@const chrTraits = traitBands.filter((b) => b.chromosome === chr).slice(0, 4)}
        <div class="chr-row">
          <div class="chr-label font-mono">Chr {chr}</div>

          <div class="chr-main">
            <div class="chr-capsule-wrapper" style="overflow-x: auto; position: relative; width: 100%; border-radius: 999px;">
              <div class="chr-capsule" style="width: {100 * zoomScale}%; min-width: 100%; position: relative; height: 18px; border-radius: 999px;">
                <div class="chr-density-fill" style="width: {pct}%"></div>

                {#each chrPins as pin}
                  {@const pinPosPct = (pin.pos / CHR_LENGTHS[chr]) * 100}
                  <button
                    class="variant-pin {pin.severity}"
                    class:pin-focused={focusRsid && pin.rsid.toLowerCase() === focusRsid.toLowerCase()}
                    style="left: {pinPosPct}%"
                    data-rsid={pin.rsid.toLowerCase()}
                    aria-label="Variant {pin.gene} at {pin.pos}"
                    onclick={() => onNavigateToVariant?.(pin.rsid, "report")}
                    onmouseenter={(e) => showTooltip(pin, e)}
                    onmouseleave={hideTooltip}
                  ></button>

                  <!-- Gene Overlay label on zoom -->
                  {#if zoomScale > 1.5}
                    <span
                      class="gene-overlay-label font-mono"
                      style="left: {pinPosPct}%; position: absolute; top: -14px; transform: translateX(-50%) rotate(-15deg); font-size: 0.58rem; font-weight: 700; color: #818cf8; background: rgba(0,0,0,0.85); border: 1px solid rgba(129,140,248,0.4); padding: 1px 4px; border-radius: 3px; pointer-events: none; z-index: 10; white-space: nowrap;"
                    >
                      {pin.gene}
                    </span>
                  {/if}
                {/each}
              </div>
            </div>

            <div class="chr-stats text-secondary font-mono">
              <span class="total-snps-val">{count.toLocaleString()} SNPs</span>
              {#if chrPins.length > 0}
                <span class="risk-count-pill">
                  ⚠️ {chrPins.length} {chrPins.length === 1 ? 'variant' : 'variants'}
                </span>
              {/if}
            </div>
            {#if chrTraits.length > 0}
              <div class="trait-overlay">
                {#each chrTraits as band}
                  <span class="trait-band" title="{band.trait_category}: {band.association_count} associations">
                    {band.trait_category.replace(/_/g, " ")} ({band.association_count})
                  </span>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if hoveredPin}
    <div class="map-tooltip" style="left: {hoveredPin.x}px; top: {hoveredPin.y}px;">
      <span class="tooltip-title">{hoveredPin.gene}</span>
      <span class="tooltip-sub font-mono">{hoveredPin.rsid}</span>
      <span class="tooltip-badge {hoveredPin.severity}">
        {hoveredPin.severity.replace('_', ' ')}
      </span>
    </div>
  {/if}
</div>

<style src="../../styles/components/genome-map.css"></style>
