<!-- ./src/lib/components/common/EmptyState.svelte -->
<script lang="ts">
  import type { AppPaths } from '../../types/genomics';
  import type { OfflineUpdateCheck } from '../../types/research';
  import { REFERENCE_CATALOG_IDS } from '../../utils/primaryCatalogs';
  import { countFreshInstalledUpdates } from '../../utils/offlineUpdates';
  import '$lib/styles/components/empty-state.css';

  /*
  Purpose: Welcome / onboarding screen when no genome profile is selected.
  Responsibilities:
  - Brand intro + ordered setup: install the coordinate map, download catalogs, then import DNA.
  - Optional path diagnostics for troubleshooting.
  */

  interface Props {
    appPaths: AppPaths | null;
    offlineStatus?: OfflineUpdateCheck | null;
    offlineStatusFresh?: boolean;
    isChainDownloaded?: boolean;
    runtimeAvailable?: boolean;
    onImportGenome?: () => void;
    onDownloadDatabases?: () => void;
    onDownloadChain?: () => void;
    onOpenHelp?: () => void;
  }

  let {
    appPaths,
    offlineStatus = null,
    offlineStatusFresh = false,
    isChainDownloaded = false,
    runtimeAvailable = true,
    onImportGenome,
    onDownloadDatabases,
    onDownloadChain,
    onOpenHelp,
  }: Props = $props();

  function assetReady(asset: { local_present: boolean; row_count: number }): boolean {
    if (!asset.local_present) return false;
    // A catalog is ready only when its local index contains rows.
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
    for (const id of REFERENCE_CATALOG_IDS) {
      const asset = byId.get(id);
      if (asset && assetReady(asset)) ready += 1;
    }
    // A persisted update flag is actionable only after a complete fresh probe.
    // Missing files remain part of the download/attention path instead.
    const updates = countFreshInstalledUpdates(offlineStatus, offlineStatusFresh, REFERENCE_CATALOG_IDS);
    return {
      ready,
      total: REFERENCE_CATALOG_IDS.length,
      missing: REFERENCE_CATALOG_IDS.length - ready,
      updates,
    };
  });
</script>

<div class="welcome-screen">
  <img src="/logo.png" alt="Genomics Caddy Logo" class="welcome-logo" />
  <h1>Genomics Caddy</h1>
  <p class="welcome-lead">
    Set up the coordinate map and reference catalogs, then import your AncestryDNA or 23andMe export. Standard reports run on this machine.
  </p>

  <div class="welcome-cta-grid">
    {#if !isChainDownloaded}
      <button type="button" class="welcome-cta primary" onclick={() => onDownloadChain?.()} disabled={!runtimeAvailable}>
        <span class="welcome-cta-kicker">Step 1 · Required</span>
        <strong>Install coordinate map</strong>
        <span class="welcome-cta-hint">Required before DNA files can be checked or analyzed. This maps genome positions; it does not change DNA calls.</span>
      </button>
    {:else}
      <div class="welcome-cta welcome-cta-static" role="status">
        <span class="welcome-cta-kicker">Step 1 · Complete</span>
        <strong>Coordinate map installed</strong>
        <span class="welcome-cta-hint">The coordinate requirement is met. Reference catalogs add broader research results.</span>
      </div>
    {/if}

    <button type="button" class="welcome-cta" class:primary={isChainDownloaded && (catalogSummary === null || catalogSummary.missing > 0)} onclick={() => onDownloadDatabases?.()} disabled={!runtimeAvailable}>
      <span class="welcome-cta-kicker">Step 2</span>
      <strong>Download all reference catalogs</strong>
      <span class="welcome-cta-hint">
        {#if catalogSummary === null}
          ClinVar, GWAS, ClinPGx, ClinGen, MANE, and dbSNP reference files
        {:else if catalogSummary.missing > 0}
          {catalogSummary.ready}/{catalogSummary.total} catalogs ready · {catalogSummary.missing} still missing
        {:else if catalogSummary.updates > 0}
          {catalogSummary.total}/{catalogSummary.total} ready · {catalogSummary.updates} update{catalogSummary.updates === 1 ? '' : 's'} available
        {:else}
          {catalogSummary.total}/{catalogSummary.total} catalogs ready — re-sync anytime
        {/if}
      </span>
    </button>

    <button type="button" class="welcome-cta" class:primary={isChainDownloaded && catalogSummary !== null && catalogSummary.missing === 0} onclick={() => onImportGenome?.()} disabled={!runtimeAvailable || !isChainDownloaded} title={!isChainDownloaded ? 'Install the coordinate map before importing DNA.' : undefined}>
      <span class="welcome-cta-kicker">Step 3</span>
      <strong>Import DNA file</strong>
      <span class="welcome-cta-hint">
        {#if !isChainDownloaded}
          Available after the required coordinate map is installed
        {:else}
          Choose a local .txt, .csv, .tsv, or .zip export
        {/if}
      </span>
    </button>
  </div>

  <p class="welcome-data-note" role="note">
    Reference catalogs download separately and can take time to install. The app shows each download and indexing status. Allele-frequency data is a separate optional gnomAD resource.
  </p>

  <div class="welcome-aside-row">
    <p class="welcome-aside">Use Data controls for downloads, profiles, and progress while you work.</p>
    <button type="button" class="welcome-help-link" onclick={() => onOpenHelp?.()}>
      <span class="welcome-help-icon" aria-hidden="true">i</span>
      <span>Help &amp; app info</span>
      <span class="welcome-help-arrow" aria-hidden="true">→</span>
    </button>
  </div>

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
