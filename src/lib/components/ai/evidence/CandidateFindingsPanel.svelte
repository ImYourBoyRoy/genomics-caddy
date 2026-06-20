<!-- ./src/lib/components/ai/evidence/CandidateFindingsPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { listCandidateMarkers, updateCandidateMarkerStatus } from "../../../api/tauri";
  import type { CandidateMarkerRow } from "../../../types/research";

  interface Props {
    onSelectRsid?: (rsid: string) => void;
  }

  let { onSelectRsid }: Props = $props();

  let rows = $state<CandidateMarkerRow[]>([]);
  let loading = $state(true);
  let error = $state("");
  let note = $state("");

  async function load() {
    loading = true;
    error = "";
    try {
      rows = await listCandidateMarkers(80);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function setStatus(candidateId: string, status: string) {
    try {
      await updateCandidateMarkerStatus({
        candidate_id: candidateId,
        status,
        reviewer_note: note || undefined,
      });
      await load();
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(load);
</script>

<section class="candidates-panel">
  <header class="cp-header">
    <h4>Dynamic candidate findings</h4>
    <button type="button" class="btn btn-secondary btn-xs" onclick={load} disabled={loading}>Refresh</button>
  </header>
  <p class="hint">LLM may propose candidates but cannot promote to curated_core. Rejected rows stay stored to reduce rediscovery spam.</p>

  <label class="note-field">
    Reviewer note (optional)
    <input type="text" bind:value={note} placeholder="Reason for promote/reject" />
  </label>

  {#if loading}
    <p>Loading candidates…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if rows.length === 0}
    <p class="muted">No dynamic candidates yet. Run a vector enrichment sweep first.</p>
  {:else}
    <div class="candidate-list">
      {#each rows as row (row.candidate_id)}
        <article class="candidate-row">
          <div class="candidate-main">
            <button type="button" class="rsid-link" onclick={() => onSelectRsid?.(row.rsid)}>{row.rsid}</button>
            {#if row.gene_symbol}<span class="gene">{row.gene_symbol}</span>{/if}
            {#if row.trait_name}<span class="trait">{row.trait_name}</span>{/if}
            <span class="status">{row.status.replace(/_/g, " ")}</span>
          </div>
          <div class="candidate-scores">
            DQ {(row.data_quality_score * 100).toFixed(0)}% · Strength {(row.association_strength_score * 100).toFixed(0)}%
            {#if row.source_count != null} · Sources {row.source_count}{/if}
            {#if row.best_p_value != null} · p {row.best_p_value.toExponential(1)}{/if}
          </div>
          <div class="candidate-flags">
            {#if row.has_gwas}<span class="flag">GWAS</span>{/if}
            {#if row.has_clinvar}<span class="flag">ClinVar</span>{/if}
            {#if row.has_gtex}<span class="flag">GTEx</span>{/if}
            {#if row.has_pubmed}<span class="flag">PubMed</span>{/if}
            {#if row.has_pgs}<span class="flag">PGS</span>{/if}
            {#if row.has_pharmgkb}<span class="flag">PharmGKB</span>{/if}
          </div>
          <div class="candidate-actions">
            <button type="button" class="btn btn-secondary btn-xs" onclick={() => setStatus(row.candidate_id, "needs_deeper_research")}>Needs research</button>
            <button type="button" class="btn btn-secondary btn-xs" onclick={() => setStatus(row.candidate_id, "evidence_backed_candidate")}>Evidence backed</button>
            <button type="button" class="btn btn-secondary btn-xs" onclick={() => setStatus(row.candidate_id, "curated_lite")}>Promote lite</button>
            <button type="button" class="btn btn-secondary btn-xs" onclick={() => setStatus(row.candidate_id, "rejected")}>Reject</button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .candidates-panel { padding: 4px 0; }
  .cp-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .cp-header h4 { margin: 0; }
  .hint, .muted { font-size: 0.82rem; opacity: 0.75; margin-bottom: 10px; }
  .note-field { display: block; font-size: 0.82rem; margin-bottom: 10px; }
  .note-field input { width: 100%; margin-top: 4px; }
  .candidate-list { display: flex; flex-direction: column; gap: 10px; }
  .candidate-row {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 10px;
    background: rgba(0, 0, 0, 0.12);
  }
  .candidate-main { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 6px; }
  .rsid-link { background: none; border: none; color: inherit; font-weight: 600; cursor: pointer; padding: 0; }
  .gene, .trait, .status { font-size: 0.75rem; padding: 2px 8px; border-radius: 999px; background: rgba(120, 160, 255, 0.12); }
  .candidate-scores { font-size: 0.78rem; opacity: 0.8; margin-bottom: 4px; }
  .candidate-flags { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 8px; }
  .flag { font-size: 0.68rem; padding: 1px 6px; border-radius: 999px; background: rgba(120, 160, 255, 0.12); }
  .candidate-actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .error { color: #f08080; }
</style>
