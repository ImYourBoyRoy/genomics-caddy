<!-- ./src/lib/components/research/ResearchScopeSection.svelte -->
<script lang="ts">
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import GnomadSetupPanel from "./GnomadSetupPanel.svelte";
  import {
    previewResearchScope,
    saveResearchScope,
    syncGwasReference,
    getReferenceStatus,
  } from "../../api/tauri";
  import type {
    ResearchScopeConfig,
    ResearchScopePreview,
    ReferenceStatus,
  } from "../../types/research";
  import { DEFAULT_RESEARCH_SCOPE } from "../../types/research";
  import {
    applySourcePreset as applyScopePreset,
    ensureEnrichmentSources,
    syncSweepFastFromSources,
  } from "../../utils/researchScopeHelpers";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    sweepRunning?: boolean;
    scope?: ResearchScopeConfig;
    preview?: ResearchScopePreview | null;
    onScopeChange?: (scope: ResearchScopeConfig) => void;
    onLog?: (msg: string) => void;
    onGnomadReadyChange?: (ready: boolean) => void;
  }

  let {
    selectedSample = null,
    sweepRunning = false,
    scope = $bindable({ ...DEFAULT_RESEARCH_SCOPE }),
    preview = $bindable<ResearchScopePreview | null>(null),
    onScopeChange,
    onLog,
    onGnomadReadyChange,
  }: Props = $props();

  let refStatus = $state<ReferenceStatus | null>(null);
  let isPreviewLoading = $state(false);
  let isSyncing = $state(false);
  let resyncConfirm = $state(false);
  let scopeHydrated = $state(false);
  let scopeDirty = $state(false);
  let previewTimer: ReturnType<typeof setTimeout> | null = null;

  const gwasReady = $derived(
    !!refStatus &&
      refStatus.gwas_rsid_count > 1000 &&
      (refStatus.gwas_file_present || refStatus.gwas_gz_present)
  );

  async function refreshReferenceStatus() {
    try {
      refStatus = await getReferenceStatus();
    } catch {
      refStatus = null;
    }
  }

  async function refreshPreview() {
    if (sweepRunning) return;
    if (!selectedSample) {
      preview = null;
      return;
    }
    isPreviewLoading = true;
    try {
      preview = await previewResearchScope(selectedSample.id, scope);
    } catch (e: any) {
      preview = null;
      onLog?.(`Scope preview failed: ${e.message || String(e)}`);
    } finally {
      isPreviewLoading = false;
    }
  }

  async function persistScope() {
    if (sweepRunning) return;
    try {
      await saveResearchScope(scope);
      onScopeChange?.(scope);
      await refreshPreview();
    } catch (e: any) {
      onLog?.(`Failed to save scope: ${e.message || String(e)}`);
    }
  }

  async function handleSyncGwas() {
    if (gwasReady && !resyncConfirm) {
      resyncConfirm = true;
      onLog?.("Click Re-sync again to replace the local GWAS reference catalog.");
      return;
    }

    isSyncing = true;
    resyncConfirm = false;
    onLog?.("Downloading GWAS Catalog reference (may take a few minutes)...");
    try {
      const result = await syncGwasReference();
      onLog?.(result.message);
      await refreshReferenceStatus();
      await refreshPreview();
    } catch (e: any) {
      onLog?.(`GWAS sync failed: ${e.message || String(e)}`);
    } finally {
      isSyncing = false;
    }
  }

  function applySourcePreset(preset: "fast" | "clinical" | "full") {
    onLog?.(applyScopePreset(scope, preset));
    markScopeDirty();
  }

  function onSourceToggle() {
    syncSweepFastFromSources(scope);
    markScopeDirty();
  }

  const gnomadSourceEnabled = $derived(scope.enrichment_sources?.gnomad ?? false);

  function markScopeDirty() {
    scopeDirty = true;
  }

  $effect(() => {
    ensureEnrichmentSources(scope);
    syncSweepFastFromSources(scope);
  });

  function queueFullGwasOverlap() {
    if (!preview || preview.gwas_genome_overlap === 0) return;
    scope.gwas_discovery_limit = Math.min(preview.gwas_genome_overlap, 100_000);
    scope.gwas_discovery = true;
    markScopeDirty();
    onLog?.(
      `GWAS cap raised to ${scope.gwas_discovery_limit.toLocaleString()} (full genome overlap). Click Expand queue to enrich newly queued rsIDs.`
    );
  }

  function schedulePreview(delayMs = 700) {
    if (previewTimer) clearTimeout(previewTimer);
    previewTimer = setTimeout(() => {
      previewTimer = null;
      void refreshPreview();
    }, delayMs);
  }

  $effect(() => {
    void refreshReferenceStatus();
  });

  $effect(() => {
    selectedSample;
    scopeHydrated = false;
    scopeDirty = false;
    if (previewTimer) {
      clearTimeout(previewTimer);
      previewTimer = null;
    }
  });

  $effect(() => {
    scope;
    if (!selectedSample || sweepRunning) return;

    if (!scopeHydrated) {
      scopeHydrated = true;
      schedulePreview();
      return;
    }

    if (!scopeDirty) return;

    const t = setTimeout(() => {
      scopeDirty = false;
      void persistScope();
    }, 400);
    return () => clearTimeout(t);
  });
