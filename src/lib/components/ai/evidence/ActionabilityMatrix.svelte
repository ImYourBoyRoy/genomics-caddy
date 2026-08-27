<!-- ./src/lib/components/ai/evidence/ActionabilityMatrix.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { getActionabilityMatrix } from "../../../api/tauri";
  import type { ActionabilityPoint } from "../../../types/research";
  import Tooltip from "../../common/Tooltip.svelte";

  interface Props {
    sampleId: number;
  }

  let { sampleId }: Props = $props();

  let points = $state<ActionabilityPoint[]>([]);
  let loading = $state(false);
  let error = $state("");

  async function load() {
    loading = true;
    error = "";
    try {
      points = await getActionabilityMatrix(sampleId, 500);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
      points = [];
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function xPct(v: number) {
    return `${Math.min(100, Math.max(0, v * 100))}%`;
  }

  function yPct(v: number) {
    return `${100 - Math.min(100, Math.max(0, v * 100))}%`;
  }

  function pointDescription(point: ActionabilityPoint): string {
    return `${point.rsid} — clinical ${Math.round(point.clinical_actionability_score * 100)}%, wellness ${Math.round(point.wellness_actionability_score * 100)}%`;
  }
</script>

<section class="matrix-panel">
  <header class="matrix-header">
    <h4>Actionability matrix</h4>
    <button type="button" class="btn btn-secondary btn-xs" onclick={load} disabled={loading}>
      {loading ? "Loading…" : "Reload"}
    </button>
  </header>
  <p class="hint">Wellness (Y) vs clinical (X) actionability from cached association facts. Top-right = higher on both axes.</p>

  {#if error}
    <p class="error">{error}</p>
  {:else if points.length === 0 && !loading}
    <p class="muted">No scored associations yet — run vector research enrichment first.</p>
  {:else}
    <div class="matrix-wrap">
      <div class="axis-label y">Wellness ↑</div>
      <div class="plot" role="img" aria-label="Actionability scatter plot">
        {#each points as p (p.rsid)}
          <Tooltip interactiveChildren label="Actionability point" description={pointDescription(p)} placement="right">
            <button
              type="button"
              class="dot"
              aria-label={pointDescription(p)}
              style:left={xPct(p.clinical_actionability_score)}
              style:top={yPct(p.wellness_actionability_score)}
            ></button>
          </Tooltip>
        {/each}
      </div>
      <div class="axis-label x">Clinical →</div>
    </div>
    <p class="muted count">{points.length} variants plotted</p>
  {/if}
</section>

<style>
  .matrix-panel { padding: 4px 0; }
  .matrix-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .matrix-header h4 { margin: 0; }
  .hint, .muted { font-size: 0.82rem; opacity: 0.75; }
  .error { color: #f08080; }
  .matrix-wrap {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: 1fr auto;
    gap: 6px;
    align-items: center;
  }
  .axis-label { font-size: 0.72rem; opacity: 0.65; }
  .axis-label.y { writing-mode: vertical-rl; transform: rotate(180deg); }
  .plot {
    position: relative;
    height: 280px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    background: linear-gradient(135deg, rgba(40, 60, 100, 0.15), rgba(20, 30, 50, 0.2));
  }
  .dot {
    position: absolute;
    width: 8px;
    height: 8px;
    margin: -4px 0 0 -4px;
    border-radius: 50%;
    border: none;
    padding: 0;
    background: rgba(120, 200, 255, 0.85);
    cursor: help;
  }
  .dot:hover { background: rgba(180, 230, 255, 1); transform: scale(1.4); }
  .dot:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .count { margin-top: 8px; }
</style>
