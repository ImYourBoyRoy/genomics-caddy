<!-- ./src/lib/components/genome/GenomeMap.svelte -->
<script lang="ts">
  import { getChromosomeCounts, queryRsids, getVectorPromotedFindings, getChromosomeTraitOverlay, getSamples } from "../../api/tauri";
  import type { ChromosomeTraitBand } from "../../types/research";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import type { VariantNavTarget } from "../../constants/traitCategories";
  import { CHR_LENGTHS, CHR_ORDER } from "../../constants/chromosomeLayout";
  import PanelLoadingState from "../common/loading/PanelLoadingState.svelte";
  import Tooltip from "../common/Tooltip.svelte";
  import "$lib/styles/components/genome-map.css";

  /*
  Purpose: Live database-backed chromosome visualization with interactive zoom, overlays, and comparison views.
  Responsibilities:
  - Display chromosomes scaled to GRCh38 length with SNP-density intensity.
  - Place curated / vector variant pins at genomic coordinates.
  - Support zoom, gene labels, and multi-sample genotype comparison.
  Key Inputs: selectedSample, generatedReport, focusRsid.
  Key Outputs: Interactive karyotype map.
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
  let zoomScale = $state(1);

  let allSamples = $state<GenomeSample[]>([]);
  interface ComparisonRow {
    rsid: string;
    gene: string;
    severity: string;
    genotypes: Record<number, string>;
  }
  let comparisonData = $state<ComparisonRow[]>([]);
  let isComparing = $state(false);

  const maxChrLen = CHR_LENGTHS["1"];

  let maxCount = $derived(
    Object.values(chromosomeCounts).reduce((max, val) => Math.max(max, val), 1)
  );

  let totalSnps = $derived(
    Object.values(chromosomeCounts).reduce((sum, n) => sum + n, 0)
  );

  let pinSummary = $derived.by(() => {
    const counts = { high: 0, moderate: 0, confirm: 0, vector: 0 };
    for (const p of riskPins) {
      if (p.severity === "high_risk") counts.high += 1;
      else if (p.severity === "moderate_risk") counts.moderate += 1;
      else if (p.severity === "confirmation_required") counts.confirm += 1;
      else if (p.severity === "vector_promoted") counts.vector += 1;
    }
    return counts;
  });

  /** Confirm + stronger associations — what to review first */
  let attentionPins = $derived(
    riskPins
      .filter((p) => p.severity === "confirmation_required" || p.severity === "high_risk")
      .slice()
      .sort((a, b) => {
        const rank = (s: string) => (s === "confirmation_required" ? 0 : 1);
        return rank(a.severity) - rank(b.severity) || a.gene.localeCompare(b.gene);
      })
  );

  type PinFilter = "all" | "attention" | "high_risk" | "moderate_risk" | "confirmation_required" | "vector_promoted";
  let pinFilter = $state<PinFilter>("all");
  let activeChr = $state<string | null>(null);

  function severityLabel(severity: string): string {
    switch (severity) {
      case "high_risk":
        return "Stronger association";
      case "moderate_risk":
        return "Possible association";
      case "confirmation_required":
        return "Confirm clinically";
      case "vector_promoted":
        return "Vector discovery";
      default:
        return severity.replace(/_/g, " ");
    }
  }

  function pinGlyph(severity: string): string {
    switch (severity) {
      case "high_risk":
        return "!";
      case "moderate_risk":
        return "?";
      case "confirmation_required":
        return "C";
      case "vector_promoted":
        return "V";
      default:
        return "·";
    }
  }

  function pinMatchesFilter(pin: RiskPin): boolean {
    if (pinFilter === "all") return true;
    if (pinFilter === "attention") {
      return pin.severity === "confirmation_required" || pin.severity === "high_risk";
    }
    return pin.severity === pinFilter;
  }

  function pinsForChr(chr: string): RiskPin[] {
    return riskPins
      .filter((p) => p.chr === chr && pinMatchesFilter(p))
      .slice()
      .sort((a, b) => a.pos - b.pos);
  }

  function chrPinBreakdown(chrPins: RiskPin[]): string {
    const c = { confirm: 0, high: 0, moderate: 0, vector: 0 };
    for (const p of chrPins) {
      if (p.severity === "confirmation_required") c.confirm += 1;
      else if (p.severity === "high_risk") c.high += 1;
      else if (p.severity === "moderate_risk") c.moderate += 1;
      else if (p.severity === "vector_promoted") c.vector += 1;
    }
    const parts: string[] = [];
    if (c.confirm) parts.push(`${c.confirm} confirm`);
    if (c.high) parts.push(`${c.high} stronger`);
    if (c.moderate) parts.push(`${c.moderate} possible`);
    if (c.vector) parts.push(`${c.vector} vector`);
    return parts.join(" · ") || `${chrPins.length} pin${chrPins.length === 1 ? "" : "s"}`;
  }

  function focusPin(pin: RiskPin) {
    activeChr = pin.chr;
    const el = document.querySelector(`[data-rsid="${pin.rsid.toLowerCase()}"]`);
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
    if (el instanceof HTMLElement) el.focus();
    onNavigateToVariant?.(pin.rsid, "report");
  }

  function chrWidthPct(chr: string): number {
    const len = CHR_LENGTHS[chr] || maxChrLen;
    return Math.max(8, (len / maxChrLen) * 100);
  }

  function formatMb(bp: number): string {
    return `${(bp / 1_000_000).toFixed(0)} Mb`;
  }

  async function loadComparison() {
    if (riskPins.length === 0) return;
    try {
      allSamples = await getSamples();
      if (allSamples.length <= 1) {
        isComparing = true;
        return;
      }

      const rsids = riskPins.map((p) => p.rsid);
      const tempRows: ComparisonRow[] = riskPins.map((p) => ({
        rsid: p.rsid,
        gene: p.gene,
        severity: p.severity,
        genotypes: {},
      }));

      for (const sample of allSamples) {
        const records = await queryRsids(sample.id, rsids);
        for (const rec of records) {
          const row = tempRows.find((r) => r.rsid.toLowerCase() === rec.rsid.toLowerCase());
          if (row) {
            row.genotypes[sample.id] = `${rec.allele1 || "-"}${rec.allele2 || "-"}`;
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
      chromosomeCounts = await getChromosomeCounts(selectedSample.id);
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
          const rsids = riskMarkers.map((m) => m.rsid);
          const records = await queryRsids(selectedSample.id, rsids);
          const pins: RiskPin[] = [];
          for (const rec of records) {
            if (!rec.position_grch38) continue;
            const marker = riskMarkers.find((m) => m.rsid === rec.rsid);
            if (marker) {
              pins.push({
                rsid: rec.rsid,
                gene: marker.gene,
                severity: marker.severity_class,
                chr: rec.chromosome,
                pos: rec.position_grch38,
              });
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
    } catch (e: unknown) {
      error = String(e);
    } finally {
      isLoading = false;
    }
  }

  $effect(() => {
    if (selectedSample) {
      void loadData();
    }
  });

  $effect(() => {
    if (!focusRsid || riskPins.length === 0) return;
    const pin = riskPins.find((p) => p.rsid.toLowerCase() === focusRsid.toLowerCase());
    if (!pin) return;
    const el = document.querySelector(`[data-rsid="${pin.rsid.toLowerCase()}"]`);
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
  });

</script>

<div class="card map-container">
  <div class="map-header">
    <div>
      <span class="map-kicker">GRCh38 karyotype</span>
      <h3>Chromosome map</h3>
    </div>
    <span class="privacy-pill">Local query · on-device</span>
  </div>

  <p class="map-intro">
    Bars are scaled to real chromosome length. Fill intensity reflects SNP coverage in your import.
  </p>

  <div class="map-chrome">
    <aside class="map-pin-insight" aria-labelledby="map-pin-insight-title">
      <div class="map-pin-insight-head">
        <span class="map-pin-insight-mark" aria-hidden="true">C</span>
        <strong id="map-pin-insight-title">What pins are for</strong>
      </div>
      <p>
        Letter badges mark variants already in your Trait Report or vector discovery, placed at their
        GRCh38 coordinates. They are <em>not</em> a diagnosis — use them to jump into the report.
        <strong>C</strong> = confirm clinically, <strong>!</strong> = stronger association,
        <strong>?</strong> = possible, <strong>V</strong> = vector find.
      </p>
    </aside>

    {#if !isLoading && !error && attentionPins.length > 0}
      <section class="map-attention" aria-labelledby="map-attention-title">
        <div class="map-attention-head">
          <h4 id="map-attention-title">Check these first</h4>
          <span class="map-attention-count">{attentionPins.length} finding{attentionPins.length === 1 ? "" : "s"}</span>
        </div>
        <p class="map-attention-hint">
          Clinical-confirm and stronger-association hits from your packs. Click a gene to focus it on
          the map and open the report card.
        </p>
        <ul class="map-attention-list">
          {#each attentionPins as pin (`attn:${pin.rsid}:${pin.pos}`)}
            <li>
              <button
                type="button"
                class="attn-chip {pin.severity}"
                onclick={() => focusPin(pin)}
              >
                <span class="attn-glyph" aria-hidden="true">{pinGlyph(pin.severity)}</span>
                <span class="attn-gene">{pin.gene}</span>
                <span class="attn-meta font-mono">chr{pin.chr} · {pin.rsid}</span>
                <span class="attn-why">{severityLabel(pin.severity)}</span>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if !isLoading && !error}
      <div class="map-summary" aria-label="Map summary">
        <div class="map-stat">
          <span class="map-stat-value">{totalSnps.toLocaleString()}</span>
          <span class="map-stat-label">Genotyped SNPs</span>
        </div>
        <div class="map-stat">
          <span class="map-stat-value">{riskPins.length}</span>
          <span class="map-stat-label">Mapped pins</span>
        </div>
        <div class="map-stat map-stat-warn">
          <span class="map-stat-value">{pinSummary.confirm}</span>
          <span class="map-stat-label">Confirm clinically</span>
        </div>
        <div class="map-stat">
          <span class="map-stat-value">{pinSummary.high}</span>
          <span class="map-stat-label">Stronger assoc.</span>
        </div>
      </div>
    {/if}

    <div class="map-legend" aria-label="Pin type legend">
      <div class="legend-item">
        <span class="legend-swatch high_risk" aria-hidden="true">!</span>
        <span>
          <strong>Stronger association (!)</strong>
          <span class="legend-desc">Two matched risk alleles in curated packs</span>
        </span>
      </div>
      <div class="legend-item">
        <span class="legend-swatch moderate_risk" aria-hidden="true">?</span>
        <span>
          <strong>Possible association (?)</strong>
          <span class="legend-desc">One matched allele or moderate pack signal</span>
        </span>
      </div>
      <div class="legend-item">
        <span class="legend-swatch confirmation_required" aria-hidden="true">C</span>
        <span>
          <strong>Confirm clinically (C)</strong>
          <span class="legend-desc">Worth verifying with a clinical lab test</span>
        </span>
      </div>
      <div class="legend-item">
        <span class="legend-swatch vector_promoted" aria-hidden="true">V</span>
        <span>
          <strong>Vector discovery (V)</strong>
          <span class="legend-desc">Promoted from local vector search</span>
        </span>
      </div>
    </div>

    <div class="map-controls-row">
      <div class="map-filter" role="group" aria-label="Filter pins on map">
        <button type="button" class="filter-chip" class:active={pinFilter === "all"} onclick={() => (pinFilter = "all")}>All</button>
        <button type="button" class="filter-chip" class:active={pinFilter === "attention"} onclick={() => (pinFilter = "attention")}>Needs check</button>
        <button type="button" class="filter-chip" class:active={pinFilter === "confirmation_required"} onclick={() => (pinFilter = "confirmation_required")}>Confirm</button>
        <button type="button" class="filter-chip" class:active={pinFilter === "high_risk"} onclick={() => (pinFilter = "high_risk")}>Stronger</button>
        <button type="button" class="filter-chip" class:active={pinFilter === "moderate_risk"} onclick={() => (pinFilter = "moderate_risk")}>Possible</button>
        <button type="button" class="filter-chip" class:active={pinFilter === "vector_promoted"} onclick={() => (pinFilter = "vector_promoted")}>Vector</button>
      </div>

      <div class="map-zoom">
        <span class="map-zoom-label">Zoom <strong>{zoomScale}×</strong></span>
        <input
          class="map-zoom-range"
          type="range"
          min="1"
          max="8"
          step="0.5"
          bind:value={zoomScale}
          aria-label="Chromosome zoom scale"
        />
        {#if zoomScale > 1}
          <button type="button" class="btn btn-secondary btn-xs" onclick={() => (zoomScale = 1)}>
            Reset
          </button>
        {/if}
      </div>

      <button
        type="button"
        class="btn btn-secondary btn-xs"
        onclick={loadComparison}
        disabled={isLoading || riskPins.length === 0}
      >
        Compare genotypes
      </button>
    </div>
  </div>

  {#if isComparing}
    <div class="comparison-overlay">
      <div class="comparison-header">
        <h4>Multi-sample comparison</h4>
        <button type="button" class="btn btn-secondary btn-xs" onclick={closeComparison}>Close</button>
      </div>
      {#if allSamples.length <= 1}
        <p class="comparison-hint">Import multiple samples to compare genotypes across profiles.</p>
      {:else}
        <div class="comparison-table-wrap">
          <table class="comparison-table">
            <thead>
              <tr>
                <th scope="col">Variant</th>
                <th scope="col">Gene</th>
                {#each allSamples as sample (sample.id)}
                  <th
                    scope="col"
                    class:active-sample={selectedSample && sample.id === selectedSample.id}
                  >
                    {sample.name}{selectedSample && sample.id === selectedSample.id ? " · active" : ""}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each comparisonData as row (`${row.rsid}:${row.gene}`)}
                <tr>
                  <td class="font-mono">{row.rsid}</td>
                  <td>{row.gene}</td>
                  {#each allSamples as sample (sample.id)}
                    <td class="genotype-cell {row.severity}">{row.genotypes[sample.id] || "--"}</td>
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
      accent="#2dd4bf"
      compact
    />
  {:else if error}
    <div class="map-error" role="alert">Failed to query database: {error}</div>
    <button type="button" class="btn btn-secondary btn-xs" onclick={loadData}>Retry</button>
  {:else}
    <div class="chromosome-list">
      {#each CHR_ORDER as chr, idx (chr)}
        {@const count = chromosomeCounts[chr] || 0}
        {@const density = count / maxCount}
        {@const widthPct = chrWidthPct(chr)}
        {@const chrPins = pinsForChr(chr)}
        {@const allChrPins = riskPins.filter((p) => p.chr === chr)}
        {@const chrTraits = traitBands.filter((b) => b.chromosome === chr).slice(0, 4)}
        {@const isActive = activeChr === chr}
        <div
          class="chr-row"
          class:chr-row-active={isActive}
          class:chr-row-has-pins={chrPins.length > 0}
          style={`animation-delay: ${Math.min(idx, 20) * 18}ms`}
          role="group"
          aria-label="Chromosome {chr}"
          onmouseenter={() => (activeChr = chr)}
          onfocusin={() => (activeChr = chr)}
        >
          <div class="chr-label font-mono">{chr}</div>

          <div class="chr-main">
            <div class="chr-track">
              <div
                class="chr-capsule-wrapper"
                style={`width: ${widthPct}%`}
                role="group"
                aria-label={`Chromosome ${chr}; ${formatMb(CHR_LENGTHS[chr])}; ${count.toLocaleString()} SNPs`}
              >
                <div
                  class="chr-capsule"
                  class:chr-capsule-dim={pinFilter !== "all" && chrPins.length === 0 && allChrPins.length > 0}
                  style={`width: ${100 * zoomScale}%; min-width: 100%; --density: ${density}`}
                >
                  <div class="chr-density-fill"></div>
                  <div class="chr-ends" aria-hidden="true">
                    <span class="chr-end-tick start">pter</span>
                    <span class="chr-end-tick end">{formatMb(CHR_LENGTHS[chr])}</span>
                  </div>

                  {#each chrPins as pin (`${pin.rsid}:${pin.pos}`)}
                    {@const pinPosPct = (pin.pos / CHR_LENGTHS[chr]) * 100}
                    <Tooltip
                      label={`${pin.gene} ${pin.rsid}`}
                      description={`Chromosome ${pin.chr} finding: ${severityLabel(pin.severity)}. Select to open this finding in the report.`}
                      placement="top"
                      interactiveChildren
                      interactiveClickBehavior="dismiss"
                    >
                      <button
                        type="button"
                        class="variant-pin {pin.severity}"
                        class:pin-focused={focusRsid && pin.rsid.toLowerCase() === focusRsid.toLowerCase()}
                        style={`left: ${pinPosPct}%`}
                        data-rsid={pin.rsid.toLowerCase()}
                        aria-label="{pin.gene} {pin.rsid} — {severityLabel(pin.severity)}"
                        onclick={() => focusPin(pin)}
                      >
                        <span class="pin-glyph" aria-hidden="true">{pinGlyph(pin.severity)}</span>
                      </button>
                    </Tooltip>
                  {/each}
                </div>
              </div>
            </div>

            {#if isActive && chrPins.length > 0}
              <div class="chr-findings" aria-label="Findings on chromosome {chr}">
                <span class="chr-findings-label">On chr{chr}</span>
                {#each chrPins as pin (`find:${pin.rsid}:${pin.pos}`)}
                  <button
                    type="button"
                    class="finding-chip {pin.severity}"
                    onclick={() => focusPin(pin)}
                  >
                    <span class="finding-glyph" aria-hidden="true">{pinGlyph(pin.severity)}</span>
                    <span class="finding-gene">{pin.gene}</span>
                    <span class="finding-why">{severityLabel(pin.severity)}</span>
                  </button>
                {/each}
              </div>
            {:else if chrPins.length > 0}
              <p class="chr-findings-hint">Hover or focus this row to list {chrPins.length} finding{chrPins.length === 1 ? "" : "s"}.</p>
            {/if}

            {#if chrTraits.length > 0}
              <div class="trait-overlay">
                {#each chrTraits as band (`${band.trait_category}:${band.association_count}`)}
                  <Tooltip
                    label={band.trait_category.replace(/_/g, " ")}
                    description={`${band.association_count} associations in this chromosome trait overlay.`}
                    placement="bottom"
                    triggerClass="trait-band-trigger"
                  >
                    <span class="trait-band">
                      {band.trait_category.replace(/_/g, " ")} ({band.association_count})
                    </span>
                  </Tooltip>
                {/each}
              </div>
            {/if}
          </div>

          <div class="chr-stats font-mono">
            <span class="total-snps-val">{count.toLocaleString()} SNPs</span>
            {#if allChrPins.length > 0}
              <span class="risk-count-pill" class:has-confirm={allChrPins.some((p) => p.severity === "confirmation_required")}>
                {chrPinBreakdown(allChrPins)}
              </span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

</div>
