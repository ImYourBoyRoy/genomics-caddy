<!-- ./src/lib/components/research/ResearchLiveLog.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getResearchDebugLog, setResearchDebugLog } from "../../api/tauri";
  import { liveProgress } from "../../research/liveProgress.svelte";
  import Tooltip from "../common/Tooltip.svelte";

  interface Props {
    logs?: string[];
    sweepRunning?: boolean;
    debugLogEnabled?: boolean;
    onDebugLogEnabledChange?: (enabled: boolean) => void;
  }

  let {
    logs = [],
    sweepRunning = false,
    debugLogEnabled = $bindable(false),
    onDebugLogEnabledChange,
  }: Props = $props();

  let lastActivity = $derived.by(() => {
    void liveProgress.tick;
    return liveProgress.lastActivityMessage;
  });

  let togglingDebug = $state(false);

  async function toggleDebugLog() {
    if (togglingDebug) return;
    togglingDebug = true;
    try {
      const next = !debugLogEnabled;
      const saved = await setResearchDebugLog(next);
      debugLogEnabled = saved;
      onDebugLogEnabledChange?.(saved);
    } catch (e) {
      console.error("Failed to toggle research debug log:", e);
    } finally {
      togglingDebug = false;
    }
  }

  onMount(() => {
    void (async () => {
      try {
        const enabled = await getResearchDebugLog();
        debugLogEnabled = enabled;
        onDebugLogEnabledChange?.(enabled);
      } catch {
        // Tauri not ready in non-desktop preview
      }
    })();
  });
</script>

<div class="glass-card log-card">
  <div class="log-header">
    <h2 class="card-title">Sweep activity</h2>
    <Tooltip interactiveChildren interactiveClickBehavior="dismiss" label="Detailed log" description="Shows verbose sweep logs in this feed and the terminal. The DNA_RESEARCH_DEBUG setting also enables it.">
      <label class="debug-toggle">
        <input
          type="checkbox"
          checked={debugLogEnabled}
          disabled={togglingDebug}
          onchange={toggleDebugLog}
        />
        <span>Detailed log</span>
      </label>
    </Tooltip>
  </div>

  <div class="log-container">
    {#if logs.length === 0}
      <div class="log-empty">
        {#if sweepRunning}
          {#if lastActivity}
            <span class="log-live-indicator">Sweep active</span>
            <span class="log-latest">{lastActivity}</span>
          {:else}
            Sweep running — waiting for first activity pulse…
          {/if}
          {#if debugLogEnabled}
            <span class="log-debug-hint">Verbose log on — detailed [sweep]/[enrich]/[prefetch] lines will appear here.</span>
          {/if}
        {:else}
          Waiting for activity…
        {/if}
      </div>
    {:else}
      {#each logs as log}
        <div class="log-entry" class:debug-entry={log.startsWith("[")}>{log}</div>
      {/each}
    {/if}
  </div>
</div>
