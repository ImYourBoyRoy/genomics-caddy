<!-- ./src/lib/components/report/ReportView.svelte -->
<script lang="ts">
  import type { GeneratedReport, NormalizedReport, GenomeSample } from '../../types/genomics';
  import { saveReportJson, exportDiscoveryFindings } from '../../api/tauri';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import ReportHeader from './ReportHeader.svelte';
  import SectionCard from './SectionCard.svelte';
  import DashboardSummaryPanel from './DashboardSummaryPanel.svelte';
  import DiscoveredFindingsBanner from './DiscoveredFindingsBanner.svelte';
  import VectorPromotedSection from './VectorPromotedSection.svelte';
  import PanelLoadingState from '../common/loading/PanelLoadingState.svelte';
  import type { VariantNavTarget } from '../../constants/traitCategories';
  import { untrack } from 'svelte';
  import { browser } from '$app/environment';

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
    rawReport: NormalizedReport | null;
    isGeneratingReport: boolean;
    selectedSample: GenomeSample;
    foundMarkersCount: number;
    totalMarkersChecked: number;
    reportError?: string;
    highlightRsid?: string;
    onExploreResearch?: (rsid: string) => void;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
    onOpenDiscovery?: () => void;
  }

  let {
    generatedReport,
    rawReport,
    isGeneratingReport,
    selectedSample,
    foundMarkersCount,
    totalMarkersChecked,
    reportError,
    highlightRsid = "",
    onExploreResearch,
    onNavigateToVariant,
    onOpenDiscovery,
  }: Props = $props();

  let showBenign = $state(false);
  let severityFilter = $state<"all" | "risk_only">("all");
  let tierFilter = $state<string>("all");
  let sortBy = $state<"default" | "severity">("default");
  let viewMode = $state<"simple" | "clinical" | "dual">("dual");
  let showHelpGuide = $state(false);
  let collapsedSections = $state<Record<string, boolean>>({});

  function handleJumpToMarker(linkId: string) {
    if (!generatedReport) return;
    let foundSectionName = "";
    let foundRsid = "";
    for (const sec of generatedReport.sections) {
      const match = sec.markers.find(m => m.link_id === linkId);
      if (match) {
        foundSectionName = sec.name;
        foundRsid = match.rsid;
        break;
      }
    }
    if (foundSectionName && foundRsid) {
      collapsedSections = { ...collapsedSections, [foundSectionName]: false };
      highlightRsid = foundRsid;
      setTimeout(() => {
        const el = document.getElementById(`variant-${foundRsid.toLowerCase()}`);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }, 100);
    }
  }

  $effect(() => {
    if (generatedReport) {
      const sections = generatedReport.sections;
      untrack(() => {
        const initial = { ...collapsedSections };
        let changed = false;
        for (const sec of sections) {
          if (initial[sec.name] === undefined) {
            const storageKey = `section-collapsed-${sec.name}`;
            const val = browser ? (localStorage.getItem(storageKey) !== 'false') : true;
            initial[sec.name] = val;
            changed = true;
          }
        }
        if (changed) {
          collapsedSections = initial;
        }
      });
    }
  });

  function expandAll() {
    for (const sec of filteredSections) {
      collapsedSections[sec.name] = false;
    }
  }

  function collapseAll() {
    for (const sec of filteredSections) {
      collapsedSections[sec.name] = true;
    }
  }

  function getSeverityRank(severity: string): number {
    switch (severity) {
      case "high_risk": return 1;
      case "confirmation_required": return 2;
      case "moderate_risk": return 3;
      case "protective": return 4;
      case "trait": return 5;
      case "context_dependent": return 6;
      case "benign": return 7;
      case "no_data": return 8;
      default: return 9;
    }
  }

  let filteredSections = $derived(
    generatedReport?.sections.map(sec => {
      let markers = sec.markers.filter(m => {
        if (!showBenign && (m.severity_class === "benign" || m.severity_class === "no_data")) return false;
        if (tierFilter === "ab" && !m.evidence_tier.startsWith("A") && !m.evidence_tier.startsWith("B")) return false;
        if (severityFilter === "risk_only" && 
            m.severity_class !== "high_risk" && 
            m.severity_class !== "moderate_risk" && 
            m.severity_class !== "confirmation_required") {
          return false;
        }
        return true;
      });

      if (sortBy === "severity") {
        markers = [...markers].sort((a, b) => getSeverityRank(a.severity_class) - getSeverityRank(b.severity_class));
      }

      return {
        ...sec,
        markers
      };
    }).filter(sec => sec.markers.length > 0) ?? []
  );

  let discoveryExportBusy = $state(false);
  let discoveryExportHint = $state('');

  /** Curated pack report (same shape as roy_ancestrydna_report_v3/v4). */
  async function exportCuratedJson() {
    const targetReport = rawReport || generatedReport;
    if (!targetReport) return;
    try {
      const envelope = {
        schema_version: "2.0.0",
        app_version: "0.1.0",
        export_kind: "curated_marker_packs",
        exported_at: new Date().toISOString(),
        sample: {
          name: selectedSample.name,
          genetic_sex: selectedSample.genetic_sex,
          imported_at: selectedSample.imported_at
        },
        report: targetReport
      };
      const content = JSON.stringify(envelope, null, 2);
      const defaultFilename = `${selectedSample.name.toLowerCase().replace(/\s+/g, '_')}_report.json`;
      await saveReportJson(content, defaultFilename);
    } catch (e: unknown) {
      dialogStore.alert("Failed to save curated JSON report: " + String(e));
    }
  }

  /**
   * Full genome × catalog associations (ClinVar/GWAS/PharmGKB hits beyond packs).
   * Writes under App/Data/exports/ — not the same format as roy_v3/v4.
   */
  async function exportFullCatalogJson() {
    if (!selectedSample?.id || discoveryExportBusy) return;
    discoveryExportBusy = true;
    discoveryExportHint = '';
    try {
      const result = await exportDiscoveryFindings(selectedSample.id);
      discoveryExportHint =
        `Wrote ${result.findings_beyond_packs.toLocaleString()} beyond-pack + ${result.findings_in_packs.toLocaleString()} in-pack hits → App/Data/exports/`;
      await dialogStore.alert(
        `Full catalog export complete.\n\n` +
          `Beyond packs: ${result.findings_beyond_packs.toLocaleString()} associations\n` +
          `In packs: ${result.findings_in_packs.toLocaleString()}\n\n` +
          `Pack coverage:\n${result.pack_coverage_path}\n\n` +
          `Full findings:\n${result.full_findings_path}`
      );
    } catch (e: unknown) {
      discoveryExportHint = String(e);
      dialogStore.alert("Full catalog export failed: " + String(e));
    } finally {
      discoveryExportBusy = false;
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
  <PanelLoadingState
    message="Analyzing genetic markers across marker packs…"
    submessage="Evaluating curated SNPs against your local genotype database."
    accent="#fbbf24"
  />
{:else if generatedReport}
  <header class="report-hero no-print">
    <div>
      <span class="report-kicker">Curated packs · on-device</span>
      <h3 class="report-hero-title">Trait report</h3>
      <p class="report-hero-lead">
        Matched alleles from curated marker packs for <strong>{selectedSample.name}</strong>.
        Letter badges match the Chromosome Map:
        <span class="report-glyph">C</span> confirm ·
        <span class="report-glyph">!</span> stronger ·
        <span class="report-glyph">?</span> possible ·
        <span class="report-glyph">+</span> protective.
        Educational only — not a diagnosis.
      </p>
      {#if onOpenDiscovery}
        <p class="report-hero-cta">
          Looking beyond packs?
          <button type="button" class="btn btn-link btn-sm" onclick={() => onOpenDiscovery?.()}>
            Open Discovery
          </button>
        </p>
      {/if}
    </div>
  </header>

  <!-- Disclaimer Banner -->
  <VectorPromotedSection {selectedSample} {highlightRsid} {onExploreResearch} onNavigate={onNavigateToVariant} />
  <DiscoveredFindingsBanner {selectedSample} {onExploreResearch} onNavigate={onNavigateToVariant} />

  <div class="disclaimer-banner">
    <strong>Important:</strong> This report shows curated marker-pack matches from your raw DNA file. It is <strong>not</strong> a medical diagnosis. “Matched alleles” is coverage of association-direction alleles in packs — not a disease probability.
  </div>

  {#if generatedReport.catalog_warnings?.length}
    <div class="catalog-warnings-banner" role="status">
      <strong>Reference catalogs:</strong>
      <ul>
        {#each generatedReport.catalog_warnings as warning}
          <li>{warning}</li>
        {/each}
      </ul>
    </div>
  {/if}

  <details class="report-chrome-details no-print">
    <summary>Export &amp; print</summary>
    <div class="report-actions">
      <span class="export-privacy-note">
        🔒 Everything stays on your computer. No data is uploaded.
      </span>
      <button
        class="btn btn-primary btn-sm"
        onclick={exportCuratedJson}
        title="Same format as roy_ancestrydna_report_v3/v4 — curated marker-pack report only"
      >
        Export curated report JSON
      </button>
      <button
        class="btn btn-secondary btn-sm"
        onclick={exportFullCatalogJson}
        disabled={discoveryExportBusy}
        title="Genome × ClinVar/GWAS/PharmGKB associations → App/Data/exports/"
      >
        {discoveryExportBusy ? 'Exporting…' : 'Export full catalog associations'}
      </button>
      <button class="btn btn-primary btn-sm" onclick={() => window.print()}>
        Export PDF
      </button>
    </div>
    {#if discoveryExportHint}
      <p class="export-hint">{discoveryExportHint}</p>
    {/if}
    <p class="export-hint">
      Prefer the <strong>Discovery</strong> tab to browse beyond-pack associations in-app.
    </p>
  </details>

  <details class="report-chrome-details report-legend-details">
    <summary>How to read this report</summary>
    <div class="report-legend card">
      <p class="legend-intro">Each card represents a single genetic marker. Color and icon summarize what was found:</p>
      <div class="legend-grid">
        <div class="legend-item">
          <span class="legend-swatch signal-high-risk"></span>
          <div>
            <strong>Stronger association (2 copies)</strong>
            <span>Both copies match the researched association allele. Discuss with a healthcare provider if relevant.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-moderate-risk"></span>
          <div>
            <strong>Possible association (1 copy)</strong>
            <span>One copy matches the association allele. Effect is usually smaller.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-protective"></span>
          <div>
            <strong>Protective</strong>
            <span>This variant may be linked to a beneficial or lower-association effect.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-trait"></span>
          <div>
            <strong>Trait</strong>
            <span>Describes a personal characteristic (e.g. caffeine metabolism), not a disease.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-context"></span>
          <div>
            <strong>Context-Dependent</strong>
            <span>The effect depends on other factors like diet, medications, or lifestyle.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-confirm"></span>
          <div>
            <strong>Needs Confirmation</strong>
            <span>Consumer DNA chips can report false positives. A clinical lab test is required.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-benign"></span>
          <div>
            <strong>Not Detected</strong>
            <span>The effect allele was not found at this position. Card is collapsed since no action is needed.</span>
          </div>
        </div>
        <div class="legend-item">
          <span class="legend-swatch signal-nodata"></span>
          <div>
            <strong>Not Tested</strong>
            <span>Your DNA file did not include data for this position.</span>
          </div>
        </div>
      </div>
    </div>
  </details>

  <!-- Filter Bar -->
  <div class="filter-bar card no-print">
    <span class="filter-label">Filter:</span>
    <label class="filter-toggle">
      <input type="checkbox" bind:checked={showBenign} />
      Show undetected
    </label>
    <label class="filter-toggle">
      <input type="checkbox" checked={severityFilter === "risk_only"} onchange={(e) => severityFilter = e.currentTarget.checked ? "risk_only" : "all"} />
      Risk variants only
    </label>
    <select class="filter-select" bind:value={tierFilter}>
      <option value="all">All evidence tiers</option>
      <option value="ab">Tier A & B only</option>
    </select>
    
    <span class="filter-label" style="margin-left: 12px;">Sort:</span>
    <select class="filter-select" bind:value={sortBy}>
      <option value="default">Default Order</option>
      <option value="severity">By Severity (highest first)</option>
    </select>

    <span class="filter-label" style="margin-left: 12px;">Sections:</span>
    <div style="display: flex; gap: 6px;">
      <button class="view-mode-btn" style="padding: 4px 8px; font-size: 0.72rem; border-radius: 4px;" onclick={expandAll}>📂 Expand All</button>
      <button class="view-mode-btn" style="padding: 4px 8px; font-size: 0.72rem; border-radius: 4px;" onclick={collapseAll}>📁 Collapse All</button>
    </div>
    
    <span class="filter-label" style="margin-left: auto;">View Mode:</span>
    <div class="view-mode-buttons" style="display: flex; gap: 4px;">
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
      <button 
        class="view-mode-btn" 
        style="background: rgba(255,255,255,0.06); margin-left: 8px; border-color: rgba(255,255,255,0.15); display: flex; align-items: center; gap: 4px;"
        onclick={() => showHelpGuide = true}
      >
        📖 Guide
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

  {#if generatedReport}
    <DashboardSummaryPanel report={generatedReport} onJumpToMarker={handleJumpToMarker} />
  {/if}

  <div class="sections-container">
    {#each filteredSections as section}
      <SectionCard 
        {section} 
        {viewMode} 
        {onExploreResearch} 
        {highlightRsid} 
        onNavigateToVariant={onNavigateToVariant} 
        bind:collapsed={collapsedSections[section.name]} 
      />
    {/each}
  </div>

  {#if showHelpGuide}
    <div class="modal-backdrop help-backdrop" onclick={() => showHelpGuide = false} role="presentation">
      <div class="modal-content help-content" onclick={(e) => e.stopPropagation()} role="presentation">
        <div class="modal-header">
          <h3>📖 Genomics &amp; Genetics Guide</h3>
          <button class="modal-close" onclick={() => showHelpGuide = false}>&times;</button>
        </div>
        <div class="modal-body help-body">
          <section class="help-section">
            <h5>🧬 What are DNA Letters and Genotypes?</h5>
            <p>
              Your DNA contains genetic instruction markers called **SNPs** (Single Nucleotide Polymorphisms). For each marker, you inherit two DNA letters (one from your mother, one from your father). This pair of letters is called your **Genotype** (e.g. <code>AG</code> or <code>GG</code>).
            </p>
          </section>

          <section class="help-section">
            <h5>📊 What does "Your Result" Mean?</h5>
            <p>
              We scan your raw DNA file to find your specific genetic letters. Depending on what is found:
            </p>
            <ul>
              <li><strong>🔴 Two association copies:</strong> Both copies match the allele used by the pack's researched association rule.</li>
              <li><strong>🟡 One association copy:</strong> One copy matches the association allele; effects are usually smaller and remain context-dependent.</li>
              <li><strong>🟢 Protective association:</strong> This variant may be linked to a beneficial or lower-risk direction, not guaranteed protection.</li>
              <li><strong>🟣 Context-dependent:</strong> The variant's effect depends on other environmental factors (e.g. diet, exercise, drugs).</li>
            </ul>
          </section>

          <section class="help-section">
            <h5>📚 Evidence Tiers (How certain is this science?)</h5>
            <p>
              Not all genetic research is equal. We sort findings by scientific credibility:
            </p>
            <ul>
              <li><strong>Tier A / B (Well-Studied):</strong> Strongly backed by multiple clinical studies and consensus medical guidelines.</li>
              <li><strong>Tier C (Preliminary):</strong> Shows a statistical link in early studies, but requires more research.</li>
              <li><strong>Tier D / E (Research-Only):</strong> Early scientific hypotheses based on small cohorts. Treat these as ideas to explore, not facts.</li>
            </ul>
          </section>

          <section class="help-section warning-section">
            <h5>⚠️ Crucial Safety Information</h5>
            <p>
              <strong>This tool uses raw, unvalidated consumer DNA data.</strong> Consumer tests (like AncestryDNA or 23andMe) are designed for recreational ancestry and can contain raw sequence errors or false positives (up to 40% error rate on rare health variants). 
            </p>
            <p>
              <em>Never change medications, supplement dosages, or medical therapies based on this report alone.</em> Always verify high-risk or clinical findings with a medical-grade clinical lab test (e.g. CLIA/CAP certified) ordered by your physician.
            </p>
          </section>
        </div>
        <div class="modal-footer">
          <button class="btn btn-accent" onclick={() => showHelpGuide = false}>Got it, thank you!</button>
        </div>
      </div>
    </div>
  {/if}
{:else}
  <div class="report-idle-empty" role="status">
    <strong>No report loaded yet</strong>
    <p>
      Import a genome and ensure reference catalogs are ready. The trait report builds automatically
      from curated marker packs once your profile is selected.
    </p>
  </div>
{/if}

<style>
  .help-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(10px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1200;
    animation: modalFadeIn 0.2s ease-out;
  }

  .help-content {
    background: rgba(18, 20, 32, 0.96);
    border: 1px solid var(--border-color);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    border-radius: 12px;
    width: 90%;
    max-width: 600px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .help-body {
    padding: 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
    color: #cbd5e1;
    font-size: 0.9rem;
    line-height: 1.6;
    text-align: left;
  }

  .help-section h5 {
    margin: 0 0 8px 0;
    font-size: 0.95rem;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .help-section p {
    margin: 0;
    color: #94a3b8;
  }

  .help-section ul {
    margin: 8px 0 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: #94a3b8;
  }

  .warning-section {
    background: rgba(239, 68, 68, 0.08);
    border-left: 4px solid var(--danger);
    padding: 12px 16px;
    border-radius: 6px;
  }

  .warning-section h5 {
    color: #f87171;
  }

  .warning-section p {
    color: #cbd5e1;
  }

  .warning-section em {
    color: #fca5a5;
    font-weight: 600;
  }

  /* Modal header/footer styling */
  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.2);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .modal-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 1.5rem;
    cursor: pointer;
    line-height: 1;
    padding: 0;
    transition: color 0.2s;
  }

  .modal-close:hover {
    color: var(--danger);
  }

  .modal-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    background: rgba(0, 0, 0, 0.2);
  }
</style>
