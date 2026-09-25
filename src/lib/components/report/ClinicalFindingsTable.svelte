<!-- ./src/lib/components/report/ClinicalFindingsTable.svelte -->
<script lang="ts">
  import type { EvaluatedMarker } from '../../types/genomics';
  import type { VariantNavTarget } from '../../constants/traitCategories';
  import { getEffectAllele, getEffectCount, isCallableGenotype } from '../../utils/genotype';
  import { getSeverityInfo, getTierInfo } from '../../utils/evidence';
  import { formatClinicalFollowUp } from '../../utils/clinicalPresentation';
  import {
    clinicalStateLabel,
    inheritanceModelLabel,
    interpretationClassLabel,
    normalizeFindingSemantics,
  } from '../../utils/findingSemantics';
  import {
    callabilityExplanation,
    callabilityStateForResult,
    callabilityStateLabel,
    orientationStateForResult,
    orientationStateLabel,
  } from '../../utils/callability';
  import {
    getSharedInterpretations,
    normalizeSharedInterpretation,
  } from '../../utils/sharedInterpretations';
  import SourcesList from './SourcesList.svelte';
  import ContextSummary from './ContextSummary.svelte';

  interface Props {
    markers: EvaluatedMarker[];
    showExtraColumns?: boolean;
    onExploreResearch?: (rsid: string) => void;
    highlightRsid?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let {
    markers,
    showExtraColumns = false,
    onExploreResearch,
    highlightRsid = '',
    onNavigateToVariant,
  }: Props = $props();

  function nextHelpfulStep(marker: EvaluatedMarker): string {
    const followUp = formatClinicalFollowUp(marker.confirm_with);
    if (followUp) return followUp;
    if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
      return 'Review need for clinical confirmation.';
    }
    return 'Interpret with history and current guidance.';
  }

  function clinicalStatusNote(marker: EvaluatedMarker): string {
    const callabilityState = marker.callability_state
      || callabilityStateForResult(marker.variant_type, marker.assertion_status);
    if (callabilityState !== 'callable') return callabilityExplanation(callabilityState);
    if (!marker.interpretation_allowed) return 'Review blocked';
    if (marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required') {
      return 'Confirmation needed';
    }
    return 'Contextual result';
  }

  function markerOrientationState(marker: EvaluatedMarker) {
    return marker.orientation_state
      || orientationStateForResult(marker.assertion_status, marker.requires_orientation_verification);
  }

  function markerCallabilityState(marker: EvaluatedMarker) {
    return marker.callability_state
      || callabilityStateForResult(marker.variant_type, marker.assertion_status);
  }

  function isHighlighted(marker: EvaluatedMarker): boolean {
    return highlightRsid !== '' && marker.rsid.toLowerCase() === highlightRsid.toLowerCase();
  }

  let sharedInterpretations = $derived(getSharedInterpretations(markers));

  let sharedInterpretationKeys = $derived.by(() =>
    new Set(sharedInterpretations.map((item) => item.key)),
  );
</script>

