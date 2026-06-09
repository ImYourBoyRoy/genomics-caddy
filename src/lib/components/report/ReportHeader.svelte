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
</script>

<div class="report-header card">
  <div class="overall-signal">
    <div class="meter-wrapper">
      <div class="meter-score">
        {overallScore.toFixed(1)}%
      </div>
      <div class="meter-label">Risk Signal</div>
    </div>
  </div>
  <div class="report-desc">
    <h3>{generatedReport.title}</h3>
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
        <span class="val">{foundMarkersCount} / {totalMarkersChecked} SNPs Found</span>
      </div>
    </div>
  </div>
</div>
