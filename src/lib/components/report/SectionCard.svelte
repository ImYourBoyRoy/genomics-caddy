<!-- ./src/lib/components/report/SectionCard.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { EvaluatedSection } from '../../types/genomics';
  import { getSectionSummaryParts } from '../../utils/evidence';
  import VariantCard from './VariantCard.svelte';
  import ClinicalFindingsTable from './ClinicalFindingsTable.svelte';
  import Tooltip from '../common/Tooltip.svelte';
  import { slide } from 'svelte/transition';
  import { browser } from '$app/environment';

  /*
  Module Docstring:
  Purpose: Card representing a thematic section of the genetic report.
  Responsibilities:
  - Display category title and either a risk-only percent score OR a
    descriptive plain-language summary (for sections like cancer/PGx).
  - Show direction-aware tallied counts so users see the breakdown.
  - Persist collapse/expand state to localStorage across report regenerations.
  Key Inputs: section (EvaluatedSection).
  Key Outputs: Category report segment.
  Operational Notes: Uses SectionSummary from backend to decide display mode.
  */

  import type { VariantNavTarget } from '../../constants/traitCategories';

  interface Props {
    section: EvaluatedSection;
    viewMode: "simple" | "clinical" | "compare";
    onExploreResearch?: (rsid: string) => void;
    highlightRsid?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
    collapsed?: boolean;
  }

  let {
    section,
    viewMode,
    onExploreResearch,
    highlightRsid = "",
    onNavigateToVariant,
    collapsed = $bindable()
  }: Props = $props();

  // If collapsed is undefined, we default to true (collapsed by default)
  let isCollapsed = $derived(collapsed ?? true);
  let reduceMotion = $state(true);

  onMount(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const updateMotionPreference = () => {
      reduceMotion = mediaQuery.matches;
    };

    updateMotionPreference();
    mediaQuery.addEventListener('change', updateMotionPreference);
    return () => mediaQuery.removeEventListener('change', updateMotionPreference);
  });

  // Collapse/expand state — persisted to localStorage so it survives report regeneration.
  let storageKey = $derived(`section-collapsed-${section.name}`);
  let sectionBodyId = $derived(`section-body-${section.name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`);

  // Write new collapse state to localStorage whenever it changes
  $effect(() => {
    if (browser) {
      localStorage.setItem(storageKey, String(isCollapsed));
    }
  });

  function toggleCollapse() {
    collapsed = !isCollapsed;
  }

  let summaryParts = $derived(getSectionSummaryParts(section.summary));
  let showPercent = $derived(section.summary.show_percent_score);
  let activeCount = $derived(
    section.markers.filter((marker) => marker.severity_class !== 'benign' && marker.severity_class !== 'no_data').length
  );
  let noDataCount = $derived(section.summary.no_data_count ?? 0);
  let callableCount = $derived(Math.max(0, section.summary.total_markers - noDataCount));
  let coveragePercent = $derived(
    section.summary.total_markers > 0
      ? Math.round((callableCount / section.summary.total_markers) * 100)
      : 0
  );
</script>

