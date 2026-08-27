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

  interface DisplayReference {
    kind: 'Evidence' | 'Catalog';
    title: string;
    organization: string;
    date: string;
    role: string;
    url?: string;
  }

  function buildReferences(curated: MarkerSource[], catalog: EnrichedSource[]): DisplayReference[] {
    const references: DisplayReference[] = [];
    const seen = new Set<string>();
    for (const source of curated) {
      const reference: DisplayReference = {
        kind: 'Evidence',
        title: source.name,
        organization: source.name,
        date: source.accessed || 'Access date not recorded',
        role: source.evidence_type || 'Marker-pack reference',
        url: source.url,
      };
      const key = reference.url
        ? `url:${reference.url.trim().replace(/\/$/, '').toLowerCase()}`
        : [reference.title, reference.role].join('|').toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        references.push(reference);
      }
    }
    for (const source of catalog) {
      const reference: DisplayReference = {
        kind: 'Catalog',
        title: source.citation,
        organization: source.source_type,
        date: 'Local catalog record',
        role: source.details || 'Catalog evidence',
        url: source.url,
      };
      const key = reference.url
        ? `url:${reference.url.trim().replace(/\/$/, '').toLowerCase()}`
        : [reference.title, reference.role].join('|').toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        references.push(reference);
      }
    }
    return references;
  }

  let references = $derived(buildReferences(sources, dbSources));
</script>

{#if references.length > 0}
  <details class="marker-sources-details">
    <summary>📚 References ({references.length})</summary>
    <div class="marker-sources">
      <ul class="sources-list">
        {#each references as reference}
          <li>
            <span class="db-source-type">{reference.kind}</span>
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
            <span class="source-notes">— {reference.role}; {reference.date}</span>
          </li>
        {/each}
      </ul>
    </div>
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
