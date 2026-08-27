<!-- ./src/lib/components/report/EvidenceBadge.svelte -->
<script lang="ts">
  import { getSimpleTierLabel, getTierInfo } from '../../utils/evidence';
  import Tooltip from '../common/Tooltip.svelte';

  /*
  Module Docstring:
  Purpose: Badge displaying scientific evidence tier (Tiers A-D) with tooltip.
  Responsibilities:
  - Resolve appropriate tier labels and style classes based on tier codes.
  - Show a plain-language tooltip explaining what each tier means.
  Key Inputs: tier (string).
  Key Outputs: Styled tier badge span with title tooltip.
  Operational Notes: Resolves classes through getTierInfo utility.
  */

  interface Props {
    tier: string;
    simple?: boolean;
  }

  let { tier, simple = false }: Props = $props();
  let info = $derived(getTierInfo(tier));
  let badgeLabel = $derived(simple ? getSimpleTierLabel(tier) : info.label);
</script>

<Tooltip label={badgeLabel} description={info.description}>
  <span class="tier-badge {info.colorClass}">
    {badgeLabel}
  </span>
</Tooltip>
