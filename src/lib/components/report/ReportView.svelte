<!-- ./src/lib/components/report/ReportView.svelte -->
<script lang="ts">
  import type { GeneratedReport, NormalizedReport, GenomeSample, SeverityClass } from '../../types/genomics';
  import { saveReportBundle, saveReportJson, exportDiscoveryFindings } from '../../api/tauri';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import ReportHeader from './ReportHeader.svelte';
  import ReportExportActions from './ReportExportActions.svelte';
  import ReferenceIndex from './ReferenceIndex.svelte';
  import SectionCard from './SectionCard.svelte';
  import DashboardSummaryPanel from './DashboardSummaryPanel.svelte';
  import DiscoveredFindingsBanner from './DiscoveredFindingsBanner.svelte';
  import VectorPromotedSection from './VectorPromotedSection.svelte';
  import type { VariantNavTarget } from '../../constants/traitCategories';
  import { getSeverityInfo } from '../../utils/evidence';
  import { tick, untrack } from 'svelte';
  import { browser } from '$app/environment';
  import type { ProfileContext } from '../../utils/profileContext';
  import { EMPTY_PROFILE_CONTEXT } from '../../utils/profileContext';
  import {
    DEFAULT_PRESENTATION_MODE,
    readPresentationMode,
    writePresentationMode,
    type PresentationMode,
  } from '../../utils/presentationPreferences';
  import {
    type ReportExportAudience,
  } from '../../utils/reportAudienceExport';
  import { buildAiReviewJson, buildReportBundleFiles, reportBundleFilename } from '../../utils/reportBundleExport';
  import { buildCanonicalFindingGroups } from '../../utils/findingIdentity';

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
    profileContext?: ProfileContext;
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
    profileContext,
    highlightRsid = "",
    onExploreResearch,
    onNavigateToVariant,
    onOpenDiscovery,
  }: Props = $props();

  let showBenign = $state(false);
  let severityFilter = $state<"all" | "risk_only">("all");
  let tierFilter = $state<string>("all");
  let sortBy = $state<"default" | "severity">("default");
  let reportFiltersOpen = $state(false);
  let presentationMode = $state<PresentationMode>(DEFAULT_PRESENTATION_MODE);
  let loadedPresentationModeKey = $state("");
  let showHelpGuide = $state(false);
  let helpDialogElement = $state<HTMLDivElement | undefined>(undefined);
  let helpCloseButton = $state<HTMLButtonElement | undefined>(undefined);
  let previousHelpFocus: HTMLElement | null = null;
  let collapsedSections = $state<Record<string, boolean>>({});
  let loadedCollapseProfileId = $state<number | null>(null);
  let isPreparingPrint = $state(false);
  let printRestore: (() => void) | null = null;
  let clinicalProvenanceExpanded = $state(false);

  const REPORT_LOADING_STAGES = [
    { title: 'Reading your local profile', detail: 'Opening the stored genotype data on this device.' },
    { title: 'Matching curated markers', detail: 'Checking the curated marker packs against this profile.' },
    { title: 'Organizing evidence', detail: 'Grouping findings, evidence tiers, and follow-up guidance.' },
    { title: 'Preparing your dashboard', detail: 'Finishing the report so it opens ready for review.' },
  ] as const;
  let reportLoadingElapsedSeconds = $state(0);

  $effect(() => {
    if (!browser || !isGeneratingReport) {
      reportLoadingElapsedSeconds = 0;
      return;
    }
    const startedAt = Date.now();
    const timer = window.setInterval(() => {
      reportLoadingElapsedSeconds = Math.floor((Date.now() - startedAt) / 1000);
    }, 1000);
    return () => window.clearInterval(timer);
  });

  let reportLoadingStageIndex = $derived(
    Math.min(REPORT_LOADING_STAGES.length - 1, Math.floor(reportLoadingElapsedSeconds / 4)),
  );
  let reportLoadingStage = $derived(REPORT_LOADING_STAGES[reportLoadingStageIndex]);

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
    'not_evaluated',
    'benign',
    'no_data',
  ];

  function sectionAnchorId(sectionName: string): string {
    return `report-section-${sectionName.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  }

  function sectionCollapseStorageKey(sampleId: number, sectionName: string): string {
    // Version the key so stale preferences cannot reopen a dense section on first view.
    return `section-collapsed-v2-${sampleId}-${sectionName}`;
  }

  function persistSectionCollapsed(sectionName: string, isCollapsed: boolean): void {
    if (!browser || !selectedSample?.id) return;
    try {
      localStorage.setItem(
        sectionCollapseStorageKey(selectedSample.id, sectionName),
        String(isCollapsed),
      );
    } catch {
      // The in-memory report state remains usable when localStorage is unavailable.
    }
  }

  function handleJumpToSection(sectionName: string) {
    if (!generatedReport?.sections.some((section) => section.name === sectionName)) return;
    setSectionCollapsed(sectionName, false);
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
      setSectionCollapsed(foundSectionName, false);
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
    if (!generatedReport || !selectedSample?.id) return;
    const profileId = selectedSample.id;
    const sections = generatedReport.sections;
    untrack(() => {
      const profileChanged = loadedCollapseProfileId !== profileId;
      const initial = profileChanged ? {} : { ...collapsedSections };
      let changed = profileChanged;
      for (const sec of sections) {
        if (initial[sec.name] === undefined) {
          const storageKey = sectionCollapseStorageKey(profileId, sec.name);
          let value = true;
          if (browser) {
            try {
              value = localStorage.getItem(storageKey) !== 'false';
            } catch {
              value = true;
            }
          }
          initial[sec.name] = value;
          changed = true;
        }
      }
      loadedCollapseProfileId = profileId;
      if (changed) {
        collapsedSections = initial;
      }
    });
  });

  function expandAll() {
    for (const sec of filteredSections) {
      setSectionCollapsed(sec.name, false);
    }
  }

  function handleJumpToMarkers(linkIds: string[]) {
    if (!generatedReport || linkIds.length === 0) return;
    const targets = linkIds.flatMap((linkId) => generatedReport.sections.flatMap((section) => {
      const marker = section.markers.find((candidate) => candidate.link_id === linkId);
      return marker ? [{ sectionName: section.name, rsid: marker.rsid }] : [];
    }));
    const uniqueSections = Array.from(new Set(targets.map((target) => target.sectionName)));
    uniqueSections.forEach((sectionName) => setSectionCollapsed(sectionName, false));
    const first = targets[0];
    if (!first) return;
    highlightRsid = first.rsid;
    void tick().then(() => {
      document.getElementById(`variant-${first.rsid.toLowerCase()}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
  }

  function collapseAll() {
    for (const sec of filteredSections) {
      setSectionCollapsed(sec.name, true);
    }
  }

  function setSectionCollapsed(sectionName: string, isCollapsed: boolean) {
    collapsedSections = { ...collapsedSections, [sectionName]: isCollapsed };
    persistSectionCollapsed(sectionName, isCollapsed);
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
      case "not_evaluated": return 7;
      case "benign": return 8;
      case "no_data": return 9;
      default: return 10;
    }
  }

  let filteredSourceSections = $derived(
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

  // Source rows remain available in Clinical mode. Simple mode instead shows
  // one representative card per canonical locus so a repeated rsID across
  // packs does not read like several unrelated concerns.
  let simpleCanonicalGroups = $derived(
    presentationMode === 'simple'
      ? buildCanonicalFindingGroups({ sections: filteredSourceSections })
      : [],
  );
  let simpleRepresentativeLinkIds = $derived(
    new Set(simpleCanonicalGroups.map((group) => group.representativeSource.marker.link_id)),
  );
  let simpleRelatedMarkerCounts = $derived(
    Object.fromEntries(
      simpleCanonicalGroups.map((group) => [
        group.representativeSource.marker.link_id,
        group.sourceMarkerIds.length,
      ]),
    ) as Record<string, number>,
  );
  let filteredSections = $derived(
    filteredSourceSections
      .map((section) => ({
        ...section,
        markers: presentationMode === 'simple'
          ? section.markers.filter((marker) => simpleRepresentativeLinkIds.has(marker.link_id))
          : section.markers,
      }))
      .filter((section) => section.markers.length > 0),
  );

  let clinicalSectionsExpanded = $derived(
    presentationMode !== 'clinical'
      || filteredSections.some((section) => collapsedSections[section.name] === false),
  );

  let discoveryExportBusy = $state(false);
  let discoveryExportHint = $state('');
  let audienceExportBusy = $state<ReportExportAudience | ''>('');
  let aiJsonBusy = $state(false);
  let audienceExportHint = $state('');

  /** Curated pack report (same shape as the legacy report v3/v4 fixtures). */
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

  function audienceBundleOptions(audience: ReportExportAudience) {
    return {
      audience,
      report: generatedReport!,
      sample: selectedSample,
      includeRawGenotypes: audience !== 'personal',
      reproductiveContext: profileContext?.selectedReproductiveContext,
      personalSafetyContext: profileContext?.safety,
      profileContext: profileContext || EMPTY_PROFILE_CONTEXT,
    };
  }

  async function exportAiReviewJson() {
    if (!generatedReport || aiJsonBusy) return;
    aiJsonBusy = true;
    audienceExportHint = '';
    try {
      const content = buildAiReviewJson(audienceBundleOptions('ai'));
      const defaultFilename = `${selectedSample.name.toLowerCase().replace(/[^a-z0-9]+/g, '_')}_ai_review.json`;
      const saved = await saveReportJson(content, defaultFilename);
      audienceExportHint = saved
        ? 'AI-ready JSON saved locally. It includes DNA findings, references, entered context, and diary data.'
        : 'Export canceled; no file was written.';
    } catch (e: unknown) {
      audienceExportHint = `Export failed: ${String(e)}`;
      await dialogStore.alert('AI-ready JSON export failed: ' + String(e));
    } finally {
      aiJsonBusy = false;
    }
  }

  async function exportAudienceReport(audience: ReportExportAudience) {
    if (!generatedReport || audienceExportBusy) return;
    audienceExportBusy = audience;
    audienceExportHint = '';
    try {
      const options = audienceBundleOptions(audience);
      const saved = await saveReportBundle(
        buildReportBundleFiles(options),
        reportBundleFilename(selectedSample.name, audience),
      );
      audienceExportHint = saved
        ? `${audience === 'personal' ? 'Personal Simple' : audience === 'clinician' ? 'Clinician Handoff' : 'AI Review'} bundle saved locally.`
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
  <section
    class="report-loading-state"
    aria-busy="true"
    aria-labelledby="report-loading-title"
    role="status"
  >
    <div class="report-loading-content">
      <div class="report-loading-heading">
        <span class="report-loading-kicker">Preparing report</span>
        <span class="report-loading-device">On-device · live</span>
      </div>
      <div class="report-loading-mark" aria-hidden="true">🧬</div>
      <h3 id="report-loading-title">Mapping {selectedSample.name}'s DNA</h3>
      <p class="report-loading-lead">Reviewing curated markers and assembling the most useful findings for this profile.</p>
      <div class="report-loading-activity" role="group" aria-label="Live report preparation status">
        <div class="report-loading-activity-heading">
          <div>
            <span class="report-loading-activity-label">Working now</span>
            <strong>{reportLoadingStage.title}</strong>
          </div>
          <span class="report-loading-elapsed">
            {reportLoadingElapsedSeconds < 1 ? 'Starting…' : `${reportLoadingElapsedSeconds}s elapsed`}
          </span>
        </div>
        <div class="report-loading-indeterminate" aria-hidden="true"><span></span></div>
        <p>{reportLoadingStage.detail}</p>
        <ol class="report-loading-steps" aria-label="Report preparation phases">
          {#each REPORT_LOADING_STAGES as stage, index}
            <li class:active={index === reportLoadingStageIndex}>
              <span class="report-loading-step-node" aria-hidden="true">{index + 1}</span>
              <span>
                <strong>{stage.title}</strong>
                <small>{index === reportLoadingStageIndex ? 'In progress' : 'Queued'}</small>
              </span>
            </li>
          {/each}
        </ol>
      </div>
      <div class="report-loading-meta" aria-label="Report preparation details">
        <span><span class="report-loading-dot" aria-hidden="true"></span> Your DNA stays on this computer</span>
        <span>•</span>
        <span>The report will open automatically when ready</span>
      </div>
    </div>
  </section>
{:else if generatedReport}
  <!-- Start with profile/data quality, then the bounded action queue. -->
  <ReportHeader
    {generatedReport}
    {foundMarkersCount}
    {totalMarkersChecked}
    presentationMode={presentationMode}
  />

  <ReportExportActions
    {audienceExportBusy}
    {aiJsonBusy}
    {discoveryExportBusy}
    {isPreparingPrint}
    {audienceExportHint}
    {discoveryExportHint}
    onExportAiJson={exportAiReviewJson}
    onExportAudience={exportAudienceReport}
    onExportCuratedJson={exportCuratedJson}
    onExportFullCatalogJson={exportFullCatalogJson}
    onPrintReport={printReport}
  />

  {#if generatedReport}
    <DashboardSummaryPanel
      report={generatedReport}
      sampleId={selectedSample.id}
      presentationMode={presentationMode}
      onJumpToMarker={handleJumpToMarker}
      onJumpToMarkers={handleJumpToMarkers}
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
    <details class="catalog-warnings-banner" aria-label="Reference catalog status">
      <summary>
        <span>Reference data needs attention</span>
        <span class="catalog-warnings-count">
          {generatedReport.catalog_warnings.length} {generatedReport.catalog_warnings.length === 1 ? 'detail' : 'details'}
        </span>
      </summary>
      <ul>
        {#each generatedReport.catalog_warnings as warning}
          <li>{warning}</li>
        {/each}
      </ul>
    </details>
  {/if}

  {#if generatedReport.import_provenance && generatedReport.import_provenance.liftover_unmapped_rows > 0}
    <details class="catalog-warnings-banner import-quality-banner" aria-label="Imported profile coordinate coverage">
      <summary>
        <span>Profile coordinate coverage</span>
        <span class="catalog-warnings-count">
          {generatedReport.import_provenance.liftover_unmapped_rows.toLocaleString()} records need build mapping
        </span>
      </summary>
      <p>
        {generatedReport.import_provenance.liftover_mapped_rows.toLocaleString()} of
        {generatedReport.import_provenance.diagnostics.accepted_rows.toLocaleString()} accepted records have GRCh38 coordinates.
        The remaining {generatedReport.import_provenance.liftover_unmapped_rows.toLocaleString()} records remain in the local profile,
        but build-specific reference lookups may not include them until a compatible mapping is available.
      </p>
    </details>
  {/if}

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

    <details class="report-filter-details" bind:open={reportFiltersOpen}>
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

      </div>
    </details>

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
        simpleRelatedMarkerCounts={simpleRelatedMarkerCounts}
        collapsed={collapsedSections[section.name]}
        onCollapsedChange={(isCollapsed) => setSectionCollapsed(section.name, isCollapsed)}
      />
    {/each}
  </div>

  <ReferenceIndex report={generatedReport} />

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
  .report-loading-state {
    position: relative;
    display: grid;
    place-items: center;
    width: 100%;
    min-height: clamp(420px, 64vh, 620px);
    padding: clamp(1.5rem, 4vw, 3rem);
    overflow: hidden;
    border: 1px solid var(--border-color);
    border-radius: 1rem;
    background:
      radial-gradient(circle at 50% 0%, color-mix(in srgb, var(--accent) 12%, transparent), transparent 44%),
      linear-gradient(145deg, color-mix(in srgb, var(--surface-raised) 92%, var(--accent)), var(--surface-raised));
    box-shadow: var(--shadow-card);
    box-sizing: border-box;
    text-align: center;
  }

  .report-loading-state::before {
    content: "";
    position: absolute;
    inset: auto 14% -45% 14%;
    height: 70%;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    filter: blur(40px);
    pointer-events: none;
  }

  .report-loading-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: min(100%, 720px);
  }

  .report-loading-heading {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 0.55rem 0.75rem;
  }

  .report-loading-mark {
    display: grid;
    place-items: center;
    width: 3.25rem;
    height: 3.25rem;
    margin-bottom: 0.9rem;
    border: 1px solid var(--status-accent-border);
    border-radius: 1rem;
    background: var(--status-accent-bg);
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 18%, transparent);
    font-size: 1.45rem;
    animation: report-loading-breathe 2.4s ease-in-out infinite;
  }

  .report-loading-kicker {
    color: var(--accent);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .report-loading-device {
    display: inline-flex;
    align-items: center;
    min-height: 1.35rem;
    padding: 0.18rem 0.5rem;
    border: 1px solid var(--status-success-border);
    border-radius: 999px;
    background: var(--status-success-bg);
    color: var(--status-success-text);
    font-family: var(--font-mono), monospace;
    font-size: 0.62rem;
    font-weight: 700;
  }

  .report-loading-state h3 {
    margin: 0.3rem 0 0;
    color: var(--text-primary);
    font-size: clamp(1.25rem, 2vw, 1.6rem);
  }

  .report-loading-lead {
    max-width: 38rem;
    margin: 0.55rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.5;
  }

  .report-loading-activity {
    width: min(100%, 600px);
    margin-top: 1.45rem;
    padding: 1rem;
    border: 1px solid var(--border-strong);
    border-radius: 0.85rem;
    background: color-mix(in srgb, var(--surface-control) 88%, transparent);
    text-align: left;
  }

  .report-loading-activity-heading {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
  }

  .report-loading-activity-heading > div {
    display: grid;
    gap: 0.2rem;
    min-width: 0;
  }

  .report-loading-activity-label {
    color: var(--accent);
    font-size: 0.62rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .report-loading-activity-heading strong {
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.3;
  }

  .report-loading-elapsed {
    flex: 0 0 auto;
    color: var(--text-muted);
    font-family: var(--font-mono), monospace;
    font-size: 0.64rem;
  }

  .report-loading-indeterminate {
    position: relative;
    height: 0.45rem;
    margin-top: 0.85rem;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--border-color) 75%, transparent);
  }

  .report-loading-indeterminate span {
    position: absolute;
    inset: 0 auto 0 -30%;
    width: 42%;
    border-radius: inherit;
    background: linear-gradient(90deg, transparent, var(--accent), var(--success), transparent);
    animation: report-loading-sweep 1.8s ease-in-out infinite;
  }

  .report-loading-activity > p {
    margin: 0.65rem 0 0;
    color: var(--text-secondary);
    font-size: 0.76rem;
    line-height: 1.45;
  }

  .report-loading-steps {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.55rem;
    margin: 1rem 0 0;
    padding: 0;
    list-style: none;
  }

  .report-loading-steps li {
    display: flex;
    align-items: flex-start;
    gap: 0.45rem;
    min-width: 0;
    color: var(--text-muted);
  }

  .report-loading-steps li.active {
    color: var(--text-primary);
  }

  .report-loading-step-node {
    display: grid;
    place-items: center;
    width: 1.35rem;
    height: 1.35rem;
    flex: 0 0 auto;
    border: 1px solid var(--border-color);
    border-radius: 50%;
    color: var(--text-muted);
    font-family: var(--font-mono), monospace;
    font-size: 0.6rem;
  }

  .report-loading-steps li.active .report-loading-step-node {
    border-color: var(--accent);
    background: var(--status-accent-bg);
    color: var(--accent);
    box-shadow: 0 0 0 0.2rem color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .report-loading-steps li > span:last-child {
    display: grid;
    gap: 0.08rem;
    min-width: 0;
  }

  .report-loading-steps strong {
    color: inherit;
    font-size: 0.68rem;
    line-height: 1.3;
  }

  .report-loading-steps small {
    color: var(--text-muted);
    font-size: 0.58rem;
  }

  .report-loading-steps li.active small {
    color: var(--accent);
  }

  .report-loading-meta {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem 0.75rem;
    margin-top: 0.15rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .report-loading-meta > span {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .report-loading-dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--status-success-text);
    box-shadow: 0 0 0.55rem color-mix(in srgb, var(--status-success-text) 55%, transparent);
  }

  @keyframes report-loading-breathe {
    0%,
    100% {
      transform: translateY(0) scale(1);
    }
    50% {
      transform: translateY(-2px) scale(1.04);
    }
  }

  @keyframes report-loading-sweep {
    0% {
      transform: translateX(0);
    }
    100% {
      transform: translateX(310%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .report-loading-mark,
    .report-loading-indeterminate span {
      animation: none;
    }
  }

  @media (max-width: 700px) {
    .report-loading-steps {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 480px) {
    .report-loading-activity-heading {
      align-items: flex-start;
      flex-direction: column;
      gap: 0.35rem;
    }

    .report-loading-steps {
      grid-template-columns: 1fr;
    }
  }

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
