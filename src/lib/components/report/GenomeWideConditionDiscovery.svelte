<script lang="ts">
  import type { GenomeWideClinVarDiscovery } from '../../types/genomics';
  import { getConditionCoverageGaps } from '../../utils/conditionEvidence';
  import { groupGenomeWideConditionAssociations } from '../../utils/genomewideConditionDiscovery';
  import sourceRegistry from '../../marker-packs/source_registry.json';

  interface Props {
    discovery: GenomeWideClinVarDiscovery;
  }

  interface SourceReference {
    name: string;
    url?: string;
  }

  let { discovery }: Props = $props();
  let visibleLimit = $state(8);
  const conditionGroups = $derived(groupGenomeWideConditionAssociations(discovery.associations));
  const visibleGroups = $derived(conditionGroups.slice(0, visibleLimit));
  const hasMore = $derived(conditionGroups.length > visibleLimit || discovery.omitted_association_count > 0);
  const contextGaps = getConditionCoverageGaps().filter((gap) => gap.id === 'pots' && gap.clinical_next_step);
  const sources = (sourceRegistry as { sources: Record<string, SourceReference> }).sources;

  function gapSourceLinks(ids: readonly string[]): Array<{ name: string; url: string }> {
    return ids.flatMap((id) => {
      const source = sources[id];
      return source?.url ? [{ name: source.name, url: source.url }] : [];
    });
  }

  function toggleVisibleGroups() {
    visibleLimit = visibleLimit >= conditionGroups.length ? 8 : conditionGroups.length;
  }
</script>

