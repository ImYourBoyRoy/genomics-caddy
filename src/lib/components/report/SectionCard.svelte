<!-- ./src/lib/components/report/SectionCard.svelte -->
<script lang="ts">
  import type { EvaluatedSection } from '../../types/genomics';
  import { getSectionSummaryParts } from '../../utils/evidence';
  import VariantCard from './VariantCard.svelte';
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
    viewMode: "simple" | "clinical" | "dual";
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

  // Collapse/expand state — persisted to localStorage so it survives report regeneration.
  let storageKey = $derived(`section-collapsed-${section.name}`);

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
  let activeCount = $derived(section.summary.active_marker_count ?? 0);
  let noDataCount = $derived(section.summary.no_data_count ?? 0);
  let callableCount = $derived(Math.max(0, section.summary.total_markers - noDataCount));
  let coveragePercent = $derived(
    section.summary.total_markers > 0
      ? Math.round((callableCount / section.summary.total_markers) * 100)
      : 0
  );
</script>

<div class="section-card card" class:collapsed-card={isCollapsed}>
  <div 
    class="section-header" 
    onclick={toggleCollapse}
    onkeydown={(e) => e.key === 'Enter' && toggleCollapse()}
    role="button" 
    tabindex="0"
    style="cursor: pointer; user-select: none; border-bottom: {isCollapsed ? 'none' : '1px solid var(--border-color)'}; padding-bottom: {isCollapsed ? '0' : '10px'}; margin-bottom: {isCollapsed ? '0' : '16px'};"
  >
    <div class="section-title-area" style="display: flex; align-items: center; gap: 8px;">
      <span class="collapse-icon" style="opacity: 0.6; font-size: 0.8rem; width: 12px; display: inline-block; transition: transform 0.2s;">
        {isCollapsed ? '▶' : '▼'}
      </span>
      <h4 style="margin: 0;">{section.name}</h4>
      <span class="pill" style="font-size: 0.7rem; padding: 2px 6px; background: rgba(255, 255, 255, 0.05); color: var(--text-secondary);">
        {section.markers.length} {section.markers.length === 1 ? 'variant' : 'variants'}
      </span>
      {#if activeCount > 0}
        <span class="pill active-pill" style="font-size: 0.7rem; padding: 2px 6px; background: rgba(239, 68, 68, 0.12); color: #f87171;">
          {activeCount} active {activeCount === 1 ? 'finding' : 'findings'}
        </span>
      {/if}
      <span class="pill coverage-pill" title="A missing or uncalled marker is unknown, not evidence of low risk.">
        DNA calls: {callableCount}/{section.summary.total_markers} ({coveragePercent}%)
      </span>
    </div>

    <div class="section-score-area">
      {#if showPercent}
        <span class="sec-score">
          Matched alleles: {section.section_signal_score.toFixed(1)}%
        </span>
        <span class="sec-score-hint">
          (association-direction pack markers only)
        </span>
      {:else if section.summary.all_require_confirmation}
        <span class="sec-score-badge badge-warning" title="High-stakes clinical variants require medical-grade confirmation before assigning raw risk percentages.">
          ⚠️ Clinical Validation Required
        </span>
      {:else if section.summary.risk_possible === 0}
        <span class="sec-score-badge badge-info" title="This category contains environmental modifiers, dietary, and lifestyle traits rather than direct risk indicators.">
          ℹ️ Context &amp; Modifier Traits
        </span>
      {:else}
        <span class="sec-score-descriptive">
          {section.summary.total_markers} markers evaluated
        </span>
      {/if}
    </div>
  </div>

<style>
  .sec-score-badge {
    display: inline-block;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 600;
    text-align: right;
    letter-spacing: 0.01em;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    border: 1px solid transparent;
  }

  .badge-warning {
    background: rgba(217, 119, 6, 0.12);
    color: #fbbf24;
    border-color: rgba(217, 119, 6, 0.3);
  }

  .badge-info {
    background: rgba(14, 165, 233, 0.15);
    color: #38bdf8;
    border-color: rgba(14, 165, 233, 0.3);
  }

  .sec-score-descriptive {
    font-size: 0.8rem;
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .coverage-pill {
    color: #c4b5fd;
    background: rgba(139, 92, 246, 0.1);
    border: 1px solid rgba(139, 92, 246, 0.22);
  }

  .coverage-note {
    color: #cbd5e1;
    background: rgba(148, 163, 184, 0.1);
  }
</style>

  {#if !isCollapsed}
    <div class="section-body" transition:slide={{ duration: 200 }}>
      <!-- Direction-aware breakdown pills -->
      <div class="section-summary-pills">
        {#each summaryParts as part}
          <span class="summary-pill">{part}</span>
        {/each}
        {#if noDataCount > 0}
          <span class="summary-pill coverage-note">{noDataCount} not called; unknown, not negative</span>
        {/if}
      </div>

      <div class="markers-grid">
        {#each section.markers as marker}
          <VariantCard {marker} {viewMode} {onExploreResearch} {highlightRsid} {onNavigateToVariant} />
        {/each}
      </div>
    </div>
  {/if}
</div>
