<!-- ./src/lib/components/common/EmptyState.svelte -->
<script lang="ts">
  import type { AppPaths } from '../../types/genomics';
  import type { OfflineUpdateCheck } from '../../types/research';
  import { PRIMARY_CATALOG_IDS } from '../../utils/primaryCatalogs';
  import '$lib/styles/components/empty-state.css';

  /*
  Purpose: Welcome / onboarding screen when no genome profile is selected.
  Responsibilities:
  - Brand intro + primary CTAs (import genome, download reference DBs).
  - Optional path diagnostics for troubleshooting.
  */

  interface Props {
    appPaths: AppPaths | null;
    offlineStatus?: OfflineUpdateCheck | null;
    isChainDownloaded?: boolean;
    runtimeAvailable?: boolean;
    onImportGenome?: () => void;
    onDownloadDatabases?: () => void;
    onDownloadChain?: () => void;
  }

  let {
    appPaths,
    offlineStatus = null,
    isChainDownloaded = true,
    runtimeAvailable = true,
    onImportGenome,
    onDownloadDatabases,
    onDownloadChain,
  }: Props = $props();

  function assetReady(asset: { local_present: boolean; row_count: number; local_bytes: number }): boolean {
    if (!asset.local_present) return false;
    // Primary catalogs are ready only when SQLite has indexed rows.
    return asset.row_count > 0;
  }

  let catalogSummary = $derived.by(() => {
    if (!offlineStatus) return null;
    const byId = new Map<string, { local_present: boolean; row_count: number; local_bytes: number; update_available: boolean }>();
    for (const tier of offlineStatus.tiers) {
      for (const asset of tier.assets) {
        byId.set(asset.asset_id, asset);
      }
    }
    let ready = 0;
    let updates = 0;
    for (const id of PRIMARY_CATALOG_IDS) {
      const asset = byId.get(id);
      if (asset && assetReady(asset)) ready += 1;
      if (asset?.update_available) updates += 1;
    }
    return {
      ready,
      total: PRIMARY_CATALOG_IDS.length,
      missing: PRIMARY_CATALOG_IDS.length - ready,
      updates,
    };
  });
</script>

<div class="welcome-screen">
  <img src="/logo.png" alt="Genomics Caddy Logo" class="welcome-logo" />
  <h1>Genomics Caddy</h1>
  <p class="welcome-lead">
    Import your AncestryDNA or 23andMe export, then enrich it with local ClinVar, GWAS, PharmGKB, and dbSNP
    <em>rsID history</em> — all offline on your machine. Allele frequencies come from the local gnomAD cache when available (not from dbSNP).
  </p>

  <div class="welcome-cta-grid">
    <button type="button" class="welcome-cta primary" onclick={() => onImportGenome?.()} disabled={!runtimeAvailable}>
      <span class="welcome-cta-kicker">Step 1</span>
      <strong>Import genome file</strong>
      <span class="welcome-cta-hint">Browse a raw DNA export in the sidebar</span>
    </button>

    <button type="button" class="welcome-cta" onclick={() => onDownloadDatabases?.()} disabled={!runtimeAvailable}>
      <span class="welcome-cta-kicker">Step 2</span>
      <strong>Download reference catalogs</strong>
      <span class="welcome-cta-hint">
        {#if catalogSummary === null}
          GWAS, ClinVar, PharmGKB, and dbSNP (rsID merge/withdrawn history — not alleles/AF)
        {:else if catalogSummary.missing > 0}
          {catalogSummary.ready}/{catalogSummary.total} catalogs ready · {catalogSummary.missing} still missing
        {:else if catalogSummary.updates > 0}
          {catalogSummary.total}/{catalogSummary.total} ready · {catalogSummary.updates} update{catalogSummary.updates === 1 ? '' : 's'} available
        {:else}
          {catalogSummary.total}/{catalogSummary.total} catalogs ready — re-sync anytime
        {/if}
      </span>
    </button>

    <div class="welcome-cta welcome-cta-static" role="note">
      <span class="welcome-cta-kicker">Step 3</span>
      <strong>Allele frequencies (gnomAD)</strong>
      <span class="welcome-cta-hint">
        Report AF chips use the local gnomAD variant cache / VCF indexes from Vector Research — not the dbSNP merge map.
        Offline dbSNP is rsID history only; NCBI <code>refsnp-chr*</code> allele JSON is not ingested yet.
      </span>
    </div>

    {#if !isChainDownloaded}
      <button type="button" class="welcome-cta" onclick={() => onDownloadChain?.()} disabled={!runtimeAvailable}>
        <span class="welcome-cta-kicker">Optional</span>
        <strong>Download liftover chain</strong>
        <span class="welcome-cta-hint">Maps GRCh37 coordinates to GRCh38</span>
      </button>
    {/if}
  </div>

  <p class="welcome-aside">
    The left sidebar stays available for downloads, profiles, and progress while you work.
  </p>

  {#if !runtimeAvailable}
    <p class="welcome-runtime-note" role="note">
      This is a browser preview. Local DNA import, catalogs, and saved profiles are available in the desktop app.
    </p>
  {/if}

  {#if appPaths}
    <details class="app-paths-info">
      <summary>Storage paths</summary>
      <p><strong>Database</strong>: <code>{appPaths.db_path}</code></p>
      <p><strong>Liftover chain</strong>: <code>{appPaths.chain_path}</code></p>
    </details>
  {/if}
</div>
