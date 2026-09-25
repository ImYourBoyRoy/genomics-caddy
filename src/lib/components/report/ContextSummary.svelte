<script lang="ts">
  import { getContextIndicators } from '../../utils/contextIndicators';

  interface Props {
    contextTags?: readonly (string | null | undefined)[] | null;
    scope?: string | null;
    maxVisible?: number;
  }

  let {
    contextTags = [],
    scope = null,
    maxVisible = 1,
  }: Props = $props();

  let indicators = $derived(getContextIndicators(contextTags, scope));
  let visibleLimit = $derived(Math.max(1, maxVisible));
  let visibleIndicators = $derived(indicators.slice(0, visibleLimit));
  let overflowIndicators = $derived(indicators.slice(visibleLimit));
</script>

{#if indicators.length > 0}
  <div class="context-summary" role="group" aria-label="Relevant biological contexts">
    {#each visibleIndicators as indicator (indicator.id)}
      <span
        class="context-indicator"
        data-context-group={indicator.group}
        role="note"
        title={indicator.description}
        aria-label={`${indicator.label}. ${indicator.description}`}
      >
        <span class="context-indicator-dot" aria-hidden="true"></span>
        <span>{indicator.shortLabel}</span>
      </span>
    {/each}

    {#if overflowIndicators.length > 0}
      <details class="context-overflow">
        <summary
          class="context-overflow-summary"
          aria-label={`Show ${overflowIndicators.length} additional biological contexts`}
        >+{overflowIndicators.length} more</summary>
        <div class="context-overflow-panel">
          <p>Also relevant</p>
          <ul>
            {#each overflowIndicators as indicator (indicator.id)}
              <li>
                <strong>{indicator.label}</strong>
                <small>{indicator.description}</small>
              </li>
            {/each}
          </ul>
        </div>
      </details>
    {/if}
  </div>
{/if}

<style>
  .context-summary {
    display: flex;
    min-width: 0;
    max-width: 100%;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem 0.4rem;
  }

  .context-indicator,
  .context-overflow-summary {
    display: inline-flex;
    min-width: 0;
    max-width: 100%;
    align-items: center;
    box-sizing: border-box;
    color: var(--text-secondary);
    font-size: 0.67rem;
    line-height: 1.25;
  }

  .context-indicator {
    gap: 0.32rem;
    font-weight: 650;
    overflow-wrap: anywhere;
  }

  .context-indicator-dot {
    width: 0.28rem;
    height: 0.28rem;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--report-scope-text);
  }

  .context-overflow {
    min-width: 0;
    max-width: 100%;
  }

  .context-overflow[open] {
    flex-basis: 100%;
  }

  .context-overflow-summary {
    color: var(--accent);
    font-weight: 650;
    cursor: pointer;
    list-style: none;
    user-select: none;
    text-decoration: underline;
    text-decoration-color: transparent;
    text-underline-offset: 0.15em;
    transition: text-decoration-color 0.15s ease;
  }

  .context-overflow-summary:hover,
  .context-overflow-summary:focus-visible {
    text-decoration-color: currentColor;
  }

  .context-overflow-summary::-webkit-details-marker {
    display: none;
  }

  .context-overflow-summary::before {
    content: '›';
    display: inline-block;
    margin-right: 0.2rem;
    transition: transform 0.15s ease;
  }

  .context-overflow[open] .context-overflow-summary::before {
    transform: rotate(90deg);
  }

  .context-overflow-summary:focus-visible {
    border-radius: 0.15rem;
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .context-overflow-panel {
    min-width: 0;
    margin-top: 0.35rem;
    padding: 0.1rem 0 0.1rem 0.65rem;
    border-left: 2px solid var(--report-scope-border);
    color: var(--text-secondary);
    font-size: 0.68rem;
    line-height: 1.4;
  }

  .context-overflow-panel p {
    margin: 0 0 0.45rem;
    color: var(--text-muted);
    font-size: 0.64rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .context-overflow-panel ul {
    display: grid;
    gap: 0.5rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .context-overflow-panel li {
    display: grid;
    min-width: 0;
    gap: 0.12rem;
  }

  .context-overflow-panel strong {
    min-width: 0;
    color: var(--text-primary);
    overflow-wrap: anywhere;
  }

  .context-overflow-panel small {
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }
</style>
