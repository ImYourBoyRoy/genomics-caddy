<!-- ./src/lib/components/report/ReportView.svelte -->
<script lang="ts">
  import type { GeneratedReport, NormalizedReport, GenomeSample, SeverityClass } from '../../types/genomics';
  import { saveReportBundle, saveReportJson, exportDiscoveryFindings } from '../../api/tauri';
  import { dialogStore } from '../../utils/dialogState.svelte';
  import ReportHeader from './ReportHeader.svelte';
  import ReportLoadingState from './ReportLoadingState.svelte';
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
  import { getRuntimeAppVersion } from '../../utils/appVersion';

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
    onOpenHelp?: () => void;
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
    onOpenHelp,
  }: Props = $props();

  let showBenign = $state(false);
  let severityFilter = $state<"all" | "higher_concern" | "priority">("all");
  let findingSearch = $state("");
  let sectionFilter = $state("all");
  let tierFilter = $state<string>("all");
  let sortBy = $state<"default" | "severity">("default");
  let reportFiltersOpen = $state(false);
  let presentationMode = $state<PresentationMode>(DEFAULT_PRESENTATION_MODE);
  let loadedPresentationModeKey = $state("");
  let collapsedSections = $state<Record<string, boolean>>({});
  let loadedCollapseProfileId = $state<number | null>(null);
  let isPreparingPrint = $state(false);
  let printRestore: (() => void) | null = null;
  let clinicalProvenanceExpanded = $state(false);
  let showClinicalExtraColumns = $state(false);

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

  let quickFilterCounts = $derived.by(() => {
    const sections = generatedReport?.sections.map((section) => ({
      ...section,
      markers: section.markers.filter((marker) => marker.severity_class !== 'benign' && marker.severity_class !== 'no_data'),
    })) ?? [];
    const signals = buildCanonicalFindingGroups({ sections });
    return {
      all: signals.length,
      higherConcern: signals.filter((signal) =>
        signal.severityClasses.some((severity) => severity === 'high_risk' || severity === 'confirmation_required'),
      ).length,
      priority: signals.filter((signal) =>
        signal.severityClasses.some((severity) =>
          severity === 'high_risk' || severity === 'moderate_risk' || severity === 'confirmation_required',
        ),
      ).length,
    };
  });

  function matchesFindingSearch(marker: GeneratedReport['sections'][number]['markers'][number], sectionName: string): boolean {
    const query = findingSearch.trim().toLocaleLowerCase();
    if (!query) return true;
    const searchable = [
      sectionName,
      marker.gene,
      marker.rsid,
      marker.variant_name,
      marker.impact,
      marker.interpretation,
      marker.clinvar_conditions,
      marker.gwas_top_trait,
      marker.clinical_semantics?.condition_label,
    ].filter(Boolean).join(' ').toLocaleLowerCase();
    return searchable.includes(query);
  }

  function clearReportFilters() {
    findingSearch = '';
    sectionFilter = 'all';
    severityFilter = 'all';
    tierFilter = 'all';
    showBenign = false;
    sortBy = 'default';
  }

  let filteredSourceSections = $derived(
    generatedReport?.sections.map(sec => {
      let markers = sec.markers.filter(m => {
        if (!showBenign && (m.severity_class === "benign" || m.severity_class === "no_data")) return false;
        if (tierFilter === "ab" && !m.evidence_tier.startsWith("A") && !m.evidence_tier.startsWith("B")) return false;
        if (severityFilter === "higher_concern" &&
            m.severity_class !== "high_risk" &&
            m.severity_class !== "confirmation_required") {
          return false;
        }
        if (severityFilter === "priority" &&
            m.severity_class !== "high_risk" &&
            m.severity_class !== "moderate_risk" &&
            m.severity_class !== "confirmation_required") {
          return false;
        }
        if (!matchesFindingSearch(m, sec.name)) return false;
        return true;
      });

      if (sortBy === "severity") {
        markers = [...markers].sort((a, b) => getSeverityRank(a.severity_class) - getSeverityRank(b.severity_class));
      }

      return {
        ...sec,
        markers
      };
    }).filter(sec => (sectionFilter === 'all' || sec.name === sectionFilter) && sec.markers.length > 0) ?? []
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
  let visibleMarkerCount = $derived(
    presentationMode === 'simple'
      ? simpleCanonicalGroups.length
      : filteredSourceSections.reduce((total, section) => total + section.markers.length, 0),
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
      const appVersion = await getRuntimeAppVersion();
      const envelope = {
        schema_version: "2.0.0",
        app_version: appVersion,
        export_kind: "curated_marker_packs",
        exported_at: new Date().toISOString(),
        sample: {
          name: selectedSample.name,
          chromosome_call_context: selectedSample.genetic_sex,
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
  {#key selectedSample.id}
    <ReportLoadingState sampleName={selectedSample.name} />
  {/key}
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
        onclick={() => onOpenHelp?.()}
      >
        Help &amp; app info
      </button>
      </div>
    </div>

    <div class="quick-finding-filters" aria-label="Find and filter report entries">
      <label class="quick-search-label">
        <span>Find a gene, marker, or condition</span>
        <input type="search" bind:value={findingSearch} placeholder="Search this report" aria-label="Search report genes, markers, conditions, and health areas" />
      </label>
      <div class="quick-filter-block">
        <span class="filter-label">Focus</span>
        <div class="quick-filter-buttons" role="group" aria-label="Quick finding filters">
          <button type="button" class:quick-filter-active={severityFilter === 'all'} aria-pressed={severityFilter === 'all'} onclick={() => severityFilter = 'all'}>
            All findings <span>{quickFilterCounts.all}</span>
          </button>
          <button type="button" class:quick-filter-active={severityFilter === 'higher_concern'} aria-pressed={severityFilter === 'higher_concern'} onclick={() => severityFilter = 'higher_concern'}>
            Higher concern <span>{quickFilterCounts.higherConcern}</span>
          </button>
          <button type="button" class:quick-filter-active={severityFilter === 'priority'} aria-pressed={severityFilter === 'priority'} onclick={() => severityFilter = 'priority'}>
            Review first <span>{quickFilterCounts.priority}</span>
          </button>
        </div>
      </div>
      <p class="quick-filter-result" role="status" aria-live="polite">
        Showing {visibleMarkerCount.toLocaleString()} {presentationMode === 'simple'
          ? (visibleMarkerCount === 1 ? 'finding card' : 'finding cards')
          : (visibleMarkerCount === 1 ? 'entry' : 'entries')}.
        <span>Filter counts are unique DNA signals; detailed views may show one signal in more than one health area.</span>
      </p>
      {#if findingSearch || sectionFilter !== 'all' || severityFilter !== 'all' || tierFilter !== 'all' || showBenign || sortBy !== 'default'}
        <button type="button" class="clear-report-filters" onclick={clearReportFilters}>Clear filters</button>
      {/if}
    </div>

    {#if presentationMode === 'clinical'}
      <label class="clinical-extra-columns-toggle">
        <input type="checkbox" bind:checked={showClinicalExtraColumns} />
        Show additional clinical fields
      </label>
    {/if}

    <details class="report-filter-details" bind:open={reportFiltersOpen}>
      <summary>Filters &amp; ordering</summary>
      <div class="filter-bar card">
    <div class="filter-group">
      <span class="filter-label">Show</span>
      <label class="filter-toggle">
        <input type="checkbox" bind:checked={showBenign} aria-label="Show benign and uncalled markers" />
        Show benign &amp; uncalled
      </label>
      <select class="filter-select" aria-label="Evidence tier filter" bind:value={tierFilter}>
        <option value="all">All evidence tiers</option>
        <option value="ab">Tier A & B only</option>
      </select>
      <select class="filter-select" aria-label="Health area filter" bind:value={sectionFilter}>
        <option value="all">All health areas</option>
        {#each generatedReport.sections as section (section.name)}
          <option value={section.name}>{section.name}</option>
        {/each}
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
        showClinicalExtraColumns={showClinicalExtraColumns}
        collapsed={collapsedSections[section.name]}
        onCollapsedChange={(isCollapsed) => setSectionCollapsed(section.name, isCollapsed)}
      />
    {/each}
  </div>

  <ReferenceIndex report={generatedReport} />

  <!-- Secondary research surfaces stay below the core report and remain optional. -->
  <VectorPromotedSection {selectedSample} {presentationMode} {highlightRsid} {onExploreResearch} onNavigate={onNavigateToVariant} />
  <DiscoveredFindingsBanner {selectedSample} {presentationMode} {onExploreResearch} onNavigate={onNavigateToVariant} />

{:else}
  <div class="report-idle-empty" role="status">
    <strong>No report loaded yet</strong>
    <p>
      Import a genome and select its profile. You can browse local DNA calls while reference catalogs download;
      database-backed findings become more complete as those catalogs are installed and indexed.
    </p>
  </div>
{/if}

<style>
  .quick-finding-filters {
    display: grid;
    grid-template-columns: minmax(14rem, 0.85fr) minmax(0, 1.5fr);
    gap: 0.65rem 1rem;
    align-items: end;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border-color);
    border-radius: 0.7rem;
    background: var(--surface-subtle);
  }

  .quick-search-label,
  .quick-filter-block {
    display: grid;
    gap: 0.35rem;
    min-width: 0;
  }

  .quick-search-label > span,
  .quick-filter-block > .filter-label {
    color: var(--text-secondary);
    font-size: 0.76rem;
    font-weight: 650;
  }

  .quick-search-label input {
    box-sizing: border-box;
    width: 100%;
    min-width: 0;
    min-height: 2.7rem;
    padding: 0.45rem 0.65rem;
    border: 1px solid var(--border-color);
    border-radius: 0.45rem;
    background: var(--surface-raised);
    color: var(--text-primary);
    font: inherit;
  }

  .quick-search-label input:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }

  .quick-filter-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .quick-filter-buttons button {
    display: inline-flex;
    min-height: 2.7rem;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.6rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    background: var(--surface-raised);
    color: var(--text-secondary);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 650;
    cursor: pointer;
  }

  .quick-filter-buttons button:hover,
  .quick-filter-buttons button:focus-visible {
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .quick-filter-buttons button:focus-visible,
  .clear-report-filters:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }

  .quick-filter-buttons button.quick-filter-active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  .quick-filter-buttons button span {
    min-width: 1.35rem;
    padding: 0.08rem 0.25rem;
    border-radius: 999px;
    background: var(--surface-subtle);
    color: var(--text-secondary);
    font-size: 0.72rem;
    text-align: center;
  }

  .quick-filter-result {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.55rem;
    margin: -0.1rem 0 0;
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .quick-filter-result span { opacity: 0.85; }

  .clear-report-filters {
    grid-column: 2;
    justify-self: end;
    min-height: 2rem;
    padding: 0.25rem 0.45rem;
    border: 0;
    border-radius: 0.4rem;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: 0.76rem;
    font-weight: 650;
    cursor: pointer;
  }

  .clear-report-filters:hover { background: var(--accent-soft); }

  .clinical-extra-columns-toggle {
    display: flex;
    min-height: 2.5rem;
    align-items: center;
    gap: 0.45rem;
    color: var(--text-secondary);
    font-size: 0.8rem;
  }

  .clinical-extra-columns-toggle input { width: 1rem; height: 1rem; accent-color: var(--accent); }

  @media (max-width: 760px) {
    .quick-finding-filters { grid-template-columns: minmax(0, 1fr); }
    .quick-filter-result, .clear-report-filters { grid-column: 1; }
    .clear-report-filters { justify-self: start; }
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

</style>
