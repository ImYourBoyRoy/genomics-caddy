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
    onSelectSample: (sample: GenomeSample) => void;
    onDeleteSample: (id: number) => void;
  }

  let {
    samples,
    selectedSample,
    disabled = false,
    onSelectSample,
    onDeleteSample
  }: Props = $props();

  function sexSymbol(value: string): string {
    const label = formatGeneticSexLabel(value);
    return label === 'Female' ? '♀' : label === 'Male' ? '♂' : '•';
  }

  function sexSymbolLabel(value: string): string {
    return `${formatGeneticSexLabel(value)} profile`;
  }
</script>

<div class="samples-card card">
  <h3>Active Profiles</h3>
  {#if samples.length === 0}
    <p class="empty-hint">No genomes imported yet.</p>
  {:else}
    <ul class="sample-list">
      {#each samples as s}
        <li class="sample-item" class:active={selectedSample && selectedSample.id === s.id} class:disabled={disabled}>
          <div class="sample-name-wrap">
            <button
              type="button"
              class="sample-name"
              disabled={disabled}
              aria-current={selectedSample?.id === s.id ? 'true' : undefined}
              onclick={() => onSelectSample(s)}
            >
              <span
                class="sample-sex-symbol"
                role="img"
                aria-label={sexSymbolLabel(s.genetic_sex)}
              >{sexSymbol(s.genetic_sex)}</span>
              <span class="sample-name-text">{s.name}</span>
            </button>
          </div>
          <button type="button" class="btn-delete" aria-label={`Delete profile ${s.name}`} disabled={disabled} onclick={() => onDeleteSample(s.id)}>🗑️</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