<div class="section-card card" class:collapsed-card={isCollapsed}>
  <div class="section-header">
    <div class="section-title-area">
      <button
        type="button"
        class="section-toggle"
        onclick={toggleCollapse}
        aria-expanded={!isCollapsed}
        aria-controls={sectionBodyId}
      >
        <span class="collapse-icon" aria-hidden="true">
          {isCollapsed ? '▶' : '▼'}
        </span>
        <h4>{section.name}</h4>
        <span class="pill section-count-pill">
          {section.markers.length} shown
        </span>
        {#if activeCount > 0}
          <span class="pill active-pill section-active-pill">
            {activeCount} active {activeCount === 1 ? 'finding' : 'findings'}
          </span>
        {/if}
      </button>
      <Tooltip
        label="DNA call coverage"
        description="A missing or uncalled marker is unknown, not evidence of low risk."
      >
        <span class="pill coverage-pill">
          DNA calls: {callableCount}/{section.summary.total_markers} ({coveragePercent}%)
        </span>
      </Tooltip>
    </div>

    <div class="section-score-area">
      {#if showPercent && viewMode === 'simple'}
        <span class="sec-score-descriptive">Association context</span>
      {:else if showPercent}
        <span class="sec-score">
          Association match rate: {section.section_signal_score.toFixed(1)}%
        </span>
        <span class="sec-score-hint">
          (curated association markers only)
        </span>
      {:else if section.summary.all_require_confirmation}
        <Tooltip label="Clinical validation required" description="High-stakes clinical variants require medical-grade confirmation before assigning risk estimates.">
          <span class="sec-score-badge badge-warning">⚠️ Clinical validation required</span>
        </Tooltip>
      {:else if section.summary.risk_possible === 0}
        <Tooltip label="Context and modifier traits" description="This category contains environmental, dietary, lifestyle, or other context markers rather than direct disease indicators.">
          <span class="sec-score-badge badge-info">ℹ️ Context and modifier traits</span>
        </Tooltip>
      {:else}
        <span class="sec-score-descriptive">
          {section.summary.total_markers} markers evaluated
        </span>
      {/if}
    </div>
  </div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    min-height: 2.75rem;
    user-select: none;
  }

  .section-title-area {
    display: flex;
    flex: 1 1 30rem;
    min-width: 0;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .section-toggle {
    display: flex;
    flex: 1 1 auto;
    min-width: 0;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0.35rem;
    border: 1px solid transparent;
    border-radius: 0.45rem;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .section-toggle:hover {
    background: var(--surface-subtle);
  }

  .section-toggle:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .collapse-icon {
    width: 1rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
    transition: transform 160ms ease;
  }

  .section-title-area h4 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
    overflow-wrap: anywhere;
  }

  .section-count-pill,
  .section-active-pill {
    padding: 0.2rem 0.45rem;
    font-size: 0.68rem;
  }

  .section-count-pill {
    background: var(--surface-subtle);
    color: var(--text-secondary);
  }

  .section-active-pill {
    background: var(--status-danger-bg);
    color: var(--status-danger-strong-text);
    border: 1px solid var(--status-danger-border);
  }

  .section-score-area {
    display: flex;
    flex: 0 1 22rem;
    max-width: 100%;
    min-width: 0;
    align-items: center;
    justify-content: flex-end;
    gap: 0.4rem;
    flex-wrap: wrap;
    text-align: right;
  }

  .section-score-area > * {
    max-width: 100%;
    overflow-wrap: anywhere;
  }

  .section-body {
    border-top: 1px solid var(--border-color);
    margin-top: 0.5rem;
    padding-top: 1rem;
  }

  .sec-score-badge {
    display: inline-block;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 600;
    text-align: right;
    letter-spacing: 0.01em;
    box-shadow: 0 2px 8px var(--shadow-subtle);
    border: 1px solid transparent;
  }

  .badge-warning {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
    border-color: var(--status-warning-border);
  }

  .badge-info {
    background: var(--status-info-bg);
    color: var(--status-info-text);
    border-color: var(--status-info-border);
  }

  .sec-score-descriptive {
    font-size: 0.8rem;
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .coverage-pill {
    color: var(--coverage-text);
    background: var(--coverage-bg);
    border: 1px solid var(--coverage-border);
  }

  .coverage-note {
    color: var(--coverage-note-text);
    background: var(--coverage-note-bg);
  }

  @media (max-width: 720px) {
    .section-header,
    .section-score-area {
      align-items: flex-start;
      flex-direction: column;
    }

    .section-score-area {
      justify-content: flex-start;
      text-align: left;
    }
  }
</style>

  {#if !isCollapsed}
    <div class="section-body" id={sectionBodyId} transition:slide={{ duration: reduceMotion ? 0 : 200 }}>
      <!-- Direction-aware breakdown pills -->
      <div class="section-summary-pills">
        {#each summaryParts as part}
          <span class="summary-pill">{part}</span>
        {/each}
        {#if noDataCount > 0}
          <span class="summary-pill coverage-note">{noDataCount} not called; unknown, not negative</span>
        {/if}
      </div>

      {#if viewMode === 'clinical'}
        <ClinicalFindingsTable
          markers={section.markers}
          {onExploreResearch}
          {highlightRsid}
          {onNavigateToVariant}
        />
      {:else}
        <div class="markers-grid">
          {#each section.markers as marker}
            <VariantCard {marker} {viewMode} {onExploreResearch} {highlightRsid} {onNavigateToVariant} />
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
