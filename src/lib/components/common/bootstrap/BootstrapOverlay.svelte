<!-- ./src/lib/components/common/bootstrap/BootstrapOverlay.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus } from "../../../types/genomics";
  import type { BootstrapPhase } from "./bootstrapPhases";
  import AppBootstrapScreen from "../AppBootstrapScreen.svelte";

  interface Props {
    phase: BootstrapPhase;
    message: string;
    status?: AppBootstrapStatus | null;
    error?: string;
  }

  let { phase, message, status = null, error = "" }: Props = $props();
</script>

<div
  class="bootstrap-overlay bootstrap-motion"
  role="dialog"
  aria-modal="true"
  aria-busy={phase !== "ready" && phase !== "error"}
  aria-label="Application startup"
>
  <AppBootstrapScreen {phase} {message} {status} {error} />
</div>

<style>
  .bootstrap-overlay {
    position: fixed;
    inset: 0;
    z-index: 100000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
    background:
      radial-gradient(circle at 18% 12%, rgba(88, 80, 236, 0.14), transparent 42%),
      radial-gradient(circle at 82% 78%, rgba(16, 185, 129, 0.08), transparent 45%),
      radial-gradient(circle at top right, #1a1b36, var(--bg-primary));
    backdrop-filter: blur(2px);
    pointer-events: all;
  }
</style>
