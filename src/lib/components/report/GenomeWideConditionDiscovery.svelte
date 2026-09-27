<script lang="ts">
  import type { GenomeWideClinVarDiscovery, GenomeWideConditionRelevance } from '../../types/genomics';
  import {
    copyCountLabel,
    groupGenomeWideConditionAssociations,
    sectionConditionGroupsByRelevance,
  } from '../../utils/genomewideConditionDiscovery';
  import { plainConditionSummary, yourCopiesLabel } from '../../utils/plainConditionCopy';
  import Tooltip from '../common/Tooltip.svelte';

  interface Props {
    discovery: GenomeWideClinVarDiscovery;
  }

  const INITIAL_VISIBLE = 8;
  const INITIAL_VISIBLE_LAB_REPORTS = 5;

  let { discovery }: Props = $props();
  let expanded = $state<Partial<Record<GenomeWideConditionRelevance, boolean>>>({});
  let expandedLabReports = $state<Record<string, boolean>>({});
  const conditionGroups = $derived(groupGenomeWideConditionAssociations(discovery.associations));
  const sections = $derived(sectionConditionGroupsByRelevance(conditionGroups));
  const summaryCounts = $derived(
    sections.map((section) => {
      const label = section.relevance === 'may_be_relevant'
        ? 'possible links to review'
        : section.relevance === 'carrier'
          ? `possible carrier pattern${section.groups.length === 1 ? '' : 's'}`
          : 'with unclear or mixed evidence';
      return `${section.groups.length.toLocaleString()} ${label}`;
    }).join(' · ')
  );

  function toggleSection(relevance: GenomeWideConditionRelevance) {
    expanded[relevance] = !expanded[relevance];
  }

  function toggleLabReports(id: string) {
    expandedLabReports[id] = !expandedLabReports[id];
  }
</script>

