<!-- ./src/lib/components/report/WarningBlocks.svelte -->
<script lang="ts">
  import { getClinicalWarning } from '../../utils/evidence';

  /*
  Module Docstring:
  Purpose: Aggregated safety warnings, diagnostic limitations, and clinical validation flags.
  Responsibilities:
  - Render raw DNA limitation notices when present.
  - Display clinical validation required blocks if variants are high-stakes.
  - Display diagnostic boundary alerts (do_not_claim arrays).
  Key Inputs: gene, rawDnaLimitation, clinicalConfirmationRequired, doNotClaim.
  Key Outputs: Visual safety warnings.
  Operational Notes: Uses .raw-limitation-warning, .clinical-confirmation-warning, and .claim-warning classes.
  */

  interface Props {
    gene: string;
    rawDnaLimitation?: string;
    clinicalConfirmationRequired?: boolean;
    doNotClaim?: string[];
  }

  let {
    gene,
    rawDnaLimitation,
    clinicalConfirmationRequired,
    doNotClaim = []
  }: Props = $props();

  let hasWarnings = $derived(Boolean(rawDnaLimitation || clinicalConfirmationRequired || doNotClaim.length > 0));
</script>

{#if hasWarnings}
  <details class="warning-details">
    <summary>
      <span>Limits &amp; confirmation</span>
      {#if clinicalConfirmationRequired}
        <span class="warning-summary-badge">Clinical review</span>
      {/if}
    </summary>
    <div class="warning-details-body">
      {#if rawDnaLimitation}
        <div class="raw-limitation-warning">
          <span class="sec-title">Raw DNA limitation</span>
          <p>{rawDnaLimitation}</p>
        </div>
      {/if}

      {#if clinicalConfirmationRequired}
        <div class="clinical-confirmation-warning">
          {getClinicalWarning(gene)}
        </div>
      {/if}

      {#if doNotClaim.length > 0}
        <div class="claim-warning">
          <span class="sec-title">Important limits</span>
          <ul class="warning-list">
            {#each doNotClaim as item}
              <li>{item}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  </details>
{/if}
