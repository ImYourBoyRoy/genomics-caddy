<!-- ./src/lib/components/report/VariantCard.svelte -->
<script lang="ts">
  import type { EvaluatedMarker, SeverityClass } from '../../types/genomics';
  import EvidenceBadge from './EvidenceBadge.svelte';
  import EffectDirectionBadge from './EffectDirectionBadge.svelte';
  import SourcesList from './SourcesList.svelte';
  import ConfirmWithList from './ConfirmWithList.svelte';
  import WarningBlocks from './WarningBlocks.svelte';
  import { getEffectCount, getEffectAllele } from '../../utils/genotype';
  import { getSeverityInfo } from '../../utils/evidence';
  import { LAYPERSON_MAP } from '../../utils/layperson';

  /*
  Module Docstring:
  Purpose: Card component for a single genetic variant, styled by severity_class.
  Responsibilities:
  - Display allele counts and genotyping status with plain-language labels.
  - Apply severity-class-driven CSS (not raw effect_count).
  - Collapse cards where the variant was not detected or no data exists — keeping
    user attention on actual findings.
  - Show direction badge, warnings, and sources ONLY when the variant was detected.
  Key Inputs: marker (EvaluatedMarker).
  Key Outputs: Visual representation of a single gene marker.
  Operational Notes: severity_class comes from the Rust backend, not local computation.
    Design targets: average users, clinical professionals, and AI/MCP consumers.
  */

  interface Props {
    marker: EvaluatedMarker;
    viewMode: "simple" | "clinical" | "dual";
  }

  let { marker, viewMode }: Props = $props();

  let effectCount = $derived(getEffectCount(marker));
  let effectAllele = $derived(getEffectAllele(marker));
  let severity = $derived(getSeverityInfo(marker.severity_class as SeverityClass));
  let laypersonTranslation = $derived(LAYPERSON_MAP[marker.rsid]);

  /** True when the variant was actually detected (not benign / no_data) */
  let isActiveFindings = $derived(
    marker.severity_class !== "benign" && marker.severity_class !== "no_data"
  );
</script>

<div
  class="marker-card {severity.cssClass}"
  class:marker-card-collapsed={!isActiveFindings}
>
  <!-- Header row: gene name + evidence tier -->
  <div class="marker-top">
    <span class="gene-label">
      <strong>{marker.gene}</strong> 
      {#if marker.variant_name}
        <span class="variant-sub">({marker.variant_name})</span>
      {/if}
      <span class="rsid-sub">{marker.rsid}</span>
    </span>
    <EvidenceBadge tier={marker.evidence_tier} />
  </div>

  <!-- Status line: genotype result + severity verdict -->
  <div class="marker-middle">
    <span class="genotype-val">Your Result: <strong>{marker.user_genotype}</strong></span>
    <span class="marker-severity-label">
      {severity.emoji} {severity.label}
      {#if effectCount > 0 && isActiveFindings}
        <span class="allele-detail">({effectCount}× {effectAllele})</span>
      {/if}
    </span>
  </div>

  {#if isActiveFindings}
    <!-- Severity explanation — only shown for active findings -->
    <div class="severity-explainer">
      {severity.description}
    </div>

    <div class="marker-body">
      <!-- Direction badge: only when variant IS detected -->
      {#if marker.effect_direction}
        <EffectDirectionBadge direction={marker.effect_direction} />
      {/if}

      <div class="impact-section">
        <strong>What this gene does:</strong>
        {#if viewMode === "simple"}
          <span class="layperson-text">{laypersonTranslation?.simpleImpact || marker.impact}</span>
        {:else if viewMode === "clinical"}
          <span class="clinical-text">{marker.impact}</span>
        {:else}
          <div class="dual-explanations">
            <div class="dual-row layperson-box">
              <span class="dual-tag layperson-tag">🌱 Simple:</span>
              <span class="layperson-text">{laypersonTranslation?.simpleImpact || marker.impact}</span>
            </div>
            <div class="dual-row clinical-box">
              <span class="dual-tag clinical-tag">🏥 Medical:</span>
              <span class="clinical-text">{marker.impact}</span>
            </div>
          </div>
        {/if}
      </div>

      {#if viewMode !== "simple"}
        <WarningBlocks
          gene={marker.gene}
          rawDnaLimitation={marker.raw_dna_limitation}
          clinicalConfirmationRequired={marker.clinical_confirmation_required}
          doNotClaim={marker.do_not_claim}
        />
      {:else if marker.clinical_confirmation_required}
        <div class="simple-alert no-print">
          ⚠️ <strong>Clinical test needed:</strong> Consumer DNA tests can sometimes report false positives on rare variants. A medical-grade lab test is required to confirm this finding before making any therapy changes.
        </div>
      {/if}

      <div class="interpretation-section">
        <strong>What your result means:</strong>
        {#if viewMode === "simple"}
          <span class="layperson-text">{laypersonTranslation?.simpleMeaning || marker.interpretation}</span>
        {:else if viewMode === "clinical"}
          <span class="clinical-text">{marker.interpretation}</span>
        {:else}
          <div class="dual-explanations">
            <div class="dual-row layperson-box">
              <span class="dual-tag layperson-tag">🌱 Simple:</span>
              <span class="layperson-text">{laypersonTranslation?.simpleMeaning || marker.interpretation}</span>
            </div>
            <div class="dual-row clinical-box">
              <span class="dual-tag clinical-tag">🏥 Medical:</span>
              <span class="clinical-text">{marker.interpretation}</span>
            </div>
          </div>
        {/if}
      </div>

      {#if viewMode !== "simple" && marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
        <ConfirmWithList confirmWith={marker.confirm_with} />
        <SourcesList sources={marker.sources} />
      {/if}
    </div>
  {/if}
</div>