{#if sharedInterpretations.length > 0}
  <details class="clinical-shared-context">
    <summary>Shared clinical context ({sharedInterpretations.length})</summary>
    <div class="clinical-shared-context-body">
      <p>These explanations apply to more than one finding in this section and are shown once to keep the table readable.</p>
      <ul>
        {#each sharedInterpretations as item (item.text)}
          <li><strong>{item.count} findings:</strong> {item.text}</li>
        {/each}
      </ul>
    </div>
  </details>
{/if}

<div class="clinical-table-wrap" role="region" aria-label="Clinical findings table">
  <table class="clinical-findings-table">
    <caption>Structured findings for clinical review.</caption>
    <thead>
      <tr>
        <th scope="col">Finding</th>
        <th scope="col">DNA result</th>
        <th scope="col">Evidence</th>
        {#if showExtraColumns}
          <th scope="col">Interpretation</th>
          <th scope="col">Inheritance</th>
          <th scope="col">Applicability</th>
        {/if}
        <th scope="col">Clinical status</th>
        <th scope="col">Next helpful step</th>
        <th scope="col"><span class="sr-only">Details</span></th>
      </tr>
    </thead>
    <tbody>
      {#each markers as marker (marker.link_id || marker.rsid)}
        {@const severity = getSeverityInfo(marker.severity_class)}
        {@const tier = getTierInfo(marker.evidence_tier)}
        {@const effectCount = getEffectCount(marker)}
        {@const effectAllele = getEffectAllele(marker)}
        {@const semantics = normalizeFindingSemantics(marker)}
        <tr class:clinical-row-highlight={isHighlighted(marker)}>
          <td data-label="Finding">
            <span class="clinical-mobile-label">Finding</span>
            <strong>{marker.gene}</strong>
            <span class="clinical-finding-sub">{marker.variant_name || marker.rsid}</span>
            <div class="clinical-row-actions no-print">
              {#if isCallableGenotype(marker.user_genotype)}
                <button type="button" class="clinical-row-link" aria-label="Open {marker.rsid} on the genome map" onclick={() => onNavigateToVariant?.(marker.rsid, 'map')}>Map</button>
                <button type="button" class="clinical-row-link" aria-label="Open {marker.rsid} in the raw browser" onclick={() => onNavigateToVariant?.(marker.rsid, 'browser')}>Browser</button>
                <button type="button" class="clinical-row-link" aria-label="Search evidence for {marker.rsid}" onclick={() => onExploreResearch?.(marker.rsid)}>Evidence</button>
              {/if}
            </div>
          </td>
          <td data-label="DNA result">
            <span class="clinical-mobile-label">DNA result</span>
            <strong class="clinical-genotype">{marker.user_genotype}</strong>
            <span class="clinical-cell-note">{marker.assertion_status}</span>
          </td>
          <td data-label="Evidence">
            <span class="clinical-mobile-label">Evidence</span>
            <strong>{tier.label}</strong>
            <span class="clinical-cell-note">{tier.confidenceLabel}</span>
          </td>
          {#if showExtraColumns}
            <td data-label="Interpretation">
              <span class="clinical-mobile-label">Interpretation</span>
              {interpretationClassLabel(semantics.interpretation_class)}
              {#if semantics.condition_label}
                <span class="clinical-cell-note">{semantics.condition_label}</span>
              {/if}
            </td>
            <td data-label="Inheritance">
              <span class="clinical-mobile-label">Inheritance</span>
              {inheritanceModelLabel(semantics.inheritance_model)}
            </td>
            <td data-label="Applicability">
              <span class="clinical-mobile-label">Applicability</span>
              {#if (marker.context_tags && marker.context_tags.length > 0) || (marker.sex_scope && marker.sex_scope !== 'all')}
                <ContextSummary contextTags={marker.context_tags} scope={marker.sex_scope} maxVisible={2} />
              {:else}
                All users unless context says otherwise
              {/if}
            </td>
          {/if}
          <td data-label="Clinical status">
            <span class="clinical-mobile-label">Clinical status</span>
            <span class="clinical-severity {severity.cssClass}">{severity.label}</span>
            <span class="clinical-cell-note">{clinicalStatusNote(marker)}</span>
          </td>
          <td data-label="Next helpful step">
            <span class="clinical-mobile-label">Next helpful step</span>
            {nextHelpfulStep(marker)}
          </td>
          <td data-label="Details">
            <details class="clinical-details">
              <summary>Technical details</summary>
              <dl>
                <div><dt>Gene / marker</dt><dd>{marker.gene} · {marker.rsid}</dd></div>
                <div><dt>Variant</dt><dd>{marker.variant_name || 'Not recorded'}</dd></div>
                <div><dt>Effect allele / count</dt><dd>{effectAllele} / {effectCount}</dd></div>
                <div><dt>Assertion</dt><dd>{marker.assertion_status}</dd></div>
                <div><dt>Callability</dt><dd>{callabilityStateLabel(markerCallabilityState(marker))}</dd></div>
                <div><dt>Orientation</dt><dd>{orientationStateLabel(markerOrientationState(marker))}</dd></div>
                <div><dt>Clinical confirmation</dt><dd>{marker.clinical_confirmation_required ? 'Discuss confirmation' : 'Not specifically required by this marker'}</dd></div>
                {#if semantics.condition_label}
                  <div><dt>Condition / topic</dt><dd>{semantics.condition_label}</dd></div>
                {/if}
                <div><dt>Interpretation class</dt><dd>{interpretationClassLabel(semantics.interpretation_class)}</dd></div>
                <div><dt>Inheritance model</dt><dd>{inheritanceModelLabel(semantics.inheritance_model)}</dd></div>
                <div><dt>Clinical state</dt><dd>{clinicalStateLabel(semantics.clinical_state)}</dd></div>
                {#if sharedInterpretationKeys.has(normalizeSharedInterpretation(marker.interpretation))}
                  <div><dt>Interpretation</dt><dd>Shared clinical context shown above.</dd></div>
                {:else}
                  <div><dt>Interpretation</dt><dd>{marker.interpretation}</dd></div>
                {/if}
                <div><dt>Claim boundary</dt><dd>{marker.do_not_claim.join('; ') || marker.raw_dna_limitation || 'Do not treat as diagnostic.'}</dd></div>
              </dl>
              <SourcesList sources={marker.sources} dbSources={marker.db_enriched_sources} />
            </details>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .clinical-table-wrap {
    max-width: 100%;
    overflow-x: auto;
    border: 1px solid var(--border-color);
    border-radius: 0.65rem;
    background: var(--surface-subtle);
  }

  .clinical-shared-context {
    margin-bottom: 0.7rem;
    border: 1px solid var(--border-color);
    border-radius: 0.55rem;
    background: var(--surface-subtle);
  }

  .clinical-shared-context summary {
    min-height: 2.5rem;
    padding: 0.65rem 0.8rem;
    color: var(--accent);
    font-weight: 700;
    cursor: pointer;
  }

  .clinical-shared-context-body {
    padding: 0 0.8rem 0.75rem;
    color: var(--text-secondary);
    font-size: 0.74rem;
    line-height: 1.45;
  }

  .clinical-shared-context-body p {
    margin: 0 0 0.55rem;
  }

  .clinical-shared-context-body ul {
    display: grid;
    gap: 0.45rem;
    margin: 0;
    padding-left: 1.15rem;
  }

  .clinical-table-wrap:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .clinical-findings-table {
    width: 100%;
    min-width: 760px;
    border-collapse: collapse;
    color: var(--text-primary);
    font-size: 0.82rem;
  }

  .clinical-findings-table caption {
    padding: 0.7rem 0.8rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: left;
  }

  .clinical-findings-table th,
  .clinical-findings-table td {
    padding: 0.65rem 0.7rem;
    border-top: 1px solid var(--border-color);
    vertical-align: top;
    text-align: left;
    overflow-wrap: anywhere;
  }

  .clinical-findings-table th {
    color: var(--text-secondary);
    font-size: 0.7rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .clinical-findings-table tbody tr:hover,
  .clinical-row-highlight {
    background: var(--accent-soft);
  }

  .clinical-finding-sub,
  .clinical-cell-note {
    display: block;
    margin-top: 0.2rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .clinical-mobile-label {
    display: none;
  }

  .clinical-genotype {
    font-family: var(--font-mono), monospace;
    white-space: nowrap;
  }

  .clinical-severity {
    display: inline-block;
    font-weight: 700;
  }

  .clinical-row-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.45rem;
  }

  .clinical-row-link {
    min-height: 2rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 0.35rem;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .clinical-row-link:hover,
  .clinical-row-link:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .clinical-details {
    min-width: 9rem;
  }

  .clinical-details summary {
    color: var(--accent);
    cursor: pointer;
    font-weight: 700;
    white-space: nowrap;
  }

  .clinical-details dl {
    display: grid;
    gap: 0.45rem;
    min-width: 18rem;
    margin: 0.65rem 0 0;
  }

  .clinical-details dl > div {
    display: grid;
    grid-template-columns: minmax(7rem, 0.4fr) minmax(12rem, 1fr);
    gap: 0.5rem;
  }

  .clinical-details dt {
    color: var(--text-secondary);
    font-weight: 700;
  }

  .clinical-details dd {
    margin: 0;
    color: var(--text-primary);
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* The sidebar leaves a narrow clinical reading surface well before the
     application viewport itself becomes tablet-sized. Stack the table while
     there is still room for comfortable labels and values. */
  @media (max-width: 1400px) {
    .clinical-table-wrap {
      overflow-x: visible;
    }

    .clinical-findings-table {
      display: block;
      width: 100%;
      min-width: 0;
      max-width: 100%;
      overflow: hidden;
    }

    .clinical-findings-table,
    .clinical-findings-table tbody,
    .clinical-findings-table tr,
    .clinical-findings-table td {
      display: block;
    }

    .clinical-findings-table thead {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }

    .clinical-findings-table tbody {
      display: grid;
      gap: 0.75rem;
      width: 100%;
      min-width: 0;
      box-sizing: border-box;
      padding: 0.75rem;
    }

    .clinical-findings-table tbody tr {
      width: 100%;
      min-width: 0;
      box-sizing: border-box;
      border: 1px solid var(--border-color);
      border-radius: 0.6rem;
      background: var(--surface-raised);
      overflow: hidden;
    }

    .clinical-findings-table tbody tr:hover,
    .clinical-row-highlight {
      background: var(--accent-soft);
    }

    .clinical-findings-table tbody td {
      display: grid;
      grid-template-columns: minmax(7rem, 0.45fr) minmax(0, 1fr);
      gap: 0.65rem;
      min-width: 0;
      box-sizing: border-box;
      align-items: start;
      padding: 0.65rem 0.75rem;
      border-top: 1px solid var(--border-color);
    }

    .clinical-findings-table tbody td:first-child {
      border-top: 0;
    }

    .clinical-findings-table tbody td::before {
      content: none;
    }

    .clinical-mobile-label {
      display: block;
      color: var(--text-secondary);
      font-size: 0.64rem;
      font-weight: 700;
      letter-spacing: 0.04em;
      line-height: 1.35;
      text-transform: uppercase;
    }

    .clinical-findings-table tbody td[data-label="Details"] {
      display: block;
    }

    .clinical-findings-table tbody td[data-label="Details"]::before {
      content: none;
    }

    .clinical-details {
      min-width: 0;
      max-width: 100%;
    }

    .clinical-details dl {
      min-width: 0;
      max-width: 100%;
    }

    .clinical-details dl > div {
      grid-template-columns: minmax(6rem, 0.45fr) minmax(0, 1fr);
    }
  }
</style>
