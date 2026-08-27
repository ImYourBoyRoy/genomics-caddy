<!-- ./src/lib/components/report/ReportView.svelte -->
<script lang="ts">
  import type { GeneratedReport, NormalizedReport, GenomeSample, SeverityClass } from '../../types/genomics';
  import { saveReportJson, exportDiscoveryFindings } from '../../api/tauri';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import ReportHeader from './ReportHeader.svelte';
  import SectionCard from './SectionCard.svelte';
  import DashboardSummaryPanel from './DashboardSummaryPanel.svelte';
  import DiscoveredFindingsBanner from './DiscoveredFindingsBanner.svelte';
  import VectorPromotedSection from './VectorPromotedSection.svelte';
  import PanelLoadingState from '../common/loading/PanelLoadingState.svelte';
  import type { VariantNavTarget } from '../../constants/traitCategories';
  import { getSeverityInfo } from '../../utils/evidence';
  import { tick, untrack } from 'svelte';
  import { browser } from '$app/environment';
  import {
    loadReproductiveContext,
    reproductiveMarkerContextRank,
    reproductiveSectionHasContext,
    reproductiveContextStorageKey,
    selectedReproductiveContextOption,
  } from '../../utils/reproductiveContext';
  import type { PersonalSafetyContext } from '../../utils/personalSafetyContext';
  import {
    DEFAULT_PRESENTATION_MODE,
    readPresentationMode,
    writePresentationMode,
    type PresentationMode,
  } from '../../utils/presentationPreferences';
  import {
    buildReportAudienceMarkdown,
    reportAudienceFilename,
    type ReportExportAudience,
  } from '../../utils/reportAudienceExport';

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
    personalSafetyContext?: PersonalSafetyContext;
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
    personalSafetyContext = $bindable(),
    highlightRsid = "",
    onExploreResearch,
    onNavigateToVariant,
    onOpenDiscovery,
  }: Props = $props();

  let showBenign = $state(false);
  let severityFilter = $state<"all" | "risk_only">("all");
  let tierFilter = $state<string>("all");
  let sortBy = $state<"default" | "severity">("default");
  let presentationMode = $state<PresentationMode>(DEFAULT_PRESENTATION_MODE);
  let loadedPresentationModeKey = $state("");
  let showHelpGuide = $state(false);
  let helpDialogElement = $state<HTMLDivElement | undefined>(undefined);
  let helpCloseButton = $state<HTMLButtonElement | undefined>(undefined);
  let previousHelpFocus: HTMLElement | null = null;
  let collapsedSections = $state<Record<string, boolean>>({});
  let isPreparingPrint = $state(false);
  let printRestore: (() => void) | null = null;
  let clinicalProvenanceExpanded = $state(false);
  let reproductiveContext = $state('');
  let loadedReproductiveContextKey = $state('');
  let prioritizeReproductiveContext = $state(true);

  function presentationModeStorageKey(sampleId: number): string {
    return `genomics_presentation_mode_${sampleId}`;
  }

  $effect(() => {
    if (!browser || !selectedSample?.id) return;
    const key = presentationModeStorageKey(selectedSample.id);
    if (loadedPresentationModeKey === key) return;
    loadedPresentationModeKey = key;
    presentationMode = readPresentationMode(localStorage, key);
  });

  function setPresentationMode(mode: PresentationMode) {
    presentationMode = mode;
    if (browser && selectedSample?.id) {
      writePresentationMode(localStorage, presentationModeStorageKey(selectedSample.id), mode);
    }
  }

  function openHelpGuide() {
    previousHelpFocus = browser && document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    showHelpGuide = true;
    void tick().then(() => helpCloseButton?.focus());
  }

  function closeHelpGuide() {
    showHelpGuide = false;
    const focusTarget = previousHelpFocus;
    previousHelpFocus = null;
    void tick().then(() => focusTarget?.focus());
  }

  function handleHelpKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      closeHelpGuide();
      return;
    }
    if (event.key !== 'Tab' || !helpDialogElement) return;

    const focusable = Array.from(
      helpDialogElement.querySelectorAll<HTMLElement>(
        'button, a[href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    ).filter((element) => !element.hasAttribute('aria-hidden') && element.offsetParent !== null);

    if (focusable.length === 0) {
      event.preventDefault();
      helpDialogElement.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  // Presentation order is fixed; labels and descriptions come from the shared evidence policy.
  const severityLegend: SeverityClass[] = [
    'high_risk',
    'moderate_risk',
    'protective',
    'trait',
    'context_dependent',
    'confirmation_required',
    'benign',
    'no_data',
  ];

  $effect(() => {
    const contextKey = reproductiveContextStorageKey(selectedSample?.id);
    if (loadedReproductiveContextKey === contextKey) return;
    loadedReproductiveContextKey = contextKey;
    reproductiveContext = browser ? loadReproductiveContext(selectedSample?.id) : '';
  });

  function selectedReproductiveContextLabel(): string {
    return selectedReproductiveContextOption(reproductiveContext)?.label || 'the selected context';
  }

  function sectionAnchorId(sectionName: string): string {
    return `report-section-${sectionName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  }

  function handleJumpToSection(sectionName: string) {
    if (!generatedReport?.sections.some((section) => section.name === sectionName)) return;
    collapsedSections = { ...collapsedSections, [sectionName]: false };
    void tick().then(() => {
      document.getElementById(sectionAnchorId(sectionName))?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }

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

  function setSectionCollapsed(sectionName: string, isCollapsed: boolean) {
    collapsedSections = { ...collapsedSections, [sectionName]: isCollapsed };
  }

  async function printReport() {
    if (!browser || !generatedReport || isPreparingPrint) return;

    const previousCollapsedSections = { ...collapsedSections };
    const previousClinicalProvenanceExpanded = clinicalProvenanceExpanded;
    isPreparingPrint = true;
    const restore = () => {
      if (printRestore !== restore) return;
      window.removeEventListener('afterprint', restore);
      collapsedSections = previousCollapsedSections;
      clinicalProvenanceExpanded = previousClinicalProvenanceExpanded;
      printRestore = null;
      isPreparingPrint = false;
    };

    printRestore = restore;
    window.addEventListener('afterprint', restore, { once: true });
    collapsedSections = {
      ...collapsedSections,
      ...Object.fromEntries(filteredSections.map((section) => [section.name, false])),
    };
    if (presentationMode === 'clinical') clinicalProvenanceExpanded = true;

    await tick();
    await new Promise<void>((resolve) => window.setTimeout(resolve, 300));
    if (printRestore !== restore) return;

    try {
      window.print();
    } catch {
      restore();
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

      if (
        reproductiveContext
        && prioritizeReproductiveContext
        && reproductiveSectionHasContext(sec.name, sec.markers.map((marker) => marker.rsid))
      ) {
        markers = [...markers].sort((a, b) => {
          const contextRank = reproductiveMarkerContextRank(b.rsid, reproductiveContext)
            - reproductiveMarkerContextRank(a.rsid, reproductiveContext);
          if (contextRank !== 0) return contextRank;
          if (sortBy === "severity") return getSeverityRank(a.severity_class) - getSeverityRank(b.severity_class);
          return 0;
        });
      } else if (sortBy === "severity") {
        markers = [...markers].sort((a, b) => getSeverityRank(a.severity_class) - getSeverityRank(b.severity_class));
      }

      return {
        ...sec,
        markers
      };
    }).filter(sec => sec.markers.length > 0) ?? []
  );

  let clinicalSectionsExpanded = $derived(
    presentationMode !== 'clinical'
      || filteredSections.some((section) => collapsedSections[section.name] === false),
  );

  let discoveryExportBusy = $state(false);
  let discoveryExportHint = $state('');
  let audienceExportBusy = $state<ReportExportAudience | ''>('');
  let audienceExportHint = $state('');

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

  async function exportAudienceReport(audience: ReportExportAudience) {
    if (!generatedReport || audienceExportBusy) return;
    audienceExportBusy = audience;
    audienceExportHint = '';
    try {
      const content = buildReportAudienceMarkdown({
        audience,
        report: generatedReport,
        sample: selectedSample,
        includeRawGenotypes: audience !== 'personal',
        reproductiveContext,
        personalSafetyContext,
      });
      const saved = await saveReportJson(content, reportAudienceFilename(selectedSample.name, audience));
      audienceExportHint = saved
        ? `${audience === 'personal' ? 'Personal Simple' : audience === 'clinician' ? 'Clinician Handoff' : 'AI Review'} export saved locally.`
        : 'Export canceled; no file was written.';
    } catch (e: unknown) {
      audienceExportHint = `Export failed: ${String(e)}`;
      await dialogStore.alert('Audience export failed: ' + String(e));
    } finally {
      audienceExportBusy = '';
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
    accent="var(--status-warning-text)"
  />
{:else if generatedReport}
  <!-- Start with profile/data quality, then the bounded action queue. -->
  <ReportHeader
    {generatedReport}
    {foundMarkersCount}
    {totalMarkersChecked}
    presentationMode={presentationMode}
  />

  {#if generatedReport}
    <DashboardSummaryPanel
      report={generatedReport}
      sampleId={selectedSample.id}
      geneticSex={selectedSample.genetic_sex}
      bind:personalSafetyContext
      bind:reproductiveContext
      presentationMode={presentationMode}
      onJumpToMarker={handleJumpToMarker}
      onJumpToSection={handleJumpToSection}
      />
  {/if}

  {#if presentationMode !== 'simple'}
    <header class="report-hero no-print">
      <div>
        <span class="report-kicker">Curated packs · on-device</span>
        <h3 class="report-hero-title">Trait report</h3>
        <p class="report-hero-lead">
          Curated genetic context for <strong>{selectedSample.name}</strong>.
          Technical marker symbols and raw calls are available in the detailed view.
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
  {/if}

  {#if presentationMode === 'clinical'}
    <details class="clinical-provenance" bind:open={clinicalProvenanceExpanded}>
      <summary>About the data in Clinical view</summary>
      <div class="clinical-provenance-grid">
        <span><strong>DNA array</strong> Genotype calls shown in the tables.</span>
        <span><strong>Clinical confirmation</strong> Separate testing is shown as a follow-up status.</span>
        <span><strong>Personal context</strong> Symptoms, medications, and goals are not DNA findings.</span>
        <span><strong>Clinician-entered data</strong> Not included unless separately documented.</span>
      </div>
    </details>
  {/if}

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
        type="button"
        class="btn btn-primary btn-sm"
        onclick={exportCuratedJson}
      >
        Export curated report JSON
      </button>
      <div class="export-audience-group" aria-label="Audience-specific markdown exports">
        <span class="export-audience-label">Audience exports</span>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => exportAudienceReport('personal')} disabled={Boolean(audienceExportBusy)}>
          {audienceExportBusy === 'personal' ? 'Saving…' : 'Personal Simple'}
        </button>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => exportAudienceReport('clinician')} disabled={Boolean(audienceExportBusy)}>
          {audienceExportBusy === 'clinician' ? 'Saving…' : 'Clinician Handoff'}
        </button>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => exportAudienceReport('ai')} disabled={Boolean(audienceExportBusy)}>
          {audienceExportBusy === 'ai' ? 'Saving…' : 'AI Review'}
        </button>
      </div>
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        onclick={exportFullCatalogJson}
        disabled={discoveryExportBusy}
      >
        {discoveryExportBusy ? 'Exporting…' : 'Export full catalog associations'}
      </button>
      <button type="button" class="btn btn-primary btn-sm" onclick={printReport} disabled={isPreparingPrint}>
        {isPreparingPrint ? 'Preparing PDF…' : 'Export PDF'}
      </button>
    </div>
    {#if discoveryExportHint}
      <p class="export-hint">{discoveryExportHint}</p>
    {/if}
    {#if audienceExportHint}
      <p class="export-hint" role="status">{audienceExportHint}</p>
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
        {#each severityLegend as severityClass (severityClass)}
          {@const legend = getSeverityInfo(severityClass)}
          <div class="legend-item">
            <span class="legend-swatch {legend.cssClass}"></span>
            <div>
              <strong>{legend.legendLabel}</strong>
              <span>{legend.legendDescription}</span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </details>

  <!-- Keep advanced filtering out of the Simple-first reading path while keeping it one click away. -->
  <div class="report-controls no-print">
    <details class="report-filter-details" open={presentationMode !== 'simple'}>
      <summary>Filters &amp; ordering</summary>
      <div class="filter-bar card">
    <div class="filter-group">
      <span class="filter-label">Show</span>
      <label class="filter-toggle">
        <input type="checkbox" bind:checked={showBenign} aria-label="Show benign and uncalled markers" />
        Show benign &amp; uncalled
      </label>
      <label class="filter-toggle">
        <input type="checkbox" checked={severityFilter === "risk_only"} onchange={(e) => severityFilter = e.currentTarget.checked ? "risk_only" : "all"} />
        Risk-focused
      </label>
      <select class="filter-select" aria-label="Evidence tier filter" bind:value={tierFilter}>
        <option value="all">All evidence tiers</option>
        <option value="ab">Tier A & B only</option>
      </select>
    </div>

    <div class="filter-group">
      <span class="filter-label">Order</span>
      <select class="filter-select" aria-label="Sort report sections" bind:value={sortBy}>
        <option value="default">Default order</option>
        <option value="severity">Highest priority first</option>
      </select>
      <div class="filter-actions" role="group" aria-label="Section visibility">
        <button type="button" class="view-mode-btn" onclick={expandAll}>Expand all</button>
        <button type="button" class="view-mode-btn" onclick={collapseAll}>Collapse all</button>
      </div>
    </div>

    {#if reproductiveContext}
      <label class="filter-toggle" aria-describedby="reproductive-priority-hint">
        <input type="checkbox" bind:checked={prioritizeReproductiveContext} />
        Prioritize {selectedReproductiveContextLabel()}
      </label>
      <span id="reproductive-priority-hint" class="filter-context-hint">This changes ordering only; all reproductive markers remain visible.</span>
    {/if}
      </div>
    </details>

    <div class="mode-group" role="group" aria-labelledby="reading-mode-label">
      <span id="reading-mode-label" class="filter-label">Reading mode</span>
      <div class="view-mode-buttons">
      <button
        type="button"
        class="view-mode-btn"
        class:view-mode-active={presentationMode === 'simple'}
        aria-pressed={presentationMode === 'simple'}
        onclick={() => setPresentationMode('simple')}
      >
        🌱 Simple
      </button>
      <button
        type="button"
        class="view-mode-btn"
        class:view-mode-active={presentationMode === 'clinical'}
        aria-pressed={presentationMode === 'clinical'}
        onclick={() => setPresentationMode('clinical')}
      >
        🏥 Clinical
      </button>
      <button
        type="button"
        class="view-mode-btn"
        class:view-mode-active={presentationMode === 'compare'}
        aria-pressed={presentationMode === 'compare'}
        onclick={() => setPresentationMode('compare')}
      >
        👥 Compare
      </button>
      <button
        type="button"
        class="view-mode-btn guide-mode-btn"
        onclick={openHelpGuide}
        aria-haspopup="dialog"
        aria-expanded={showHelpGuide}
      >
        📖 Guide
      </button>
      </div>
    </div>
  </div>

  <div class="sections-container">
    {#if presentationMode === 'clinical' && filteredSections.length > 0 && !clinicalSectionsExpanded}
      <div class="clinical-empty-hint" role="status">
        <strong>Clinical view is ready.</strong>
        <span>Expand a health area below, or use “Expand all” under Filters &amp; ordering, to open its structured findings table.</span>
      </div>
    {/if}
    {#each filteredSections as section}
      <SectionCard 
        {section} 
        viewMode={presentationMode}
        {onExploreResearch} 
        {highlightRsid} 
        onNavigateToVariant={onNavigateToVariant}
        collapsed={collapsedSections[section.name]}
        onCollapsedChange={(isCollapsed) => setSectionCollapsed(section.name, isCollapsed)}
      />
    {/each}
  </div>

  <!-- Secondary research surfaces stay below the core report and remain optional. -->
  <VectorPromotedSection {selectedSample} {presentationMode} {highlightRsid} {onExploreResearch} onNavigate={onNavigateToVariant} />
  <DiscoveredFindingsBanner {selectedSample} {presentationMode} {onExploreResearch} onNavigate={onNavigateToVariant} />

  {#if showHelpGuide}
    <div class="modal-backdrop help-backdrop" onclick={closeHelpGuide} role="presentation">
      <div
        class="modal-content help-content"
        bind:this={helpDialogElement}
        role="dialog"
        aria-modal="true"
        aria-labelledby="genomics-guide-title"
        aria-describedby="genomics-guide-description"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
        onkeydown={handleHelpKeydown}
      >
        <div class="modal-header">
          <h3 id="genomics-guide-title">📖 Genomics &amp; Genetics Guide</h3>
          <button type="button" class="modal-close" bind:this={helpCloseButton} aria-label="Close genetics guide" onclick={closeHelpGuide}>&times;</button>
        </div>
        <div class="modal-body help-body">
          <p id="genomics-guide-description" class="sr-only">Plain-language explanations of DNA results, evidence tiers, and the limits of this report.</p>
          <section class="help-section">
            <h5>🧬 What are DNA Letters and Genotypes?</h5>
            <p>
              Your DNA contains genetic instruction markers called <strong>SNPs</strong> (Single Nucleotide Polymorphisms). For each marker, you inherit two DNA letters (one from each biological parent). This pair of letters is called your <strong>genotype</strong> (e.g. <code>AG</code> or <code>GG</code>).
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

        </div>
        <div class="modal-footer">
          <button type="button" class="btn btn-accent" onclick={closeHelpGuide}>Got it, thank you!</button>
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
  .filter-context-hint {
    color: var(--text-secondary);
    font-size: 0.72rem;
    max-width: 180px;
  }

  .clinical-empty-hint {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.6rem;
    align-items: baseline;
    margin: 0 0 0.75rem;
    padding: 0.75rem 0.9rem;
    border: 1px solid var(--border-color);
    border-radius: 0.6rem;
    background: var(--surface-subtle);
    color: var(--text-secondary);
    font-size: 0.82rem;
  }

  .clinical-empty-hint strong {
    color: var(--text-primary);
  }

  .clinical-provenance {
    margin: 0 0 0.75rem;
    padding: 0.55rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.55rem;
    background: var(--surface-subtle);
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .clinical-provenance summary {
    color: var(--accent);
    cursor: pointer;
    font-weight: 700;
  }

  .clinical-provenance-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem 1rem;
    margin-top: 0.6rem;
  }

  .clinical-provenance-grid span {
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .clinical-provenance-grid strong {
    color: var(--text-primary);
  }

  @media (max-width: 900px) {
    .clinical-provenance-grid {
      grid-template-columns: 1fr;
    }
  }

  .help-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: var(--modal-backdrop-bg);
    backdrop-filter: blur(10px);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1200;
    animation: modalFadeIn 0.2s ease-out;
  }

  .help-content {
    background: var(--surface-raised);
    color: var(--text-primary);
    border: 1px solid var(--border-color);
    box-shadow: 0 20px 40px var(--shadow-modal);
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
    color: var(--text-primary);
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
    color: var(--text-secondary);
  }

  .help-section ul {
    margin: 8px 0 0 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-secondary);
  }

  .warning-section {
    background: color-mix(in srgb, var(--danger) 10%, var(--surface-raised));
    border-left: 4px solid var(--danger);
    padding: 12px 16px;
    border-radius: 6px;
  }

  /* Modal header/footer styling */
  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--surface-subtle);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .modal-close {
    min-width: 44px;
    min-height: 44px;
    background: transparent;
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

  .modal-close:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .modal-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border-color);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    background: var(--surface-subtle);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
