<!-- ./src/lib/components/report/ReportLoadingState.svelte -->
<script lang="ts">
  import '$lib/styles/components/report-loading.css';

  /*
  Purpose: Calm in-place Trait Report preparation card.
  How to use: Render while `isGeneratingReport` is true.
  Inputs: sampleName.
  Outputs: Accessible busy status matching the finished report hero.
  Notes: Does not fake pipeline stages or elapsed timers. The backend
  does not currently expose a real generation phase.
  */

  interface Props {
    sampleName: string;
  }

  let { sampleName }: Props = $props();
</script>

<section
  class="report-loading-state"
  aria-busy="true"
  aria-labelledby="report-loading-title"
  role="status"
>
  <p class="report-kicker">Trait report</p>
  <h3 id="report-loading-title">Preparing {sampleName}'s report</h3>
  <p class="report-loading-lead">
    Matching this profile to curated markers. The dashboard opens when it is ready.
  </p>
  <div class="report-loading-progress" aria-hidden="true"><span></span></div>
  <p class="report-loading-note">Your DNA stays on this computer.</p>
</section>
