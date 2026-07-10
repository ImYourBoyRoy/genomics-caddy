<!-- ./src/lib/components/sidebar/Sidebar.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import GenomeImportPanel from '../import/GenomeImportPanel.svelte';
  import SampleList from '../samples/SampleList.svelte';
  import type { GenomeSample, GeneratedReport } from '../../types/genomics';
  import type { OfflineUpdateCheck, OfflineTierStatus, OfflineAssetStatus, OfflineSyncResult } from '../../types/research';
  import {
    getCustomDownloadDir,
    setCustomDownloadDir,
    selectDirectory,
    checkOfflineDataUpdates,
    syncSingleOfflineAsset,
    syncAllOfflineMissing,
    getOfflineReferenceStatus,
  } from '../../api/tauri';
  import type { ReferenceStatusDetails } from '../../api/tauri';

  /*
  Module Docstring:
  Purpose: Sidebar panel container aggregating assembly settings, file import, profiles, and reference DB downloader.
  Responsibilities:
  - Render branding header.
  - Render Liftover assembly status and download button.
  - Render collapsible Reference Databases pane with:
      • Per-asset download progress bars showing % and MB/s.
      • Smart button labels: Download / Update (force) / Re-sync.
      • Force-redownload checkbox that persists while expanded.
      • "Sync All Missing" bulk download button.
      • Error callout when a download fails.
      • Accurate file sizes from manifest display_size field.
  - Mount GenomeImportPanel and SampleList components.
  */

  interface Props {
    isChainDownloaded: boolean;
    isDownloadingChain: boolean;
    filePath: string;
    sampleNameInput: string;
    isImporting: boolean;
    progressPercent: number;
    progressStatus: string;
    importError: string;
    importSuccess: string;
    samples: GenomeSample[];
    selectedSample: GenomeSample | null;
    report: GeneratedReport | null;
    sweepRunning?: boolean;
    onDownloadChain: () => void;
    onBrowseFile: () => void;
    onImportGenome: (e: Event) => void;
    onSelectSample: (sample: GenomeSample) => void;
    onDeleteSample: (id: number) => void;
  }

  let {
    isChainDownloaded,
    isDownloadingChain,
    filePath = $bindable(),
    sampleNameInput = $bindable(),
    isImporting,
    progressPercent,
    progressStatus,
    importError,
    importSuccess,
    samples,
    selectedSample,
    report = null,
    sweepRunning = false,
    onDownloadChain,
    onBrowseFile,
    onImportGenome,
    onSelectSample,
    onDeleteSample,
  }: Props = $props();

  interface DownloadProgress {
    percent: number;       // 0–100, or -1 if total unknown
    bytesDone: number;
    totalBytes: number;
    speedMbps: number;
    startedAt: number;     // Date.now()
  }

  let customDir = $state<string | null>(null);
  let offlineStatus = $state<OfflineUpdateCheck | null>(null);
  let syncingAll = $state(false);
  let isPanelCollapsed = $state(true);
  let forceRedownload = $state(false);
  let syncErrors = $state<Record<string, string>>({});
  let syncMessages = $state<Record<string, string>>({});
  /** assetId -> in-progress download stats */
  let downloadProgress = $state<Record<string, DownloadProgress>>({});
  /** assetId -> currently syncing */
  let syncingAsset = $state<Record<string, boolean>>({});
  interface ImportProgressInfo {
    percent?: number;
    rows_processed: number;
    rows_per_second?: number;
    eta_seconds?: number | null;
    message: string;
  }
  let importProgress = $state<Record<string, ImportProgressInfo>>({});
  let isCheckingStatus = $state(false);
  let referenceDetails = $state<ReferenceStatusDetails | null>(null);

  // Derive active report rsids
  let reportRsids = $derived(
    report ? Object.keys(report.variants || {}) : []
  );

  async function refreshReferenceDetails() {
    isCheckingStatus = true;
    try {
      referenceDetails = await getOfflineReferenceStatus(
        reportRsids.length > 0 ? reportRsids : null
      );
    } catch (err) {
      console.error('Failed to get reference status details:', err);
    } finally {
      isCheckingStatus = false;
    }
  }

  // Refresh details when reportRsids changes, or when offlineStatus changes
  $effect(() => {
    // Register dependencies reactively
    const _ = reportRsids;
    const __ = offlineStatus;
    refreshReferenceDetails();
  });

  // Tauri event listener cleanup
  let unlistenProgress: (() => void) | null = null;
  let unlistenImport: (() => void) | null = null;

  async function loadSettingsAndStatus() {
    try {
      customDir = await getCustomDownloadDir();
      offlineStatus = await checkOfflineDataUpdates();
    } catch (err) {
      console.error('Failed to load custom download directory / offline status:', err);
    }
  }

  onMount(async () => {
    void loadSettingsAndStatus();

    // Subscribe to streaming progress events from the backend.
    unlistenProgress = await listen<{
      asset_id: string;
      label: string;
      bytes_downloaded: number;
      total_bytes: number;
    }>('offline:download_progress', (event) => {
      const { asset_id, bytes_downloaded, total_bytes } = event.payload;
      const prev = downloadProgress[asset_id];
      const now = Date.now();
      const startedAt = prev?.startedAt ?? now;
      const elapsedSec = Math.max((now - startedAt) / 1000, 0.1);
      const speedMbps = bytes_downloaded / elapsedSec / (1024 * 1024);
      const percent = total_bytes > 0 ? Math.min(Math.round((bytes_downloaded / total_bytes) * 100), 99) : -1;

      downloadProgress = {
        ...downloadProgress,
        [asset_id]: {
          percent,
          bytesDone: bytes_downloaded,
          totalBytes: total_bytes,
          speedMbps,
          startedAt,
        },
      };
    });

    unlistenImport = await listen<ImportProgressInfo & { asset_id: string }>(
      'offline:import_progress',
      (event) => {
        const { asset_id, ...payload } = event.payload;
        importProgress = {
          ...importProgress,
          [asset_id]: payload,
        };
      }
    );
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenImport?.();
  });

  async function handleBrowseDir() {
    try {
      const selected = await selectDirectory();
      if (selected) {
        await setCustomDownloadDir(selected);
        customDir = selected;
        offlineStatus = await checkOfflineDataUpdates();
      }
    } catch (err) {
      console.error('Browse directory failed:', err);
    }
  }

  async function handleResetDir() {
    try {
      await setCustomDownloadDir(null);
      customDir = null;
      offlineStatus = await checkOfflineDataUpdates();
    } catch (err) {
      console.error('Reset directory failed:', err);
    }
  }

  async function handleSyncAsset(assetId: string, force: boolean) {
    syncingAsset = { ...syncingAsset, [assetId]: true };
    // Clear stale progress/errors for this asset.
    const { [assetId]: _, ...restProgress } = downloadProgress;
    downloadProgress = { ...restProgress, [assetId]: { percent: 0, bytesDone: 0, totalBytes: 0, speedMbps: 0, startedAt: Date.now() } };
    const { [assetId]: _e, ...restErrors } = syncErrors;
    syncErrors = restErrors;
    const { [assetId]: _m, ...restMessages } = syncMessages;
    syncMessages = restMessages;

    try {
      const result = await syncSingleOfflineAsset(
        assetId,
        force,
        assetId === 'tier2_variant_locus' ? (selectedSample?.id ?? undefined) : undefined,
      );
      if (result.errors?.length) {
        syncErrors = { ...syncErrors, [assetId]: result.errors.join('\n') };
      } else if (result.messages?.length) {
        syncMessages = { ...syncMessages, [assetId]: result.messages.join('\n') };
      }
      offlineStatus = await checkOfflineDataUpdates();
    } catch (err) {
      syncErrors = { ...syncErrors, [assetId]: String(err) };
    } finally {
      syncingAsset = { ...syncingAsset, [assetId]: false };
      // Clear import progress
      const { [assetId]: _, ...restImport } = importProgress;
      importProgress = restImport;
      // Keep final progress bar at 100% briefly, then clear.
      if (downloadProgress[assetId]) {
        downloadProgress = {
          ...downloadProgress,
          [assetId]: { ...downloadProgress[assetId], percent: 100 },
        };
        setTimeout(() => {
          const { [assetId]: _, ...rest } = downloadProgress;
          downloadProgress = rest;
        }, 2000);
      }
    }
  }

  async function handleSyncAllMissing() {
    syncingAll = true;
    syncErrors = {};
    syncMessages = {};
    try {
      const results = await syncAllOfflineMissing(selectedSample?.id ?? undefined);
      const allErrors: string[] = [];
      results.forEach((r) => {
        if (r.errors?.length) allErrors.push(...r.errors);
      });
      if (allErrors.length) {
        syncErrors = { __all__: allErrors.join('\n') };
      }
      offlineStatus = await checkOfflineDataUpdates();
    } catch (err) {
      syncErrors = { __all__: String(err) };
    } finally {
      syncingAll = false;
      importProgress = {};
    }
  }

  function findAsset(tierNum: number, assetId: string): OfflineAssetStatus | null {
    if (!offlineStatus) return null;
    const tier = offlineStatus.tiers.find((t: OfflineTierStatus) => t.tier === tierNum);
    if (!tier) return null;
    return tier.assets.find((a: OfflineAssetStatus) => a.asset_id === assetId) || null;
  }

  function getAssetStatusLine(tierNum: number, assetId: string): string {
    const asset = findAsset(tierNum, assetId);
    // Only show "Checking…" before the first offline status payload arrives.
    // Do not flip every row back to Checking while a secondary detail refresh runs.
    if (!offlineStatus) return 'Checking…';
    if (!asset) return 'Unavailable';
    const prog = downloadProgress[assetId];
    if (prog && syncingAsset[assetId]) {
      const pct = prog.percent >= 0 ? `${prog.percent}%` : `${(prog.bytesDone / 1024 / 1024).toFixed(0)} MB`;
      return `${pct} · ${prog.speedMbps.toFixed(1)} MB/s`;
    }
    if (asset.local_present) {
      const mb = asset.local_bytes / 1024 / 1024;
      const sizeStr = mb >= 1000 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(0)} MB`;
      if (asset.row_count > 0) {
        return `${asset.row_count.toLocaleString()} rows (${sizeStr})`;
      }
      // Empty placeholder .db files should not look "downloaded".
      if (asset.local_bytes > 0 && asset.local_bytes < 64 * 1024 && asset.row_count === 0) {
        return 'Not downloaded';
      }
      return `${sizeStr} on disk`;
    }
    return asset.display_size ? `Not downloaded · ${asset.display_size}` : 'Not downloaded';
  }

  /** Return the button label/variant for an asset. */
  function assetButtonState(tierNum: number, assetId: string): {
    label: string;
    variant: 'primary' | 'secondary' | 'warning';
    isForce: boolean;
  } {
    if (syncingAsset[assetId]) return { label: 'Syncing…', variant: 'secondary', isForce: false };
    const asset = findAsset(tierNum, assetId);
    if (!offlineStatus || !asset) return { label: 'Download', variant: 'primary', isForce: false };

    if (!asset.local_present) {
      return { label: '⬇ Download', variant: 'primary', isForce: false };
    }
    if (asset.update_available || forceRedownload) {
      return { label: '🔄 Update', variant: 'warning', isForce: true };
    }
    return { label: '↺ Re-sync', variant: 'secondary', isForce: false };
  }

  const DB_DEFS = [
    {
      tierNum: 0, assetId: 'gwas_catalog', label: 'GWAS Catalog',
      tooltip: 'Genome-wide association studies catalog. Size: ~340 MB. Links traits to SNPs.',
    },
    {
      tierNum: 1, assetId: 'clinvar_variant_summary', label: 'ClinVar',
      tooltip: 'NCBI ClinVar variant annotations. Size: ~300–500 MB. Pathogenicity classifications.',
    },
    {
      tierNum: 1, assetId: 'pharmgkb_clinical_variants', label: 'PharmGKB & ClinGen',
      tooltip: 'Pharmacogenomics data, ClinGen gene validity, and MANE transcripts. Size: ~50 MB.',
    },
    {
      tierNum: 2, assetId: 'dbsnp_merged_json', label: 'dbSNP References',
      tooltip: 'dbSNP merged rsID metadata. Size: ~1–3 GB. Allele orientation and cross-build support.',
    },
  ] as const;

  let anyActive = $derived(syncingAll || Object.values(syncingAsset).some(Boolean));
</script>

<aside class="sidebar">
  <div class="brand">
    <img src="/logo.png" alt="Genomics Caddy Logo" class="brand-logo" />
    <h2>Genomics Caddy</h2>
  </div>

  <!-- Chain Status Indicator -->
  <div class="chain-status-card card">
    <h4>Liftover Assembly</h4>
    {#if isChainDownloaded}
      <div class="badge success">🟢 GRCh38 Active</div>
    {:else}
      <div class="badge warning">⚠️ GRCh37 Only</div>
      <p class="card-hint">Liftover chain file is missing. Import will not map to GRCh38 coordinates.</p>
      <button class="btn btn-primary btn-sm" onclick={onDownloadChain} disabled={isDownloadingChain || sweepRunning}>
        {isDownloadingChain ? 'Downloading…' : 'Download Chain'}
      </button>
    {/if}
  </div>

  <!-- Reference Databases Card -->
  <div class="chain-status-card card">
    <div
      class="card-header"
      onclick={() => isPanelCollapsed = !isPanelCollapsed}
      onkeydown={(e) => e.key === 'Enter' && (isPanelCollapsed = !isPanelCollapsed)}
      role="button"
      tabindex="0"
      style="cursor: pointer; display: flex; justify-content: space-between; align-items: center; user-select: none;"
    >
      <h4>Reference Databases</h4>
      <span style="font-size: 0.8rem; opacity: 0.7;">{isPanelCollapsed ? '▶' : '▼'}</span>
    </div>

    {#if !isPanelCollapsed}
      <div class="card-body" style="margin-top: 0.6rem; display: flex; flex-direction: column; gap: 0.75rem;">

        <!-- Custom Directory Row -->
        <div class="dir-setting">
          <span class="setting-label" style="font-size: 0.75rem; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; display: block; margin-bottom: 0.25rem;">Download Location</span>
          <div style="display: flex; gap: 0.4rem; align-items: center;">
            <input
              type="text"
              placeholder="Default App Directory"
              value={customDir || ''}
              readonly
              style="flex: 1; font-size: 0.75rem; padding: 6px 10px; border-radius: 6px; border: 1px solid var(--border-color); background: rgba(0, 0, 0, 0.4); color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;"
              title={customDir || 'Default App Directory'}
            />
            <button
              class="btn btn-secondary btn-sm"
              onclick={handleBrowseDir}
              style="min-height: auto; font-size: 0.75rem; padding: 6px 12px;"
            >
              Browse
            </button>
          </div>
          {#if customDir}
            <button
              onclick={handleResetDir}
              class="reset-dir-btn"
              style="font-size: 0.65rem; color: var(--danger); background: none; border: none; padding: 0; margin-top: 0.25rem; cursor: pointer; display: block; opacity: 0.85;"
            >
              Reset to Default
            </button>
          {/if}
        </div>

        <!-- Force re-download checkbox -->
        <label style="display: flex; align-items: center; gap: 0.4rem; font-size: 0.75rem; opacity: 0.85; cursor: pointer; user-select: none;">
          <input type="checkbox" bind:checked={forceRedownload} style="cursor: pointer;" />
          Force re-download existing files
        </label>

        <hr style="border: 0; border-top: 1px solid var(--border-color); margin: 0.1rem 0;" />

        <!-- Sync All Missing -->
        <button
          class="btn btn-primary btn-sm"
          onclick={handleSyncAllMissing}
          disabled={anyActive || syncingAll || !offlineStatus || isCheckingStatus || sweepRunning}
          style="font-size: 0.75rem; padding: 6px 12px; width: 100%;"
        >
          {syncingAll ? '⏳ Syncing All…' : '⬇️ Sync All Missing'}
        </button>

        {#if syncErrors.__all__}
          <div class="sync-error-block">
            <strong>⚠️ Sync errors:</strong>
            <pre style="white-space: pre-wrap; font-size: 0.65rem; margin-top: 0.25rem; opacity: 0.85;">{syncErrors.__all__}</pre>
          </div>
        {/if}

        <hr style="border: 0; border-top: 1px solid var(--border-color); margin: 0.1rem 0;" />

        <!-- Individual Databases list -->
        <div class="db-list" style="display: flex; flex-direction: column; gap: 0.75rem;">
          {#each DB_DEFS as db}
            {@const btnState = assetButtonState(db.tierNum, db.assetId)}
            {@const prog = downloadProgress[db.assetId]}
            {@const isActive = !!syncingAsset[db.assetId]}
            <div class="db-item-wrap">
              <div class="db-item" style="display: flex; justify-content: space-between; align-items: center; gap: 0.5rem;">
                <div style="display: flex; flex-direction: column; min-width: 0; flex: 1;">
                  <span style="font-size: 0.75rem; display: flex; align-items: center; gap: 0.25rem;">
                    <strong style="text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">{db.label}</strong>
                    <span class="info-icon" style="cursor: help; opacity: 0.6; font-size: 0.75rem;" title={db.tooltip}>ⓘ</span>
                  </span>
                  <span style="font-size: 0.65rem; opacity: 0.7; margin-top: 0.1rem;">
                    {getAssetStatusLine(db.tierNum, db.assetId)}
                  </span>
                </div>
                <button
                  class="btn btn-xs"
                  class:btn-primary={btnState.variant === 'primary'}
                  class:btn-secondary={btnState.variant === 'secondary'}
                  class:btn-warning={btnState.variant === 'warning'}
                  onclick={() => handleSyncAsset(db.assetId, btnState.isForce || forceRedownload)}
                  disabled={anyActive || !offlineStatus || isCheckingStatus || sweepRunning || (db.assetId === 'dbsnp_merged_json' && !selectedSample)}
                  style="font-size: 0.65rem; padding: 4px 8px; min-height: auto; min-width: 72px; white-space: nowrap;"
                >
                  {btnState.label}
                </button>
              </div>

              <!-- Per-asset progress bar (shown while downloading or importing) -->
              {#if isActive}
                {#if prog && !importProgress[db.assetId]}
                  <div class="progress-track">
                    <div
                      class="progress-fill"
                      style="width: {prog.percent >= 0 ? prog.percent + '%' : '100%'}; animation: {prog.percent < 0 ? 'indeterminate 1.4s ease infinite' : 'none'};"
                    ></div>
                  </div>
                  <div style="display: flex; justify-content: space-between; font-size: 0.6rem; opacity: 0.7; margin-top: 0.1rem;">
                    <span>{prog.percent >= 0 ? prog.percent + '%' : 'streaming…'}</span>
                    <span>{prog.speedMbps.toFixed(1)} MB/s</span>
                  </div>
                {/if}
                {#if importProgress[db.assetId]}
                  {@const imp = importProgress[db.assetId]}
                  <div class="progress-track" style="background: rgba(165, 180, 252, 0.15); margin-top: 0.25rem;">
                    <div
                      class="progress-fill"
                      style="width: {imp.percent !== undefined && imp.percent >= 0 ? imp.percent + '%' : '100%'}; background: linear-gradient(90deg, #818cf8, #a5b4fc); animation: {imp.percent === undefined || imp.percent < 0 ? 'indeterminate 1.4s ease infinite' : 'none'};"
                    ></div>
                  </div>
                  <div style="display: flex; justify-content: space-between; font-size: 0.6rem; color: #a5b4fc; margin-top: 0.1rem; font-family: var(--font-mono), monospace;">
                    <span>{imp.percent !== undefined && imp.percent >= 0 ? imp.percent + '%' : 'indexing…'}</span>
                    {#if imp.eta_seconds !== undefined && imp.eta_seconds !== null}
                      <span>{imp.eta_seconds}s remaining</span>
                    {/if}
                  </div>
                  <div style="font-size: 0.65rem; color: #a5b4fc; margin-top: 0.25rem; display: flex; align-items: center; gap: 0.25rem;">
                    <span class="import-dot"></span>
                    <span>{imp.message}</span>
                  </div>
                {/if}
              {/if}

              <!-- Per-asset error display -->
              {#if syncErrors[db.assetId]}
                <div class="sync-error-block" style="margin-top: 0.35rem;">
                  <strong>⚠️ Error:</strong>
                  <span style="font-size: 0.65rem; opacity: 0.85;">{syncErrors[db.assetId]}</span>
                </div>
              {/if}

              <!-- Per-asset success message -->
              {#if syncMessages[db.assetId] && !isActive}
                <div class="sync-ok-block" style="margin-top: 0.35rem;">
                  <span style="font-size: 0.65rem; opacity: 0.85;">✅ {syncMessages[db.assetId].split('\n')[0]}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>

        {#if referenceDetails}
          <div class="ref-status-details" style="margin-top: 0.5rem; padding: 0.5rem 0.6rem; background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; font-size: 0.7rem; display: flex; flex-direction: column; gap: 0.4rem;">
            <div style="font-weight: bold; font-size: 0.75rem; color: #a5b4fc; border-bottom: 1px solid rgba(255,255,255,0.08); padding-bottom: 0.2rem; margin-bottom: 0.1rem;">
              🗄️ Ingestion & In-Use Status
            </div>

            <!-- ClinVar Status Group -->
            <div class="status-group">
              <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 0.15rem;">ClinVar Annotations</div>
              <div style="display: grid; grid-template-columns: 1fr auto; gap: 0.25rem; opacity: 0.85; padding-left: 0.25rem;">
                <span>Raw file found:</span>
                <strong style="color: {referenceDetails.clinvar_raw_found ? '#34d399' : '#f87171'}">
                  {referenceDetails.clinvar_raw_found ? 'Yes' : 'No'}
                </strong>
                <span>Indexed rows:</span>
                <strong>{referenceDetails.clinvar_indexed_rows.toLocaleString()}</strong>
                {#if report}
                  <span>Report rsID hits:</span>
                  <strong style="color: {referenceDetails.clinvar_rsid_hits > 0 ? '#60a5fa' : '#9ca3af'}">
                    {referenceDetails.clinvar_rsid_hits.toLocaleString()}
                  </strong>
                {/if}
                {#if referenceDetails.clinvar_last_indexed}
                  <span>Last indexed:</span>
                  <strong>{new Date(referenceDetails.clinvar_last_indexed * 1000).toLocaleDateString()}</strong>
                {/if}
              </div>
            </div>

            <!-- dbSNP Status Group -->
            <div class="status-group" style="margin-top: 0.25rem;">
              <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 0.15rem;">dbSNP Normalizations</div>
              <div style="display: grid; grid-template-columns: 1fr auto; gap: 0.25rem; opacity: 0.85; padding-left: 0.25rem;">
                <span>Raw file found:</span>
                <strong style="color: {referenceDetails.dbsnp_merged_raw_found ? '#34d399' : '#f87171'}">
                  {referenceDetails.dbsnp_merged_raw_found ? 'Yes' : 'No'}
                </strong>
                <span>Merge mappings:</span>
                <strong>{referenceDetails.dbsnp_merge_mappings_indexed.toLocaleString()}</strong>
                {#if report}
                  <span>Report normalized:</span>
                  <strong style="color: {referenceDetails.dbsnp_rsids_normalized > 0 ? '#60a5fa' : '#9ca3af'}">
                    {referenceDetails.dbsnp_rsids_normalized.toLocaleString()}
                  </strong>
                {/if}
                <span>Placement index:</span>
                <strong style="color: {referenceDetails.dbsnp_placement_index_available ? '#34d399' : '#f87171'}">
                  {referenceDetails.dbsnp_placement_index_available ? 'Yes' : 'No'}
                </strong>
                <span>Orientation database:</span>
                <strong style="color: {referenceDetails.orientation_verification_available ? '#34d399' : '#f87171'}">
                  {referenceDetails.orientation_verification_available ? 'Yes' : 'No'}
                </strong>
              </div>
            </div>
          </div>
        {/if}

      </div>
    {/if}
  </div>

  <!-- Import DNA Form -->
  <GenomeImportPanel
    bind:filePath
    bind:sampleNameInput
    {isImporting}
    {progressPercent}
    {progressStatus}
    {importError}
    {importSuccess}
    disabled={sweepRunning}
    {onBrowseFile}
    {onImportGenome}
  />

  <!-- Active Profiles -->
  <SampleList
    {samples}
    {selectedSample}
    disabled={sweepRunning}
    {onSelectSample}
    {onDeleteSample}
  />
</aside>

<style>
  .sync-error-block {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
    font-size: 0.68rem;
    color: #fca5a5;
  }

  .sync-ok-block {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.25);
    border-radius: 4px;
    padding: 0.25rem 0.5rem;
    color: #86efac;
  }

  .progress-track {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 0.35rem;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #60a5fa, #818cf8);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  :global(.btn-warning) {
    background: rgba(217, 119, 6, 0.18) !important;
    border-color: rgba(251, 191, 36, 0.4) !important;
    color: #fbbf24 !important;
  }
  :global(.btn-warning:hover) {
    background: rgba(217, 119, 6, 0.3) !important;
  }

  @keyframes indeterminate {
    0% { transform: translateX(-100%); width: 60%; }
    100% { transform: translateX(200%); width: 60%; }
  }

  .reset-dir-btn {
    transition: opacity 0.2s ease;
  }
  .reset-dir-btn:hover {
    opacity: 1 !important;
  }

  .import-dot {
    width: 6px;
    height: 6px;
    background-color: #818cf8;
    border-radius: 50%;
    display: inline-block;
    box-shadow: 0 0 8px #818cf8;
    animation: import-pulse-dot 1.2s ease-in-out infinite;
  }

  @keyframes import-pulse-dot {
    0%, 100% { transform: scale(0.85); opacity: 0.6; }
    50% { transform: scale(1.15); opacity: 1; }
  }
</style>
