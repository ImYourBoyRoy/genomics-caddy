<!-- ./src/lib/components/report/WarningBlocks.svelte -->
<script lang="ts">
  import { getClinicalWarning } from '../../utils/evidence';
  import {
    classifyWarning,
    isWarningVisibleInAudience,
    type WarningAudience,
  } from '../../utils/warningTaxonomy';

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
    audience?: WarningAudience;
  }

  let {
    gene,
    rawDnaLimitation,
    clinicalConfirmationRequired,
    doNotClaim = [],
    audience = 'clinical',
  }: Props = $props();

  let limitationKind = $derived(rawDnaLimitation ? classifyWarning(rawDnaLimitation) : null);
  let clinicalKind = $derived(clinicalConfirmationRequired ? 'clinical_review' as const : null);
  let claimKinds = $derived(doNotClaim.map((item) => ({ item, kind: classifyWarning(item) })));
  let hasWarnings = $derived(
    Boolean(
      (limitationKind && isWarningVisibleInAudience(limitationKind, audience))
      || (clinicalKind && isWarningVisibleInAudience(clinicalKind, audience))
      || claimKinds.some(({ kind }) => isWarningVisibleInAudience(kind, audience)),
    ),
  );
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
      {#if rawDnaLimitation && limitationKind && isWarningVisibleInAudience(limitationKind, audience)}
        <div class="raw-limitation-warning" data-warning-kind={limitationKind}>
          <span class="sec-title">Raw DNA limitation</span>
          <p>{rawDnaLimitation}</p>
        </div>
      {/if}

      {#if clinicalConfirmationRequired && clinicalKind && isWarningVisibleInAudience(clinicalKind, audience)}
        <div class="clinical-confirmation-warning" data-warning-kind={clinicalKind}>
          {getClinicalWarning(gene)}
        </div>
      {/if}

      {#if claimKinds.some(({ kind }) => isWarningVisibleInAudience(kind, audience))}
        <div class="claim-warning" data-warning-kind="general_interpretation">
          <span class="sec-title">Important limits</span>
          <ul class="warning-list">
            {#each claimKinds as claim}
              {#if isWarningVisibleInAudience(claim.kind, audience)}
                <li data-warning-kind={claim.kind}>{claim.item}</li>
              {/if}
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  </details>
{/if}
