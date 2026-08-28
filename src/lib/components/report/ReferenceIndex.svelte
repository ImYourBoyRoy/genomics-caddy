<script lang="ts">
  import type { GeneratedReport } from '../../types/genomics';
  import { safeExternalHref } from '../../utils/urlSafety';
  import { buildReportReferenceRegistry } from '../../utils/reportReferences';

  interface Props {
    report: GeneratedReport;
  }

  let { report }: Props = $props();
  let registry = $derived(buildReportReferenceRegistry(report));
</script>

{#if registry.references.length > 0}
  <details class="reference-index">
    <summary>Reference index ({registry.references.length})</summary>
    <p class="reference-index-intro">
      One compact list for the sources attached to findings in this report.
    </p>
    <ol class="reference-list">
      {#each registry.references as reference (reference.id)}
        <li id={`reference-${reference.id.toLowerCase()}`}>
          <span class="reference-id">{reference.id}</span>
          <div class="reference-body">
            <div class="reference-title-row">
              {#if reference.url}
                {@const safeUrl = safeExternalHref(reference.url)}
                {#if safeUrl}
                  <a href={safeUrl} target="_blank" rel="noopener noreferrer">{reference.title}</a>
                {:else}
                  <strong>{reference.title}</strong>
                {/if}
              {:else}
                <strong>{reference.title}</strong>
              {/if}
            </div>
            <div class="reference-meta">
              <span>{reference.organization}</span>
              <span>{reference.date}</span>
              <span>{reference.evidenceRole}</span>
            </div>
          </div>
        </li>
      {/each}
    </ol>
  </details>
{/if}

<style>
  .reference-index {
    margin: var(--space-6) 0 var(--space-6);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border-color);
    border-radius: 0.75rem;
    background: var(--surface-subtle);
    color: var(--text-secondary);
  }

  .reference-index summary {
    color: var(--text-primary);
    cursor: pointer;
    font-weight: 750;
  }

  .reference-index-intro {
    margin: var(--space-3) 0;
    font-size: 0.78rem;
  }

  .reference-list {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .reference-list li {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--border-color);
    border-radius: 0.55rem;
    background: var(--surface-raised);
  }

  .reference-id {
    align-self: start;
    color: var(--status-info-soft-text);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .reference-body,
  .reference-title-row,
  .reference-title-row a {
    min-width: 0;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .reference-title-row {
    color: var(--text-primary);
    font-size: 0.82rem;
    font-weight: 700;
  }

  .reference-title-row a {
    color: var(--accent);
  }

  .reference-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.75rem;
    margin-top: 0.3rem;
    color: var(--text-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }

  .reference-meta span + span::before {
    content: '·';
    margin-right: 0.75rem;
    color: var(--border-color);
  }

  @media print {
    .reference-index {
      break-inside: avoid;
      background: #fff;
      color: #222;
    }

    .reference-list li {
      background: #fff;
      border-color: #bbb;
    }
  }
</style>
