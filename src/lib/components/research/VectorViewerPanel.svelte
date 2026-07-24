<!-- ./src/lib/components/research/VectorViewerPanel.svelte -->
<script lang="ts">
  /*
  Purpose: Browse indexed vectors + optional Qdrant named-vector semantic search.
  */

  import {
    browseVectorStore,
    searchQdrantEvidence,
    searchQdrantTraitDiscovery,
  } from "../../api/tauri";
  import type { QdrantHit } from "../../types/research";
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import "$lib/styles/components/vector-workbench.css";

  interface Props {
    sampleId: number;
    ollamaUrl: string;
    initialRsid?: string;
    actionsEnabled?: boolean;
    onSelectRsid?: (rsid: string) => void;
    onLog?: (msg: string) => void;
  }

  let {
    sampleId,
    ollamaUrl,
    initialRsid = "",
    actionsEnabled = true,
    onSelectRsid,
    onLog,
  }: Props = $props();

  type Mode = "browse" | "evidence" | "trait";

  let points = $state<QdrantHit[]>([]);
  let loading = $state(false);
  let error = $state("");
  let note = $state("");
  let provider = $state("");
  let totalHint = $state<number | null>(null);
  let nextOffset = $state<unknown | null>(null);
  let rsidQuery = $state("");
  let semanticQuery = $state("");
  let mode = $state<Mode>("browse");
  let selected = $state<QdrantHit | null>(null);
  let pageSize = $state(50);

  async function loadBrowse(reset = true) {
    loading = true;
    error = "";
    try {
      const page = await browseVectorStore(
        sampleId,
        ollamaUrl,
        pageSize,
        reset ? undefined : (nextOffset as object | undefined),
        rsidQuery.trim() || undefined
      );
      provider = page.provider;
      note = page.note || "";
      totalHint = page.total_hint ?? null;
      points = reset ? page.points : [...points, ...page.points];
      nextOffset = page.next_offset ?? null;
      onLog?.(
        `Vector viewer: ${page.points.length} point(s) from ${page.provider}` +
          (totalHint != null ? ` · ~${totalHint.toLocaleString()} indexed` : "")
      );
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function loadSemantic() {
    loading = true;
    error = "";
    note = "";
    nextOffset = null;
    try {
      const q = semanticQuery.trim() || rsidQuery.trim();
      if (!q && mode === "evidence") {
        error = "Enter a semantic query (or rsID) for evidence search.";
        return;
      }
      const hits =
        mode === "trait"
          ? await searchQdrantTraitDiscovery(q || undefined, undefined, sampleId, ollamaUrl, pageSize)
          : await searchQdrantEvidence(q, ollamaUrl, sampleId, pageSize);
      points = hits;
      provider = "qdrant";
      note =
        mode === "trait"
          ? "Trait discovery search (routes to trait_dense when named vectors are enabled)."
          : "Evidence search (routes to evidence_dense / actionability when named vectors are enabled).";
      onLog?.(`Semantic ${mode}: ${hits.length} hit(s)`);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function load(reset = true) {
    if (!actionsEnabled) return;
    if (mode === "browse") await loadBrowse(reset);
    else await loadSemantic();
  }

  function openHit(hit: QdrantHit) {
    selected = hit;
    if (hit.rsid) onSelectRsid?.(hit.rsid);
  }

  $effect(() => {
    if (sampleId && actionsEnabled) void load(true);
  });

  $effect(() => {
    const rsid = initialRsid?.trim();
    if (!rsid || !actionsEnabled) return;
    rsidQuery = rsid;
    mode = "browse";
    void loadBrowse(true);
  });
</script>

<section class="vector-workbench-card" aria-labelledby="vector-viewer-title">
  <header class="vector-workbench-header">
    <div>
      <h3 id="vector-viewer-title">Vector viewer</h3>
      <p class="vector-workbench-lead">
        Browse payloads or run Qdrant semantic search (named-vector routing when enabled).
        {#if provider}<span class="provider-chip">{provider}</span>{/if}
        {#if totalHint != null}
          <span class="provider-chip">~{totalHint.toLocaleString()} for sample</span>
        {/if}
      </p>
    </div>
    <div class="vector-workbench-actions">
      <select
        bind:value={mode}
        aria-label="Search mode"
        disabled={!actionsEnabled || loading}
        onchange={() => load(true)}
      >
        <option value="browse">Browse / rsID</option>
        <option value="evidence">Evidence search (named)</option>
        <option value="trait">Trait discovery (named)</option>
      </select>
      {#if mode === "browse"}
        <input
          type="search"
          bind:value={rsidQuery}
          placeholder="rsID lookup"
          aria-label="Filter by rsID"
          class="viewer-search"
          disabled={!actionsEnabled || loading}
        />
      {:else}
        <input
          type="search"
          bind:value={semanticQuery}
          placeholder="Semantic query (e.g. sleep chronotype)"
          aria-label="Semantic query"
          class="viewer-search"
          disabled={!actionsEnabled || loading}
        />
      {/if}
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={!actionsEnabled || loading}
        onclick={() => load(true)}
      >
        {loading ? "Loading…" : "Search"}
      </button>
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={!actionsEnabled || loading || mode !== "browse" || !nextOffset || !!rsidQuery.trim()}
        onclick={() => load(false)}
      >
        Load more
      </button>
    </div>
  </header>

  {#if !actionsEnabled}
    <div class="workbench-boot-banner" role="status">
      <ActivityPulse message="Waiting for connections…" accent="#5eead4" />
    </div>
  {/if}

  {#if note}
    <p class="vector-workbench-note">{note}</p>
  {/if}
  {#if error}
    <p class="vector-workbench-error" role="alert">{error}</p>
  {/if}

  {#if loading}
    <div class="viewer-loading">
      <ActivityPulse
        message={mode === "browse" ? "Loading indexed vectors…" : `Running ${mode} search…`}
        accent="#38bdf8"
      />
    </div>
  {/if}

  <div class="viewer-layout">
    <div class="viewer-table-wrap">
      {#if points.length === 0 && !loading}
        <p class="vector-workbench-empty">No points yet. Run a sweep or check the collection exists.</p>
      {:else if !loading || points.length > 0}
        <table class="viewer-table">
          <thead>
            <tr>
              <th>rsID</th>
              <th>Gene</th>
              <th>Score</th>
              <th>Source</th>
            </tr>
          </thead>
          <tbody>
            {#each points as hit (hit.rsid + String(hit.score))}
              <tr
                class:selected={selected?.rsid === hit.rsid}
                onclick={() => openHit(hit)}
              >
                <td class="font-mono">{hit.rsid}</td>
                <td>{hit.gene || "—"}</td>
                <td>{hit.significance_score?.toFixed?.(2) ?? hit.score?.toFixed?.(3) ?? "—"}</td>
                <td>{hit.source || "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
    <aside class="viewer-detail" aria-label="Point detail">
      {#if selected}
        <h4>{selected.rsid}</h4>
        <dl class="detail-grid">
          <dt>Gene</dt><dd>{selected.gene || "—"}</dd>
          <dt>Genotype</dt><dd>{selected.genotype || "—"}</dd>
          <dt>Sig</dt><dd>{selected.significance_score ?? "—"}</dd>
          <dt>AF</dt><dd>{selected.gnomad_af ?? "—"}</dd>
        </dl>
        <p class="detail-text">{selected.text}</p>
      {:else}
        <p class="vector-workbench-empty">Select a row to inspect payload text.</p>
      {/if}
    </aside>
  </div>
</section>
