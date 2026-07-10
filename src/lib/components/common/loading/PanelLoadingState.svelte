<!-- ./src/lib/components/common/loading/PanelLoadingState.svelte -->
<script lang="ts">
  import ActivityPulse from "./ActivityPulse.svelte";

  interface Props {
    message: string;
    submessage?: string;
    accent?: string;
    compact?: boolean;
  }

  let {
    message,
    submessage = "",
    accent = "var(--accent)",
    compact = false,
  }: Props = $props();
</script>

<div class="panel-loading bootstrap-motion" class:compact aria-busy="true">
  <ActivityPulse {message} {accent} maxWidth={compact ? "420px" : "560px"} />
  {#if submessage}
    <p class="submessage">{submessage}</p>
  {/if}
</div>

<style>
  .panel-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.65rem;
    padding: 1.5rem 1rem;
    min-height: 120px;
    animation: bootstrap-fade-up 0.5s ease both;
  }

  .panel-loading.compact {
    min-height: 72px;
    padding: 0.75rem 0.5rem;
  }

  .submessage {
    margin: 0;
    font-size: 0.72rem;
    color: var(--text-secondary);
    text-align: center;
    max-width: 480px;
  }
</style>
