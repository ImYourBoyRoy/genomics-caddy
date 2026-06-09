<!-- ./src/lib/components/report/EffectDirectionBadge.svelte -->
<script lang="ts">
  import type { EffectDirection } from '../../types/genomics';
  import { getDirectionInfo } from '../../utils/evidence';

  /*
  Module Docstring:
  Purpose: Badge displaying effect direction with plain-language explanation.
  Responsibilities:
  - Render direction-specific color badges (risk, protective, trait, etc.).
  - Show plain-language label and tooltip description.
  Key Inputs: direction (EffectDirection).
  Key Outputs: Styled direction badge with explanatory text.
  Operational Notes: Maps direction classifications to CSS styles and user-friendly text.
  */

  interface Props {
    direction: EffectDirection;
  }

  let { direction }: Props = $props();
  let info = $derived(getDirectionInfo(direction));
</script>

{#if direction}
  <div class="direction-section">
    <span class="direction-badge {info.colorClass}" title={info.description}>
      {info.plainLabel}
    </span>
  </div>
{/if}