<details class="genomewide-discovery summary-card card">
  <summary class="genomewide-discovery-summary">
    <span class="genomewide-discovery-heading">
      <span class="section-kicker">ClinVar lab reports</span>
      <span class="genomewide-discovery-title">Condition links found at your DNA markers</span>
      <span class="genomewide-discovery-hint">
        {#if !discovery.local_index_ready}
          ClinVar condition data is not downloaded yet
        {:else if conditionGroups.length > 0}
          {conditionGroups.length.toLocaleString()} ClinVar condition links ({summaryCounts}). These links are not diagnoses.
        {:else}
          No condition matches found
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
        No matching condition records were found among the {discovery.genotypes_scanned.toLocaleString()} DNA positions in this file.
      </p>
    {:else}
      <p class="genomewide-discovery-meta" title={discovery.allele_orientation}>
        ClinVar is a public database where labs submit variant findings. We checked {discovery.genotypes_scanned.toLocaleString()} DNA positions from your file against it; this is not a measure of how much of your whole genome was read. Each item links one condition to one marker.
      </p>

      {#each sections as section (section.relevance)}
        {@const showAll = expanded[section.relevance] ?? false}
        {@const groups = showAll ? section.groups : section.groups.slice(0, INITIAL_VISIBLE)}
        {#snippet conditionList()}
          <div class="genomewide-condition-list">
            {#each groups as group (group.id)}
              <article class="genomewide-condition" class:genomewide-condition-unnamed={group.unnamed}>
                <div class="genomewide-condition-heading">
                  <div class="genomewide-condition-title">
                    {#if group.categoryId !== 'other' && !group.unnamed}
                      <span class="genomewide-condition-topic">{group.categoryLabel}</span>
                    {/if}
                    <h4>{group.condition}</h4>
                    <span class="genomewide-condition-marker">
                      DNA marker {group.rsid}{#if group.geneSymbol} · {group.geneSymbol} gene{/if}
                    </span>
                  </div>
                  <Tooltip
                    label="DNA variant copy count"
                    description={group.copyCount === null
                      ? 'The lab reports disagree about the count at this marker, so no single number is shown.'
                      : 'Number of copies of this DNA variant found at this marker. A copy count by itself does not mean you have the condition.'}
                    interactiveChildren
                  >
                    <button class="genomewide-condition-count" type="button">
                      {yourCopiesLabel(group.copyCount)} <span aria-hidden="true">ⓘ</span>
                    </button>
                  </Tooltip>
                </div>
                <p class="genomewide-condition-plain">
                  {plainConditionSummary(group)}
                </p>
                <details class="genomewide-condition-details">
                  <summary>View lab reports ({group.assertions.length})</summary>
                  <ul>
                    {#each (expandedLabReports[group.id] ? group.assertions : group.assertions.slice(0, INITIAL_VISIBLE_LAB_REPORTS)) as assertion (`${assertion.rsid}:${assertion.variation_id}:${assertion.scv_accession}:${assertion.condition}`)}
                      <li>
                        <span>
                          <strong>{assertion.scv_accession}</strong>
                          <small>
                            Lab classification: {assertion.clinical_significance || 'Not provided'} · {assertion.review_status || 'Review status not provided'}
                            {#if assertion.variant_summary_clinical_significance}
                              <br />Variant-wide summary (not specific to this condition): {assertion.variant_summary_clinical_significance}
                            {/if}
                            {#if group.unnamed}<br />Original database label: {assertion.condition}{/if}
                            <br />Copies of this variant: {copyCountLabel(assertion.alt_allele_copies)}
                            {#if assertion.inheritance?.length}<br />Inheritance listed: {assertion.inheritance.join(' / ')}{/if}
                          </small>
                        </span>
                        <a href={assertion.source_url} target="_blank" rel="noopener noreferrer">Open ClinVar record</a>
                      </li>
                    {/each}
                  </ul>
                  {#if group.assertions.length > INITIAL_VISIBLE_LAB_REPORTS}
                    <button class="genomewide-discovery-more" type="button" onclick={() => toggleLabReports(group.id)}>
                      {expandedLabReports[group.id]
                        ? 'Show fewer lab reports'
                        : `Show all ${group.assertions.length} lab reports`}
                    </button>
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
        {/snippet}

        {#if section.relevance === 'unclear'}
          <details class="genomewide-relevance genomewide-relevance-collapsible" aria-label={section.title}>
            <summary class="genomewide-relevance-header">
              <h3>
                {section.title}
                <span>{section.groups.length.toLocaleString()} {section.groups.length === 1 ? 'match' : 'matches'}</span>
              </h3>
              <p>{section.description}</p>
            </summary>
            {@render conditionList()}
          </details>
        {:else}
          <section
            class="genomewide-relevance"
            class:genomewide-relevance-priority={section.relevance === 'may_be_relevant'}
            aria-label={section.title}
          >
            <header class="genomewide-relevance-header">
              <h3>
                {section.title}
                <span>{section.groups.length.toLocaleString()} {section.groups.length === 1 ? 'match' : 'matches'}</span>
              </h3>
              <p>{section.description}</p>
            </header>
            {@render conditionList()}
          </section>
        {/if}
      {/each}
      {#if discovery.omitted_association_count > 0}
        <p class="genomewide-discovery-footnote">
          To keep this report responsive, it shows the 500 highest-priority matches. {discovery.omitted_association_count.toLocaleString()} additional ClinVar matches were not included.
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
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    min-width: 0;
    align-items: center;
    gap: 0.8rem;
    cursor: pointer;
    list-style: none;
  }

  .genomewide-discovery-summary::marker { content: ''; }
  .genomewide-discovery-summary::-webkit-details-marker { display: none; }
  .genomewide-discovery-summary::after {
    flex: 0 0 auto;
    color: var(--text-secondary);
    content: '▸';
    font-size: 1.15rem;
  }
  .genomewide-discovery[open] > .genomewide-discovery-summary::after { content: '▾'; }
  .genomewide-discovery-summary:focus-visible,
  .genomewide-relevance-collapsible > summary:focus-visible,
  .genomewide-condition-details summary:focus-visible,
  .genomewide-discovery-more:focus-visible {
    outline: 2px solid var(--accent-strong, var(--accent-primary));
    outline-offset: 3px;
    border-radius: 0.25rem;
  }
  .genomewide-discovery-heading { display: grid; min-width: 0; gap: 0.12rem; overflow-wrap: anywhere; }
  .genomewide-discovery-title { color: var(--text-primary); font-size: 1rem; font-weight: 680; line-height: 1.35; }
  .genomewide-discovery-hint,
  .genomewide-discovery-status,
  .genomewide-discovery-meta,
  .genomewide-discovery-footnote { color: var(--text-secondary); font-size: 0.84rem; line-height: 1.5; }
  .genomewide-discovery-status,
  .genomewide-discovery-meta { color: var(--text-secondary); font-size: 0.88rem; line-height: 1.55; }
  .genomewide-discovery-count {
    flex: 0 0 auto;
    color: var(--text-secondary);
    font-size: 0.78rem;
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
  .genomewide-condition-heading { display: flex; min-width: 0; align-items: flex-start; justify-content: space-between; gap: 0.5rem 1rem; }
  .genomewide-condition-title { min-width: 0; }
  .genomewide-condition-topic { color: var(--text-secondary); font-size: 0.76rem; font-weight: 650; }
  .genomewide-condition h4 { margin: 0.13rem 0 0; color: var(--text-primary); font-size: 0.96rem; line-height: 1.35; overflow-wrap: anywhere; }
  .genomewide-condition-marker { display: block; margin-top: 0.14rem; color: var(--text-secondary); font-size: 0.78rem; line-height: 1.4; }
  .genomewide-condition-count { flex: 0 0 auto; max-width: 15rem; color: var(--text-primary); font-size: 0.8rem; font-weight: 650; text-align: right; }
  button.genomewide-condition-count { padding: 0; border: 0; background: transparent; font: inherit; cursor: help; }
  button.genomewide-condition-count:focus-visible { outline: 2px solid var(--accent-strong, var(--accent-primary)); outline-offset: 3px; border-radius: 0.25rem; }
  .genomewide-condition-plain { margin: 0.4rem 0 0; color: var(--text-secondary); font-size: 0.86rem; line-height: 1.55; }
  .genomewide-condition-unnamed h4 { color: var(--text-secondary); font-weight: 600; }
  .genomewide-relevance-collapsible > summary { cursor: pointer; list-style: none; }
  .genomewide-relevance-collapsible > summary::-webkit-details-marker { display: none; }
  .genomewide-relevance-collapsible > summary h3::before { content: '▸'; color: var(--text-secondary); font-size: 1rem; }
  .genomewide-relevance-collapsible[open] > summary h3::before { content: '▾'; }
  .genomewide-condition-details { margin-top: 0.5rem; color: var(--text-secondary); font-size: 0.82rem; }
  .genomewide-condition-details summary { width: fit-content; min-height: 1.75rem; color: var(--accent-strong, var(--accent-primary)); font-weight: 650; cursor: pointer; }
  .genomewide-condition-details ul { display: grid; gap: 0.35rem; margin: 0.45rem 0 0; padding: 0; list-style: none; }
  .genomewide-condition-details li { display: flex; min-width: 0; justify-content: space-between; gap: 0.35rem 0.7rem; padding: 0.4rem 0; border-top: 1px solid var(--border-subtle, rgba(120, 130, 145, 0.18)); }
  .genomewide-condition-details li > span { min-width: 0; overflow-wrap: anywhere; }
  .genomewide-condition-details small { display: block; margin-top: 0.12rem; color: var(--text-secondary); font-size: 0.78rem; line-height: 1.5; }
  .genomewide-condition-details a { flex: 0 0 auto; align-self: flex-start; color: var(--accent-strong, var(--accent-primary)); }
  .genomewide-discovery-more { width: fit-content; min-height: 2rem; margin: 0.1rem 0 0; padding: 0.2rem 0; border: 0; color: var(--accent-strong, var(--accent-primary)); background: transparent; font: inherit; font-size: 0.82rem; text-align: left; cursor: pointer; }
  .genomewide-discovery-footnote { margin: 0.4rem 0 0; }
  .genomewide-relevance { display: grid; gap: 0.45rem; margin-top: 0.9rem; }
  .genomewide-relevance-header h3 { display: flex; align-items: baseline; gap: 0.45rem; margin: 0; color: var(--text-primary); font-size: 0.96rem; }
  .genomewide-relevance-header h3 span { color: var(--text-secondary); font-size: 0.8rem; font-variant-numeric: tabular-nums; }
  .genomewide-relevance-header p { margin: 0.15rem 0 0; max-width: 78ch; color: var(--text-secondary); font-size: 0.86rem; line-height: 1.55; }
  .genomewide-relevance-priority .genomewide-condition { border-left: 3px solid var(--status-warning-text, #a16b12); }

  @media (max-width: 580px) {
    .genomewide-discovery { padding: 0.7rem; }
    .genomewide-condition-heading { align-items: flex-start; flex-direction: column; gap: 0.2rem; }
    .genomewide-condition-count { text-align: left; }
    .genomewide-condition-details li { flex-direction: column; }
  }
</style>