</script>

<div class="glass-card scope-card">
  <div class="card-header-row">
    <h2 class="card-title">Sweep Scopes</h2>
    {#if isPreviewLoading}
      <ActivityPulse message="Counting sweep scopes…" accent="#a78bfa" maxWidth="220px" />
    {:else if preview}
      <span class="scope-total">{preview.total_unique.toLocaleString()} queued</span>
    {/if}
  </div>

  <p class="scope-help">
    Counts update automatically when you toggle scopes or finish a GWAS sync — no sweep run required.
    The sweep processes the merged queue; Qdrant search uses indexed vectors after enrichment completes.
  </p>

  <div class="scope-grid">
    <label class="scope-option">
      <input type="checkbox" bind:checked={scope.curated} onchange={markScopeDirty} />
      <span>
        <strong>Curated markers</strong>
        <small>ClinVar + marker packs in your genome{#if preview} — {preview.curated.toLocaleString()}{/if}</small>
      </span>
    </label>

    <label class="scope-option">
      <input type="checkbox" bind:checked={scope.agent_discoveries} onchange={markScopeDirty} />
      <span>
        <strong>Agent discoveries</strong>
        <small>Research Agent saved findings{#if preview} — {preview.agent_discoveries.toLocaleString()}{/if}</small>
      </span>
    </label>

    <label class="scope-option">
      <input type="checkbox" bind:checked={scope.gwas_discovery} onchange={markScopeDirty} />
      <span>
        <strong>GWAS discovery</strong>
        <small>
          Your rsIDs intersecting GWAS Catalog
          {#if preview}
            — {preview.gwas_discovery.toLocaleString()}
            {#if preview.gwas_genome_overlap > 0}
              <span class="overlap-hint">({preview.gwas_genome_overlap.toLocaleString()} overlap in genome)</span>
            {/if}
          {/if}
        </small>
      </span>
    </label>

    <label class="scope-option">
      <input type="checkbox" bind:checked={scope.non_reference} onchange={markScopeDirty} />
      <span>
        <strong>GWAS het variants</strong>
        <small>Heterozygous at GWAS loci (capped){#if preview} — {preview.non_reference.toLocaleString()}{/if}</small>
      </span>
    </label>
  </div>

  <div class="sources-section">
    <div class="sources-header">
      <h3 class="sources-title">Enrichment sources</h3>
      <div class="source-presets">
        <button type="button" class="preset-btn" onclick={() => applySourcePreset("fast")}>Fast index</button>
        <button type="button" class="preset-btn" onclick={() => applySourcePreset("clinical")}>Clinical</button>
        <button type="button" class="preset-btn" onclick={() => applySourcePreset("full")}>Full depth</button>
      </div>
    </div>
    <p class="sources-help">
      Local GWAS + embed always run. For 65k+ variants use <strong>Fast index</strong> first (minutes–hours).
      Clinical / Full depth can take days — add gnomAD or other sources later with Supplement missing only.
    </p>
    <div class="sources-grid">
      <label class="scope-option source-slow">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.gnomad}
          onchange={onSourceToggle}
        />
        <span>
          <strong>gnomAD</strong>
          <small>Population allele frequency — slow on remote tabix; skip for fast sweeps.</small>
        </span>
      </label>
      <label class="scope-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.clinvar_live}
          onchange={onSourceToggle}
        />
        <span>
          <strong>ClinVar (live)</strong>
          <small>NCBI E-utilities clinical significance.</small>
        </span>
      </label>
      <label class="scope-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.pubmed}
          onchange={onSourceToggle}
        />
        <span>
          <strong>PubMed</strong>
          <small>Trait-linked abstracts for narrative context.</small>
        </span>
      </label>
      <label class="scope-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.gtex}
          onchange={onSourceToggle}
        />
        <span>
          <strong>GTEx</strong>
          <small>Tissue eQTL associations.</small>
        </span>
      </label>
      <label class="scope-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.vep_dbsnp}
          onchange={onSourceToggle}
        />
        <span>
          <strong>VEP / dbSNP</strong>
          <small>Gene symbols and GRCh38 coordinates when missing.</small>
        </span>
      </label>
      <label class="scope-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.secondary}
          onchange={onSourceToggle}
        />
        <span>
          <strong>Secondary adapters</strong>
          <small>PGS Catalog, Reactome, Open Targets, PharmGKB, trait ontology.</small>
        </span>
      </label>
    </div>
    <label class="scope-option supplement-option">
      <input
        type="checkbox"
        bind:checked={scope.enrichment_sources!.supplement_missing}
        onchange={markScopeDirty}
      />
      <span>
        <strong>Supplement missing only</strong>
        <small>Re-run enabled sources on already-indexed variants that skipped them — use after a fast sweep to add gnomAD (or others) without re-embedding everything.</small>
      </span>
    </label>
  </div>

  <div class="limit-row">
    <label>
      GWAS cap
      <input type="number" min="100" max="100000" step="100" bind:value={scope.gwas_discovery_limit} onchange={markScopeDirty} />
    </label>
    <label>
      GWAS het cap
      <input type="number" min="100" max="100000" step="100" bind:value={scope.non_reference_limit} onchange={markScopeDirty} />
    </label>
  </div>

  {#if preview}
    <div class="preview-meta">
      <span>Genotypes in sample: {preview.genotype_total.toLocaleString()}</span>
      <span>GWAS reference rsIDs: {preview.gwas_reference_count.toLocaleString()}</span>
      <span>GWAS rsIDs in your genome: {preview.gwas_genome_overlap.toLocaleString()}</span>
      {#if preview.gwas_beyond_cap > 0}
        <span class="cap-warn">
          {preview.gwas_beyond_cap.toLocaleString()} GWAS overlap rsIDs not queued (cap limits to top by association count)
        </span>
      {/if}
      {#if isPreviewLoading}<span>Updating…</span>{/if}
    </div>
    {#if preview.gwas_beyond_cap > 0}
      <button type="button" class="btn btn-secondary btn-sm cap-preset-btn" onclick={queueFullGwasOverlap}>
        Queue full GWAS overlap ({Math.min(preview.gwas_genome_overlap, 100_000).toLocaleString()})
      </button>
    {/if}
  {/if}

  <div class="ref-row">
    <div class="ref-status">
      {#if refStatus}
        {#if gwasReady}
          <span class="ref-ok">Downloaded and indexed</span>
          <span>{refStatus.gwas_rsid_count.toLocaleString()} GWAS rsIDs in reference table</span>
          {#if preview && preview.gwas_genome_overlap === 0}
            <span class="ref-warn">Genome overlap is 0 — restart app after update to normalize rsID casing.</span>
          {/if}
        {:else}
          <span class="ref-missing">Not downloaded</span>
          <span>GWAS discovery scope requires a catalog sync first.</span>
        {/if}
      {:else}
        <ActivityPulse message="Loading reference status…" accent="#34d399" maxWidth="280px" />
      {/if}
    </div>
    <button
      class="btn btn-secondary btn-sm"
      onclick={handleSyncGwas}
      disabled={isSyncing}
    >
      {#if isSyncing}
        Syncing GWAS…
      {:else if gwasReady}
        {resyncConfirm ? "Confirm Re-sync" : "Re-sync GWAS Catalog"}
      {:else}
        Download GWAS Catalog
      {/if}
    </button>
  </div>

  {#if gnomadSourceEnabled}
    <GnomadSetupPanel
      onLog={onLog}
      onReadyChange={(ready) => onGnomadReadyChange?.(ready)}
    />
  {:else}
    <p class="gnomad-fast-note">gnomAD is off for this sweep — enable it above or use Supplement missing only to add frequency data later.</p>
  {/if}
</div>

<style src="../../styles/components/research-scope-section.css"></style>
