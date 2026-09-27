<!-- ./src/lib/components/report/ReportHeader.svelte -->
<script lang="ts">
  import type { GeneratedReport } from '../../types/genomics';
  import { computeReportOverviewStats, type ReportFocus, type ReportOverviewStats } from '../../utils/reportOverview';

  /*
  Module Docstring:
  Purpose: Report header dashboard presenting summary statistics.
  Responsibilities:
  - Display how many curated report marker checks have a DNA call.
  - Add explanatory subtitle so users understand what the score represents.
  - Summarize basic user biological statistics.
  Key Inputs: generatedReport, foundMarkersCount, totalMarkersChecked.
  Key Outputs: Visual report header element.
  Operational Notes: Coverage is for curated report markers, not the whole genome.
  */

  interface Props {
    generatedReport: GeneratedReport;
    foundMarkersCount: number;
    totalMarkersChecked: number;
    presentationMode?: "simple" | "clinical" | "compare";
    activeFocus?: ReportFocus;
    onSelectFocus?: (focus: ReportFocus) => void;
  }

  let {
    generatedReport,
    foundMarkersCount,
    totalMarkersChecked,
    presentationMode = "simple",
    activeFocus = "all",
    onSelectFocus,
  }: Props = $props();

  let reportStats = $derived<ReportOverviewStats>(computeReportOverviewStats(generatedReport));
  let reviewQueueCount = $derived(Math.min(reportStats.priority, 9));
  let coveragePercent = $derived(
    totalMarkersChecked > 0
      ? Math.min(100, Math.max(0, Math.round((foundMarkersCount / totalMarkersChecked) * 100)))
      : null,
  );

  function computeSummaryLine(stats: ReportOverviewStats, mode: "simple" | "clinical" | "compare"): string {
    const high = stats.higherConcern;
    const mod = Math.max(0, stats.priority - stats.higherConcern);
    const prot = stats.protective;
    if (mode === 'simple') {
      const associationCount = high + mod;
      const simpleParts = [];
      if (associationCount > 0) {
        simpleParts.push(`${associationCount} research ${associationCount === 1 ? 'finding' : 'findings'} to review`);
      }
      if (prot > 0) {
        simpleParts.push(`${prot} protective-context ${prot === 1 ? 'finding' : 'findings'}`);
      }
      if (simpleParts.length === 0) return "No prioritized findings in this report.";
      return simpleParts.join(' · ');
    }

    const parts = [];
    if (high > 0) parts.push(`${high} stronger ${high === 1 ? 'association' : 'associations'}`);
    if (mod > 0) parts.push(`${mod} possible ${mod === 1 ? 'association' : 'associations'}`);
    if (prot > 0) parts.push(`${prot} protective ${prot === 1 ? 'variant' : 'variants'}`);
    
    if (parts.length === 0) return "No active associations or protective variants detected.";
    return parts.join(', ') + ' among curated pack markers.';
  }

  let summaryLine = $derived(computeSummaryLine(reportStats, presentationMode));
</script>

