<!-- ./src/lib/components/ai/evidence/VectorAtlasPanel.svelte -->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { buildVectorAtlas, getVectorAtlasCached, enableNamedVectorsCollection } from "../../../api/tauri";
  import type { AtlasPoint } from "../../../types/research";
  import ActivityPulse from "../../common/loading/ActivityPulse.svelte";
  import { atlasCategoryColor, atlasDotRadius } from "../../../utils/atlasColors";
  import "$lib/styles/components/vector-workbench.css";

  interface Props {
    sampleId: number;
    ollamaUrl: string;
    actionsEnabled?: boolean;
    onSelectRsid?: (rsid: string) => void;
  }

  let { sampleId, ollamaUrl, actionsEnabled = true, onSelectRsid }: Props = $props();

  let points = $state<AtlasPoint[]>([]);
  let loading = $state(false);
  let building = $state(false);
  let error = $state("");
  let warning = $state("");
  let method = $state("");
  let namedMsg = $state("");
  let categoryFilter = $state("");
  let minQuality = $state(0);
  let progressPercent = $state(0);
  let progressMessage = $state("");
  let hoveredRsid = $state<string | null>(null);
  let unlistenProgress: UnlistenFn | null = null;

  const pad = 36;
  let width = $state(720);
  let height = $state(460);

  let busy = $derived(loading || building);
  let actionsLocked = $derived(!actionsEnabled || busy);

  let categories = $derived(
    [...new Set(points.map((p) => p.trait_category).filter(Boolean) as string[])].sort(),
  );

  let visiblePoints = $derived(
    points
      .filter((p) => {
        if (p.data_quality_score < minQuality) return false;
        if (!categoryFilter) return true;
        return (p.trait_category || "") === categoryFilter;
      })
      .slice()
      .sort((a, b) => a.data_quality_score - b.data_quality_score),
  );

  let legendItems = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const p of visiblePoints) {
      const key = p.trait_category?.trim() || "Uncategorized";
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([label, count]) => ({
        label,
        count,
        color: atlasCategoryColor(label === "Uncategorized" ? "" : label),
      }));
  });

  let hoveredPoint = $derived(
    hoveredRsid ? visiblePoints.find((p) => p.rsid === hoveredRsid) ?? null : null,
  );

  function scaleX(x: number) {
    return pad + ((x + 1) / 2) * (width - pad * 2);
  }
  function scaleY(y: number) {
    return pad + ((1 - y) / 2) * (height - pad * 2);
  }

  async function loadCached() {
    loading = true;
    error = "";
    progressMessage = "Loading cached atlas…";
    progressPercent = 10;
    try {
      points = await getVectorAtlasCached(sampleId, 1500);
      if (points.length === 0) {
        warning = "No cached atlas yet. Build one from enriched vectors already in Qdrant.";
      } else {
        warning = "";
      }
      progressPercent = 100;
      progressMessage = points.length
        ? `Loaded ${points.length.toLocaleString()} cached points`
        : "No cached atlas";
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function buildAtlas() {
    if (actionsLocked) return;
    building = true;
    error = "";
    progressPercent = 0;
    progressMessage = "Starting atlas build…";
    try {
      const result = await buildVectorAtlas(sampleId, 1500);
      points = result.points;
      warning = result.warning;
      method = result.projection_method;
      progressPercent = 100;
      progressMessage = `Done — ${result.point_count.toLocaleString()} points`;
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
      progressMessage = "Build failed";
    } finally {
      building = false;
    }
  }

  async function enableNamedVectors() {
    if (actionsLocked) return;
    namedMsg = "Enabling named vectors on Qdrant collection…";
    try {
      namedMsg = await enableNamedVectorsCollection(ollamaUrl);
    } catch (e: unknown) {
      namedMsg = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(async () => {
    unlistenProgress = await listen<{
      phase: string;
      percent: number;
      message: string;
    }>("vector:atlas_progress", (event) => {
      progressPercent = Math.max(0, Math.min(100, event.payload.percent ?? 0));
      progressMessage = event.payload.message || progressMessage;
    });
    await loadCached();
  });

  onDestroy(() => {
    unlistenProgress?.();
  });

  $effect(() => {
    if (sampleId) void loadCached();
  });
</script>

<section class="atlas-panel vector-workbench-card">
  <header class="atlas-header vector-workbench-header">
    <div class="atlas-title-block">
      <h4>Vector atlas</h4>
      <p class="hint">
        Semantic map of enriched variants
        {#if method}<span class="atlas-method"> · {method}</span>{/if}
      </p>
    </div>
    <div class="atlas-toolbar">
      <button
        type="button"
        class="btn btn-secondary btn-xs"
        onclick={loadCached}
        disabled={actionsLocked}
      >
        Reload
      </button>
      <button
        type="button"
        class="btn btn-primary btn-xs"
        onclick={buildAtlas}
        disabled={actionsLocked}
      >
        {building ? "Building…" : "Build atlas"}
      </button>
      <button
        type="button"
        class="btn btn-secondary btn-xs"
        onclick={enableNamedVectors}
        disabled={actionsLocked || !ollamaUrl}
        title="Qdrant only — named multi-vectors"
      >
        Named vectors
      </button>
    </div>
  </header>

  <div class="atlas-filters">
    <label class="atlas-filter">
      <span>Category</span>
      <select bind:value={categoryFilter} disabled={busy || points.length === 0}>
        <option value="">All categories</option>
        {#each categories as c (c)}
          <option value={c}>{c}</option>
        {/each}
      </select>
    </label>
    <label class="atlas-filter atlas-filter-range">
      <span>Min quality {(minQuality * 100).toFixed(0)}%</span>
      <input type="range" min="0" max="1" step="0.05" bind:value={minQuality} disabled={busy || points.length === 0} />
    </label>
  </div>

  {#if !actionsEnabled}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse message="Waiting for connections to finish loading…" accent="#5eead4" />
    </div>
  {/if}

  {#if busy}
    <div class="atlas-progress" role="status" aria-live="polite">
      <ActivityPulse
        message={progressMessage || (building ? "Building atlas…" : "Loading…")}
        accent="#38bdf8"
      />
      <div class="atlas-progress-track">
        <div
          class="atlas-progress-fill"
          style="width: {progressPercent > 0 ? progressPercent + '%' : '100%'}; animation: {progressPercent > 0
            ? 'none'
            : 'indeterminate 1.4s ease infinite'};"
        ></div>
      </div>
      <div class="atlas-progress-meta">
        <span>{progressPercent > 0 ? `${progressPercent}%` : "working…"}</span>
        <span>{building ? "build" : "cache"}</span>
      </div>
    </div>
  {/if}

  {#if warning && !busy}
    <p class="warning">{warning}</p>
  {/if}
  {#if namedMsg}
    <p class="info">{namedMsg}</p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if !busy && points.length === 0}
    <div class="atlas-empty">
      <div class="atlas-empty-visual" aria-hidden="true">
        <span></span><span></span><span></span><span></span><span></span>
      </div>
      <strong>No atlas yet</strong>
      <p>Build after a sweep has indexed variants into Qdrant. The map projects embeddings into 2D for exploration.</p>
    </div>
  {:else if !busy && visiblePoints.length === 0}
    <div class="atlas-empty compact">
      <strong>No points match filters</strong>
      <p>Relax category or minimum quality to see more of the map.</p>
    </div>
  {:else if !busy}
    <div class="atlas-stage">
      <svg
        class="atlas-svg"
        viewBox="0 0 {width} {height}"
        role="img"
        aria-label="Vector atlas scatter plot"
      >
        <defs>
          <radialGradient id="atlasGlow" cx="50%" cy="45%" r="65%">
            <stop offset="0%" stop-color="rgba(56, 189, 248, 0.12)" />
            <stop offset="55%" stop-color="rgba(15, 23, 42, 0.35)" />
            <stop offset="100%" stop-color="rgba(2, 6, 23, 0.9)" />
          </radialGradient>
          <filter id="atlasSoft" x="-40%" y="-40%" width="180%" height="180%">
            <feGaussianBlur stdDeviation="0.6" />
          </filter>
        </defs>
        <rect x="0" y="0" {width} {height} fill="url(#atlasGlow)" />
        <!-- subtle grid -->
        {#each [0.25, 0.5, 0.75] as g (g)}
          <line
            x1={pad}
            y1={pad + g * (height - pad * 2)}
            x2={width - pad}
            y2={pad + g * (height - pad * 2)}
            class="atlas-grid"
          />
          <line
            x1={pad + g * (width - pad * 2)}
            y1={pad}
            x2={pad + g * (width - pad * 2)}
            y2={height - pad}
            class="atlas-grid"
          />
        {/each}
        <rect
          x={pad}
          y={pad}
          width={width - pad * 2}
          height={height - pad * 2}
          class="atlas-frame"
        />
        {#each visiblePoints as p (p.rsid)}
          {@const color = atlasCategoryColor(p.trait_category)}
          {@const r = atlasDotRadius(p.data_quality_score)}
          {@const active = hoveredRsid === p.rsid}
          <circle
            cx={scaleX(p.x)}
            cy={scaleY(p.y)}
            r={active ? r + 2.2 : r}
            fill={color}
            opacity={active ? 1 : 0.42 + p.data_quality_score * 0.5}
            class="atlas-dot"
            class:atlas-dot-active={active}
            tabindex="0"
            role="button"
            aria-label="{p.rsid} {p.trait_category || ''}"
            onmouseenter={() => (hoveredRsid = p.rsid)}
            onmouseleave={() => {
              if (hoveredRsid === p.rsid) hoveredRsid = null;
            }}
            onclick={() => onSelectRsid?.(p.rsid)}
            onkeydown={(e) => e.key === "Enter" && onSelectRsid?.(p.rsid)}
          />
        {/each}
      </svg>

      <aside class="atlas-side">
        {#if hoveredPoint}
          <div class="atlas-tooltip-card">
            <strong>{hoveredPoint.rsid}</strong>
            <span>{hoveredPoint.gene_symbol || "Unknown gene"}</span>
            <span class="atlas-tooltip-cat" style:color={atlasCategoryColor(hoveredPoint.trait_category)}>
              {hoveredPoint.trait_category || "Uncategorized"}
            </span>
            <span>Quality {(hoveredPoint.data_quality_score * 100).toFixed(0)}%</span>
            <span class="atlas-tooltip-hint">Click to open in Viewer</span>
          </div>
        {:else}
          <div class="atlas-tooltip-card muted">
            <strong>Explore</strong>
            <span>Hover a point for details. Size reflects data quality; color is trait category.</span>
          </div>
        {/if}

        {#if legendItems.length > 0}
          <ul class="atlas-legend">
            {#each legendItems as item (item.label)}
              <li>
                <span class="atlas-legend-swatch" style:background={item.color}></span>
                <span class="atlas-legend-label">{item.label}</span>
                <span class="atlas-legend-count">{item.count}</span>
              </li>
            {/each}
          </ul>
        {/if}

        <p class="footnote">
          {visiblePoints.length.toLocaleString()} / {points.length.toLocaleString()} points shown
        </p>
      </aside>
    </div>
  {/if}
</section>
