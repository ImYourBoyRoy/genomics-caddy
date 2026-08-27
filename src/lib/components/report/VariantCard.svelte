<!-- ./src/lib/components/report/VariantCard.svelte -->
<script lang="ts">
  import type { EvaluatedMarker, SeverityClass, EnrichedSource } from '../../types/genomics';
  import EvidenceBadge from './EvidenceBadge.svelte';
  import EffectDirectionBadge from './EffectDirectionBadge.svelte';
  import SourcesList from './SourcesList.svelte';
  import ConfirmWithList from './ConfirmWithList.svelte';
  import Tooltip from '../common/Tooltip.svelte';
  import { getEffectCount, getEffectAllele } from '../../utils/genotype';
  import { getClaimFrame, getScopeLabel, getSeverityInfo } from '../../utils/evidence';
  import { getCompactSimpleMeaning, getLaypersonTranslation, getSimpleFindingTitle, getSimpleNextStep } from '../../utils/layperson';
  import type { VariantNavTarget } from '../../constants/traitCategories';

  interface Props {
    marker: EvaluatedMarker;
    viewMode: "simple" | "clinical" | "compare";
    onExploreResearch?: (rsid: string) => void;
    highlightRsid?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
  }

  let { marker, viewMode, onExploreResearch, highlightRsid = "", onNavigateToVariant }: Props = $props();

  let effectCount = $derived(getEffectCount(marker));
  let effectAllele = $derived(getEffectAllele(marker));
  let severity = $derived(getSeverityInfo(marker.severity_class as SeverityClass));
  let laypersonTranslation = $derived(getLaypersonTranslation(marker));
  let simpleFindingTitle = $derived(getSimpleFindingTitle(laypersonTranslation.simpleImpact));
  let compactSimpleMeaning = $derived(getCompactSimpleMeaning(laypersonTranslation));

  /** True when the variant was actually detected (not benign / no_data) */
  let isActiveFindings = $derived(
    marker.severity_class !== "benign" && marker.severity_class !== "no_data"
  );
  let isHighlighted = $derived(
    highlightRsid !== "" && marker.rsid.toLowerCase() === highlightRsid.toLowerCase()
  );

  function cardAccessibleName(): string {
    if (viewMode === 'simple') return simpleFindingTitle;
    return `${marker.gene}${marker.variant_name ? ` ${marker.variant_name}` : ''}`;
  }

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

  function nextHelpfulStep(): string {
    return getSimpleNextStep(marker);
  }

</script>

<article
  id="variant-{marker.rsid.toLowerCase()}"
  class="marker-card {severity.cssClass}"
  class:marker-card-collapsed={!isActiveFindings}
  class:marker-card-highlight={isHighlighted}
  aria-label={cardAccessibleName()}