<div class="report-header card" data-presentation-mode={presentationMode}>
  {#if presentationMode === 'simple'}
    <div class="simple-report-overview" aria-label="Report overview">
      <div class="simple-overview-intro">
        <div class="overview-kicker-row">
          <span class="quality-kicker">Report overview</span>
          <span class="overview-queue-badge">
            <span class="overview-queue-dot" aria-hidden="true"></span>
            {reviewQueueCount} highlighted
          </span>
        </div>
        <h3>Your DNA overview</h3>
        <p>The most useful DNA signals found in this profile.</p>
      </div>
      <div class="report-stat-grid" role="group" aria-label="Report overview statistics — select one to list those findings">
        <button type="button" class="report-stat report-stat-priority" class:report-stat-active={activeFocus === 'priority'} aria-pressed={activeFocus === 'priority'} disabled={!onSelectFocus} onclick={() => onSelectFocus?.('priority')}>
          <strong>{reviewQueueCount.toLocaleString()}</strong>
          <span>Review first</span>
          <small>top {reviewQueueCount.toLocaleString()} of {reportStats.priority.toLocaleString()} reviewable signals</small>
        </button>
        <button type="button" class="report-stat report-stat-high" class:report-stat-active={activeFocus === 'higher_concern'} aria-pressed={activeFocus === 'higher_concern'} disabled={!onSelectFocus} onclick={() => onSelectFocus?.('higher_concern')}>
          <strong>{reportStats.higherConcern.toLocaleString()}</strong>
          <span>Higher-priority</span>
        </button>
        <button type="button" class="report-stat report-stat-context" class:report-stat-active={activeFocus === 'context'} aria-pressed={activeFocus === 'context'} disabled={!onSelectFocus} onclick={() => onSelectFocus?.('context')}>
          <strong>{reportStats.context.toLocaleString()}</strong>
          <span>Context signals</span>
        </button>
        <button type="button" class="report-stat report-stat-protective" class:report-stat-active={activeFocus === 'protective'} aria-pressed={activeFocus === 'protective'} disabled={!onSelectFocus} onclick={() => onSelectFocus?.('protective')}>
          <strong>{reportStats.protective.toLocaleString()}</strong>
          <span>Potentially favorable</span>
        </button>
      </div>
      <details class="technical-score-details simple-overview-coverage">
        <summary>
          <span class="overview-coverage-label">
            <span>Markers with a DNA result</span>
            <small>{#if totalMarkersChecked > 0}{foundMarkersCount.toLocaleString()} of {totalMarkersChecked.toLocaleString()} report markers{:else}No report markers available{/if}</small>
          </span>
          <strong title="Share of report markers with a usable DNA result. This is not the percentage of your whole genome that was read.">
            {coveragePercent === null ? '—' : `${coveragePercent}%`}
          </strong>
        </summary>
        {#if coveragePercent !== null}
          <div
            class="overview-coverage-meter"
            role="progressbar"
            aria-label="Curated report marker call coverage"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={coveragePercent}
          >
            <span style={`width: ${coveragePercent}%`}></span>
          </div>
          <p>This is the share of markers used in this report that have a usable DNA result. It is not the percentage of your whole genome that was read. A marker without a result is unknown.</p>
        {:else}
          <p>No curated report markers were available to check.</p>
        {/if}
      </details>
    </div>
  {:else}
    <div class="report-quality-summary" aria-label="Report data quality summary">
      <span class="quality-kicker">Curated DNA marker calls</span>
      {#if totalMarkersChecked > 0}
        <strong>{foundMarkersCount.toLocaleString()} / {totalMarkersChecked.toLocaleString()}</strong>
        <span>usable calls / marker checks</span>
        <span class="quality-note">A missing call is unknown</span>
      {:else}
        <strong>—</strong>
        <span>No curated marker checks available</span>
      {/if}
    </div>
    <div class="report-desc">
      <div class="header-title-row">
        <h3>{generatedReport.title}</h3>
      </div>
      <p>{generatedReport.description}</p>
      <span class="overall-summary" aria-label="Report finding summary">{summaryLine}</span>
    </div>
  {/if}
</div>

<style>
  .report-header[data-presentation-mode="simple"] {
    display: block;
    width: min(100%, var(--report-dashboard-surface-width));
    margin-inline: auto;
    box-sizing: border-box;
  }

  .simple-report-overview {
    display: grid;
    grid-template-columns: minmax(14rem, 1fr) minmax(0, 2.4fr) minmax(9rem, 13rem);
    align-items: center;
    gap: 1rem;
  }

  .simple-overview-intro,
  .report-stat-grid,
  .simple-overview-coverage {
    min-width: 0;
  }

  .simple-overview-intro h3 {
    margin: 0.25rem 0 0;
    color: var(--text-primary);
    font-size: 1.1rem;
  }

  .overview-kicker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .overview-queue-badge {
    display: inline-flex;
    min-width: 0;
    max-width: 100%;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.45rem;
    border: 1px solid var(--border-color);
    border-radius: 999px;
    color: var(--text-secondary);
    font-size: 0.64rem;
    font-weight: 700;
    line-height: 1.2;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .overview-queue-dot {
    width: 0.4rem;
    height: 0.4rem;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--status-warning-border);
  }

  .simple-overview-intro p {
    margin: 0.35rem 0 0;
    color: var(--text-secondary);
    font-size: 0.88rem;
    line-height: 1.35;
  }

  .report-stat-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(6.5rem, 1fr));
    gap: 0.5rem;
  }

  .report-stat {
    display: flex;
    min-width: 0;
    min-height: 3.6rem;
    flex-direction: column;
    justify-content: center;
    gap: 0.15rem;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--border-color);
    border-left: 3px solid var(--border-strong);
    border-radius: 0.55rem;
    background: var(--surface-subtle);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }

  .report-stat:disabled {
    cursor: default;
  }

  .report-stat:not(:disabled):hover,
  .report-stat.report-stat-active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .report-stat:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .report-stat strong {
    color: var(--text-primary);
    font-size: 1.2rem;
    line-height: 1;
  }

  .report-stat span {
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .report-stat small {
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.2;
    overflow-wrap: anywhere;
  }

  .report-stat-priority { border-left-color: var(--status-warning-border); }
  .report-stat-high { border-left-color: var(--status-danger-border); }
  .report-stat-context { border-left-color: var(--status-caution-border); }
  .report-stat-protective { border-left-color: var(--status-success-border); }

  .simple-overview-coverage {
    margin: 0;
  }

  .simple-overview-coverage summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.65rem;
  }

  .overview-coverage-label {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }

  .overview-coverage-label small {
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-weight: 500;
  }

  .simple-overview-coverage summary > strong {
    color: var(--accent);
    font-size: 1.2rem;
    line-height: 1;
  }

  .overview-coverage-meter {
    height: 0.35rem;
    margin-top: 0.65rem;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-subtle);
  }

  .overview-coverage-meter span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  .report-quality-summary {
    display: flex;
    flex: 0 0 170px;
    flex-direction: column;
    gap: 0.25rem;
    padding: 1rem;
    border: 1px solid var(--border-color);
    border-radius: 0.75rem;
    background: var(--surface-subtle);
  }

  .report-quality-summary strong {
    color: var(--accent);
    font-size: 1.65rem;
    line-height: 1;
  }

  .report-quality-summary > span:not(.quality-kicker) {
    color: var(--text-secondary);
    font-size: 0.82rem;
    line-height: 1.35;
  }

  .quality-kicker {
    color: var(--text-primary);
    font-size: 0.74rem;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .quality-note {
    margin-top: 0.25rem;
  }

  .header-title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .header-title-row h3 {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .report-desc {
    flex: 1 1 auto;
    min-width: 0;
  }

  .overall-summary {
    display: block;
    max-width: 100%;
    margin-top: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.84rem;
    font-weight: 600;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .report-desc > p {
    overflow-wrap: anywhere;
  }

  .technical-score-details {
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.45;
  }

  .technical-score-details summary {
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 700;
  }

  .technical-score-details p {
    margin: 0.5rem 0 0;
  }

  @media (max-width: 720px) {
    .report-header {
      align-items: stretch;
      flex-direction: column;
    }

    .simple-report-overview {
      grid-template-columns: 1fr;
    }

    .overview-kicker-row {
      align-items: flex-start;
      flex-direction: column;
    }

    .report-stat-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .report-quality-summary {
      flex-basis: auto;
    }
  }

  @media (min-width: 721px) and (max-width: 1200px) {
    .simple-report-overview {
      grid-template-columns: minmax(12rem, 0.9fr) minmax(0, 1.8fr);
    }

    .simple-overview-coverage {
      grid-column: 2;
    }

    .report-stat-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
