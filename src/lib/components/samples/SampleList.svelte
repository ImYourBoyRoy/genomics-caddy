<!-- ./src/lib/components/samples/SampleList.svelte -->
<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';
  import Tooltip from '../common/Tooltip.svelte';
  import { formatGeneticSexLabel } from '../../utils/uiLabels';

  /*
  Module Docstring:
  Purpose: Displays a list of active imported genomic profiles.
  Responsibilities:
  - Render list items containing sample name and conservative chromosome-call context.
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
              👤 {s.name} <span class="sex-pill">{formatGeneticSexLabel(s.genetic_sex)}</span>
            </button>
            <Tooltip
              label="Sex estimate from DNA"
              description="Based on X/Y chromosome call coverage. It is not gender identity, anatomy, fertility, pregnancy, or hormone status."
            >
              <span class="sample-scope-help" aria-label="Explain sex estimate from DNA">ⓘ</span>
            </Tooltip>
          </div>
          <button type="button" class="btn-delete" aria-label={`Delete profile ${s.name}`} disabled={disabled} onclick={() => onDeleteSample(s.id)}>🗑️</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
