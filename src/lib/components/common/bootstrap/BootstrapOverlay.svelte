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
    color-scheme: dark;
    --bg-primary: #080b14;
    --bg-secondary: #0e1422;
    --surface-raised: #131a2b;
    --surface-card: rgba(20, 27, 44, 0.86);
    --surface-subtle: rgba(148, 163, 184, 0.1);
    --surface-control: rgba(4, 8, 18, 0.72);
    --border-color: rgba(148, 163, 184, 0.2);
    --border-strong: rgba(191, 219, 254, 0.34);
    --text-primary: #eef2ff;
    --text-secondary: #aebbd0;
    --focus-ring: #7dd3fc;
    --accent: #2dd4bf;
    --accent-hover: #14b8a6;
    --success: #34d399;
    --danger: #fb7185;
    --warning: #fbbf24;
    --status-danger-bg: rgba(190, 24, 93, 0.18);
    --status-danger-strong-text: #fda4af;
    --status-danger-border: rgba(251, 113, 133, 0.48);
    --bootstrap-glow: rgba(45, 212, 191, 0.24);
    --bootstrap-glow-soft: rgba(129, 140, 248, 0.13);
    --bootstrap-glow-success: rgba(52, 211, 153, 0.28);
    --bootstrap-glow-error: rgba(251, 113, 133, 0.28);
    --bootstrap-shimmer: rgba(255, 255, 255, 0.06);
    background:
      radial-gradient(circle at 50% 38%, rgba(30, 64, 175, 0.2), transparent 35%),
      radial-gradient(circle at 82% 78%, rgba(16, 185, 129, 0.08), transparent 42%),
      linear-gradient(135deg, #070a12 0%, #0d1424 54%, #0a1718 100%);
    pointer-events: all;
  }
</style>
