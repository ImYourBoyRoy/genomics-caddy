<!-- ./src/lib/components/genome/GenomeMap.svelte -->
<script lang="ts">
  import { getChromosomeCounts, queryRsids, getVectorPromotedFindings, getChromosomeTraitOverlay } from "../../api/tauri";
  import type { ChromosomeTraitBand } from "../../types/research";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import { CHR_LENGTHS, CHR_ORDER } from "../../constants/chromosomeLayout";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";

  /*
  Module Docstring:
  Purpose: Live database-backed chromosome visualization.
  Responsibilities:
  - Display all 24 chromosomes with relative SNP density heatbars.
  - Query the local database for SNP density and position coordinates.
  - Map moderate and high-risk variants as pins on their respective chromosomes.
  Key Inputs: selectedSample, generatedReport.
  Key Outputs: Interactive, fully realized chromosome heat map.
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

  {#if isLoading}
    <PanelLoadingState
      message="Mapping chromosome density from your local database…"
      submessage="Counting SNPs per chromosome and placing report variant pins."
      accent="#34d399"
      compact
    />
  {:else if error}
    <div class="map-error">Failed to query database: {error}</div>
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
            <div class="chr-capsule-wrapper">
              <div class="chr-capsule">
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
