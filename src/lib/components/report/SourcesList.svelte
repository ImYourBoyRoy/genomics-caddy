<!-- ./src/lib/components/report/SourcesList.svelte -->
<script lang="ts">
  import type { MarkerSource } from '../../types/genomics';
  import { safeExternalHref } from '../../utils/urlSafety';

  /*
  Module Docstring:
  Purpose: Formatted list of academic/clinical sources for genomic markers.
  Responsibilities:
  - Render hyperlinked source names if URLs are available.
  - Display evidence types and auxiliary notes in small typography.
  Key Inputs: sources (MarkerSource[]).
  Key Outputs: Structured sources block.
  Operational Notes: Encapsulates sources list styling.
  */

  interface Props {
    sources?: MarkerSource[];
  }

  let { sources = [] }: Props = $props();
</script>

{#if sources && sources.length > 0}
  <div class="marker-sources">
    <span class="sec-title">📚 Evidence Sources:</span>
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
            <span class="source-notes">- {source.notes}</span>
          {/if}
        </li>
      {/each}
    </ul>
  </div>
{/if}
