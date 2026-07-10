<!-- ./src/lib/components/common/bootstrap/BootstrapActivityPulse.svelte -->
<script lang="ts">
  import type { BootstrapPhase } from "./bootstrapPhases";
  import { PHASE_ACCENT } from "./bootstrapPhases";
  import ActivityPulse from "../loading/ActivityPulse.svelte";

  interface Props {
    phase: BootstrapPhase;
    message: string;
    isError?: boolean;
  }

  let { phase, message, isError = false }: Props = $props();
</script>

<ActivityPulse
  {message}
  {isError}
  accent={PHASE_ACCENT[phase]}
  maxWidth="560px"
/>
