<!-- ./src/lib/components/samples/SampleList.svelte -->
<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';
  import { formatGeneticSexLabel } from '../../utils/uiLabels';

  /*
  Module Docstring:
  Purpose: Displays a list of active imported genomic profiles.
  Responsibilities:
  - Render list items containing sample name and concise chromosome-call context.
  - Highlight the currently active profile.
  - Expose select and delete event bindings.
  Key Inputs: samples, selectedSample, onSelectSample, onDeleteSample.
  Key Outputs: Profile selection list.
  Operational Notes: Triggers selected sample evaluation on click.
  */

  interface Props {
    samples: GenomeSample[];
    selectedSample: GenomeSample | null;
    disabled?: boolean;
    disabledReason?: string;
    onSelectSample: (sample: GenomeSample) => void;
    onOpenChromosomeContext: (sample: GenomeSample) => void;
    onDeleteSample: (id: number) => void;
  }

  let {
    samples,
    selectedSample,
    disabled = false,
    disabledReason = 'Profile actions are temporarily unavailable.',
    onSelectSample,
    onOpenChromosomeContext,
    onDeleteSample
  }: Props = $props();

  function sexSymbol(value: string): string {
    const label = formatGeneticSexLabel(value);
    return label === 'Female-like' ? '♀' : label === 'Male-like' ? '♂' : '•';
  }

  function sexSymbolLabel(value: string): string {
    return `${formatGeneticSexLabel(value)} profile`;
  }
</script>

<div class="samples-card card">
  <h3>Active profiles</h3>
  <p class="samples-card-hint">Select a genome to view its report.</p>
  {#if disabled}
    <p class="samples-actions-note" role="status">{disabledReason}</p>
  {/if}
  {#if samples.length === 0}
    <p class="empty-hint">No genomes imported yet.</p>
  {:else}
    <ul class="sample-list">
      {#each samples as s}
        <li class="sample-item" class:active={selectedSample && selectedSample.id === s.id} class:disabled={disabled}>
          <div class="sample-name-wrap">
            <button
              type="button"
              class="sample-sex-indicator"
              disabled={disabled}
              aria-label={`Open chromosome context for ${s.name}: ${sexSymbolLabel(s.genetic_sex)}`}
              title={disabled ? disabledReason : `Review chromosome pattern for ${s.name}`}
              onclick={() => onOpenChromosomeContext(s)}
            >
              <span class="sample-sex-symbol" aria-hidden="true">{sexSymbol(s.genetic_sex)}</span>
            </button>
            <button
              type="button"
              class="sample-name"
              disabled={disabled}
              aria-current={selectedSample?.id === s.id ? 'true' : undefined}
              onclick={() => onSelectSample(s)}
            >
              <span class="sample-name-text">{s.name}</span>
            </button>
          </div>
          <button
            type="button"
            class="btn-delete"
            aria-label={`Delete profile ${s.name}`}
            title={disabled ? disabledReason : `Delete ${s.name} and its local data`}
            disabled={disabled}
            onclick={() => onDeleteSample(s.id)}
          >
            <svg viewBox="0 0 20 20" aria-hidden="true" focusable="false">
              <path d="M4.5 6h11l-.7 10.2a1.3 1.3 0 0 1-1.3 1.2h-7a1.3 1.3 0 0 1-1.3-1.2L4.5 6Z" />
              <path d="M3.5 4.5h13M7.2 4.5V3.4c0-.5.4-.9.9-.9h3.8c.5 0 .9.4.9.9v1.1M8 8.5v5.8M12 8.5v5.8" />
            </svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
