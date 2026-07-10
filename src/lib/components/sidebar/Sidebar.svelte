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
    cancelOfflineImport,
    exportDiscoveryFindings,
  } from '../../api/tauri';
  import type { ReferenceStatusDetails } from '../../api/tauri';
  import { PRIMARY_CATALOG_IDS } from '../../utils/primaryCatalogs';
  import '$lib/styles/components/sidebar.css';

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
    /** Expand the Reference Databases panel (e.g. from main CTA). */
    expandDatabases?: boolean;
    onReady?: (api: {
      syncAllMissing: () => void;
      expandDatabases: () => void;
    }) => void;
    onOfflineStatusChange?: (status: OfflineUpdateCheck | null) => void;
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
    expandDatabases = false,
    onReady,
    onOfflineStatusChange,
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
  interface SyncPhaseInfo {
    phase: string;
    step: number;
    total_steps: number;
    message: string;
  }
  let syncPhase = $state<Record<string, SyncPhaseInfo>>({});
  let isCheckingStatus = $state(false);
  let referenceDetails = $state<ReferenceStatusDetails | null>(null);

  $effect(() => {
    if (expandDatabases) {
      isPanelCollapsed = false;
    }
  });

  // Derive active report rsids
  let reportRsids = $derived(
    report ? Object.keys(report.variants || {}) : []
  );

  async function refreshReferenceDetails() {
    // Detail enrichment only — never gate Download buttons on this.
    try {
      referenceDetails = await getOfflineReferenceStatus(
        reportRsids.length > 0 ? reportRsids : null
      );
    } catch (err) {
      console.error('Failed to get reference status details:', err);
    }
  }

  // Refresh details when report rsIDs or offline inventory changes.
  // Keep this off the download enable/disable path.
  $effect(() => {
    const _ = reportRsids;
    const __ = offlineStatus;
    if (offlineStatus) {
      void refreshReferenceDetails();
    }
  });

  // Tauri event listener cleanup
  let unlistenProgress: (() => void) | null = null;
  let unlistenImport: (() => void) | null = null;
  let unlistenPhase: (() => void) | null = null;

  async function loadSettingsAndStatus() {
    isCheckingStatus = true;
    try {
      customDir = await getCustomDownloadDir();
      offlineStatus = await checkOfflineDataUpdates();
    } catch (err) {
      console.error('Failed to load custom download directory / offline status:', err);
      offlineStatus = null;
    } finally {
      isCheckingStatus = false;
    }
  }

  /** Refresh inventory without blocking UI; never await inside sync finally. */
  function refreshStatusInBackground() {
    void checkOfflineDataUpdates()
      .then((status) => {
        offlineStatus = status;
      })
      .catch((err) => {
        console.error('Background offline status refresh failed:', err);
      });
  }

  function clearAssetProgress(assetId: string) {
    const { [assetId]: _d, ...restD } = downloadProgress;
    downloadProgress = restD;
    const { [assetId]: _i, ...restI } = importProgress;
    importProgress = restI;
    const { [assetId]: _p, ...restP } = syncPhase;
    syncPhase = restP;
  }

  async function handleSyncAllMissing() {
    if (syncingAll || Object.values(syncingAsset).some(Boolean)) return;
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
    } catch (err) {
      syncErrors = { __all__: String(err) };
    } finally {
      syncingAll = false;
      importProgress = {};
      syncPhase = {};
      downloadProgress = {};
      refreshStatusInBackground();
    }
  }

  /** Public entry for main-dashboard CTA. */
  function startSyncAllMissing() {
    isPanelCollapsed = false;
    void handleSyncAllMissing();
  }

  onMount(async () => {
    onReady?.({
      syncAllMissing: startSyncAllMissing,
      expandDatabases: () => {
        isPanelCollapsed = false;
      },
    });

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
        // Clear download bar once import starts.
        if (downloadProgress[asset_id]) {
          const { [asset_id]: _, ...rest } = downloadProgress;
          downloadProgress = rest;
        }
      }
    );

    unlistenPhase = await listen<{
      asset_id: string;
      phase: string;
      step: number;
      total_steps: number;
      message: string;
    }>('offline:sync_phase', (event) => {
      const { asset_id, phase, step, total_steps, message } = event.payload;
      syncPhase = {
        ...syncPhase,
        [asset_id]: { phase, step, total_steps, message },
      };
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenImport?.();
    unlistenPhase?.();
  });

  async function handleBrowseDir() {
    try {
      const selected = await selectDirectory();
      if (selected) {
        await setCustomDownloadDir(selected);
        customDir = selected;
        refreshStatusInBackground();
      }
    } catch (err) {
      console.error('Browse directory failed:', err);
    }
  }

  async function handleResetDir() {
    try {
      await setCustomDownloadDir(null);
      customDir = null;
      refreshStatusInBackground();
    } catch (err) {
      console.error('Reset directory failed:', err);
    }
  }

  async function handleSyncAsset(assetId: string, force: boolean) {
    if (syncingAsset[assetId] || syncingAll) return;
    syncingAsset = { ...syncingAsset, [assetId]: true };
    clearAssetProgress(assetId);
    const { [assetId]: _e, ...restErrors } = syncErrors;
    syncErrors = restErrors;
    const { [assetId]: _m, ...restMessages } = syncMessages;
    syncMessages = restMessages;
    syncPhase = {
      ...syncPhase,
      [assetId]: {
        phase: 'prepare',
        step: 1,
        total_steps: 2,
        message: 'Step 1/2 · Starting…',
      },
    };

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
    } catch (err) {
      syncErrors = { ...syncErrors, [assetId]: String(err) };
    } finally {
      // Clear busy state FIRST so other Download buttons unlock immediately.
      syncingAsset = { ...syncingAsset, [assetId]: false };
      clearAssetProgress(assetId);
      // Inventory refresh is background-only — never block the Syncing… clear.
      refreshStatusInBackground();
    }
  }

  $effect(() => {
    onOfflineStatusChange?.(offlineStatus);
  });

  function findAsset(tierNum: number, assetId: string): OfflineAssetStatus | null {
    if (!offlineStatus) return null;
    const tier = offlineStatus.tiers.find((t: OfflineTierStatus) => t.tier === tierNum);
    if (!tier) return null;
    return tier.assets.find((a: OfflineAssetStatus) => a.asset_id === assetId) || null;
  }

  function formatByteSize(n: number, approx = false): string {
    const prefix = approx ? '~' : '';
    const abs = Math.abs(n);
    if (abs >= 1024 * 1024 * 1024) return `${prefix}${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
    if (abs >= 1024 * 1024) return `${prefix}${(n / (1024 * 1024)).toFixed(2)} MB`;
    if (abs >= 1024) return `${prefix}${(n / 1024).toFixed(2)} KB`;
    return `${prefix}${Math.round(n)} B`;
  }

  function formatRemoteSize(n: number): string {
    return formatByteSize(n, true);
  }

  function getAssetStatusLine(tierNum: number, assetId: string): string {
    const asset = findAsset(tierNum, assetId);
    if (!offlineStatus) return 'Checking…';
    if (!asset) return 'Unavailable';
    const phase = syncPhase[assetId];
    const imp = importProgress[assetId];
    if (imp && syncingAsset[assetId]) {
      const step = phase ? `Step ${phase.step}/${phase.total_steps} · ` : '';
      const pct = imp.percent !== undefined && imp.percent >= 0 ? `${imp.percent}%` : 'indexing…';
      return `${step}${pct} · ${imp.message || 'Importing…'}`;
    }
    const prog = downloadProgress[assetId];
    if (prog && syncingAsset[assetId]) {
      const step = phase ? `Step ${phase.step}/${phase.total_steps} · ` : 'Step 1/2 · ';
      const pct = prog.percent >= 0 ? `${prog.percent}%` : formatByteSize(prog.bytesDone);
      return `${step}${pct} · ${prog.speedMbps.toFixed(2)} MB/s`;
    }
    if (syncingAsset[assetId]) {
      return phase?.message || 'Preparing sync…';
    }
    if (asset.update_available) {
      const remote = asset.remote_content_length
        ? formatRemoteSize(asset.remote_content_length)
        : asset.display_size;
      return `Update available${remote ? ` · ${remote}` : ''}`;
    }
    if (asset.local_present) {
      const sizeStr = formatByteSize(asset.local_bytes);
      if (asset.row_count > 0) {
        return `${asset.row_count.toLocaleString()} rows (${sizeStr})`;
      }
      if (asset.local_bytes > 0 && asset.local_bytes < 64 * 1024 && asset.row_count === 0) {
        return 'Not downloaded';
      }
      if (asset.row_count === 0 && asset.local_bytes > 0) {
        return `Downloaded · not indexed yet (${sizeStr})`;
      }
      return `${sizeStr} on disk`;
    }
    const remoteHint = asset.remote_content_length
      ? formatRemoteSize(asset.remote_content_length)
      : asset.display_size;
    return remoteHint ? `Not downloaded · ${remoteHint}` : 'Not downloaded';
  }

  function assetTooltip(db: (typeof DB_DEFS)[number]): string {
    const asset = findAsset(db.tierNum, db.assetId);
    const size = asset?.remote_content_length
      ? formatRemoteSize(asset.remote_content_length)
      : asset?.display_size || 'size unknown';
    const update = asset?.update_available ? ' Update available on server.' : '';
    return `${db.blurb} Size: ${size}.${update}`;
  }

  /** Return the button label/variant for an asset.
   * Update = newer remote file proven (ETag/Last-Modified).
   * Re-sync = re-import local file into SQLite (no remote update).
   * Force checkbox alone does not relabel as Update.
   */
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
    if (asset.update_available) {
      // Only show Update when the server has a newer identity (not merely force-checked).
      return { label: '🔄 Update', variant: 'warning', isForce: true };
    }
    if (forceRedownload) {
      return { label: '⬇ Re-download', variant: 'warning', isForce: true };
    }
    return { label: '↺ Re-sync', variant: 'secondary', isForce: false };
  }

  const DB_DEFS = [
    {
      tierNum: 0, assetId: 'gwas_catalog', label: 'GWAS Catalog',
      blurb: 'Genome-wide association studies catalog. Links traits to SNPs.',
    },
    {
      tierNum: 1, assetId: 'clinvar_variant_summary', label: 'ClinVar',
      blurb: 'NCBI ClinVar variant annotations. Pathogenicity classifications.',
    },
    {
      tierNum: 1, assetId: 'pharmgkb_clinical_variants', label: 'PharmGKB & ClinGen',
      blurb: 'Pharmacogenomics data, ClinGen gene validity, and MANE transcripts.',
    },
    {
      tierNum: 2, assetId: 'dbsnp_merged_json', label: 'dbSNP References',
      blurb: 'NCBI RefSNP merge + withdrawn map (old rsIDs → current). Not allele frequencies — those come from gnomAD. Large JSON → compact SQLite alias table is expected.',
    },
  ] as const;

  let updatesAvailable = $derived.by(() => {
    if (!offlineStatus) return 0;
    let n = 0;
    for (const tier of offlineStatus.tiers) {
      for (const asset of tier.assets) {
        if (
          (PRIMARY_CATALOG_IDS as readonly string[]).includes(asset.asset_id) &&
          asset.update_available
        ) {
          n += 1;
        }
      }
    }
    return n;
  });

  /** Catalogs with no local file yet (not "downloaded but not indexed"). */
  let notDownloadedPrimary = $derived.by(() => {
    if (!offlineStatus) return [] as string[];
    const byId = new Map<string, OfflineAssetStatus>();
    for (const tier of offlineStatus.tiers) {
      for (const asset of tier.assets) byId.set(asset.asset_id, asset);
    }
    const labels: Record<string, string> = {
      gwas_catalog: 'GWAS',
      clinvar_variant_summary: 'ClinVar',
      pharmgkb_clinical_variants: 'PharmGKB',
      dbsnp_merged_json: 'dbSNP',
    };
    const out: string[] = [];
    for (const id of PRIMARY_CATALOG_IDS) {
      const asset = byId.get(id);
      if (!asset?.local_present) out.push(labels[id] ?? id);
    }
    return out;
  });

  /** Local file present but SQLite has 0 indexed rows. */
  let notIndexedPrimary = $derived.by(() => {
    if (!offlineStatus) return [] as string[];
    const byId = new Map<string, OfflineAssetStatus>();
    for (const tier of offlineStatus.tiers) {
      for (const asset of tier.assets) byId.set(asset.asset_id, asset);
    }
    const labels: Record<string, string> = {
      gwas_catalog: 'GWAS',
      clinvar_variant_summary: 'ClinVar',
      pharmgkb_clinical_variants: 'PharmGKB',
      dbsnp_merged_json: 'dbSNP',
    };
    const out: string[] = [];
    for (const id of PRIMARY_CATALOG_IDS) {
      const asset = byId.get(id);
      if (!asset?.local_present) continue;
      if (asset.row_count === 0) out.push(labels[id] ?? id);
    }
    return out;
  });

  let missingPrimaryCount = $derived(notDownloadedPrimary.length + notIndexedPrimary.length);

  let importingAny = $derived(Object.values(syncingAsset).some(Boolean) || syncingAll);
  let exportBusy = $state(false);
  let exportMessage = $state('');

  async function handleCancelImport() {
    try {
      await cancelOfflineImport();
    } catch (err) {
      console.error('Cancel import failed:', err);
    }
  }

  async function handleExportDiscovery() {
    if (!selectedSample?.id || exportBusy) return;
    exportBusy = true;
    exportMessage = '';
    try {
      const result = await exportDiscoveryFindings(selectedSample.id);
      exportMessage = `Exported ${result.findings_beyond_packs.toLocaleString()} beyond-pack hits · ${result.findings_in_packs.toLocaleString()} in packs → App/Data/exports/`;
    } catch (err) {
      exportMessage = String(err);
    } finally {
      exportBusy = false;
    }
  }

  let needsAttention = $derived(updatesAvailable > 0 || missingPrimaryCount > 0);

  // Only block the same asset or bulk sync — allow parallel downloads of other DBs.
  let bulkBusy = $derived(syncingAll);
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
      <h4 style="display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap;">
        Reference Databases
        {#if missingPrimaryCount > 0}
          <span
            class="missing-pill"
            title="{[
              notDownloadedPrimary.length ? `${notDownloadedPrimary.join(', ')} not downloaded` : '',
              notIndexedPrimary.length ? `${notIndexedPrimary.join(', ')} downloaded but not indexed` : '',
            ].filter(Boolean).join(' · ')}"
          >{missingPrimaryCount} need attention</span>
        {/if}
        {#if updatesAvailable > 0}
          <span class="update-pill" title="{updatesAvailable} database update(s) available">{updatesAvailable} update{updatesAvailable === 1 ? '' : 's'}</span>
        {/if}
      </h4>
      <span style="font-size: 0.8rem; opacity: 0.7;">{isPanelCollapsed ? '▶' : '▼'}</span>
    </div>

    {#if !isPanelCollapsed}
      <div class="card-body" style="margin-top: 0.6rem; display: flex; flex-direction: column; gap: 0.75rem;">

        {#if needsAttention || importingAny}
          <div class="db-attention-banner">
            {#if importingAny}
              <div>Import in progress — other catalogs may wait for a SQLite slot (downloads can still run in parallel).</div>
            {/if}
            {#if notDownloadedPrimary.length > 0}
              <div><strong>Not downloaded:</strong> {notDownloadedPrimary.join(', ')}.</div>
            {/if}
            {#if notIndexedPrimary.length > 0}
              <div><strong>Downloaded but not indexed:</strong> {notIndexedPrimary.join(', ')} — use Re-sync (or wait if import is already running).</div>
            {/if}
            {#if updatesAvailable > 0}
              <div><strong>{updatesAvailable}</strong> newer remote file{updatesAvailable === 1 ? '' : 's'} available — use Update to re-download. Re-sync only re-imports the local file.</div>
            {/if}
          </div>
        {/if}

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
          disabled={bulkBusy || Object.values(syncingAsset).some(Boolean) || !offlineStatus || sweepRunning}
          style="font-size: 0.75rem; padding: 6px 12px; width: 100%;"
        >
          {syncingAll ? '⏳ Syncing All…' : '⬇️ Sync All Missing'}
        </button>

        {#if importingAny}
          <button
            class="btn btn-secondary btn-sm"
            onclick={handleCancelImport}
            style="font-size: 0.75rem; padding: 6px 12px; width: 100%;"
          >
            ⏹ Cancel import
          </button>
        {/if}

        <button
          class="btn btn-secondary btn-sm"
          onclick={handleExportDiscovery}
          disabled={!selectedSample || exportBusy || sweepRunning}
          style="font-size: 0.75rem; padding: 6px 12px; width: 100%;"
          title="Writes marker_pack_coverage + genome_catalog_findings JSON under App/Data/exports/"
        >
          {exportBusy ? 'Exporting…' : '📤 Export pack vs genome findings'}
        </button>
        {#if exportMessage}
          <div style="font-size: 0.65rem; opacity: 0.85; line-height: 1.35;">{exportMessage}</div>
        {/if}

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
                    {#if findAsset(db.tierNum, db.assetId)?.update_available}
                      <span class="update-pill" title="Newer file available on server">update</span>
                    {/if}
                    <span class="info-icon" style="cursor: help; opacity: 0.6; font-size: 0.75rem;" title={assetTooltip(db)}>ⓘ</span>
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
                  disabled={!!syncingAsset[db.assetId] || bulkBusy || !offlineStatus || sweepRunning}
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
                {:else if importProgress[db.assetId]}
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
                {:else}
                  <div class="progress-track" style="margin-top: 0.25rem;">
                    <div class="progress-fill" style="width: 100%; animation: indeterminate 1.4s ease infinite;"></div>
                  </div>
                  <div style="font-size: 0.6rem; opacity: 0.7; margin-top: 0.1rem;">
                    Preparing import (no download needed)…
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
              Catalog readiness
            </div>

            <div class="status-group">
              <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 0.15rem;">ClinVar</div>
              <div style="display: grid; grid-template-columns: 1fr auto; gap: 0.25rem; opacity: 0.85; padding-left: 0.25rem;">
                <span>Status:</span>
                <strong style="color: {referenceDetails.clinvar_indexed_rows > 0 ? '#34d399' : referenceDetails.clinvar_raw_found ? '#fbbf24' : '#f87171'}">
                  {#if referenceDetails.clinvar_indexed_rows > 0}
                    Ready · {referenceDetails.clinvar_indexed_rows.toLocaleString()} rows
                  {:else if referenceDetails.clinvar_raw_found}
                    Downloaded · not indexed
                  {:else}
                    Not downloaded
                  {/if}
                </strong>
                {#if report}
                  <span>Hits in current report:</span>
                  <strong style="color: {referenceDetails.clinvar_rsid_hits > 0 ? '#60a5fa' : '#9ca3af'}">
                    {referenceDetails.clinvar_rsid_hits.toLocaleString()}
                  </strong>
                {/if}
                {#if referenceDetails.clinvar_last_indexed}
                  <span>Last indexed:</span>
                  <strong>{new Date(referenceDetails.clinvar_last_indexed * 1000).toLocaleDateString()}</strong>
                {/if}
              </div>
              {#if referenceDetails.clinvar_raw_found && referenceDetails.clinvar_indexed_rows === 0}
                <div class="status-warn">Raw ClinVar file is on disk but SQLite has 0 rows — use Re-sync to import (Update only if a newer remote file exists).</div>
              {/if}
            </div>

            <div class="status-group" style="margin-top: 0.25rem;">
              <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 0.15rem;">dbSNP (rsID merge map)</div>
              <div style="display: grid; grid-template-columns: 1fr auto; gap: 0.25rem; opacity: 0.85; padding-left: 0.25rem;">
                <span>Status:</span>
                <strong style="color: {referenceDetails.dbsnp_merge_mappings_indexed > 0 ? '#34d399' : referenceDetails.dbsnp_merged_raw_found ? '#fbbf24' : '#f87171'}">
                  {#if referenceDetails.dbsnp_merge_mappings_indexed > 0}
                    Ready · {referenceDetails.dbsnp_merge_mappings_indexed.toLocaleString()} mappings
                  {:else if referenceDetails.dbsnp_merged_raw_found}
                    Downloaded · not indexed
                  {:else}
                    Not downloaded
                  {/if}
                </strong>
                {#if report}
                  <span>rsIDs remapped in report:</span>
                  <strong style="color: {referenceDetails.dbsnp_rsids_normalized > 0 ? '#60a5fa' : '#9ca3af'}">
                    {referenceDetails.dbsnp_rsids_normalized.toLocaleString()}
                  </strong>
                {/if}
              </div>
              {#if referenceDetails.dbsnp_merged_raw_found && referenceDetails.dbsnp_merge_mappings_indexed === 0}
                <div class="status-warn">Archive on disk but 0 merge mappings indexed — import still needed (large job).</div>
              {/if}
              <div class="status-note">Offline dbSNP here is the NCBI <em>refsnp-merged</em> + withdrawn catalog (rsID merge map + dates/citations). Alleles, placements, and AF live in the huge per-chromosome <em>refsnp-chr*.json</em> files — we do not ingest those yet. Report AF chips use the local gnomAD cache when present (not this merge DB).</div>
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

<!-- Keep an empty style block so Vite/Svelte HMR does not request a stale
     virtual CSS module after styles were moved to sidebar.css. -->
<style>
</style>
