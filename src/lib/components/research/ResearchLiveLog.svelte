<!-- ./src/lib/components/research/ResearchLiveLog.svelte -->

<script lang="ts">
  import { onMount } from "svelte";
  import { getResearchDebugLog, setResearchDebugLog } from "../../api/tauri";
  import { liveProgress } from "../../research/liveProgress.svelte";



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

    <h2 class="card-title">Live System Feed</h2>

    <label class="debug-toggle" title="Verbose sweep logs in this feed and the terminal (DNA_RESEARCH_DEBUG=1 also works)">

      <input

        type="checkbox"

        checked={debugLogEnabled}

        disabled={togglingDebug}

        onchange={toggleDebugLog}

      />

      <span>Verbose sweep log</span>

    </label>

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



<style>

  .glass-card {

    background: rgba(255, 255, 255, 0.03);

    border: 1px solid rgba(255, 255, 255, 0.08);

    border-radius: 12px;

    padding: 18px;

  }



  .log-header {

    display: flex;

    justify-content: space-between;

    align-items: center;

    gap: 12px;

    margin-bottom: 12px;

  }



  .card-title {

    font-size: 0.8rem;

    font-weight: 700;

    color: var(--text-primary);

    margin: 0;

    text-transform: uppercase;

    letter-spacing: 0.05em;

  }



  .debug-toggle {

    display: flex;

    align-items: center;

    gap: 6px;

    font-size: 0.62rem;

    color: var(--text-secondary);

    cursor: pointer;

    user-select: none;

    white-space: nowrap;

  }



  .debug-toggle input {

    accent-color: var(--accent);

  }



  .log-container {

    background: rgba(0, 0, 0, 0.3);

    border: 1px solid rgba(255, 255, 255, 0.05);

    border-radius: 8px;

    padding: 8px;

    max-height: 160px;

    overflow-y: auto;

    display: flex;

    flex-direction: column;

    gap: 4px;

    min-height: 80px;

  }



  .log-empty {

    font-size: 0.7rem;

    color: var(--text-secondary);

    text-align: center;

    margin: auto;

    font-style: italic;

    display: flex;

    flex-direction: column;

    gap: 6px;

    padding: 8px;

  }



  .log-debug-hint {

    font-style: normal;

    font-size: 0.62rem;

    color: #7dd3fc;

    line-height: 1.35;

  }



  .log-live-indicator {

    font-style: normal;

    font-weight: 700;

    color: #93c5fd;

    font-size: 0.68rem;

    text-transform: uppercase;

    letter-spacing: 0.04em;

  }



  .log-latest {

    font-style: normal;

    font-family: monospace;

    font-size: 0.62rem;

    color: var(--text-primary);

    line-height: 1.35;

    word-break: break-word;

  }



  .log-entry {

    font-family: monospace;

    font-size: 0.65rem;

    color: var(--text-secondary);

    white-space: pre-wrap;

    line-height: 1.3;

    border-bottom: 1px solid rgba(255, 255, 255, 0.02);

    padding-bottom: 2px;

  }



  .log-entry:first-child {

    color: var(--text-primary);

    font-weight: 550;

  }



  .log-entry.debug-entry {

    color: #94a3b8;

    font-size: 0.62rem;

  }

</style>


