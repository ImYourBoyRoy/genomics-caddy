<!-- ./src/lib/components/genome/GenomeMap.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getChromosomeCounts, queryRsids } from "../../api/tauri";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";

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
  }

  let { selectedSample, generatedReport }: Props = $props();

  const CHR_LENGTHS: Record<string, number> = {
    "1": 248956422, "2": 242193529, "3": 198295559, "4": 190214555, "5": 181538259,
    "6": 170805979, "7": 159345973, "8": 145138636, "9": 138394717, "10": 133797422,
    "11": 135086622, "12": 133275309, "13": 114364328, "14": 107043718, "15": 101991189,
    "16": 90338345, "17": 83257073, "18": 80373285, "19": 58617616, "20": 64444167,
    "21": 46709983, "22": 50818468, "X": 156040895, "Y": 57227415
  };

  const CHR_ORDER = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
    "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
    "21", "22", "X", "Y"
  ];

  let chromosomeCounts = $state<Record<string, number>>({});
  interface RiskPin {
    rsid: string;
    gene: string;
    severity: string;
    chr: string;
    pos: number;
  }
  let riskPins = $state<RiskPin[]>([]);
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
      // 1. Fetch counts
      const counts = await getChromosomeCounts(selectedSample.id);
      chromosomeCounts = counts;

      // 2. Fetch risk positions
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

  function showTooltip(pin: RiskPin, e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const mapContainer = document.querySelector('.map-container');
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

  {#if isLoading}
    <div class="map-loading">Querying local genetic database...</div>
  {:else if error}
    <div class="map-error">Failed to query database: {error}</div>
  {:else}
    <div class="chromosome-list">
      {#each CHR_ORDER as chr}
        {@const count = chromosomeCounts[chr] || 0}
        {@const pct = (count / maxCount) * 100}
        {@const chrPins = riskPins.filter(p => p.chr === chr)}
        <div class="chr-row">
          <div class="chr-label font-mono">Chr {chr}</div>
          
          <div class="chr-capsule-wrapper">
            <div class="chr-capsule">
              <!-- Density bar fill -->
              <div class="chr-density-fill" style="width: {pct}%"></div>
              
              <!-- Individual Variant Pins -->
              {#each chrPins as pin}
                {@const pinPosPct = (pin.pos / CHR_LENGTHS[chr]) * 100}
                <button
                  class="variant-pin {pin.severity}"
                  style="left: {pinPosPct}%"
                  aria-label="Variant {pin.gene} at {pin.pos}"
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
        </div>
      {/each}
    </div>
  {/if}

  <!-- Interactive Floating Tooltip -->
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

<style>
  .map-container {
    position: relative;
    padding: 24px;
  }
  .map-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  .privacy-pill {
    font-size: 0.72rem;
    padding: 4px 10px;
    border-radius: 99px;
    background: rgba(16, 185, 129, 0.1);
    color: #10b981;
    border: 1px solid rgba(16, 185, 129, 0.2);
    font-weight: 500;
  }
  .map-intro {
    font-size: 0.88rem;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 24px;
    max-width: 800px;
  }
  .map-loading {
    padding: 40px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 0.95rem;
  }
  .map-error {
    padding: 16px;
    border-radius: 8px;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.15);
    color: #ef4444;
    font-size: 0.9rem;
    margin-bottom: 16px;
  }
  .chromosome-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .chr-row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .chr-label {
    width: 60px;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .chr-capsule-wrapper {
    flex-grow: 1;
    position: relative;
    height: 18px;
  }
  .chr-capsule {
    width: 100%;
    height: 100%;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 9px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    position: relative;
    overflow: hidden;
  }
  .chr-density-fill {
    height: 100%;
    background: linear-gradient(90deg, rgba(59, 130, 246, 0.05) 0%, rgba(59, 130, 246, 0.25) 100%);
    border-radius: 9px 0 0 9px;
    transition: width 0.4s ease;
  }
  .variant-pin {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid #1a1a24;
    padding: 0;
    cursor: pointer;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.5);
    transition: transform 0.15s ease, box-shadow 0.15s ease;
    z-index: 10;
  }
  .variant-pin:hover {
    transform: translate(-50%, -50%) scale(1.6);
    z-index: 20;
    box-shadow: 0 0 8px currentColor;
  }
  .variant-pin.high_risk {
    background-color: #ef4444;
    color: #ef4444;
  }
  .variant-pin.moderate_risk {
    background-color: #f59e0b;
    color: #f59e0b;
  }
  .variant-pin.confirmation_required {
    background-color: #ec4899;
    color: #ec4899;
  }
  .chr-stats {
    width: 180px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.8rem;
  }
  .risk-count-pill {
    font-size: 0.75rem;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(245, 158, 11, 0.1);
    color: #f59e0b;
    border: 1px solid rgba(245, 158, 11, 0.15);
    font-weight: 500;
  }

  /* Tooltip styles */
  .map-tooltip {
    position: absolute;
    transform: translate(-50%, -100%);
    background: #1e1e2d;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 6px;
    padding: 8px 12px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    z-index: 100;
    pointer-events: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
    animation: fadeIn 0.12s ease-out;
  }
  .tooltip-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: #ffffff;
  }
  .tooltip-sub {
    font-size: 0.72rem;
    color: var(--text-secondary);
  }
  .tooltip-badge {
    font-size: 0.65rem;
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: capitalize;
    font-weight: 600;
    width: fit-content;
    margin-top: 4px;
  }
  .tooltip-badge.high_risk {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
  }
  .tooltip-badge.moderate_risk {
    background: rgba(245, 158, 11, 0.15);
    color: #f59e0b;
  }
  .tooltip-badge.confirmation_required {
    background: rgba(236, 48, 153, 0.15);
    color: #ec4899;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translate(-50%, -90%); }
    to { opacity: 1; transform: translate(-50%, -100%); }
  }
</style>
