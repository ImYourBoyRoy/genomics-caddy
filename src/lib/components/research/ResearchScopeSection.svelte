<!-- ./src/lib/components/research/ResearchScopeSection.svelte -->
<script lang="ts">
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import GnomadSetupPanel from "./GnomadSetupPanel.svelte";
  import OfflineDataPanel from "./OfflineDataPanel.svelte";
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
    detectActivePreset,
    ensureEnrichmentSources,
    syncSweepFastFromSources,
  } from "../../utils/researchScopeHelpers";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    sweepRunning?: boolean;
    scope?: ResearchScopeConfig;
    preview?: ResearchScopePreview | null;
    previewLoading?: boolean;
    onScopeChange?: (scope: ResearchScopeConfig) => void;
    onLog?: (msg: string) => void;
    onGnomadReadyChange?: (ready: boolean) => void;
  }

  let {
    selectedSample = null,
    sweepRunning = false,
    scope = $bindable({ ...DEFAULT_RESEARCH_SCOPE }),
    preview = $bindable<ResearchScopePreview | null>(null),
    previewLoading = $bindable(false),
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
    if (sweepRunning) {
      // Never leave the parent readiness gate stuck on "Counting…" after a sweep starts.
      isPreviewLoading = false;
      previewLoading = false;
      return;
    }
    if (!selectedSample) {
      preview = null;
      previewLoading = false;
      return;
    }
    isPreviewLoading = true;
    previewLoading = true;
    try {
      preview = await previewResearchScope(selectedSample.id, scope);
    } catch (e: any) {
      preview = null;
      onLog?.(`Scope preview failed: ${e.message || String(e)}`);
    } finally {
      isPreviewLoading = false;
      previewLoading = false;
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
  const activePreset = $derived(detectActivePreset(scope));

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

<div class="glass-card scope-workbench">
  <div class="scope-workbench-header">
    <h2 class="card-title">Sweep Scopes</h2>
    {#if isPreviewLoading}
      <ActivityPulse message="Counting sweep scopes…" accent="#a78bfa" maxWidth="220px" />
    {:else if preview}
      <span class="scope-total">{preview.total_unique.toLocaleString()} queued</span>
    {/if}
  </div>

  <details class="scope-help">
    <summary>How sweep scopes work</summary>
    Counts update when you toggle scopes or finish a GWAS sync. The sweep processes the merged queue;
    Qdrant search uses indexed vectors after enrichment completes.
  </details>

  <div class="scope-workbench-grid" class:scope-locked={sweepRunning}>
    <section class="scope-subcard">
      <h3 class="subcard-title">Variant queue</h3>
      <div class="scope-grid scope-grid--queue">
        <label class="scope-option">
          <input type="checkbox" bind:checked={scope.curated} onchange={markScopeDirty} />
          <span>
            <strong>Curated markers</strong>
            <small>ClinVar + marker packs{#if preview} — {preview.curated.toLocaleString()}{/if}</small>
          </span>
        </label>

        <label class="scope-option">
          <input type="checkbox" bind:checked={scope.agent_discoveries} onchange={markScopeDirty} />
          <span>
            <strong>Agent discoveries</strong>
            <small>Saved Research Agent findings{#if preview} — {preview.agent_discoveries.toLocaleString()}{/if}</small>
          </span>
        </label>

        <label class="scope-option">
          <input type="checkbox" bind:checked={scope.gwas_discovery} onchange={markScopeDirty} />
          <span>
            <strong>GWAS discovery</strong>
            <small>
              rsIDs intersecting GWAS Catalog
              {#if preview}
                — {preview.gwas_discovery.toLocaleString()}
                {#if preview.gwas_genome_overlap > 0}
                  <span class="overlap-hint">({preview.gwas_genome_overlap.toLocaleString()} in genome)</span>
                {/if}
              {/if}
            </small>
          </span>
        </label>

        <label class="scope-option">
          <input type="checkbox" bind:checked={scope.non_reference} onchange={markScopeDirty} />
          <span>
            <strong>GWAS het variants</strong>
            <small>Heterozygous at GWAS loci{#if preview} — {preview.non_reference.toLocaleString()}{/if}</small>
          </span>
        </label>
      </div>
    </section>

    <section class="scope-subcard">
      <div class="sources-header">
        <h3 class="subcard-title">Enrichment sources</h3>
        <div class="source-presets" role="group" aria-label="Enrichment depth preset">
          <button
            type="button"
            class="preset-btn"
            class:active={activePreset === "fast"}
            disabled={sweepRunning}
            onclick={() => applySourcePreset("fast")}
          >
            Fast index
          </button>
          <button
            type="button"
            class="preset-btn"
            class:active={activePreset === "clinical"}
            disabled={sweepRunning}
            onclick={() => applySourcePreset("clinical")}
          >
            Clinical
          </button>
          <button
            type="button"
            class="preset-btn"
            class:active={activePreset === "full"}
            disabled={sweepRunning}
            onclick={() => applySourcePreset("full")}
          >
            Full depth
          </button>
        </div>
      </div>

      <div class="sources-grid">
        <label class="scope-option source-slow">
          <input
            type="checkbox"
            bind:checked={scope.enrichment_sources!.gnomad}
            disabled={sweepRunning}
            onchange={onSourceToggle}
          />
          <span>
            <strong>gnomAD</strong>
            <small>Population AF — slow on remote tabix.</small>
          </span>
        </label>
        <label class="scope-option">
          <input
            type="checkbox"
            bind:checked={scope.enrichment_sources!.clinvar_live}
            disabled={sweepRunning}
            onchange={onSourceToggle}
          />
          <span>
            <strong>ClinVar (live)</strong>
            <small>Clinical significance via NCBI.</small>
          </span>
        </label>
        <label class="scope-option">
          <input
            type="checkbox"
            bind:checked={scope.enrichment_sources!.pubmed}
            disabled={sweepRunning}
            onchange={onSourceToggle}
          />
          <span>
            <strong>PubMed</strong>
            <small>Trait-linked abstracts.</small>
          </span>
        </label>
        <label class="scope-option">
          <input
            type="checkbox"
            bind:checked={scope.enrichment_sources!.gtex}
            disabled={sweepRunning}
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
            disabled={sweepRunning}
            onchange={onSourceToggle}
          />
          <span>
            <strong>VEP / dbSNP</strong>
            <small>Gene symbols and coordinates.</small>
          </span>
        </label>
        <label class="scope-option">
          <input
            type="checkbox"
            bind:checked={scope.enrichment_sources!.secondary}
            disabled={sweepRunning}
            onchange={onSourceToggle}
          />
          <span>
            <strong>Secondary adapters</strong>
            <small>PGS, Reactome, Open Targets, PharmGKB.</small>
          </span>
        </label>
      </div>

      <label class="scope-option supplement-option">
        <input
          type="checkbox"
          bind:checked={scope.enrichment_sources!.supplement_missing}
          disabled={sweepRunning}
          onchange={markScopeDirty}
        />
        <span>
          <strong>Supplement missing only</strong>
          <small>Add sources to already-indexed variants without re-embedding everything.</small>
        </span>
      </label>
    </section>

    <section class="scope-subcard scope-subcard--full">
      <div class="scope-toolbar">
        <label class="limit-field">
          <span>GWAS cap</span>
          <input type="number" min="100" max="100000" step="100" bind:value={scope.gwas_discovery_limit} disabled={sweepRunning} onchange={markScopeDirty} />
        </label>
        <label class="limit-field">
          <span>GWAS het cap</span>
          <input type="number" min="100" max="100000" step="100" bind:value={scope.non_reference_limit} disabled={sweepRunning} onchange={markScopeDirty} />
        </label>

        <div class="ref-row">
          <div class="ref-status">
            {#if refStatus === null}
              <ActivityPulse message="Loading reference status…" accent="#34d399" maxWidth="220px" />
            {:else if gwasReady}
              <span class="ref-ok">GWAS catalog ready</span>
              <span>{refStatus.gwas_rsid_count.toLocaleString()} reference rsIDs</span>
            {:else}
              <span class="ref-missing">GWAS catalog not downloaded</span>
            {/if}
          </div>
          {#if refStatus !== null}
            {#if gwasReady}
              <button
                class="btn btn-secondary btn-sm ref-resync-btn"
                onclick={handleSyncGwas}
                disabled={sweepRunning || isSyncing}
                title="Replace local GWAS reference catalog"
              >
                {#if isSyncing}
                  Syncing…
                {:else if resyncConfirm}
                  Confirm re-sync
                {:else}
                  Re-sync GWAS
                {/if}
              </button>
            {:else}
              <button
                class="btn btn-secondary btn-sm"
                onclick={handleSyncGwas}
                disabled={sweepRunning || isSyncing}
              >
                {isSyncing ? "Syncing…" : "Download GWAS catalog"}
              </button>
            {/if}
          {/if}
        </div>
      </div>

      {#if preview}
        <div class="preview-meta">
          <span>Genotypes: {preview.genotype_total.toLocaleString()}</span>
          <span>GWAS ref: {preview.gwas_reference_count.toLocaleString()}</span>
          <span>Genome overlap: {preview.gwas_genome_overlap.toLocaleString()}</span>
          {#if preview.gwas_beyond_cap > 0}
            <span class="cap-warn">{preview.gwas_beyond_cap.toLocaleString()} beyond cap</span>
          {/if}
        </div>
        {#if preview.gwas_beyond_cap > 0}
          <button type="button" class="btn btn-secondary btn-sm cap-preset-btn" disabled={sweepRunning} onclick={queueFullGwasOverlap}>
            Queue full overlap ({Math.min(preview.gwas_genome_overlap, 100_000).toLocaleString()})
          </button>
        {/if}
      {/if}

      {#if gnomadSourceEnabled}
        <GnomadSetupPanel disabled={sweepRunning} onLog={onLog} onReadyChange={(ready) => onGnomadReadyChange?.(ready)} />
      {:else}
        <p class="gnomad-fast-note">gnomAD is off — enable above or use Supplement missing after a fast sweep.</p>
      {/if}

      <OfflineDataPanel {selectedSample} {onLog} disabled={sweepRunning} />
    </section>
  </div>
</div>