>
  <!-- Header row: a plain-language title in Simple mode; technical identity stays below. -->
  <div class="marker-top">
    {#if viewMode === 'simple'}
      <h5 class="gene-label simple-finding-title">
        <strong>{simpleFindingTitle}</strong>
      </h5>
    {:else}
      <h5 class="gene-label">
        <strong>{marker.gene}</strong>
        {#if marker.variant_name}
          <span class="variant-sub">({marker.variant_name})</span>
        {/if}
        <span class="rsid-sub">
          {marker.rsid}
          {#if marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
            <span class="nav-links no-print">
              <button type="button" class="research-explore-link" aria-label="Open on genome map" onclick={() => onNavigateToVariant?.(marker.rsid, "map")}>🗺️</button>
              <button type="button" class="research-explore-link" aria-label="Open in raw browser" onclick={() => onNavigateToVariant?.(marker.rsid, "browser")}>🔍</button>
              <button
                type="button"
                class="research-explore-link"
                onclick={() => onExploreResearch?.(marker.rsid)}
                aria-label="Search enriched vector research"
              >
                🔬
              </button>
            </span>
          {/if}
        </span>
      </h5>
    {/if}
    <EvidenceBadge tier={marker.evidence_tier} simple={viewMode === 'simple'} />
    {#if marker.sex_scope && marker.sex_scope !== 'all'}
      <Tooltip label={getScopeLabel(marker.sex_scope)} description="Biological applicability hint only; this is not gender identity, anatomy, fertility, pregnancy, or hormone status.">
        <span class="scope-badge">{getScopeLabel(marker.sex_scope)}</span>
      </Tooltip>
    {/if}
    {#if viewMode === 'simple'}
      <span class="marker-severity-label simple-severity-label">
        <span class="severity-glyph" aria-hidden="true">{severity.glyph}</span>
        {severity.plainLabel}
      </span>
    {/if}
  </div>

  <!-- Clinical status row keeps the full DNA call visible; Simple keeps raw calls in technical details. -->
  {#if viewMode !== 'simple'}
    <div class="marker-middle">
      <span class="genotype-val">Your Result: <strong>{marker.user_genotype}</strong></span>
      <span class="marker-severity-label">
        <span class="severity-glyph" aria-hidden="true">{severity.glyph}</span>
        {severity.label}
        {#if effectCount > 0 && isActiveFindings}
          <span class="allele-detail">({effectCount}× {effectAllele})</span>
        {/if}
      </span>
    </div>
  {/if}

  {#if isActiveFindings}
    <!-- Simple mode keeps the primary explanation concise; Clinical/Compare retain the full evidence framing. -->
    {#if viewMode !== 'simple'}
      <div class="severity-explainer">
        {severity.description}
      </div>
    {/if}
    {#if viewMode !== 'simple'}
      <details class="claim-details">
        <summary>Evidence boundary</summary>
        <div class="claim-frame" role="note">
          {getClaimFrame(marker.evidence_tier, marker.clinical_confirmation_required === true, marker.interpretation_allowed)}
        </div>
        {#if marker.clinical_confirmation_required || marker.severity_class === 'confirmation_required'}
          <p class="claim-detail-note">Consumer-array results may need clinical-laboratory confirmation before guiding care.</p>
        {/if}
        {#if marker.raw_dna_limitation}
          <p class="claim-detail-note">{marker.raw_dna_limitation}</p>
        {/if}
        {#if marker.do_not_claim.length > 0}
          <ul class="claim-limit-list">
            {#each marker.do_not_claim as item}
              <li>{item}</li>
            {/each}
          </ul>
        {/if}
      </details>
    {/if}

    <div class="marker-body">
      <!-- Direction badge: only when variant IS detected -->
      {#if marker.effect_direction && viewMode !== 'simple'}
        <EffectDirectionBadge direction={marker.effect_direction} />
      {/if}

      <!-- Reference DB enrichment chips (clinical + dual view only) -->
      {#if viewMode !== 'simple' && (marker.clinvar_significance || marker.population_rarity || marker.gwas_top_trait || marker.pharmgkb || marker.clingen || marker.mane)}
        <div class="enrichment-row">
          {#if marker.clinvar_significance}
            {#if isHighPriorityClinvar(marker.clinvar_significance)}
              <Tooltip label="ClinVar classification" description={marker.clinvar_conditions || 'A local clinical-variant database annotation; review status and conditions are shown in Clinical details.'}>
                <span class="clinvar-chip clinvar-{clinvarClass(marker.clinvar_significance)}">
                  🏛️ ClinVar: {marker.clinvar_significance}
                  {#if marker.clinvar_review_status}
                    <span class="review-status">({marker.clinvar_review_status})</span>
                  {/if}
                </span>
              </Tooltip>
            {:else}
              <details class="clinvar-low-priority-detail">
                <summary class="enrichment-detail-summary">
                  <span>🏛️ ClinVar (VUS/Benign)</span>
                </summary>
                <div class="enrichment-detail-body">
                  <Tooltip label="ClinVar classification" description={marker.clinvar_conditions || 'A local clinical-variant database annotation; review status and conditions are shown in Clinical details.'}>
                    <span class="clinvar-chip clinvar-{clinvarClass(marker.clinvar_significance)}">
                      🏛️ ClinVar: {marker.clinvar_significance}
                      {#if marker.clinvar_review_status}
                        <span class="review-status">({marker.clinvar_review_status})</span>
                      {/if}
                    </span>
                  </Tooltip>
                </div>
              </details>
            {/if}
          {/if}
          {#if marker.population_rarity && marker.population_af != null}
            <Tooltip label="gnomAD allele frequency" description="Population frequency context from the local gnomAD cache; this is not a personal disease probability.">
              <span class="population-chip">
                🌍 {marker.population_rarity} ({formatAf(marker.population_af)})
              </span>
            </Tooltip>
          {/if}
          {#if marker.pharmgkb}
            <Tooltip label="PharmGKB medication context" description={marker.pharmgkb.phenotype || 'A medication-response annotation that requires the complete clinical PGx context.'}>
              <span class="pharmgkb-chip">
                💊 PharmGKB PGx: Level {marker.pharmgkb.evidence_level} ({marker.pharmgkb.drug})
              </span>
            </Tooltip>
          {/if}
          {#if marker.clingen}
            <Tooltip label="ClinGen classification" description={marker.clingen.disease_label || 'A gene-validity annotation from the local ClinGen catalog.'}>
              <span class="clingen-chip">
                🧬 ClinGen: {marker.clingen.classification}
              </span>
            </Tooltip>
          {/if}
          {#if marker.mane}
            <Tooltip label="MANE transcript" description={`MANE status: ${marker.mane.mane_status}`}>
              <span class="mane-chip">
                🧬 MANE: {marker.mane.refseq_transcript}
              </span>
            </Tooltip>
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

      {#if viewMode === 'simple'}
        <div class="simple-meaning-block">
          <strong>What this might mean for you:</strong>
          <span class="layperson-text">{compactSimpleMeaning}</span>
        </div>

        <div class="simple-next-step">
          <strong>Next helpful step:</strong>
          <span>{nextHelpfulStep()}</span>
        </div>

        <!-- Simple mode: plain context labels; exact catalog data stays in technical details. -->
        <div class="simple-evidence-summary">
          {#if (marker.sources?.length ?? 0) > 0 || (marker.db_enriched_sources?.length ?? 0) > 0}
            <span class="evidence-count-pill">
              📚 Evidence sources available
            </span>
          {/if}
          {#if marker.clinvar_significance}
            <span class="simple-context-pill">🏛️ Medical reference context available</span>
          {/if}
          {#if marker.population_rarity}
            <span class="simple-context-pill">🌍 Population context available</span>
          {/if}
        </div>
        <details class="technical-details">
          <summary>Technical data</summary>
          <dl>
            <div><dt>Gene / marker</dt><dd>{marker.gene} · {marker.rsid}</dd></div>
            <div><dt>DNA call</dt><dd>{marker.user_genotype}</dd></div>
            <div><dt>Evidence tier</dt><dd>{marker.evidence_tier}</dd></div>
            <div><dt>Claim boundary</dt><dd>{getClaimFrame(marker.evidence_tier, marker.clinical_confirmation_required === true, marker.interpretation_allowed)}</dd></div>
            {#if marker.clinvar_significance}
              <div>
                <dt>Clinical catalog</dt>
                <dd>
                  {marker.clinvar_significance}
                  {#if marker.clinvar_review_status} · {marker.clinvar_review_status}{/if}
                  {#if marker.clinvar_conditions} · {marker.clinvar_conditions}{/if}
                </dd>
              </div>
            {/if}
            {#if marker.population_rarity}
              <div>
                <dt>Population context</dt>
                <dd>
                  {marker.population_rarity}
                  {#if marker.population_af != null} ({formatAf(marker.population_af)}){/if}
                </dd>
              </div>
            {/if}
          </dl>
          {#if marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
            <ConfirmWithList confirmWith={marker.confirm_with} />
          {/if}
        </details>
        {#if marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
          <SourcesList sources={marker.sources} dbSources={marker.db_enriched_sources} />
        {/if}
      {:else}
        <div class="impact-section">
          <strong>What this gene does:</strong>
          {#if viewMode === "clinical"}
            <span class="clinical-text">{marker.impact}</span>
          {:else}
            <div class="dual-explanations">
              <div class="dual-row layperson-box">
                <span class="dual-tag layperson-tag">🌱 Simple:</span>
                <span class="layperson-text">{laypersonTranslation.simpleImpact}</span>
              </div>
              <div class="dual-row clinical-box">
                <span class="dual-tag clinical-tag">🏥 Medical:</span>
                <span class="clinical-text">{marker.impact}</span>
              </div>
            </div>
          {/if}
        </div>

        <div class="interpretation-section">
          <strong>What your result means:</strong>
          {#if viewMode === "clinical"}
            <span class="clinical-text">{marker.interpretation}</span>
          {:else}
            <div class="dual-explanations">
              <div class="dual-row layperson-box">
                <span class="dual-tag layperson-tag">🌱 Simple:</span>
                <span class="layperson-text">{laypersonTranslation.simpleMeaning}</span>
              </div>
              <div class="dual-row clinical-box">
                <span class="dual-tag clinical-tag">🏥 Medical:</span>
                <span class="clinical-text">{marker.interpretation}</span>
              </div>
            </div>
          {/if}
        </div>

        {#if marker.user_genotype !== "--" && !marker.user_genotype.includes('-')}
          <ConfirmWithList confirmWith={marker.confirm_with} />
          <SourcesList sources={marker.sources} dbSources={marker.db_enriched_sources} />
        {/if}
      {/if}
    </div>
  {/if}
</article>

<style>
  .research-explore-link {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0 4px;
    min-width: 44px;
    min-height: 44px;
    margin-left: 4px;
    opacity: 0.6;
    transition: opacity 0.2s, transform 0.2s;
    vertical-align: middle;
  }

  .simple-finding-title {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .marker-card .gene-label {
    margin: 0;
  }

  .simple-finding-title strong {
    color: var(--text-primary);
    font-size: 0.95rem;
    line-height: 1.3;
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
    outline: 2px solid var(--status-info-text);
    outline-offset: 2px;
    box-shadow: 0 0 0 4px var(--status-info-border);
  }

  .claim-frame {
    margin: 0.45rem 0 0.7rem;
    padding: 0.45rem 0.6rem;
    border-left: 3px solid var(--report-claim-border);
    background: var(--report-claim-bg);
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.4;
  }

  .claim-context {
    margin: 0.45rem 0 0;
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.4;
  }

  .claim-details {
    margin: 0.15rem 0 0.35rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .claim-details summary {
    width: fit-content;
    max-width: 100%;
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 700;
    line-height: 1.35;
  }

  .claim-details summary:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
    border-radius: 0.25rem;
  }

  .claim-details[open] summary {
    color: var(--text-primary);
  }

  .claim-details .claim-frame {
    margin-bottom: 0;
  }

  .claim-detail-note {
    margin: 0.45rem 0 0;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .claim-limit-list {
    margin: 0.45rem 0 0;
    padding-left: 1.1rem;
    color: var(--text-secondary);
  }

  /* Reference enrichment chips */
  .enrichment-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0.5rem 0 0.75rem;
  }

  .clinvar-low-priority-detail {
    margin-top: 0.2rem;
  }

  .enrichment-detail-summary {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.72rem;
    list-style: none;
  }

  .enrichment-detail-summary::-webkit-details-marker {
    display: none;
  }

  .enrichment-detail-body {
    margin-top: 0.25rem;
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

  .clinvar-pathogenic {
    background: var(--status-danger-bg);
    color: var(--status-danger-strong-text);
    border: 1px solid var(--status-danger-border);
  }
  .clinvar-likely-pathogenic {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
    border: 1px solid var(--status-warning-border);
  }
  .clinvar-benign {
    background: var(--status-success-bg);
    color: var(--status-success-text);
    border: 1px solid var(--status-success-border);
  }
  .clinvar-uncertain {
    background: var(--status-accent-bg);
    color: var(--status-accent-text);
    border: 1px solid var(--status-accent-border);
  }
  .clinvar-other {
    background: var(--report-severity-neutral-bg);
    color: var(--report-severity-neutral-text);
    border: 1px solid var(--report-severity-neutral-border);
  }

  .review-status { opacity: 0.75; font-weight: 400; font-size: 0.68rem; }

  .population-chip {
    background: var(--status-info-soft-bg);
    color: var(--status-info-soft-text);
    border: 1px solid var(--status-info-soft-border);
  }

  .pharmgkb-chip,
  .clingen-chip,
  .mane-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .pharmgkb-chip {
    background: var(--status-info-bg);
    color: var(--status-info-text);
    border: 1px solid var(--status-info-border);
  }

  .clingen-chip {
    background: var(--status-accent-bg);
    color: var(--status-accent-text);
    border: 1px solid var(--status-accent-border);
  }

  .mane-chip {
    background: var(--status-success-bg);
    color: var(--status-success-text);
    border: 1px solid var(--status-success-border);
  }

  .gwas-detail {
    margin: 0.4rem 0;
    padding: 0.3rem 0.5rem;
    background: var(--report-explainer-bg);
    border: 1px solid var(--report-explainer-border);
    border-radius: 6px;
    font-size: 0.75rem;
  }
  .gwas-detail summary {
    cursor: pointer;
    font-weight: 600;
    color: var(--status-accent-text);
    user-select: none;
    list-style: none;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .gwas-detail summary::-webkit-details-marker { display: none; }
  .gwas-body {
    padding: 0.4rem 0 0.1rem 0.25rem;
    line-height: 1.6;
    color: var(--text-secondary);
  }
  .gwas-count { opacity: 0.7; font-size: 0.7rem; margin-left: 0.3rem; }

  /* Simple mode evidence summary */
  .simple-evidence-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.35rem;
  }

  .simple-meaning-block {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.15rem 0;
    color: var(--text-primary);
    font-size: 0.82rem;
    line-height: 1.5;
  }

  .simple-meaning-block strong {
    font-size: 0.76rem;
    letter-spacing: 0.01em;
  }

  .simple-meaning-block .layperson-text {
    color: var(--text-secondary);
  }

  .evidence-count-pill {
    font-size: 0.72rem;
    color: var(--text-secondary);
    padding: 0.15rem 0.4rem;
    background: var(--surface-subtle);
    border: 1px solid var(--border-color);
    border-radius: 999px;
  }

  .simple-context-pill {
    font-size: 0.72rem;
    color: var(--text-secondary);
  }

  .genotype-info-icon {
    font-size: 0.72rem;
    opacity: 0.6;
    margin-left: 4px;
    cursor: help;
    user-select: none;
  }

  .simple-severity-label {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    line-height: 1.35;
  }

  .simple-severity-label .severity-glyph {
    flex: 0 0 auto;
    font-size: 0.74rem;
  }

  .simple-next-step {
    display: block;
    padding: 0.45rem 0.6rem;
    border-radius: 0.5rem;
    background: var(--accent-soft);
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.4;
  }

  .simple-next-step strong {
    color: var(--text-primary);
    margin-right: 0.3rem;
  }

  .technical-details {
    margin-top: 0.25rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.55rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .technical-details summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 700;
  }

  .technical-details dl {
    display: grid;
    gap: 0.4rem;
    margin: 0.7rem 0;
  }

  .technical-details dl > div {
    display: grid;
    grid-template-columns: minmax(6rem, 0.35fr) 1fr;
    gap: 0.6rem;
  }

  .technical-details dt {
    color: var(--text-secondary);
    font-weight: 700;
  }

  .technical-details dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
