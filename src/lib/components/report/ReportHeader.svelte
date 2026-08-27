<!-- ./src/lib/components/report/ReportHeader.svelte -->
<script lang="ts">
  import type { GeneratedReport } from '../../types/genomics';

  /*
  Module Docstring:
  Purpose: Report header dashboard presenting summary statistics.
  Responsibilities:
  - Display progress gauge with the risk-direction-only signal score.
  - Add explanatory subtitle so users understand what the score represents.
  - Summarize basic user biological statistics.
  Key Inputs: generatedReport, foundMarkersCount, totalMarkersChecked.
  Key Outputs: Visual report header element.
  Operational Notes: Score reflects risk-direction markers only. Protective/trait excluded.
  */

  interface Props {
    generatedReport: GeneratedReport;
    foundMarkersCount: number;
    totalMarkersChecked: number;
    presentationMode?: "simple" | "clinical" | "compare";
  }

  let {
    generatedReport,
    foundMarkersCount,
    totalMarkersChecked,
    presentationMode = "simple"
  }: Props = $props();

  let overallScore = $derived(generatedReport.overall_signal_score ?? 0);

  function computeSummaryLine(report: GeneratedReport, mode: "simple" | "clinical" | "compare"): string {
    let high = 0;
    let mod = 0;
    let prot = 0;
    for (const sec of report.sections || []) {
      for (const m of sec.markers || []) {
        if (m.severity_class === 'high_risk' || m.severity_class === 'confirmation_required') {
          high++;
        } else if (m.severity_class === 'moderate_risk') {
          mod++;
        } else if (m.severity_class === 'protective') {
          prot++;
        }
      }
    }
    if (mode === 'simple') {
      const associationCount = high + mod;
      const simpleParts = [];
      if (associationCount > 0) {
        simpleParts.push(`${associationCount} possible ${associationCount === 1 ? 'association' : 'associations'}`);
      }
      if (prot > 0) {
        simpleParts.push(`${prot} possible protective ${prot === 1 ? 'association' : 'associations'}`);
      }
      if (simpleParts.length === 0) return "No notable associations in this report.";
      return simpleParts.join(' · ');
    }

    const parts = [];
    if (high > 0) parts.push(`${high} stronger ${high === 1 ? 'association' : 'associations'}`);
    if (mod > 0) parts.push(`${mod} possible ${mod === 1 ? 'association' : 'associations'}`);
    if (prot > 0) parts.push(`${prot} protective ${prot === 1 ? 'variant' : 'variants'}`);
    
    if (parts.length === 0) return "No active associations or protective variants detected.";
    return parts.join(', ') + ' among curated pack markers.';
  }

  let summaryLine = $derived(computeSummaryLine(generatedReport, presentationMode));
</script>

<div class="report-header card" data-presentation-mode={presentationMode}>
  <div class="report-quality-summary" aria-label="Report data quality summary">
    <span class="quality-kicker">DNA coverage</span>
    <strong>{foundMarkersCount.toLocaleString()} / {totalMarkersChecked.toLocaleString()}</strong>
    <span>markers called</span>
    <span class="quality-note">Uncalled = unknown</span>
  </div>
  <div class="report-desc">
    <div class="header-title-row">
      <h3>{generatedReport.title}</h3>
      <span class="overall-summary-badge">✨ {summaryLine}</span>
    </div>
    <p>{generatedReport.description}</p>

    <details class="technical-score-details">
      <summary>Technical coverage metric</summary>
      <p>
        The curated association match rate is {overallScore.toFixed(1)}%. It is a research-association coverage metric—not a disease probability, diagnosis, or measure of health.
      </p>
    </details>
  </div>
</div>

<style>
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
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .quality-kicker {
    color: var(--text-primary);
    font-size: 0.68rem;
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

  .overall-summary-badge {
    background: var(--status-accent-bg);
    border: 1px solid var(--status-accent-border);
    color: var(--status-accent-text);
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
    border-radius: 9999px;
    font-weight: 600;
    max-width: 100%;
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

    .report-quality-summary {
      flex-basis: auto;
    }
  }
</style>
