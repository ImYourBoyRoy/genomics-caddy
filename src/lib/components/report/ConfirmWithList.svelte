<!-- ./src/lib/components/report/ConfirmWithList.svelte -->
<script lang="ts">
  /*
  Module Docstring:
  Purpose: Displays follow-up laboratory testing suggestions for a variant.
  Responsibilities:
  - Format lab tests and physical observations as a bulleted lists.
  Key Inputs: confirmWith (string[]).
  Key Outputs: Lab confirmation recommendation block.
  Operational Notes: Stylized using the .confirm-pathway class wrapper.
  */

  interface Props {
    confirmWith?: string[];
  }

  let { confirmWith = [] }: Props = $props();
</script>

{#if confirmWith && confirmWith.length > 0}
  <div class="confirm-pathway">
    <span class="sec-title">🔬 Confirm with (Labs/Symptoms):</span>
    <ul class="confirm-list">
      {#each confirmWith as item}
        <li>{item}</li>
      {/each}
    </ul>
  </div>
{/if}
