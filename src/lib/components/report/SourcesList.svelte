<!-- ./src/lib/components/report/SourcesList.svelte -->
<script lang="ts">
  import type { MarkerSource, EnrichedSource } from '../../types/genomics';
  import { safeExternalHref } from '../../utils/urlSafety';
  import { buildReferenceEntries } from '../../utils/reportReferences';

  /*
  Module Docstring:
  Purpose: Formatted list of academic/clinical sources for genomic markers.
  Responsibilities:
  - Render hyperlinked source names if URLs are available.
  - Display evidence types and auxiliary notes in small typography.
  - Show reference DB citations (ClinVar, GWAS, etc.) in a separate block.
  Key Inputs: sources (MarkerSource[]), dbSources (EnrichedSource[]).
  Key Outputs: Structured sources block.
  Operational Notes: Encapsulates sources list styling.
  */

  interface Props {
    sources?: MarkerSource[];
    dbSources?: EnrichedSource[];
  }

  let { sources = [], dbSources = [] }: Props = $props();

  let references = $derived(buildReferenceEntries(sources, dbSources));
</script>

{#if references.length > 0}
  <details class="marker-sources-details">
    <summary aria-label="References ({references.length})"><span class="marker-sources-summary-label">📚 Sources</span></summary>
    <div class="marker-sources">
      <ul class="sources-list">
        {#each references as reference (reference.id)}
          <li>
            <span class="db-source-type">{reference.id}</span>
            {#if reference.url}
              {@const safeUrl = safeExternalHref(reference.url)}
              {#if safeUrl}
                <a href={safeUrl} target="_blank" rel="noopener noreferrer" class="source-link">{reference.title}</a>
              {:else}
                {reference.title}
              {/if}
            {:else}
              {reference.title}
            {/if}
            <span class="source-type">— {reference.organization}</span>
            <span class="source-notes">— {reference.evidenceRole}; {reference.date}</span>
          </li>
        {/each}
      </ul>
    </div>
  </details>
{/if}

<style>
  .marker-sources-details {
    min-width: 0;
    max-width: 100%;
    margin-top: 0.5rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .marker-sources-details summary {
    display: flex;
    align-items: flex-start;
    width: 100%;
    max-width: 100%;
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 700;
    min-width: 0;
    overflow: hidden;
    overflow-wrap: anywhere;
    white-space: normal;
  }

  .marker-sources-details summary::-webkit-details-marker {
    display: none;
  }

  .marker-sources-details summary::before {
    content: "▸";
    flex: 0 0 auto;
    margin-right: 0.3rem;
    color: var(--text-secondary);
  }

  .marker-sources-details[open] summary::before {
    content: "▾";
  }

  .marker-sources-summary-label {
    display: block;
    flex: 1 1 auto;
    max-width: 100%;
    min-width: 0;
    width: 100%;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .marker-sources-details[open] summary {
    margin-bottom: 0.6rem;
  }

  .db-source-type {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--status-info-soft-text);
    margin-right: 0.3rem;
    padding: 0.1rem 0.3rem;
    background: var(--status-info-soft-bg);
    border: 1px solid var(--status-info-soft-border);
    border-radius: 3px;
  }

  .sources-list li,
  .source-link,
  .source-type,
  .source-notes {
    min-width: 0;
    overflow-wrap: anywhere;
    word-break: break-word;
  }
</style>
