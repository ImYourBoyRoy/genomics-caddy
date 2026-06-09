<!-- ./src/lib/components/report/ReportView.svelte -->
<script lang="ts">
  import type { GeneratedReport, GenomeSample } from '../../types/genomics';
  import { saveReportJson } from '../../api/tauri';
  import ReportHeader from './ReportHeader.svelte';
  import SectionCard from './SectionCard.svelte';

  /*
  Module Docstring:
  Purpose: Orchestrator component displaying the completed genetic trait report.
  Responsibilities:
  - Display disclaimer banner, export actions, and a color legend.
  - Mount ReportHeader and loop through SectionCard instances.
  - Show loader state when reports are generating.
  Key Inputs: generatedReport, isGeneratingReport, selectedSample, foundMarkersCount, totalMarkersChecked.
  Key Outputs: Full HTML report template.
  Operational Notes: The legend explains card colors and terms in plain language.
  */

  interface Props {
    generatedReport: GeneratedReport | null;
    isGeneratingReport: boolean;
    selectedSample: GenomeSample;
    foundMarkersCount: number;
    totalMarkersChecked: number;
    reportError?: string;
  }

  let {
    generatedReport,
    isGeneratingReport,
    selectedSample,
    foundMarkersCount,
    totalMarkersChecked,
    reportError
  }: Props = $props();

  let showBenign = $state(false);
  let tierFilter = $state<string>("all");
  let viewMode = $state<"simple" | "clinical" | "dual">("dual");

  let filteredSections = $derived(
    generatedReport?.sections.map(sec => ({
      ...sec,
      markers: sec.markers.filter(m => {
        if (!showBenign && (m.severity_class === "benign" || m.severity_class === "no_data")) return false;
        if (tierFilter !== "all") {
          if (tierFilter === "ab" && !m.evidence_tier.startsWith("A_") && !m.evidence_tier.startsWith("B_")) return false;
        }
        return true;
      })
    })).filter(sec => sec.markers.length > 0) ?? []
  );

  async function exportJson() {
    if (!generatedReport) return;
    try {
      const envelope = {
        schema_version: "1.0.0",
        app_version: "0.1.0",
        exported_at: new Date().toISOString(),
        sample: {
          name: selectedSample.name,
          genetic_sex: selectedSample.genetic_sex,
          imported_at: selectedSample.imported_at
        },
        report: generatedReport
      };
      const content = JSON.stringify(envelope, null, 2);
      const defaultFilename = `${selectedSample.name.toLowerCase().replace(/\s+/g, '_')}_report.json`;
      await saveReportJson(content, defaultFilename);
    } catch (e: any) {
      alert("Failed to save JSON report: " + e.toString());
    }
  }
</script>

{#if reportError}
  <div class="error-card card no-print">
    <div class="error-header">
      <span class="error-icon">⚠️</span>
      <h4>Report Generation Failed</h4>
    </div>
    <p class="error-details">{reportError}</p>
  </div>
{/if}

{#if isGeneratingReport}
  <div class="loader">Analyzing genetic markers...</div>
{:else if generatedReport}
  <!-- Disclaimer Banner -->
  <div class="disclaimer-banner">
    ⚠️ <strong>Important:</strong> This report shows which genetic variants were found in your raw DNA file. It is <strong>not</strong> a medical diagnosis. Variants labeled "risk" show statistical associations — they do not guarantee you will develop a condition. Always consult a healthcare professional for medical decisions.
  </div>

  <!-- Export Actions -->
  <div class="report-actions no-print">
    <span class="export-privacy-note">
      🔒 Everything stays on your computer. No data is uploaded.
    </span>
    <button class="btn btn-primary btn-sm" onclick={exportJson}>
      💾 Export JSON
    </button>
    <button class="btn btn-primary btn-sm" onclick={() => window.print()}>
      🖨️ Export PDF
    </button>
  </div>

  <!-- Color Legend (how to read this report) -->
  <div class="report-legend card no-print">
    <h4>📖 How to Read This Report</h4>
    <p class="legend-intro">Each card below represents a single genetic marker. The card's color and icon tell you what was found:</p>
    <div class="legend-grid">
      <div class="legend-item">
        <span class="legend-swatch signal-high-risk"></span>
        <div>
          <strong>🔴 Risk (2 copies)</strong>
          <span>Both copies carry the risk variant. Discuss with a healthcare provider.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-moderate-risk"></span>
        <div>
          <strong>🟡 Risk (1 copy)</strong>
          <span>One copy carries the risk variant. Effect is usually smaller.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-protective"></span>
        <div>
          <strong>🟢 Protective</strong>
          <span>This variant may reduce risk or provide a beneficial effect.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-trait"></span>
        <div>
          <strong>🔵 Trait</strong>
          <span>Describes a personal characteristic (e.g. caffeine metabolism), not a disease.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-context"></span>
        <div>
          <strong>🟣 Context-Dependent</strong>
          <span>The effect depends on other factors like diet, medications, or lifestyle.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-confirm"></span>
        <div>
          <strong>⚠️ Needs Confirmation</strong>
          <span>Consumer DNA chips can report false positives. A clinical lab test is required.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-benign"></span>
        <div>
          <strong>○ Not Detected</strong>
          <span>The effect allele was not found at this position. Card is collapsed since no action is needed.</span>
        </div>
      </div>
      <div class="legend-item">
        <span class="legend-swatch signal-nodata"></span>
        <div>
          <strong>⚪ Not Tested</strong>
          <span>Your DNA file did not include data for this position.</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Filter Bar -->
  <div class="filter-bar card no-print">
    <span class="filter-label">Filter:</span>
    <label class="filter-toggle">
      <input type="checkbox" bind:checked={showBenign} />
      Show undetected variants
    </label>
    <select class="filter-select" bind:value={tierFilter}>
      <option value="all">All evidence tiers</option>
      <option value="ab">Tier A & B only (well-established)</option>
    </select>
    
    <span class="filter-label" style="margin-left: auto;">View Mode:</span>
    <div class="view-mode-buttons">
      <button 
        class="view-mode-btn" 
        class:view-mode-active={viewMode === 'simple'} 
        onclick={() => viewMode = 'simple'}
      >
        🌱 Simple
      </button>
      <button 
        class="view-mode-btn" 
        class:view-mode-active={viewMode === 'clinical'} 
        onclick={() => viewMode = 'clinical'}
      >
        🏥 Clinical
      </button>
      <button 
        class="view-mode-btn" 
        class:view-mode-active={viewMode === 'dual'} 
        onclick={() => viewMode = 'dual'}
      >
        👥 Dual
      </button>
    </div>
  </div>

  <!-- Health Summary Panel -->
  <ReportHeader
    {generatedReport}
    geneticSex={selectedSample.genetic_sex}
    {foundMarkersCount}
    {totalMarkersChecked}
  />

  <div class="sections-container">
    {#each filteredSections as section}
      <SectionCard {section} {viewMode} />
    {/each}
  </div>
{/if}
