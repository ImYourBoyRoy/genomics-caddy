<!-- ./src/lib/components/samples/SampleList.svelte -->
<script lang="ts">
  import type { GenomeSample } from '../../types/genomics';

  /*
  Module Docstring:
  Purpose: Displays a list of active imported genomic profiles.
  Responsibilities:
  - Render list items containing sample name and detected genetic sex.
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
          <span class="sample-name" role="button" tabindex="0"
            style={disabled ? "pointer-events: none; opacity: 0.65; cursor: not-allowed;" : ""}
            onclick={() => { if (!disabled) onSelectSample(s); }}
            onkeydown={(e) => { if (!disabled && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); onSelectSample(s); } }}>
            👤 {s.name} <span class="sex-pill">{s.genetic_sex}</span>
          </span>
          <button class="btn-delete" disabled={disabled} onclick={() => onDeleteSample(s.id)}>🗑️</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
