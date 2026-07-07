<!-- ./src/lib/components/ai/evidence/FindingsNavigator.svelte -->
<script lang="ts">
  import type { EvidenceCard, EvidenceCorpusSummary } from "../../../types/research";
  import type { VariantNavTarget } from "../../../constants/traitCategories";
  import { browseAssociations } from "../../../api/tauri";
  import VectorEvidenceCard from "./VectorEvidenceCard.svelte";
  import PanelLoadingState from "../../common/loading/PanelLoadingState.svelte";
  import {
    CATALOG_PRESETS,
    PAGE_SIZE,
    SORT_OPTIONS,
    defaultPreset,
    defaultSort,
    directionBadgeClass,
    formatDirection,
    presetLabel,
    traitLabel,
    type AudienceMode,
    type CatalogPreset,
    type CatalogSort,
  } from "../../../utils/findingsCatalog";
  import "$lib/styles/components/findings-navigator.css";

  interface Props {
    sampleId: number;
    summary: EvidenceCorpusSummary | null;
    loadingSummary?: boolean;
    initialPreset?: CatalogPreset | "";
    initialTraitCategory?: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
    onExportPacket?: (rsid: string) => void;
  }

  let {
    sampleId,
    summary,
    loadingSummary = false,
    initialPreset = "",
    initialTraitCategory = "",
    onNavigateToVariant,
    onExportPacket,
  }: Props = $props();

  let audience = $state<AudienceMode>("individual");
  let preset = $state<CatalogPreset>("actionable");
  let traitCategory = $state("");
  let sort = $state<CatalogSort>("wellness");
  let directionFilter = $state("");
  let minDq = $state(0.25);
  let textFilter = $state("");
  let page = $state(0);

  let cards = $state<EvidenceCard[]>([]);
  let totalCount = $state(0);
  let loading = $state(false);
  let error = $state("");
  let selectedRsid = $state("");

  let activePresetMeta = $derived(CATALOG_PRESETS.find((p) => p.id === preset));
  let pageCount = $derived(Math.max(1, Math.ceil(totalCount / PAGE_SIZE)));
  let pageStart = $derived(totalCount === 0 ? 0 : page * PAGE_SIZE + 1);
  let pageEnd = $derived(Math.min(totalCount, (page + 1) * PAGE_SIZE));
  let selectedCard = $derived(cards.find((c) => c.rsid === selectedRsid) ?? null);

  async function loadPage() {
    loading = true;
    error = "";
    try {
      const result = await browseAssociations({
        sample_id: sampleId,
        preset,
        trait_category: traitCategory || undefined,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
        sort,
        min_data_quality: minDq,
        direction_filter: directionFilter || undefined,
        text_filter: textFilter.trim() || undefined,
      });
      cards = result.cards;
      totalCount = result.total_count;
      if (selectedRsid && !cards.some((c) => c.rsid === selectedRsid)) {
        selectedRsid = cards[0]?.rsid ?? "";
      } else if (!selectedRsid && cards.length > 0) {
        selectedRsid = cards[0].rsid;
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
      cards = [];
      totalCount = 0;
    } finally {
      loading = false;
    }
  }

  function applyAudience(mode: AudienceMode) {
    audience = mode;
    preset = defaultPreset(mode);
    sort = defaultSort(mode);
    page = 0;
    void loadPage();
  }

  function setPreset(next: CatalogPreset) {
    preset = next;
    page = 0;
    void loadPage();
  }

  function setTraitCategory(cat: string) {
    traitCategory = traitCategory === cat ? "" : cat;
    page = 0;
    void loadPage();
  }

  function applyFilters() {
    page = 0;
    void loadPage();
  }

  $effect(() => {
    if (!sampleId) return;
    if (initialPreset) preset = initialPreset;
    if (initialTraitCategory) traitCategory = initialTraitCategory;
    page = 0;
    void loadPage();
  });
</script>

<section class="findings-navigator">
  <div class="fn-header">
    <div>
      <h4>Findings Navigator</h4>
      <p>
        Browse your indexed variants by trait category, clinical relevance, and data quality —
        no AI search required. Tap a row for full provenance and export.
      </p>
    </div>
    <div class="fn-audience">
      <button
        type="button"
        class="btn btn-secondary btn-sm fn-audience-btn"
        class:active={audience === "individual"}
        onclick={() => applyAudience("individual")}
      >
        For you
      </button>
      <button
        type="button"
        class="btn btn-secondary btn-sm fn-audience-btn"
        class:active={audience === "clinical"}
        onclick={() => applyAudience("clinical")}
      >
        Clinical view
      </button>
    </div>
  </div>

  <p class="fn-disclaimer">
    Research and educational context only — not medical advice. Clinical view emphasizes
    annotated signals; confirm with primary literature and qualified professionals before clinical use.
  </p>

  <div class="fn-layout">
    <aside class="fn-sidebar">
      <p class="fn-sidebar-title">Trait categories</p>
      <button
        type="button"
        class="fn-trait-btn"
        class:active={!traitCategory}
        onclick={() => setTraitCategory("")}
      >
        <span>All categories</span>
      </button>
      {#if loadingSummary}
        <p class="fn-empty">Loading categories…</p>
      {:else if summary?.trait_buckets?.length}
        {#each summary.trait_buckets as bucket (bucket.trait_category)}
          <button
            type="button"
            class="fn-trait-btn"
            class:active={traitCategory === bucket.trait_category}
            onclick={() => setTraitCategory(bucket.trait_category)}
          >
            <span>{traitLabel(bucket.trait_category)}</span>
            <span class="fn-trait-count">{bucket.variant_count.toLocaleString()}</span>
          </button>
        {/each}
      {:else}
        <p class="fn-empty">No trait buckets yet.</p>
      {/if}
    </aside>

    <div class="fn-main">
      <div class="fn-preset-row">
        {#each CATALOG_PRESETS as p (p.id)}
          <button
            type="button"
            class="btn btn-secondary btn-sm fn-preset-btn"
            class:active={preset === p.id}
            onclick={() => setPreset(p.id)}
          >
            {presetLabel(p, audience)}
          </button>
        {/each}
      </div>
      {#if activePresetMeta}
        <p class="fn-preset-desc">{activePresetMeta.description}</p>
      {/if}

      <div class="fn-toolbar">
        <label>
          Find rsID or gene
          <input
            type="search"
            placeholder="e.g. rs4680 or COMT"
            bind:value={textFilter}
            onkeydown={(e) => e.key === "Enter" && applyFilters()}
          />
        </label>
        <label>
          Sort by
          <select bind:value={sort} onchange={applyFilters}>
            {#each SORT_OPTIONS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </label>
        <label>
          Direction
          <select bind:value={directionFilter} onchange={applyFilters}>
            <option value="">Any</option>
            <option value="known">Known only</option>
            <option value="unknown">Unknown only</option>
          </select>
        </label>
        <label>
          Min quality {(minDq * 100).toFixed(0)}%
          <input type="range" min="0" max="1" step="0.05" bind:value={minDq} onchange={applyFilters} />
        </label>
        <button type="button" class="btn btn-primary btn-sm" onclick={applyFilters} disabled={loading}>
          Apply filters
        </button>
      </div>

      <div class="fn-results-meta">
        <span><strong>{totalCount.toLocaleString()}</strong> matching variants</span>
        {#if traitCategory}
          <span>Category: {traitLabel(traitCategory)}</span>
        {/if}
        {#if textFilter.trim()}
          <span>Filter: “{textFilter.trim()}”</span>
        {/if}
      </div>

      {#if loading}
        <PanelLoadingState message="Loading findings page…" accent="#38bdf8" compact />
      {:else if error}
        <div class="fn-empty">{error}</div>
      {:else if cards.length === 0}
        <div class="fn-empty">No variants match these filters. Try a broader preset or lower quality threshold.</div>
      {:else}
        <div class="fn-table-wrap">
          <table class="fn-table">
            <thead>
              <tr>
                <th>rsID</th>
                <th>Gene</th>
                <th>Genotype</th>
                <th>Trait / category</th>
                <th>Direction</th>
                {#if audience === "clinical"}
                  <th>Clinical</th>
                {:else}
                  <th>Wellness</th>
                {/if}
                <th>DQ</th>
              </tr>
            </thead>
            <tbody>
              {#each cards as card (card.rsid)}
                <tr
                  class:selected={selectedRsid === card.rsid}
                  onclick={() => (selectedRsid = card.rsid)}
                >
                  <td class="fn-rsid">{card.rsid}</td>
                  <td class="fn-gene">{card.gene_symbol ?? "—"}</td>
                  <td>{card.genotype ?? "—"}</td>
                  <td class="fn-trait">
                    {card.primary_trait?.slice(0, 80) ?? traitLabel(card.trait_category ?? "")}
                    {#if card.trait_category}
                      <br /><span class="fn-badge dir-neutral">{traitLabel(card.trait_category)}</span>
                    {/if}
                  </td>
                  <td>
                    <span class="fn-badge {directionBadgeClass(card.personal_direction)}">
                      {formatDirection(card.personal_direction)}
                    </span>
                  </td>
                  <td>
                    {Math.round(
                      (audience === "clinical"
                        ? card.clinical_actionability_score
                        : card.wellness_actionability_score) * 100,
                    )}%
                  </td>
                  <td>{Math.round(card.data_quality_score * 100)}%</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="fn-pagination">
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            disabled={page <= 0 || loading}
            onclick={() => {
              page -= 1;
              void loadPage();
            }}
          >
            ← Previous
          </button>
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            disabled={page + 1 >= pageCount || loading}
            onclick={() => {
              page += 1;
              void loadPage();
            }}
          >
            Next →
          </button>
          <span class="fn-page-info">
            {pageStart.toLocaleString()}–{pageEnd.toLocaleString()} of {totalCount.toLocaleString()}
            · page {page + 1} / {pageCount}
          </span>
        </div>
      {/if}

      {#if selectedCard}
        <div class="fn-detail">
          <VectorEvidenceCard
            card={selectedCard}
            expanded={true}
            {onNavigateToVariant}
            onExportPacket={onExportPacket}
          />
        </div>
      {/if}
    </div>
  </div>
</section>
