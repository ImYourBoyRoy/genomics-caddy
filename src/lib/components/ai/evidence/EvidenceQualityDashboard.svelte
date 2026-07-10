<!-- ./src/lib/components/ai/evidence/EvidenceQualityDashboard.svelte -->

<script lang="ts">

  import { onMount } from "svelte";

  import { getQualityDashboard, ensureQdrantPayloadIndexes, backfillEvidencePayloads, enableNamedVectorsCollection, reembedStaleVectors } from "../../../api/tauri";

  import type { QualityDashboard } from "../../../types/research";



  interface Props {
    sampleId: number;
    ollamaUrl?: string;
  }

  let { sampleId, ollamaUrl = "" }: Props = $props();



  let dash = $state<QualityDashboard | null>(null);

  let loading = $state(true);

  let error = $state("");

  let indexMsg = $state("");

  let backfillMsg = $state("");
  let namedMsg = $state("");



  async function refresh() {

    loading = true;

    error = "";

    try {

      dash = await getQualityDashboard(sampleId);

    } catch (e: unknown) {

      error = e instanceof Error ? e.message : String(e);

    } finally {

      loading = false;

    }

  }



  async function ensureIndexes() {

    indexMsg = "Creating payload indexes…";

    try {

      const created = await ensureQdrantPayloadIndexes();

      indexMsg = `Indexed ${created.length} fields (existing indexes skipped).`;

    } catch (e: unknown) {

      indexMsg = e instanceof Error ? e.message : String(e);

    }

  }



  async function runBackfill() {

    backfillMsg = "Backfilling Qdrant payloads…";

    try {

      const result = await backfillEvidencePayloads(sampleId, 500);

      backfillMsg = `Scanned ${result.scanned}, updated ${result.updated_payload}, skipped ${result.skipped_current}, needs re-embed ${result.needs_reembed}`;

      await refresh();

    } catch (e: unknown) {

      backfillMsg = e instanceof Error ? e.message : String(e);

    }

  }



  async function runNamedVectors() {
    if (!ollamaUrl) {
      namedMsg = "Configure Ollama URL first.";
      return;
    }
    namedMsg = "Enabling named-vector collection…";
    try {
      namedMsg = await enableNamedVectorsCollection(ollamaUrl);
    } catch (e: unknown) {
      namedMsg = e instanceof Error ? e.message : String(e);
    }
  }

  async function runReembed() {
    if (!ollamaUrl) {
      backfillMsg = "Configure Ollama URL to re-embed stale vectors.";
      return;
    }
    backfillMsg = "Re-embedding stale Qdrant points…";
    try {
      const result = await reembedStaleVectors(sampleId, ollamaUrl, 200);
      backfillMsg = `Re-embedded ${result.reembedded}, skipped fresh ${result.skipped_fresh}, errors ${result.errors}`;
      await refresh();
    } catch (e: unknown) {
      backfillMsg = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(refresh);

  $effect(() => {

    if (sampleId) refresh();

  });

</script>



<section class="quality-dashboard">

  <header class="qd-header">

    <h4>Evidence quality &amp; coverage</h4>

    <div class="qd-actions">

      <button type="button" class="btn btn-secondary btn-xs" onclick={refresh} disabled={loading}>Refresh</button>

      <button type="button" class="btn btn-secondary btn-xs" onclick={ensureIndexes}>Ensure Qdrant indexes</button>

      <button type="button" class="btn btn-secondary btn-xs" onclick={runBackfill}>Backfill payloads</button>
      <button type="button" class="btn btn-secondary btn-xs" onclick={runReembed}>Re-embed stale</button>
      <button type="button" class="btn btn-secondary btn-xs" onclick={runNamedVectors}>Named vectors</button>

    </div>

  </header>



  {#if loading}

    <p>Loading dashboard…</p>

  {:else if error}

    <p class="error">{error}</p>

  {:else if dash}

    <div class="metric-grid">

      <div class="metric"><span class="label">Genotypes</span><span class="value">{dash.total_genotypes.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Vectorized</span><span class="value">{(dash.vectorized_variants ?? 0).toLocaleString()}</span></div>

      <div class="metric"><span class="label">Association facts</span><span class="value">{dash.association_fact_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Source records</span><span class="value">{dash.source_record_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">GWAS variants</span><span class="value">{dash.variants_with_gwas.toLocaleString()}</span></div>

      <div class="metric"><span class="label">ClinVar variants</span><span class="value">{dash.variants_with_clinvar.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Known direction</span><span class="value">{dash.known_direction_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Unknown direction</span><span class="value">{dash.unknown_direction_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Stale vectors</span><span class="value">{dash.stale_vector_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Schema mismatch</span><span class="value">{dash.schema_mismatch_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Missing gene</span><span class="value">{dash.missing_gene_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Missing effect allele</span><span class="value">{dash.missing_effect_allele_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Missing study accession</span><span class="value">{dash.missing_study_accession_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Source conflicts</span><span class="value">{dash.source_conflict_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Cache stale entries</span><span class="value">{dash.cache_stale_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Unreviewed candidates</span><span class="value">{dash.candidate_unreviewed_count.toLocaleString()}</span></div>

      <div class="metric"><span class="label">Avg data quality</span><span class="value">{(dash.avg_data_quality_score * 100).toFixed(0)}%</span></div>

    </div>



    <div class="coverage-section">

      <h5>Source coverage</h5>

      <div class="coverage-bars">

        {#each [

          ["GWAS", dash.source_coverage.gwas, dash.vectorized_variants ?? 1],

          ["ClinVar", dash.source_coverage.clinvar, dash.vectorized_variants ?? 1],

          ["GTEx", dash.source_coverage.gtex, dash.vectorized_variants ?? 1],

          ["PubMed", dash.source_coverage.pubmed, dash.vectorized_variants ?? 1],

          ["PGS", dash.source_coverage.pgs, dash.vectorized_variants ?? 1],
          ["PharmGKB", dash.source_coverage.pharmgkb, dash.vectorized_variants ?? 1],
          ["Reactome", dash.source_coverage.reactome, dash.vectorized_variants ?? 1],

        ] as [label, count, total]}

          <div class="coverage-row">

            <span class="cov-label">{label}</span>

            <div class="cov-bar"><div class="cov-fill" style:width="{Math.min(100, (Number(count) / Math.max(1, Number(total))) * 100)}%"></div></div>

            <span class="cov-count">{count}</span>

          </div>

        {/each}

      </div>

    </div>



    <p class="meta">Collection: <code>{dash.collection}</code>{#if dash.enrichment_status} · Sweep: {dash.enrichment_status}{/if}</p>

  {/if}

  {#if indexMsg}<p class="index-msg">{indexMsg}</p>{/if}

  {#if backfillMsg}<p class="index-msg">{backfillMsg}</p>{/if}
  {#if namedMsg}<p class="index-msg">{namedMsg}</p>{/if}

</section>



<style>

  .quality-dashboard { padding: 4px 0; }

  .qd-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }

  .qd-header h4 { margin: 0; }

  .qd-actions { display: flex; gap: 6px; flex-wrap: wrap; }

  .metric-grid {

    display: grid;

    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));

    gap: 10px;

  }

  .metric {

    border: 1px solid rgba(255, 255, 255, 0.08);

    border-radius: 8px;

    padding: 10px;

    background: rgba(0, 0, 0, 0.15);

  }

  .metric .label { display: block; font-size: 0.72rem; opacity: 0.7; text-transform: uppercase; }

  .metric .value { font-size: 1.1rem; font-weight: 600; }

  .coverage-section { margin-top: 16px; }

  .coverage-section h5 { margin: 0 0 8px; font-size: 0.78rem; text-transform: uppercase; opacity: 0.75; }

  .coverage-bars { display: flex; flex-direction: column; gap: 8px; }

  .coverage-row { display: grid; grid-template-columns: 72px 1fr 48px; gap: 8px; align-items: center; font-size: 0.82rem; }

  .cov-bar { height: 8px; background: rgba(255,255,255,0.08); border-radius: 4px; overflow: hidden; }

  .cov-fill { height: 100%; background: rgba(100, 180, 255, 0.55); }

  .meta, .index-msg { font-size: 0.82rem; margin-top: 10px; opacity: 0.85; }

  .error { color: #f08080; }

</style>


