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
  Key Inputs: generatedReport, geneticSex, foundMarkersCount, totalMarkersChecked.
  Key Outputs: Visual report header element.
  Operational Notes: Score reflects risk-direction markers only. Protective/trait excluded.
  */

  interface Props {
    generatedReport: GeneratedReport;
    geneticSex: string;
    foundMarkersCount: number;
    totalMarkersChecked: number;
  }

  let {
    generatedReport,
    geneticSex,
    foundMarkersCount,
    totalMarkersChecked
  }: Props = $props();

  let overallScore = $derived(generatedReport.overall_signal_score ?? 0);

  let bucketLabel = $derived(
    overallScore <= 15 ? "Low baseline" :
    overallScore <= 35 ? "Moderate signal" :
    overallScore <= 60 ? "Elevated signal" :
    "High signal"
  );

  function computeSummaryLine(report: GeneratedReport): string {
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
    const parts = [];
    if (high > 0) parts.push(`${high} high-priority ${high === 1 ? 'finding' : 'findings'}`);
    if (mod > 0) parts.push(`${mod} moderate risk ${mod === 1 ? 'marker' : 'markers'}`);
    if (prot > 0) parts.push(`${prot} protective ${prot === 1 ? 'variant' : 'variants'}`);
    
    if (parts.length === 0) return "No active concerns or protective variants detected.";
    return parts.join(', ') + ' detected.';
  }

  let summaryLine = $derived(computeSummaryLine(generatedReport));
</script>

<div class="report-header card">
  <div class="overall-signal">
    <div class="meter-wrapper">
      <div class="meter-score">
        {overallScore.toFixed(1)}%
      </div>
      <div class="meter-label">Risk Signal</div>
      <div class="meter-bucket">{bucketLabel}</div>
    </div>
  </div>
  <div class="report-desc">
    <div class="header-title-row">
      <h3>{generatedReport.title}</h3>
      <span class="overall-summary-badge">✨ {summaryLine}</span>
    </div>
    <p>{generatedReport.description}</p>

    <!-- Plain-language explanation of the score -->
    <div class="score-explainer">
      <strong>What does this number mean?</strong>
      This percentage shows how many risk-direction effect alleles were found across all your tested markers.
      A higher number means more risk-associated variants were detected — but it is <em>not</em> a diagnosis.
      Protective, trait, and context-dependent markers are tracked separately and do not inflate this number.
    </div>

    <div class="health-summary-row">
      <div class="health-stat">
        <span class="lbl">Genetic Sex</span>
        <span class="val text-accent">{geneticSex}</span>
      </div>
      <div class="health-stat">
        <span class="lbl">Markers Checked</span>
        <span class="val">{foundMarkersCount} / {totalMarkersChecked} curated SNPs found</span>
      </div>
      <p class="marker-scope-note">
        Counts reflect hand-curated marker packs only (~{totalMarkersChecked} SNPs), not your full chip (~600k variants).
        Use Agent Discovery and Vector Research scopes to expand beyond this baseline.
      </p>
    </div>
  </div>
</div>

<style>
  .meter-bucket {
    font-size: 0.62rem;
    font-weight: 700;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: 2px;
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
  }

  .overall-summary-badge {
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid rgba(99, 102, 241, 0.2);
    color: #a5b4fc;
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
    border-radius: 9999px;
    font-weight: 600;
  }
</style>
