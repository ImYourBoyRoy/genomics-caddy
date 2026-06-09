<!-- ./src/lib/components/report/EvidenceBadge.svelte -->
<script lang="ts">
  import { getTierInfo } from '../../utils/evidence';

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
  }

  let { tier }: Props = $props();
  let info = $derived(getTierInfo(tier));
</script>

<span class="tier-badge {info.colorClass}" title={info.description}>
  {info.label}
</span>
