<!-- ./src/lib/components/common/bootstrap/BootstrapOverlay.svelte -->
<script lang="ts">
  import type { AppBootstrapStatus, GenomeImportPreview } from "../../../types/genomics";
  import type { BootstrapPhase } from "./bootstrapPhases";
  import type { ImportPhase, ImportStepId } from "../../../utils/importProgress";
  import AppBootstrapScreen from "../AppBootstrapScreen.svelte";

  interface Props {
    phase: BootstrapPhase;
    message: string;
    status?: AppBootstrapStatus | null;
    error?: string;
    mode?: "startup" | "resources" | "import";
    importPhase?: ImportPhase;
    importProgress?: { percentage: number; status: string } | null;
    importPreview?: GenomeImportPreview | null;
    importProfileName?: string;
    importFailedStep?: ImportStepId | null;
    showWorkspaceAction?: boolean;
    workspaceActionLabel?: string;
    onContinue?: () => void;
  }

  let {
    phase,
    message,
    status = null,
    error = "",
    mode = "startup",
    importPhase = "idle",
    importProgress = null,
    importPreview = null,
    importProfileName = "",
    importFailedStep = null,
    showWorkspaceAction = false,
    workspaceActionLabel = "",
    onContinue,
  }: Props = $props();
</script>

<div
  class="bootstrap-overlay"
  role="dialog"
  aria-modal={showWorkspaceAction ? "false" : "true"}
  aria-busy={phase !== "ready" && phase !== "error"}
  aria-label={mode === "resources"
    ? "Reference resource synchronization"
    : mode === "import"
      ? "DNA profile import"
      : "Application startup"}
>
  <!-- Single left-status / right-helix screen. Do not stack a second overlay card
       on top of AppBootstrapScreen — that pushed details off-view and left only
       the DNA graphic visible when WebKit painted incompletely. -->
  <AppBootstrapScreen
    {phase}
    {message}
    {status}
    {error}
    {mode}
    {importPhase}
    {importProgress}
    {importPreview}
    {importProfileName}
    {importFailedStep}
    {showWorkspaceAction}
    {workspaceActionLabel}
    {onContinue}
  />
</div>

<style>
  .bootstrap-overlay {
    position: fixed;
    inset: 0;
    z-index: 100000;
    display: block;
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
      radial-gradient(circle at 78% 42%, rgba(30, 64, 175, 0.18), transparent 32%),
      radial-gradient(circle at 82% 78%, rgba(16, 185, 129, 0.08), transparent 42%),
      linear-gradient(135deg, #070a12 0%, #0d1424 54%, #0a1718 100%);
    pointer-events: all;
  }
</style>
