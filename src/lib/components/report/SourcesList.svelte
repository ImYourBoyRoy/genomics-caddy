<!-- ./src/lib/components/report/SourcesList.svelte -->
<script lang="ts">
  import type { MarkerSource, EnrichedSource } from '../../types/genomics';
  import { safeExternalHref } from '../../utils/urlSafety';

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
  let sourceCount = $derived((sources?.length ?? 0) + (dbSources?.length ?? 0));
</script>

{#if sourceCount > 0}
  <details class="marker-sources-details">
    <summary>📚 References ({sourceCount})</summary>
    {#if sources && sources.length > 0}
      <div class="marker-sources">
        <span class="sec-title">Evidence sources</span>
        <ul class="sources-list">
          {#each sources as source}
            <li>
              {#if source.url}
                {@const safeUrl = safeExternalHref(source.url)}
                {#if safeUrl}
                  <a href={safeUrl} target="_blank" rel="noopener noreferrer" class="source-link">{source.name}</a>
                {:else}
                  {source.name}
                {/if}
              {:else}
                {source.name}
              {/if}
              {#if source.evidence_type}
                <span class="source-type">({source.evidence_type})</span>
              {/if}
              {#if source.notes}
                <span class="source-notes">— {source.notes}</span>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if dbSources && dbSources.length > 0}
      <div class="marker-sources db-sources">
        <span class="sec-title">Reference database citations</span>
        <ul class="sources-list">
          {#each dbSources as src}
            <li>
              <span class="db-source-type">{src.source_type}</span>
              {#if src.url}
                {@const safeUrl = safeExternalHref(src.url)}
                {#if safeUrl}
                  <a href={safeUrl} target="_blank" rel="noopener noreferrer" class="source-link">{src.citation}</a>
                {:else}
                  {src.citation}
                {/if}
              {:else}
                {src.citation}
              {/if}
              {#if src.details}
                <span class="source-notes">— {src.details}</span>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </details>
{/if}

<style>
  .marker-sources-details {
    margin-top: 0.5rem;
    border-top: 1px solid var(--border-color);
    padding-top: 0.5rem;
    color: var(--text-secondary);
    font-size: 0.72rem;
  }

  .marker-sources-details summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-weight: 700;
  }

  .marker-sources-details[open] summary {
    margin-bottom: 0.6rem;
  }

  .db-sources {
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid rgba(255,255,255,0.06);
  }
  .db-source-type {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #7dd3fc;
    margin-right: 0.3rem;
    padding: 0.1rem 0.3rem;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
  }
</style>
