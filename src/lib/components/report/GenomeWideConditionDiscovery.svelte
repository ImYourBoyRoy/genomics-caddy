<script lang="ts">
  import type { GenomeWideClinVarDiscovery, GenomeWideConditionRelevance } from '../../types/genomics';
  import {
    copyCountLabel,
    groupGenomeWideConditionAssociations,
    sectionConditionGroupsByRelevance,
  } from '../../utils/genomewideConditionDiscovery';

  interface Props {
    discovery: GenomeWideClinVarDiscovery;
  }

  const INITIAL_VISIBLE = 8;

  let { discovery }: Props = $props();
  let expanded = $state<Partial<Record<GenomeWideConditionRelevance, boolean>>>({});
  const conditionGroups = $derived(groupGenomeWideConditionAssociations(discovery.associations));
  const sections = $derived(sectionConditionGroupsByRelevance(conditionGroups));
  const summaryCounts = $derived(
    sections.map((section) => `${section.groups.length.toLocaleString()} ${section.title.toLocaleLowerCase()}`).join(' · ')
  );

  function toggleSection(relevance: GenomeWideConditionRelevance) {
    expanded[relevance] = !expanded[relevance];
  }
</script>

<details class="genomewide-discovery summary-card card">
  <summary class="genomewide-discovery-summary">
    <span class="genomewide-discovery-heading">
      <span class="section-kicker">Local reference scan</span>
      <span class="genomewide-discovery-title">Potential disease associations</span>
      <span class="genomewide-discovery-hint">
        {#if !discovery.local_index_ready}
          ClinVar condition data is not downloaded yet
        {:else if conditionGroups.length > 0}
          {summaryCounts}
        {:else}
          No linked conditions found
        {/if}
      </span>
    </span>
    {#if discovery.local_index_ready && conditionGroups.length > 0}
      <span class="genomewide-discovery-count">{conditionGroups.length.toLocaleString()}</span>
    {/if}
  </summary>

  <div class="genomewide-discovery-body">
    {#if !discovery.local_index_ready}
      <p class="genomewide-discovery-status">
        Download the ClinVar variant summary and submission summary in Data &amp; updates to see linked conditions.
      </p>
    {:else if discovery.allele_orientation.startsWith('Unverified')}
      <p class="genomewide-discovery-status">
        This file's DNA strand could not be confirmed, so variants were not matched to conditions. Re-import the original vendor file to try again.
      </p>
    {:else if discovery.associations.length === 0}
      <p class="genomewide-discovery-status">
        None of the {discovery.genotypes_scanned.toLocaleString()} calls in this profile match a reviewed disease-causing or risk-factor record in ClinVar.
      </p>
    {:else}
      <p class="genomewide-discovery-meta">
        {discovery.genotypes_scanned.toLocaleString()} calls checked against reviewed ClinVar records · {discovery.allele_orientation}
      </p>

      {#each sections as section (section.relevance)}
        {@const showAll = expanded[section.relevance] ?? false}
        <section
          class="genomewide-relevance"
          class:genomewide-relevance-priority={section.relevance === 'may_be_relevant'}
          aria-label={section.title}
        >
          <header class="genomewide-relevance-header">
            <h3>
              {section.title}
              <span>
                {section.groups.length.toLocaleString()} {section.groups.length === 1 ? 'condition' : 'conditions'}
                from {section.variantCount.toLocaleString()} {section.variantCount === 1 ? 'variant' : 'variants'}
              </span>
            </h3>
            <p>{section.description}</p>
          </header>
          <div class="genomewide-condition-list">
            {#each showAll ? section.groups : section.groups.slice(0, INITIAL_VISIBLE) as group (group.id)}
              <article class="genomewide-condition">
                <div class="genomewide-condition-heading">
                  <div>
                    <span class="genomewide-condition-topic">{group.categoryLabel}</span>
                    <h4>{group.condition}</h4>
                  </div>
                  <span class="genomewide-condition-count">
                    {copyCountLabel(group.maxCopies)}{#if group.inheritance.length} · {group.inheritance.join(' / ')}{/if}
                  </span>
                </div>
                {#if group.conflicts}
                  <p class="genomewide-condition-conflict">
                    ClinVar submitters disagree about this variant; check the individual records below.
                  </p>
                {/if}
                <details class="genomewide-condition-details">
                  <summary>Matched variants &amp; ClinVar evidence ({group.assertions.length} submissions)</summary>
                  <ul>
                    {#each group.assertions.slice(0, 5) as assertion (`${assertion.rsid}:${assertion.variation_id}:${assertion.scv_accession}:${assertion.condition}`)}
                      <li>
                        <span>
                          <strong>{assertion.rsid}</strong>
                          {#if assertion.gene_symbol} · {assertion.gene_symbol}{/if}
                          <small>
                            {assertion.clinical_significance} · {assertion.review_status} · {assertion.origin_status}
                            {#if assertion.variant_summary_clinical_significance}
                              <br />Variant-wide summary: {assertion.variant_summary_clinical_significance}
                            {/if}
                            <br />{copyCountLabel(assertion.alt_allele_copies)}
                          </small>
                        </span>
                        <a href={assertion.source_url} target="_blank" rel="noopener noreferrer">{assertion.scv_accession}</a>
                      </li>
                    {/each}
                  </ul>
                  {#if group.assertions.length > 5}
                    <p class="genomewide-discovery-footnote">
                      {group.assertions.length - 5} more submission records are in the exported report data.
                    </p>
                  {/if}
                </details>
              </article>
            {/each}
          </div>
          {#if section.groups.length > INITIAL_VISIBLE}
            <button class="genomewide-discovery-more" type="button" onclick={() => toggleSection(section.relevance)}>
              {showAll ? 'Show fewer' : `Show all ${section.groups.length.toLocaleString()}`}
            </button>
          {/if}
        </section>
      {/each}
      {#if discovery.omitted_association_count > 0}
        <p class="genomewide-discovery-footnote">
          {discovery.omitted_association_count.toLocaleString()} lower-priority ClinVar records were left out to keep the report fast.
        </p>
      {/if}
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
  .genomewide-discovery-status { margin: 0.25rem 0 0; }
  .genomewide-discovery-meta { margin: 0; overflow-wrap: anywhere; }
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
  .genomewide-condition-details a { flex: 0 0 auto; color: var(--accent-strong, var(--accent-primary)); }
  .genomewide-discovery-more { width: fit-content; margin: 0.1rem 0 0; padding: 0.2rem 0; border: 0; color: var(--accent-strong, var(--accent-primary)); background: transparent; font: inherit; font-size: 0.73rem; text-align: left; cursor: pointer; }
  .genomewide-discovery-footnote { margin: 0.4rem 0 0; font-size: 0.69rem; }
  .genomewide-relevance { display: grid; gap: 0.45rem; margin-top: 0.9rem; }
  .genomewide-relevance-header h3 { display: flex; align-items: baseline; gap: 0.45rem; margin: 0; color: var(--text-primary); font-size: 0.86rem; }
  .genomewide-relevance-header h3 span { color: var(--text-secondary); font-size: 0.72rem; font-variant-numeric: tabular-nums; }
  .genomewide-relevance-header p { margin: 0.15rem 0 0; max-width: 78ch; color: var(--text-secondary); font-size: 0.74rem; line-height: 1.5; }
  .genomewide-relevance-priority .genomewide-condition { border-left: 3px solid var(--status-warning-text, #a16b12); }

  @media (max-width: 580px) {
    .genomewide-discovery { padding: 0.7rem; }
    .genomewide-condition-heading { align-items: flex-start; flex-direction: column; gap: 0.2rem; }
    .genomewide-condition-details li { flex-direction: column; }
  }
</style>
