<!-- ./src/lib/components/report/ConfirmWithList.svelte -->
<script lang="ts">
  /*
  Module Docstring:
  Purpose: Displays follow-up laboratory testing suggestions for a variant.
  Responsibilities:
  - Format lab tests and physical observations as one compact chip row.
  - Cap the default row and let people reveal the remainder.
  Key Inputs: confirmWith (string[]).
  Key Outputs: Lab confirmation recommendation block.
  Operational Notes: Stylized using the .confirm-pathway class wrapper.
  */

  interface Props {
    confirmWith?: string[];
  }

  const VISIBLE_LIMIT = 4;

  let { confirmWith = [] }: Props = $props();
  let showAll = $state(false);
  let visible = $derived(showAll ? confirmWith : confirmWith.slice(0, VISIBLE_LIMIT));
  let hiddenCount = $derived(Math.max(0, confirmWith.length - VISIBLE_LIMIT));
</script>

{#if confirmWith && confirmWith.length > 0}
  <div class="confirm-pathway">
    <span class="sec-title">Confirm with</span>
    <ul class="confirm-list">
      {#each visible as item, i (i)}
        <li>{item}</li>
      {/each}
      {#if hiddenCount > 0}
        <li class="confirm-more">
          <button type="button" class="confirm-more-btn" aria-expanded={showAll} onclick={() => (showAll = !showAll)}>
            {showAll ? 'Show fewer' : `+${hiddenCount} more`}
          </button>
        </li>
      {/if}
    </ul>
  </div>
{/if}
