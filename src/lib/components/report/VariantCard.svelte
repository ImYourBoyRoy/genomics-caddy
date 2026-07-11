<!-- ./src/lib/components/report/VariantCard.svelte -->
<script lang="ts">
  import type { EvaluatedMarker, SeverityClass, EnrichedSource } from '../../types/genomics';
  import EvidenceBadge from './EvidenceBadge.svelte';
  import EffectDirectionBadge from './EffectDirectionBadge.svelte';
  import SourcesList from './SourcesList.svelte';
  import ConfirmWithList from './ConfirmWithList.svelte';
  import WarningBlocks from './WarningBlocks.svelte';
  import { getEffectCount, getEffectAllele } from '../../utils/genotype';
  import { getSeverityInfo } from '../../utils/evidence';
  import { LAYPERSON_MAP } from '../../utils/layperson';
  import type { VariantNavTarget } from '../../constants/traitCategories';

  interface Props {
    marker: EvaluatedMarker;
    viewMode: "simple" | "clinical" | "dual";
    onExploreResearch?: (rsid: string) => void;
    highlightRsid?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let { marker, viewMode, onExploreResearch, highlightRsid = "", onNavigateToVariant }: Props = $props();

  let effectCount = $derived(getEffectCount(marker));
  let effectAllele = $derived(getEffectAllele(marker));
  let severity = $derived(getSeverityInfo(marker.severity_class as SeverityClass));
  let laypersonTranslation = $derived(LAYPERSON_MAP[marker.rsid]);

  /** True when the variant was actually detected (not benign / no_data) */
  let isActiveFindings = $derived(
    marker.severity_class !== "benign" && marker.severity_class !== "no_data"
  );
  let isHighlighted = $derived(
    highlightRsid !== "" && marker.rsid.toLowerCase() === highlightRsid.toLowerCase()
  );

  function clinvarClass(sig: string): string {
    const s = sig.toLowerCase();
    if (s.includes('likely pathogenic')) return 'likely-pathogenic';
    if (s.includes('pathogenic')) return 'pathogenic';
    if (s.includes('benign')) return 'benign';
    if (s.includes('uncertain')) return 'uncertain';
    return 'other';
  }

  function isHighPriorityClinvar(sig: string): boolean {
    const s = sig.toLowerCase();
    return s.includes('pathogenic') || s.includes('risk factor') || s.includes('conflicting');
  }

  function formatAf(af: number): string {
    if (af < 0.0001) return `${(af * 100).toFixed(4)}%`;
    if (af < 0.01) return `${(af * 100).toFixed(3)}%`;
    return `${(af * 100).toFixed(2)}%`;
  }

  function formatPvalue(p: number): string {
    if (p < 1e-300) return '<10⁻³⁰⁰';
    const exp = Math.floor(Math.log10(p));
    const base = p / Math.pow(10, exp);
    return `${base.toFixed(1)}×10${exp < 0 ? '⁻' : ''}${Math.abs(exp)}`;
  }

  function getSimpleLabel(severityClass: string): string {
    switch (severityClass) {
      case "high_risk": return "Two copies of risk gene found";
      case "moderate_risk": return "One copy of risk gene found";
      case "low_risk": return "Preliminary risk sign";
      case "protective": return "Protective gene copy found";
      case "trait": return "Physical trait gene found";
      case "context_dependent": return "Depends on diet & lifestyle";
      case "confirmation_required": return "Needs clinical lab test to verify";
      case "no_data": return "No data available";
      default: return "Normal / Benign";
    }
  }
</script>

<div
  id="variant-{marker.rsid.toLowerCase()}"
  class="marker-card {severity.cssClass}"
  class:marker-card-collapsed={!isActiveFindings}
  class:marker-card-highlight={isHighlighted}
>
  <!-- Header row: gene name + evidence tier -->
  <div class="marker-top">
    <span class="gene-label">
      <strong>{marker.gene}</strong> 
      {#if marker.variant_name}
        <span class="variant-sub">({marker.variant_name})</span>
      {/if}
      <span class="rsid-sub">
        {marker.rsid}
        {#if marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
          <span class="nav-links no-print">
            <button type="button" class="research-explore-link" onclick={() => onNavigateToVariant?.(marker.rsid, "map")} title="Genome map">🗺️</button>
            <button type="button" class="research-explore-link" onclick={() => onNavigateToVariant?.(marker.rsid, "browser")} title="Raw browser">🔍</button>
            <button
              type="button"
              class="research-explore-link"
              onclick={() => onExploreResearch?.(marker.rsid)}
              title="Search Enriched Vector Research"
            >
              🔬
            </button>
          </span>
        {/if}
      </span>
    </span>
    <EvidenceBadge tier={marker.evidence_tier} />
  </div>

  <!-- Status line: genotype result + severity verdict -->
  <div class="marker-middle">
    <span class="genotype-val">
      Your Result: <strong>{marker.user_genotype}</strong>
      {#if viewMode === 'simple'}
        <span class="genotype-info-icon" title="Your DNA contains two copies of this marker (one from each parent). '{marker.user_genotype}' represents your specific genetic letters.">ℹ️</span>
      {/if}
    </span>
    <span class="marker-severity-label">
      <span class="severity-glyph" aria-hidden="true">{severity.glyph}</span>
      {viewMode === 'simple' ? getSimpleLabel(marker.severity_class) : severity.label}
      {#if effectCount > 0 && isActiveFindings}
        <span class="allele-detail">
          {#if viewMode === 'simple'}
            ({effectCount === 1 ? 'one copy' : 'two copies'} of DNA letter {effectAllele} found)
          {:else}
            ({effectCount}× {effectAllele})
          {/if}
        </span>
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

      <!-- Reference DB enrichment chips (clinical + dual view only) -->
      {#if viewMode !== 'simple' && (marker.clinvar_significance || marker.population_rarity || marker.gwas_top_trait || marker.pharmgkb || marker.clingen || marker.mane)}
        <div class="enrichment-row" style="display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 0.75rem;">
          {#if marker.clinvar_significance}
            {#if isHighPriorityClinvar(marker.clinvar_significance)}
              <span class="clinvar-chip clinvar-{clinvarClass(marker.clinvar_significance)}" title={marker.clinvar_conditions ?? undefined}>
                🏛️ ClinVar: {marker.clinvar_significance}
                {#if marker.clinvar_review_status}
                  <span class="review-status">({marker.clinvar_review_status})</span>
                {/if}
              </span>
            {:else}
              <details class="clinvar-low-priority-detail" style="margin-top: 0.2rem;">
                <summary style="font-size: 0.72rem; cursor: pointer; color: var(--text-secondary); list-style: none; display: flex; align-items: center; gap: 0.25rem;">
                  <span>🏛️ ClinVar (VUS/Benign)</span>
                </summary>
                <div style="margin-top: 0.25rem;">
                  <span class="clinvar-chip clinvar-{clinvarClass(marker.clinvar_significance)}" title={marker.clinvar_conditions ?? undefined}>
                    🏛️ ClinVar: {marker.clinvar_significance}
                    {#if marker.clinvar_review_status}
                      <span class="review-status">({marker.clinvar_review_status})</span>
                    {/if}
                  </span>
                </div>
              </details>
            {/if}
          {/if}
          {#if marker.population_rarity && marker.population_af != null}
            <span class="population-chip" title="gnomAD allele frequency">
              🌍 {marker.population_rarity} ({formatAf(marker.population_af)})
            </span>
          {/if}
          {#if marker.pharmgkb}
            <span class="pharmgkb-chip" style="background: rgba(59, 130, 246, 0.15); color: #60a5fa; border: 1px solid rgba(59, 130, 246, 0.3); padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.75rem;" title={marker.pharmgkb.phenotype}>
              💊 PharmGKB PGx: Level {marker.pharmgkb.evidence_level} ({marker.pharmgkb.drug})
            </span>
          {/if}
          {#if marker.clingen}
            <span class="clingen-chip" style="background: rgba(139, 92, 246, 0.15); color: #a78bfa; border: 1px solid rgba(139, 92, 246, 0.3); padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.75rem;" title={marker.clingen.disease_label}>
              🧬 ClinGen: {marker.clingen.classification}
            </span>
          {/if}
          {#if marker.mane}
            <span class="mane-chip" style="background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3); padding: 0.15rem 0.4rem; border-radius: 4px; font-size: 0.75rem;" title="MANE Status: {marker.mane.mane_status}">
              🧬 MANE: {marker.mane.refseq_transcript}
            </span>
          {/if}
        </div>
      {/if}

      <!-- GWAS context (collapsed by default, expand on click) -->
      {#if viewMode !== 'simple' && marker.gwas_top_trait && marker.gwas_best_pvalue != null && marker.gwas_best_pvalue < 1e-5}
        <details class="gwas-detail">
          <summary>📊 GWAS Association</summary>
          <div class="gwas-body">
            <strong>Top trait:</strong> {marker.gwas_top_trait}<br/>
            <strong>Best p-value:</strong> {formatPvalue(marker.gwas_best_pvalue)}
            {#if marker.gwas_association_count}
              <span class="gwas-count">({marker.gwas_association_count} studies)</span>
            {/if}
          </div>
        </details>
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
        <SourcesList sources={marker.sources} dbSources={marker.db_enriched_sources} />
      {/if}

      <!-- Simple mode: condensed source count instead of hiding all evidence -->
      {#if viewMode === 'simple'}
        <div class="simple-evidence-summary">
          {#if (marker.sources?.length ?? 0) > 0 || (marker.db_enriched_sources?.length ?? 0) > 0}
            <span class="evidence-count-pill">
              📚 {(marker.sources?.length ?? 0) + (marker.db_enriched_sources?.length ?? 0)} {((marker.sources?.length ?? 0) + (marker.db_enriched_sources?.length ?? 0)) === 1 ? 'source' : 'sources'}
            </span>
          {/if}
          {#if marker.clinvar_significance}
            {#if isHighPriorityClinvar(marker.clinvar_significance)}
              <span class="clinvar-simple">
                🏛️ ClinVar: <strong>{marker.clinvar_significance}</strong>
              </span>
            {:else}
              <details class="clinvar-simple-low-priority" style="display: inline-block;">
                <summary style="font-size: 0.72rem; cursor: pointer; color: var(--text-secondary); list-style: none; display: inline-flex; align-items: center; gap: 0.25rem;">
                  <span>🏛️ ClinVar (VUS/Benign)</span>
                </summary>
                <div style="margin-top: 0.25rem;">
                  <span class="clinvar-simple">
                    🏛️ ClinVar: <strong>{marker.clinvar_significance}</strong>
                  </span>
                </div>
              </details>
            {/if}
          {/if}
          {#if marker.population_rarity}
            <span class="pop-simple">🌍 {marker.population_rarity}</span>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .research-explore-link {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0 4px;
    margin-left: 4px;
    opacity: 0.6;
    transition: opacity 0.2s, transform 0.2s;
    vertical-align: middle;
  }

  .research-explore-link:hover {
    opacity: 1;
    transform: scale(1.2);
  }

  .nav-links {
    display: inline-flex;
    gap: 2px;
    margin-left: 4px;
  }

  :global(.marker-card-highlight) {
    outline: 2px solid rgba(56, 189, 248, 0.75);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px rgba(56, 189, 248, 0.15);
  }

  /* Reference enrichment chips */
  .enrichment-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0.5rem 0;
  }

  .clinvar-chip, .population-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .clinvar-pathogenic      { background: rgba(220, 38, 38, 0.15); color: #ef4444; border: 1px solid rgba(220, 38, 38, 0.3); }
  .clinvar-likely-pathogenic { background: rgba(234, 88, 12, 0.12); color: #f97316; border: 1px solid rgba(234, 88, 12, 0.25); }
  .clinvar-benign          { background: rgba(22, 163, 74, 0.12); color: #22c55e; border: 1px solid rgba(22, 163, 74, 0.25); }
  .clinvar-uncertain       { background: rgba(99, 102, 241, 0.12); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.25); }
  .clinvar-other           { background: rgba(100, 116, 139, 0.12); color: #94a3b8; border: 1px solid rgba(100, 116, 139, 0.25); }

  .review-status { opacity: 0.75; font-weight: 400; font-size: 0.68rem; }

  .population-chip { background: rgba(14, 165, 233, 0.1); color: #38bdf8; border: 1px solid rgba(14, 165, 233, 0.25); }

  .gwas-detail {
    margin: 0.4rem 0;
    padding: 0.3rem 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    font-size: 0.75rem;
  }
  .gwas-detail summary {
    cursor: pointer;
    font-weight: 600;
    color: #a78bfa;
    user-select: none;
    list-style: none;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .gwas-detail summary::-webkit-details-marker { display: none; }
  .gwas-body { padding: 0.4rem 0 0.1rem 0.25rem; line-height: 1.6; color: #cbd5e1; }
  .gwas-count { opacity: 0.7; font-size: 0.7rem; margin-left: 0.3rem; }

  /* Simple mode evidence summary */
  .simple-evidence-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid rgba(255,255,255,0.06);
  }
  .evidence-count-pill {
    font-size: 0.72rem;
    color: #94a3b8;
    padding: 0.15rem 0.4rem;
    background: rgba(148,163,184,0.1);
    border-radius: 999px;
  }
  .clinvar-simple, .pop-simple {
    font-size: 0.72rem;
    color: #94a3b8;
  }
  .clinvar-simple strong { color: #e2e8f0; }

  .genotype-info-icon {
    font-size: 0.72rem;
    opacity: 0.6;
    margin-left: 4px;
    cursor: help;
    user-select: none;
  }
</style>
