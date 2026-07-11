<!-- ./src/lib/components/discovery/DiscoveryPanel.svelte -->
<script lang="ts">
  /*
  Purpose: In-app browser for genome × catalog associations beyond curated packs.
  Responsibilities:
  - Query ranked ClinVar/GWAS/PharmGKB hits for the active sample.
  - Show join progress, allow cancel, and reuse backend cache for fast pagination.
  Key Inputs: selectedSample, optional navigate callback.
  Key Outputs: Interactive discovery findings (not JSON-only export).
  */

  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    queryDiscoveryFindings,
    cancelDiscoveryQuery,
    type DiscoveryFindingItem,
    type DiscoveryQueryResult,
  } from "$lib/api/tauri";
  import type { GenomeSample } from "$lib/types/genomics";
  import "$lib/styles/components/discovery-panel.css";

  interface Props {
    selectedSample: GenomeSample;
    onNavigate?: (rsid: string) => void;
  }

  let { selectedSample, onNavigate }: Props = $props();

  let loading = $state(false);
  let error = $state("");
  let result = $state<DiscoveryQueryResult | null>(null);
  let sourceFilter = $state<"all" | "clinvar" | "gwas" | "pharmgkb">("all");
  let beyondOnly = $state(true);
  let queryText = $state("");
  let offset = $state(0);
  let progressPercent = $state(0);
  let progressMessage = $state("");
  const pageSize = 50;
  let requestSeq = 0;

  async function loadAt(nextOffset: number) {
    const seq = ++requestSeq;
    loading = true;
    error = "";
    offset = nextOffset;
    progressPercent = 0;
    progressMessage = "Starting…";
    try {
      const next = await queryDiscoveryFindings(selectedSample.id, {
        beyondPacksOnly: beyondOnly,
        sourceFilter: sourceFilter === "all" ? null : sourceFilter,
        query: queryText.trim() || null,
        limit: pageSize,
        offset: nextOffset,
      });
      if (seq !== requestSeq) return;
      result = next;
      progressPercent = 100;
      progressMessage = next.cached ? "Loaded from cache" : "Complete";
    } catch (e: unknown) {
      if (seq !== requestSeq) return;
      const msg = String(e);
      error = msg.includes("cancelled") ? "" : msg;
      if (msg.includes("cancelled")) {
        progressMessage = "Cancelled";
      } else {
        result = null;
      }
    } finally {
      if (seq === requestSeq) loading = false;
    }
  }

  async function handleCancel() {
    try {
      await cancelDiscoveryQuery();
      progressMessage = "Cancelling…";
    } catch (e) {
      console.warn("cancel discovery failed", e);
    }
  }

  $effect(() => {
    selectedSample.id;
    sourceFilter;
    beyondOnly;
    void loadAt(0);
  });

  function applySearch() {
    void loadAt(0);
  }

  onMount(() => {
    let unlisten: UnlistenFn | null = null;
    void listen<{ percent?: number; message?: string }>("discovery:query_progress", (event) => {
      progressPercent = event.payload.percent ?? progressPercent;
      progressMessage = event.payload.message ?? progressMessage;
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
      void cancelDiscoveryQuery();
    };
  });

  function geneLabel(item: DiscoveryFindingItem): string {
    return item.clinvar?.gene || item.pharmgkb?.gene || item.gwas?.primary_gene || "—";
  }

  function sourcesFor(item: DiscoveryFindingItem): Array<"clinvar" | "gwas" | "pharmgkb"> {
    const out: Array<"clinvar" | "gwas" | "pharmgkb"> = [];
    if (item.clinvar) out.push("clinvar");
    if (item.gwas) out.push("gwas");
    if (item.pharmgkb) out.push("pharmgkb");
    return out;
  }

  function associationLines(item: DiscoveryFindingItem): string[] {
    const lines: string[] = [];
    if (item.clinvar?.clinical_significance) {
      const pheno = item.clinvar.phenotypes ? ` · ${item.clinvar.phenotypes}` : "";
      lines.push(`ClinVar · ${item.clinvar.clinical_significance}${pheno}`);
    }
    if (item.pharmgkb?.drug) {
      const level = item.pharmgkb.evidence_level ? ` (${item.pharmgkb.evidence_level})` : "";
      lines.push(`PharmGKB · ${item.pharmgkb.drug}${level}`);
    }
    if (item.gwas?.top_trait) {
      const p = item.gwas.best_pvalue != null ? ` · p=${item.gwas.best_pvalue}` : "";
      lines.push(`GWAS · ${item.gwas.top_trait}${p}`);
    }
    return lines.length ? lines : ["Catalog association"];
  }

  function sourceLabel(src: string): string {
    switch (src) {
      case "clinvar":
        return "ClinVar";
      case "gwas":
        return "GWAS";
      case "pharmgkb":
        return "PharmGKB";
      default:
        return src;
    }
  }

  let beyondCount = $derived(
    result ? Math.max(0, result.total_matched - (beyondOnly ? 0 : result.findings_in_packs)) : 0
  );
</script>

<section class="discovery-panel">
  <header class="discovery-hero">
    <div class="discovery-hero-copy">
      <span class="discovery-kicker">Local catalogs · on-device</span>
      <h3>Catalog discovery</h3>
      <p class="discovery-lead">
        Ranked ClinVar, GWAS, and PharmGKB hits in
        <strong>{selectedSample.name}</strong>. Default view is
        <em>beyond curated packs</em> — candidates worth reviewing for new pack entries. Educational
        only; not a clinical report.
      </p>
    </div>

    {#if result}
      <div class="discovery-stat-grid" aria-live="polite">
        <div class="discovery-stat">
          <span class="discovery-stat-value">{result.total_matched.toLocaleString()}</span>
          <span class="discovery-stat-label">Matched</span>
        </div>
        <div class="discovery-stat">
          <span class="discovery-stat-value">{result.findings_in_packs.toLocaleString()}</span>
          <span class="discovery-stat-label">In packs</span>
        </div>
        <div class="discovery-stat">
          <span class="discovery-stat-value">{result.genotype_rsid_count.toLocaleString()}</span>
          <span class="discovery-stat-label">Genotype rsIDs</span>
        </div>
        {#if result.cached}
          <div class="discovery-stat discovery-stat-cache">
            <span class="discovery-stat-value">Cache</span>
            <span class="discovery-stat-label">Fast path</span>
          </div>
        {/if}
      </div>
    {/if}
  </header>

  <aside class="discovery-insight" aria-labelledby="discovery-insight-title">
    <strong id="discovery-insight-title">How to read this</strong>
    <p>
      <span class="insight-tag beyond">Beyond</span> means the variant is not already covered by your
      curated marker packs.
      <span class="insight-tag pack">In pack</span> means it already appears in Trait Report packs.
      Open a row to inspect it in the variant browser.
    </p>
  </aside>

  <div class="discovery-toolbar">
    <div class="discovery-source-seg" role="group" aria-label="Catalog source">
      {#each [
        { id: "all", label: "All" },
        { id: "clinvar", label: "ClinVar" },
        { id: "gwas", label: "GWAS" },
        { id: "pharmgkb", label: "PharmGKB" },
      ] as src (src.id)}
        <button
          type="button"
          class="seg-btn"
          class:active={sourceFilter === src.id}
          aria-pressed={sourceFilter === src.id}
          onclick={() => (sourceFilter = src.id as typeof sourceFilter)}
        >
          {src.label}
        </button>
      {/each}
    </div>

    <label class="discovery-toggle">
      <input type="checkbox" bind:checked={beyondOnly} />
      <span class="toggle-ui" aria-hidden="true"></span>
      <span class="toggle-label">Beyond packs only</span>
    </label>

    <label class="discovery-search">
      <span class="sr-only">Search findings</span>
      <input
        type="search"
        bind:value={queryText}
        placeholder="Search rsID, gene, trait, drug…"
        onkeydown={(e) => e.key === "Enter" && applySearch()}
      />
    </label>

    <div class="discovery-toolbar-actions">
      <button type="button" class="btn btn-secondary btn-sm" onclick={applySearch} disabled={loading}>
        Apply
      </button>
      {#if loading}
        <button type="button" class="btn btn-secondary btn-sm" onclick={handleCancel}>Cancel</button>
      {/if}
    </div>
  </div>

  {#if loading}
    <div class="discovery-progress" role="status" aria-live="polite">
      <div class="discovery-progress-meta">
        <span>{progressMessage || "Scanning catalogs…"}</span>
        <span class="discovery-progress-pct">{progressPercent}%</span>
      </div>
      <div class="discovery-progress-bar">
        <div class="discovery-progress-fill" style={`width: ${progressPercent}%`}></div>
      </div>
    </div>
  {/if}

  {#if error}
    <div class="discovery-error" role="alert">{error}</div>
  {/if}

  {#if loading && !result}
    <div class="discovery-skeleton" aria-hidden="true">
      {#each [1, 2, 3, 4] as n (n)}
        <div class="discovery-skeleton-row"></div>
      {/each}
    </div>
    <p class="discovery-loading">Joining your genotypes to local ClinVar / GWAS / PharmGKB…</p>
  {:else if result && result.items.length === 0}
    <div class="discovery-empty">
      <strong>No associations matched</strong>
      <p>
        Try clearing search, switching catalog source, or turning off
        <em>Beyond packs only</em>
        {#if beyondCount === 0 && result.findings_in_packs > 0}
          — {result.findings_in_packs.toLocaleString()} pack-covered hits are currently hidden.
        {/if}
      </p>
    </div>
  {:else if result}
    <ul class="discovery-list" aria-label="Discovery findings">
      {#each result.items as item (item.rsid)}
        {@const sources = sourcesFor(item)}
        {@const lines = associationLines(item)}
        <li class="discovery-card" class:beyond={!item.in_marker_packs} class:in-pack={!!item.in_marker_packs}>
          <div class="discovery-card-main">
            <div class="discovery-card-id">
              <span class="discovery-rsid font-mono">{item.rsid}</span>
              <span class="discovery-gene">{geneLabel(item)}</span>
              {#if item.genotype}
                <span class="discovery-gt font-mono">{item.genotype}</span>
              {/if}
            </div>

            <div class="discovery-card-tags">
              {#each sources as src (src)}
                <span class="src-pill {src}">{sourceLabel(src)}</span>
              {/each}
              <span class="pack-pill" class:beyond={!item.in_marker_packs}>
                {item.in_marker_packs ? "In pack" : "Beyond packs"}
              </span>
            </div>

            <ul class="discovery-assoc">
              {#each lines as line, i (`${item.rsid}:a${i}`)}
                <li>{line}</li>
              {/each}
            </ul>
          </div>

          {#if onNavigate}
            <button
              type="button"
              class="discovery-open btn btn-secondary btn-sm"
              onclick={() => onNavigate?.(item.rsid)}
            >
              Open
            </button>
          {/if}
        </li>
      {/each}
    </ul>

    <div class="discovery-pager">
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={loading || offset <= 0}
        onclick={() => {
          void loadAt(Math.max(0, offset - pageSize));
        }}
      >
        Previous
      </button>
      <span class="discovery-pager-range">
        Showing <strong>{offset + 1}–{Math.min(offset + result.items.length, result.total_matched)}</strong>
        of <strong>{result.total_matched.toLocaleString()}</strong>
      </span>
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={loading || offset + result.items.length >= result.total_matched}
        onclick={() => {
          void loadAt(offset + pageSize);
        }}
      >
        Next
      </button>
    </div>
  {/if}
</section>
