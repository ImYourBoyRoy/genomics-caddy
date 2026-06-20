<!-- ./src/lib/components/report/SectionCard.svelte -->
<script lang="ts">
  import type { EvaluatedSection } from '../../types/genomics';
  import { getSectionSummaryParts } from '../../utils/evidence';
  import VariantCard from './VariantCard.svelte';

  /*
  Module Docstring:
  Purpose: Card representing a thematic section of the genetic report.
  Responsibilities:
  - Display category title and either a risk-only percent score OR a
    descriptive plain-language summary (for sections like cancer/PGx).
  - Show direction-aware tallied counts so users see the breakdown.
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
  }

  let { section, viewMode, onExploreResearch, highlightRsid = "", onNavigateToVariant }: Props = $props();

  let summaryParts = $derived(getSectionSummaryParts(section.summary));
  let showPercent = $derived(section.summary.show_percent_score);
</script>

<div class="section-card card">
  <div class="section-header">
    <h4>{section.name}</h4>
    <div class="section-score-area">
      {#if showPercent}
        <span class="sec-score">
          Risk Signal: {section.section_signal_score.toFixed(1)}%
        </span>
        <span class="sec-score-hint">
          (only counts risk-direction markers)
        </span>
      {:else}
        <span class="sec-score-descriptive">
          {section.summary.total_markers} markers evaluated
        </span>
      {/if}
    </div>
  </div>

  <!-- Direction-aware breakdown pills -->
  <div class="section-summary-pills">
    {#each summaryParts as part}
      <span class="summary-pill">{part}</span>
    {/each}
  </div>

  <div class="markers-grid">
    {#each section.markers as marker}
      <VariantCard {marker} {viewMode} {onExploreResearch} {highlightRsid} {onNavigateToVariant} />
    {/each}
  </div>
</div>
