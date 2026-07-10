<!-- ./src/lib/components/ai/evidence/SimilarAssociationsPanel.svelte -->
<script lang="ts">
  import { getSimilarAssociations } from "../../../api/tauri";
  import { SIMILARITY_MODES, type EvidenceCard } from "../../../types/research";
  import VectorEvidenceCard from "./VectorEvidenceCard.svelte";
  import type { VariantNavTarget } from "../../../constants/traitCategories";

  interface Props {
    sampleId: number;
    rsid: string;
    onNavigateToVariant?: (rsid: string, target: VariantNavTarget) => void;
    onExportPacket?: (rsid: string) => void;
  }

  let { sampleId, rsid, onNavigateToVariant, onExportPacket }: Props = $props();

  let mode = $state("evidence_similarity");
  let loading = $state(false);
  let error = $state("");
  let results = $state<EvidenceCard[]>([]);

  async function loadSimilar() {
    loading = true;
    error = "";
    try {
      results = await getSimilarAssociations({
        sample_id: sampleId,
        rsid,
        similarity_mode: mode,
        limit: 12,
        include_self: false,
      });
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
      results = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (sampleId && rsid) {
      loadSimilar();
    }
  });
</script>

<section class="similar-panel">
  <header class="similar-header">
    <h4>Similar associations for {rsid}</h4>
    <p class="hint">Semantic proximity is related evidence context — not biological causality.</p>
    <div class="mode-tabs">
      {#each SIMILARITY_MODES as m}
        <button
          type="button"
          class="mode-chip"
          class:active={mode === m.id}
          onclick={() => { mode = m.id; loadSimilar(); }}
        >
          {m.label}
        </button>
      {/each}
    </div>
  </header>

  {#if loading}
    <p class="status">Loading similar hits…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if results.length === 0}
    <p class="status">No similar associations found for this mode.</p>
  {:else}
    {#each results as card (card.rsid + (card.qdrant_point_id || ""))}
      <VectorEvidenceCard {card} {onNavigateToVariant} {onExportPacket} />
    {/each}
  {/if}
</section>

<style>
  .similar-panel { margin-top: 12px; }
  .similar-header h4 { margin: 0 0 4px; }
  .hint { margin: 0 0 10px; font-size: 0.82rem; opacity: 0.75; }
  .mode-tabs { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 10px; }
  .mode-chip {
    font-size: 0.75rem;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: transparent;
    cursor: pointer;
  }
  .mode-chip.active { background: rgba(120, 160, 255, 0.2); }
  .status, .error { font-size: 0.85rem; }
  .error { color: #f08080; }
</style>