<details class="genomewide-discovery summary-card card">
  <summary class="genomewide-discovery-summary">
    <span class="genomewide-discovery-heading">
      <span class="section-kicker">Local reference scan</span>
      <span class="genomewide-discovery-title">Potential disease associations</span>
      <span class="genomewide-discovery-hint">
        {#if discovery.local_index_ready}
          {discovery.exact_variant_count.toLocaleString()} matched variants · {conditionGroups.length.toLocaleString()} conditions
        {:else}
          Local ClinVar indexes are not ready
        {/if}
      </span>
    </span>
    {#if discovery.local_index_ready && conditionGroups.length > 0}
      <span class="genomewide-discovery-count">{conditionGroups.length.toLocaleString()}</span>
    {/if}
  </summary>

  <div class="genomewide-discovery-body">
    <p class="genomewide-discovery-intro">
      This section matches imported DNA calls against ClinVar’s condition-specific submissions. It is not a diagnosis,
      a personal-risk estimate, or a complete disease screen. The report keeps matched variants, review status, and source records together.
    </p>
    <p class="genomewide-discovery-footnote">
      A matching allele alone does not resolve inheritance, phase, penetrance, or clinical fit; array calls need appropriate clinical confirmation before action.
    </p>

    {#if !discovery.local_index_ready}
      <p class="genomewide-discovery-status">
        Download and index both the ClinVar variant summary and submission summary to enable this local scan.
      </p>
    {:else if discovery.allele_orientation.startsWith('Unverified')}
      <p class="genomewide-discovery-status">
        {discovery.allele_orientation}. Exact allele matching was skipped; no disease association is inferred from an unverified call.
      </p>
    {:else if discovery.associations.length === 0}
      <p class="genomewide-discovery-status">
        No qualifying exact-allele ClinVar condition submissions were found in the indexed data. This does not rule out any condition.
      </p>
    {:else}
      <div class="genomewide-discovery-meta">
        <span>{discovery.genotypes_scanned.toLocaleString()} imported calls scanned</span>
        <span>{discovery.allele_orientation}</span>
        <span>ClinVar records with explicit somatic/oncogenic labels are excluded</span>
      </div>

      <div class="genomewide-condition-list">
        {#each visibleGroups as group (group.id)}
          <article class="genomewide-condition">
            <div class="genomewide-condition-heading">
              <div>
                <span class="genomewide-condition-topic">{group.categoryLabel}</span>
                <h4>{group.condition}</h4>
              </div>
              <span class="genomewide-condition-count">
                {group.variantCount} {group.variantCount === 1 ? 'matched variant' : 'matched variants'}
              </span>
            </div>
            {#if group.conflicts}
              <p class="genomewide-condition-conflict">
                ClinVar's variant-wide summary has conflicting classifications; that conflict may span conditions. Review the condition-specific submissions with a genetics professional.
              </p>
            {/if}
            <details class="genomewide-condition-details">
              <summary>Matched variants &amp; ClinVar evidence ({group.assertions.length} submissions)</summary>
              <ul>
                {#each group.assertions.slice(0, 5) as assertion (`${assertion.rsid}:${assertion.scv_accession}`)}
                  <li>
                    <span>
                      <strong>{assertion.rsid}</strong>
                      {#if assertion.gene_symbol} · {assertion.gene_symbol}{/if}
                      <small>
                        {assertion.clinical_significance} · {assertion.review_status} · {assertion.origin_status}
                        {#if assertion.variant_summary_clinical_significance}
                          <br />Variant-wide summary: {assertion.variant_summary_clinical_significance}
                        {/if}
                        <br />{assertion.association_is} · {assertion.association_scope}-level
                      </small>
                    </span>
                    <a href={assertion.source_url} target="_blank" rel="noopener noreferrer">{assertion.scv_accession}</a>
                  </li>
                {/each}
              </ul>
              {#if group.assertions.length > 5}
                <p class="genomewide-discovery-footnote">
                  {group.assertions.length - 5} additional submission records remain in the report data.
                </p>
              {/if}
            </details>
          </article>
        {/each}
      </div>

      {#if hasMore}
        <button class="genomewide-discovery-more" type="button" onclick={toggleVisibleGroups}>
          {visibleLimit >= conditionGroups.length
            ? 'Showing all available conditions'
            : `Show ${Math.min(8, conditionGroups.length - visibleLimit)} more conditions`}
          {#if discovery.omitted_association_count > 0}
            <span>{discovery.omitted_association_count.toLocaleString()} additional source associations capped</span>
          {/if}
        </button>
      {/if}
    {/if}

    {#if contextGaps.length > 0}
      <section class="genomewide-context" aria-label="Clinical context not scored from DNA">
        <span class="section-kicker">Clinical context · not DNA-scored</span>
        {#each contextGaps as gap (gap.id)}
          <article>
            <h4>{gap.label}</h4>
            <p>{gap.display}</p>
            {#if gap.clinical_next_step}<p>{gap.clinical_next_step}</p>{/if}
            {#each gapSourceLinks(gap.sources || []) as source (source.url)}
              <a href={source.url} target="_blank" rel="noopener noreferrer">{source.name}</a>
            {/each}
          </article>
        {/each}
      </section>
    {/if}
  </div>
</details>

<style>
  .genomewide-discovery {
    min-width: 0;
    margin: 0 0 0.8rem;
    padding: 0.8rem 1rem;
    border: 1px solid var(--border-subtle, rgba(120, 130, 145, 0.22));
    border-radius: 0.85rem;
    background: var(--surface-card, var(--card-bg, rgba(255, 255, 255, 0.025)));
  }

  .genomewide-discovery-summary {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
    cursor: pointer;
    list-style: none;
  }

  .genomewide-discovery-summary::-webkit-details-marker { display: none; }
  .genomewide-discovery-heading { display: grid; min-width: 0; gap: 0.12rem; }
  .genomewide-discovery-title { color: var(--text-primary); font-size: 0.94rem; font-weight: 680; }
  .genomewide-discovery-hint,
  .genomewide-discovery-intro,
  .genomewide-discovery-status,
  .genomewide-discovery-meta,
  .genomewide-discovery-footnote { color: var(--text-secondary); font-size: 0.76rem; line-height: 1.5; }
  .genomewide-discovery-count {
    flex: 0 0 auto;
    color: var(--text-secondary);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .genomewide-discovery-body { min-width: 0; padding-top: 0.65rem; }
  .genomewide-discovery-intro { margin: 0 0 0.7rem; max-width: 78ch; }
  .genomewide-discovery-status { margin: 0.25rem 0 0; }
  .genomewide-discovery-meta { display: flex; flex-wrap: wrap; gap: 0.3rem 1rem; margin: 0 0 0.65rem; }
  .genomewide-condition-list { display: grid; gap: 0.45rem; }
  .genomewide-condition {
    min-width: 0;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--border-subtle, rgba(120, 130, 145, 0.16));
    border-radius: 0.65rem;
    background: var(--surface-subtle, rgba(120, 130, 145, 0.045));
  }
  .genomewide-condition-heading { display: flex; min-width: 0; align-items: baseline; justify-content: space-between; gap: 0.5rem 1rem; }
  .genomewide-condition-topic { color: var(--text-secondary); font-size: 0.66rem; font-weight: 650; }
  .genomewide-condition h4 { margin: 0.13rem 0 0; color: var(--text-primary); font-size: 0.84rem; overflow-wrap: anywhere; }
  .genomewide-condition-count { flex: 0 0 auto; color: var(--text-secondary); font-size: 0.7rem; }
  .genomewide-condition-conflict { margin: 0.45rem 0 0; color: var(--status-warning-text, #a16b12); font-size: 0.73rem; line-height: 1.45; }
  .genomewide-condition-details { margin-top: 0.45rem; color: var(--text-secondary); font-size: 0.72rem; }
  .genomewide-condition-details summary { cursor: pointer; }
  .genomewide-condition-details ul { display: grid; gap: 0.35rem; margin: 0.45rem 0 0; padding: 0; list-style: none; }
  .genomewide-condition-details li { display: flex; min-width: 0; justify-content: space-between; gap: 0.35rem 0.7rem; }
  .genomewide-condition-details li > span { min-width: 0; overflow-wrap: anywhere; }
  .genomewide-condition-details small { display: block; margin-top: 0.12rem; color: var(--text-tertiary, var(--text-secondary)); font-size: 0.67rem; }
  .genomewide-condition-details a,
  .genomewide-context a { flex: 0 0 auto; color: var(--accent-strong, var(--accent-primary)); }
  .genomewide-discovery-more { display: grid; gap: 0.1rem; margin: 0.55rem 0 0; padding: 0.2rem 0; border: 0; color: var(--accent-strong, var(--accent-primary)); background: transparent; font: inherit; font-size: 0.73rem; text-align: left; cursor: pointer; }
  .genomewide-discovery-more span { color: var(--text-secondary); font-size: 0.67rem; }
  .genomewide-discovery-footnote { margin: 0.4rem 0 0; font-size: 0.69rem; }
  .genomewide-context { display: grid; gap: 0.5rem; margin-top: 0.8rem; padding-top: 0.75rem; border-top: 1px solid var(--border-subtle, rgba(120, 130, 145, 0.2)); }
  .genomewide-context article { display: grid; gap: 0.28rem; }
  .genomewide-context h4 { margin: 0; color: var(--text-primary); font-size: 0.8rem; }
  .genomewide-context p { margin: 0; color: var(--text-secondary); font-size: 0.74rem; line-height: 1.5; }
  .genomewide-context a { width: fit-content; font-size: 0.72rem; }

  @media (max-width: 580px) {
    .genomewide-discovery { padding: 0.7rem; }
    .genomewide-condition-heading { align-items: flex-start; flex-direction: column; gap: 0.2rem; }
    .genomewide-condition-details li { flex-direction: column; }
  }
</style>
