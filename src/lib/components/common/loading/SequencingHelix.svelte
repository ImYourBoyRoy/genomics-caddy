<!-- ./src/lib/components/common/loading/SequencingHelix.svelte -->
<script lang="ts">
  /*
  Purpose: A small rotating double helix used while a report is prepared. Base
  pairs light up from the top as real progress advances; without a percentage a
  scan band sweeps down the strand instead.
  Inputs: percentage (0-100 or null when unknown).
  Outputs: Decorative, aria-hidden markup. The progressbar role lives with the
  caller. Motion rules are in styles/components/sequencing-helix.css.
  */
  import '#lib/styles/components/sequencing-helix.css';

  interface Props {
    percentage?: number | null;
  }

  let { percentage = null }: Props = $props();

  const ROWS = 22;
  let litRows = $derived(percentage === null ? 0 : Math.round((Math.max(0, Math.min(100, percentage)) / 100) * ROWS));
</script>

<div class="sequencing-helix" class:scanning={percentage === null} aria-hidden="true">
  {#each Array.from({ length: ROWS }, (_, i) => i) as i (i)}
    <div class="helix-row" class:read={i < litRows} style={`--i: ${i}`}>
      <span class="helix-node helix-node-a"></span>
      <span class="helix-rung"></span>
      <span class="helix-node helix-node-b"></span>
    </div>
  {/each}
</div>
