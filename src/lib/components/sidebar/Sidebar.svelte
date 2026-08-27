<!-- ./src/lib/components/sidebar/Sidebar.svelte -->
<script lang="ts">
import { onMount, onDestroy } from 'svelte';
  import { isTauri } from '@tauri-apps/api/core';
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
    getGnomadReadiness,
    downloadGnomadIndexes,
  } from '../../api/tauri';
  import type { ReferenceStatusDetails } from '../../api/tauri';
  import type { GnomadReadinessStatus } from '../../types/research';
  import { PRIMARY_CATALOG_IDS } from '../../utils/primaryCatalogs';
  import { clearOfflineUpdate, formatUpdateSummary, listOfflineUpdates } from '../../utils/offlineUpdates';
  import ActivityPulse from '../common/loading/ActivityPulse.svelte';
  import Tooltip from '../common/Tooltip.svelte';
  import ProgressTrack from './ProgressTrack.svelte';
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
    onOpenConnections?: () => void;
    onResourcesUpdated?: () => void | Promise<void>;
    runtimeAvailable?: boolean;
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
    onOpenConnections,
    onResourcesUpdated,
    runtimeAvailable = true,
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
  /** Latest bulk Sync All status line (from `__bulk__` or active asset phases). */
  let bulkSyncMessage = $state('');
  let bulkActiveAssetId = $state<string | null>(null);
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
  type ResourceUpdatePhase = 'idle' | 'checking' | 'available' | 'downloading' | 'validating' | 'installed' | 'reloading' | 'ready' | 'error';
  let updatePhase = $state<ResourceUpdatePhase>('idle');
  let updateMessage = $state('');
  let lastSuccessfulUpdateAt = $state<number | null>(null);
  let referenceDetails = $state<ReferenceStatusDetails | null>(null);
  let gnomadReadiness = $state<GnomadReadinessStatus | null>(null);
  let gnomadBusy = $state(false);
  let gnomadHint = $state('');

  function updatePhaseLabel(phase: ResourceUpdatePhase): string {
    switch (phase) {
      case 'checking': return 'Checking resources';
      case 'available': return 'Updates available';
      case 'downloading': return 'Downloading resources';
      case 'validating': return 'Validating local resources';
      case 'installed': return 'Resources installed';
      case 'reloading': return 'Reloading report';
      case 'ready': return 'Resources ready';
      case 'error': return 'Update error';
      default: return '';
    }
  }

  function setUpdateState(phase: ResourceUpdatePhase, message = '') {
    updatePhase = phase;
    updateMessage = message;
  }

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
    try {
      gnomadReadiness = await getGnomadReadiness();
    } catch (err) {
      console.warn('Failed to get gnomAD readiness:', err);
      gnomadReadiness = null;
    }
  }

  async function handleDownloadGnomadIndexes() {
    if (gnomadBusy) return;
    gnomadBusy = true;
    gnomadHint = 'Downloading gnomAD index files…';
    try {
      const result = await downloadGnomadIndexes();
      gnomadHint = result.message || `Downloaded ${result.downloaded}, skipped ${result.skipped}`;
      gnomadReadiness = await getGnomadReadiness();
    } catch (e: unknown) {
      gnomadHint = String(e);
    } finally {
      gnomadBusy = false;
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
  let statusRefreshGeneration = 0;

  async function loadSettingsAndStatus() {
    const generation = ++statusRefreshGeneration;
    isCheckingStatus = true;
    setUpdateState('checking', 'Checking local inventory and remote identities…');
    try {
      customDir = await getCustomDownloadDir();
      const status = await checkOfflineDataUpdates();
      if (generation !== statusRefreshGeneration) return;
      offlineStatus = status;
      const pendingCount = listOfflineUpdates(offlineStatus).length;
      setUpdateState(
        pendingCount > 0 ? 'available' : 'ready',
        pendingCount > 0
          ? `${pendingCount} newer remote resource version(s) available.`
          : 'Local resources are current.',
      );
    } catch (err) {
      if (generation !== statusRefreshGeneration) return;
      console.error('Failed to load custom download directory / offline status:', err);
      offlineStatus = null;
      setUpdateState('error', 'Could not check offline resource status. Retry from Reference Databases.');
    } finally {
      if (generation === statusRefreshGeneration) isCheckingStatus = false;
    }
  }

  /** Refresh inventory asynchronously; the backend waits for an authoritative probe. */
  async function refreshStatusInBackground() {
    const generation = ++statusRefreshGeneration;
    setUpdateState('checking', 'Refreshing resource status…');
    try {
      const status = await checkOfflineDataUpdates();
      if (generation !== statusRefreshGeneration) return;
      offlineStatus = status;
      const pendingCount = listOfflineUpdates(status).length;
      setUpdateState(
        pendingCount > 0 ? 'available' : 'ready',
        pendingCount > 0
          ? `${pendingCount} newer remote resource version(s) available.`
          : 'Local resources are current.',
      );
    } catch (err) {
      if (generation !== statusRefreshGeneration) return;
      console.error('Background offline status refresh failed:', err);
      setUpdateState('error', 'Resource status refresh failed. The last known local resources remain available.');
    }
  }

  function clearAssetProgress(assetId: string) {
    const { [assetId]: _d, ...restD } = downloadProgress;
    downloadProgress = restD;
    const { [assetId]: _i, ...restI } = importProgress;
    importProgress = restI;
    const { [assetId]: _p, ...restP } = syncPhase;
    syncPhase = restP;
  }

  function markAssetCurrent(assetId: string) {
    offlineStatus = clearOfflineUpdate(offlineStatus, assetId) ?? offlineStatus;
  }

  async function handleSyncAllMissing() {
    if (syncingAll || Object.values(syncingAsset).some(Boolean)) return;
    syncingAll = true;
    setUpdateState('downloading', 'Syncing missing resources…');
    bulkSyncMessage = 'Sync All Missing · starting…';
    bulkActiveAssetId = null;
    syncErrors = {};
    syncMessages = {};
    downloadProgress = {};
    importProgress = {};
    syncPhase = {};
    try {
      const results = await syncAllOfflineMissing(selectedSample?.id ?? undefined);
      for (const assetId of results.flatMap((result) => result.assets_synced)) {
        markAssetCurrent(assetId);
      }
      const allErrors: string[] = [];
      const allMessages: string[] = [];
      results.forEach((r) => {
        if (r.errors?.length) allErrors.push(...r.errors);
        if (r.messages?.length) allMessages.push(...r.messages);
      });
      if (allErrors.length) {
        syncErrors = { __all__: allErrors.join('\n') };
        setUpdateState('error', 'Some resources could not be synced. Review the error details and retry.');
      }
      if (allMessages.length) {
        const skipped = allMessages.filter((m) => m.includes('up to date') || m.includes('already')).length;
        const worked = allMessages.length - skipped;
        syncMessages = {
          __all__: worked > 0
            ? `Synced/imported ${worked} · skipped ${skipped} already current`
            : `Nothing missing — ${skipped} asset(s) already current`,
        };
      }
      if (allErrors.length === 0) {
        setUpdateState('validating', 'Validating synced resources…');
        setUpdateState('installed', 'Resources installed and indexed.');
        setUpdateState('reloading', 'Refreshing the selected profile report…');
        await onResourcesUpdated?.();
        lastSuccessfulUpdateAt = Date.now();
        setUpdateState('ready', 'Resources ready; report refreshed.');
      }
    } catch (err) {
      syncErrors = { __all__: String(err) };
      setUpdateState('error', 'Sync failed. The previous local resources remain available.');
    } finally {
      syncingAll = false;
      bulkSyncMessage = '';
      bulkActiveAssetId = null;
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
    if (!isTauri()) return;

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
      if (syncingAll || updatingAllOutdated) {
        bulkActiveAssetId = asset_id;
        bulkSyncMessage = `Downloading ${asset_id}…`;
      }
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
        if (syncingAll || updatingAllOutdated) {
          bulkActiveAssetId = asset_id;
          if (payload.message) bulkSyncMessage = payload.message;
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
      if (syncingAll || updatingAllOutdated) {
        bulkSyncMessage = message;
        if (asset_id !== '__bulk__' && phase !== 'skip') {
          bulkActiveAssetId = asset_id;
        }
      }
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

  async function handleSyncAsset(assetId: string, force: boolean, refreshAfter = true) {
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
    setUpdateState('downloading', `Updating ${assetId}…`);

    try {
      const result = await syncSingleOfflineAsset(
        assetId,
        force,
        assetId === 'tier2_variant_locus' ? (selectedSample?.id ?? undefined) : undefined,
      );
      if (result.errors?.length) {
        syncErrors = { ...syncErrors, [assetId]: result.errors.join('\n') };
        setUpdateState('error', `${assetId} failed validation or import. The previous local resource remains available.`);
      } else {
        for (const syncedAssetId of result.assets_synced) {
          markAssetCurrent(syncedAssetId);
        }
        syncMessages = { ...syncMessages, [assetId]: result.messages.join('\n') };
        setUpdateState('validating', `Validating ${assetId}…`);
        setUpdateState('installed', `${assetId} installed and indexed.`);
        setUpdateState('reloading', 'Refreshing the selected profile report…');
        await onResourcesUpdated?.();
        lastSuccessfulUpdateAt = Date.now();
        setUpdateState('ready', 'Resource ready; report refreshed.');
      }
    } catch (err) {
      syncErrors = { ...syncErrors, [assetId]: String(err) };
      setUpdateState('error', `${assetId} update failed. The previous local resource remains available.`);
    } finally {
      // Clear busy state FIRST so other Download buttons unlock immediately.
      syncingAsset = { ...syncingAsset, [assetId]: false };
      clearAssetProgress(assetId);
      // Bulk update awaits one final authoritative refresh instead of allowing
      // per-asset probes to race and restore an older pending-update snapshot.
      if (refreshAfter) void refreshStatusInBackground();
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

  function findAssetById(assetId: string): OfflineAssetStatus | null {
    if (!offlineStatus) return null;
    for (const tier of offlineStatus.tiers) {
      const hit = tier.assets.find((a: OfflineAssetStatus) => a.asset_id === assetId);
      if (hit) return hit;
    }
    return null;
  }

  function companionNeedsUpdate(db: (typeof DB_DEFS)[number]): boolean {
    if (!('companions' in db)) return false;
    return db.companions.some((c) => !!findAssetById(c.assetId)?.update_available);
  }

  /** Show per-asset progress during single sync OR bulk Sync All / Update all. */
  function assetIsShowingProgress(assetId: string): boolean {
    if (syncingAsset[assetId]) return true;
    if (!syncingAll && !updatingAllOutdated) return false;
    if (downloadProgress[assetId] || importProgress[assetId]) return true;
    if (bulkActiveAssetId !== assetId) return false;
    const phase = syncPhase[assetId]?.phase;
    return phase === 'download' || phase === 'import' || phase === 'check';
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
      tierNum: 1, assetId: 'pharmgkb_clinical_variants', label: 'PharmGKB (+ ClinGen, MANE)',
      blurb: 'Primary: PharmGKB clinical variants. Also syncs PharmGKB genes, ClinGen gene validity, and MANE transcripts as companions (listed under this row).',
      companions: [
        { assetId: 'pharmgkb_genes', label: 'PharmGKB genes' },
        { assetId: 'clingen_gene_validity', label: 'ClinGen gene validity' },
        { assetId: 'mane_select_summary', label: 'MANE Select summary' },
      ],
    },
    {
      tierNum: 2, assetId: 'dbsnp_merged_json', label: 'dbSNP References',
      blurb: 'NCBI RefSNP merge + withdrawn map (old rsIDs → current). Not allele frequencies — those come from gnomAD. Large JSON → compact SQLite alias table is expected.',
      companions: [
        { assetId: 'dbsnp_withdrawn_json', label: 'dbSNP withdrawn rsIDs' },
      ],
    },
  ] as const;

  let pendingUpdates = $derived.by(() => listOfflineUpdates(offlineStatus));

  let liftoverUpdateAvailable = $derived(
    pendingUpdates.some((update) => update.asset_id === 'liftover_chain'),
  );

  let updatesAvailable = $derived.by(() => {
    // Derive from the named list so the badge cannot outlive an optimistic
    // completion while the authoritative post-sync probe is still running.
    return pendingUpdates.length;
  });

  let primaryUpdatesAvailable = $derived(
    pendingUpdates.filter((u) => u.primary).length
  );

  let updateBadgeTitle = $derived.by(() => formatUpdateSummary(pendingUpdates));

  let updatingAllOutdated = $state(false);

  async function handleUpdateAllOutdated() {
    if (updatingAllOutdated || syncingAll || Object.values(syncingAsset).some(Boolean)) return;
    const items = listOfflineUpdates(offlineStatus);
    if (items.length === 0) return;
    updatingAllOutdated = true;
    isPanelCollapsed = false;
    try {
      for (const item of items) {
        await handleSyncAsset(item.asset_id, forceRedownload, false);
      }
    } finally {
      updatingAllOutdated = false;
      await refreshStatusInBackground();
    }
  }
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
  let bulkBusy = $derived(syncingAll || updatingAllOutdated);
</script>

<aside id="data-sidebar" class="sidebar" aria-label="Profile and data controls">
  <div class="brand">
    <img src="/logo.png" alt="Genomics Caddy Logo" class="brand-logo" />
    <h2>Genomics Caddy</h2>
  </div>

  {#if !runtimeAvailable}
    <div class="runtime-note" role="note">
      Desktop runtime required for local DNA import, catalogs, and saved profiles. The browser preview is read-only.
    </div>
  {/if}

  {#if onOpenConnections}
    <div class="connections-launch-card card">
      <div class="connections-launch-copy">
        <strong>Connections</strong>
        <span>Ollama + vector databases</span>
      </div>
      <div class="connections-launch-actions">
        <Tooltip
          label="Connections"
          description="Set Ollama and vector database endpoints, monitor health, and manage model settings under Advanced."
        >
          <span class="info-icon" aria-label="Explain Connections">ⓘ</span>
        </Tooltip>
        <button type="button" class="btn btn-secondary btn-sm" onclick={() => onOpenConnections?.()} disabled={!runtimeAvailable}>
          Open Connections
        </button>
      </div>
    </div>
  {/if}

  <!-- Chain status stays compact when healthy; attention states keep their action visible. -->
  <div class="chain-status-card card liftover-status-card" class:liftover-needs-action={!isChainDownloaded || liftoverUpdateAvailable}>
    <div class="liftover-status-copy">
      <strong>Liftover assembly</strong>
      {#if isChainDownloaded}
        <span class="badge success">🟢 GRCh38 Active</span>
      {:else}
        <span class="badge warning">⚠️ GRCh37 Only</span>
        <span class="liftover-status-detail">Download the chain to map imported coordinates to GRCh38.</span>
      {/if}
    </div>
    <div class="liftover-status-actions">
      <Tooltip
        label="Liftover assembly"
        description="The chain maps imported GRCh37 coordinates to GRCh38 for report context; it does not change your genotype calls."
      >
        <span class="info-icon" aria-label="Explain liftover assembly">ⓘ</span>
      </Tooltip>
      {#if !isChainDownloaded}
        <button type="button" class="btn btn-primary btn-sm" onclick={onDownloadChain} disabled={isDownloadingChain || sweepRunning || !runtimeAvailable}>
          {isDownloadingChain ? 'Downloading…' : 'Download chain'}
        </button>
      {/if}
      {#if liftoverUpdateAvailable}
        <Tooltip
          label="Liftover update"
          description="A newer UCSC GRCh37→GRCh38 chain is available. The current local chain remains available until the update finishes."
        >
          <span class="badge warning">Update available</span>
        </Tooltip>
        <button
          type="button"
          class="btn btn-warning btn-sm"
          disabled={!!syncingAsset['liftover_chain'] || bulkBusy || sweepRunning || updatingAllOutdated || !runtimeAvailable}
          onclick={() => handleSyncAsset('liftover_chain', forceRedownload)}
        >
          {syncingAsset['liftover_chain'] ? 'Updating…' : 'Update chain'}
        </button>
      {/if}
    </div>
  </div>

  <!-- Data and updates disclosure -->
  <div class="chain-status-card card">
    <button
      type="button"
      class="data-updates-toggle"
      onclick={() => isPanelCollapsed = !isPanelCollapsed}
      aria-expanded={!isPanelCollapsed}
      aria-controls="data-updates-panel"
    >
      <span class="data-updates-heading">
        <strong id="data-updates-title">Data &amp; updates</strong>
        <span class="data-updates-subtitle">Reference catalogs and local status</span>
        {#if missingPrimaryCount > 0}
          <span
            class="missing-pill"
            aria-label="{[
              notDownloadedPrimary.length ? `${notDownloadedPrimary.join(', ')} not downloaded` : '',
              notIndexedPrimary.length ? `${notIndexedPrimary.join(', ')} downloaded but not indexed` : '',
            ].filter(Boolean).join(' · ')}"
          >{missingPrimaryCount} need attention</span>
        {/if}
        {#if updatesAvailable > 0}
          <span
            class="update-pill"
            aria-label={updateBadgeTitle || `${updatesAvailable} newer remote version(s) available`}
          >
            {updatesAvailable} newer
            {#if pendingUpdates.length === 1}
              · {pendingUpdates[0].label}
            {/if}
          </span>
        {/if}
      </span>
      <span class="data-updates-caret" aria-hidden="true">{isPanelCollapsed ? '▶' : '▼'}</span>
    </button>

    {#if !isPanelCollapsed}
      <div id="data-updates-panel" class="data-updates-body" role="region" aria-labelledby="data-updates-title">

        {#if updatePhase !== 'idle'}
          <div class="resource-update-state" class:resource-update-state-error={updatePhase === 'error'} class:resource-update-state-ready={updatePhase === 'ready' || updatePhase === 'installed'} role="status" aria-live="polite">
            <div class="resource-update-state-heading">
              <span class="resource-update-state-dot" aria-hidden="true"></span>
              <strong>{updatePhaseLabel(updatePhase)}</strong>
            </div>
            {#if updateMessage}
              <div class="resource-update-state-message">{updateMessage}</div>
            {/if}
            {#if lastSuccessfulUpdateAt}
              <div class="resource-update-state-time">Last successful update: {new Date(lastSuccessfulUpdateAt).toLocaleString()}</div>
            {/if}
          </div>
        {/if}

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
            {#if pendingUpdates.length > 0}
              <div class="updates-panel">
                <div>
                  <strong>{pendingUpdates.length} newer remote file{pendingUpdates.length === 1 ? '' : 's'}</strong>
                  {#if primaryUpdatesAvailable === 0}
                    — supporting asset(s) only (not GWAS/ClinVar/PharmGKB/dbSNP)
                  {:else if primaryUpdatesAvailable < pendingUpdates.length}
                    — {primaryUpdatesAvailable} primary · {pendingUpdates.length - primaryUpdatesAvailable} supporting
                  {/if}
                </div>
                <ul class="updates-named-list">
                  {#each pendingUpdates as u (u.asset_id)}
                    <li>
                      <div class="update-item-meta">
                        <strong>{u.label}</strong>
                        <span class="update-item-kind">
                          Tier {u.tier} · {u.primary ? 'primary catalog' : 'supporting asset'}
                          {#if u.display_size}
                            · {u.display_size}
                          {/if}
                        </span>
                        {#if u.message}
                          <span class="update-item-msg">{u.message}</span>
                        {/if}
                      </div>
                      <button
                        type="button"
                        class="btn btn-warning btn-xs"
                        aria-label={`Update ${u.label}`}
                        disabled={!!syncingAsset[u.asset_id] || bulkBusy || sweepRunning || updatingAllOutdated || !runtimeAvailable}
                        onclick={() => handleSyncAsset(u.asset_id, forceRedownload)}
                      >
                        {syncingAsset[u.asset_id] ? 'Updating…' : 'Update'}
                      </button>
                    </li>
                  {/each}
                </ul>
                <button
                  type="button"
                  class="btn btn-primary btn-sm updates-all-btn"
                  disabled={bulkBusy || updatingAllOutdated || Object.values(syncingAsset).some(Boolean) || sweepRunning || !runtimeAvailable}
                  onclick={handleUpdateAllOutdated}
                >
                  {updatingAllOutdated ? 'Updating all…' : `Update all ${pendingUpdates.length} outdated`}
                </button>
                <div class="update-footnote">
                  Update downloads the newer remote file then re-imports. Re-sync only re-imports the local file you already have.
                </div>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Custom Directory Row -->
        <div class="dir-setting">
          <span class="setting-label">Download Location</span>
          <div class="setting-row">
            <input
              type="text"
              placeholder="Default App Directory"
              value={customDir || ''}
              readonly
              aria-label="Download location"
              class="download-location-input"
            />
            <button
              type="button"
              class="btn btn-secondary btn-sm compact-sidebar-button"
              onclick={handleBrowseDir}
            >
              Browse
            </button>
          </div>
          {#if customDir}
            <button
              type="button"
              onclick={handleResetDir}
              class="reset-dir-btn"
            >
              Reset to Default
            </button>
          {/if}
        </div>

        <!-- Force re-download checkbox -->
        <label class="force-redownload-label">
          <input type="checkbox" bind:checked={forceRedownload} />
          Force re-download existing files
        </label>

        <hr class="sidebar-rule" />

        <!-- Sync All Missing -->
        <button
          type="button"
          class="btn btn-primary btn-sm full-width-sidebar-button"
          onclick={handleSyncAllMissing}
          disabled={bulkBusy || Object.values(syncingAsset).some(Boolean) || !offlineStatus || sweepRunning || !runtimeAvailable}
        >
          {syncingAll ? '⏳ Syncing All…' : '⬇️ Sync All Missing'}
        </button>

        {#if syncingAll}
          <div class="bulk-sync-panel" role="status" aria-live="polite">
            <ActivityPulse message={bulkSyncMessage || 'Sync All Missing · working…'} accent="var(--status-success-text)" />
            {#if bulkActiveAssetId && downloadProgress[bulkActiveAssetId]}
              {@const prog = downloadProgress[bulkActiveAssetId]}
              <ProgressTrack percent={prog.percent} spaced label="Bulk download progress" />
              <div class="bulk-sync-meta">
                <span>{prog.percent >= 0 ? `${prog.percent}%` : 'streaming…'}</span>
                <span>{prog.speedMbps.toFixed(1)} MB/s</span>
              </div>
            {:else if bulkActiveAssetId && importProgress[bulkActiveAssetId]}
              {@const imp = importProgress[bulkActiveAssetId]}
              <ProgressTrack percent={imp.percent ?? -1} variant="indexing" spaced label="Bulk indexing progress" />
              <div class="bulk-sync-meta bulk-sync-meta-indexing">
                <span>{imp.percent !== undefined && imp.percent >= 0 ? `${imp.percent}%` : 'indexing…'}</span>
                {#if imp.eta_seconds != null}
                  <span>{imp.eta_seconds}s remaining</span>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

        {#if syncMessages.__all__ && !syncingAll}
          <div class="bulk-sync-summary">{syncMessages.__all__}</div>
        {/if}

        {#if importingAny}
          <button
            type="button"
            class="btn btn-secondary btn-sm full-width-sidebar-button"
            onclick={handleCancelImport}
          >
            ⏹ Cancel import
          </button>
        {/if}

        <button
          type="button"
          class="btn btn-secondary btn-sm full-width-sidebar-button"
          onclick={handleExportDiscovery}
          disabled={!selectedSample || exportBusy || sweepRunning || !runtimeAvailable}
        >
          {exportBusy ? 'Exporting…' : '📤 Export pack vs genome findings'}
        </button>
        {#if exportMessage}
          <div class="sidebar-message">{exportMessage}</div>
        {/if}

        {#if syncErrors.__all__}
          <div class="sync-error-block">
            <strong>⚠️ Sync errors:</strong>
            <pre class="sidebar-error-detail">{syncErrors.__all__}</pre>
          </div>
        {/if}

        <hr class="sidebar-rule" />

        <!-- Individual Databases list -->
        <div class="db-list">
          {#each DB_DEFS as db}
            {@const btnState = assetButtonState(db.tierNum, db.assetId)}
            {@const prog = downloadProgress[db.assetId]}
            {@const isActive = assetIsShowingProgress(db.assetId)}
            <div class="db-item-wrap">
              <div class="db-item">
                <div class="db-item-meta">
                  <span class="db-item-title">
                    <strong>{db.label}</strong>
                    {#if findAsset(db.tierNum, db.assetId)?.update_available || companionNeedsUpdate(db)}
                      <Tooltip label="Update available" description="A newer file is available on the server for this catalog or one of its supporting files.">
                        <span class="update-pill">update</span>
                      </Tooltip>
                    {/if}
                    <Tooltip label={db.label} description={assetTooltip(db)}>
                      <span class="info-icon">ⓘ</span>
                    </Tooltip>
                  </span>
                  <span class="db-item-status">
                    {getAssetStatusLine(db.tierNum, db.assetId)}
                  </span>
                </div>
                <button
                  type="button"
                  class="btn btn-xs db-action-button"
                  aria-label={`${btnState.label} ${db.label}`}
                  class:btn-primary={btnState.variant === 'primary'}
                  class:btn-secondary={btnState.variant === 'secondary'}
                  class:btn-warning={btnState.variant === 'warning'}
                  onclick={() => handleSyncAsset(db.assetId, btnState.isForce || forceRedownload)}
                  disabled={!!syncingAsset[db.assetId] || bulkBusy || !offlineStatus || sweepRunning || !runtimeAvailable}
                >
                  {btnState.label}
                </button>
              </div>

              <!-- Per-asset progress bar (shown while downloading or importing) -->
              {#if isActive}
                {#if prog && !importProgress[db.assetId]}
                  <ProgressTrack percent={prog.percent} label={`${db.label} download progress`} />
                  <div class="progress-meta">
                    <span>{prog.percent >= 0 ? prog.percent + '%' : 'streaming…'}</span>
                    <span>{prog.speedMbps.toFixed(1)} MB/s</span>
                  </div>
                {:else if importProgress[db.assetId]}
                  {@const imp = importProgress[db.assetId]}
                  <ProgressTrack percent={imp.percent ?? -1} variant="indexing" label={`${db.label} indexing progress`} />
                  <div class="progress-meta progress-meta-indexing">
                    <span>{imp.percent !== undefined && imp.percent >= 0 ? imp.percent + '%' : 'indexing…'}</span>
                    {#if imp.eta_seconds !== undefined && imp.eta_seconds !== null}
                      <span>{imp.eta_seconds}s remaining</span>
                    {/if}
                  </div>
                  <div class="import-status-line">
                    <span class="import-dot"></span>
                    <span>{imp.message}</span>
                  </div>
                {:else}
                  <ProgressTrack variant="preparing" label={`${db.label} import preparation`} />
                  <div class="progress-preparing">
                    Preparing import (no download needed)…
                  </div>
                {/if}
              {/if}

              {#if 'companions' in db}
                <ul class="db-companions">
                  {#each db.companions as c (c.assetId)}
                    {@const companion = findAssetById(c.assetId)}
                    {@const cBusy = assetIsShowingProgress(c.assetId)}
                    <li class="db-companion">
                      <div class="db-companion-meta">
                        <span class="db-companion-label">
                          {c.label}
                          {#if companion?.update_available}
                            <Tooltip label="Update available" description="A newer supporting file is available on the server.">
                              <span class="update-pill">update</span>
                            </Tooltip>
                          {/if}
                        </span>
                        <span class="db-companion-status">
                          {#if !companion}
                            Checking…
                          {:else if cBusy}
                            {syncPhase[c.assetId]?.message || importProgress[c.assetId]?.message || 'Updating…'}
                          {:else if companion.update_available}
                            Update available · {companion.display_size || companion.message}
                          {:else if companion.row_count > 0}
                            {companion.row_count.toLocaleString()} rows
                          {:else if companion.local_present}
                            On disk · not indexed
                          {:else}
                            Not downloaded
                          {/if}
                        </span>
                      </div>
                      <button
                        type="button"
                        class="btn btn-xs"
                        aria-label={`${companion?.update_available || forceRedownload ? 'Update' : !companion?.local_present ? 'Download' : 'Re-sync'} ${c.label}`}
                        class:btn-warning={!!companion?.update_available || forceRedownload}
                        class:btn-secondary={!companion?.update_available && !forceRedownload}
                        disabled={cBusy || bulkBusy || !offlineStatus || sweepRunning || updatingAllOutdated || !runtimeAvailable}
                        onclick={() => handleSyncAsset(c.assetId, forceRedownload)}
                      >
                        {#if cBusy}
                          …
                        {:else if !companion?.local_present}
                          Download
                        {:else if companion?.update_available || forceRedownload}
                          Update
                        {:else}
                          Re-sync
                        {/if}
                      </button>
                    </li>
                    {#if cBusy && (downloadProgress[c.assetId] || importProgress[c.assetId])}
                      <li class="db-companion-progress">
                        {#if downloadProgress[c.assetId] && !importProgress[c.assetId]}
                          {@const cprog = downloadProgress[c.assetId]}
                          <ProgressTrack percent={cprog.percent} label={`${c.label} download progress`} />
                        {:else if importProgress[c.assetId]}
                          {@const cimp = importProgress[c.assetId]}
                          <ProgressTrack percent={cimp.percent ?? -1} variant="indexing" label={`${c.label} indexing progress`} />
                        {/if}
                      </li>
                    {/if}
                  {/each}
                </ul>
              {/if}

              <!-- Per-asset error display -->
              {#if syncErrors[db.assetId]}
                <div class="sync-error-block asset-feedback">
                  <strong>⚠️ Error:</strong>
                  <span>{syncErrors[db.assetId]}</span>
                </div>
              {/if}

              <!-- Per-asset success message -->
              {#if syncMessages[db.assetId] && !isActive}
                <div class="sync-ok-block asset-feedback">
                  <span>✅ {syncMessages[db.assetId].split('\n')[0]}</span>
                </div>
              {/if}
            </div>
          {/each}
        </div>

        {#if referenceDetails}
          <div class="ref-status-details">
            <div class="ref-status-heading">
              Catalog readiness
            </div>

            <div class="status-group">
              <div class="status-group-title">ClinVar</div>
              <div class="status-group-grid">
                <span>Status:</span>
                <strong class:status-ready={referenceDetails.clinvar_indexed_rows > 0} class:status-pending={referenceDetails.clinvar_indexed_rows === 0 && referenceDetails.clinvar_raw_found} class:status-missing={referenceDetails.clinvar_indexed_rows === 0 && !referenceDetails.clinvar_raw_found}>
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
                  <strong class:status-hit-active={referenceDetails.clinvar_rsid_hits > 0} class:status-hit-empty={referenceDetails.clinvar_rsid_hits === 0}>
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

            <div class="status-group status-group-spaced">
              <div class="status-group-title">dbSNP (rsID merge map)</div>
              <div class="status-group-grid">
                <span>Status:</span>
                <strong class:status-ready={referenceDetails.dbsnp_merge_mappings_indexed > 0} class:status-pending={referenceDetails.dbsnp_merge_mappings_indexed === 0 && referenceDetails.dbsnp_merged_raw_found} class:status-missing={referenceDetails.dbsnp_merge_mappings_indexed === 0 && !referenceDetails.dbsnp_merged_raw_found}>
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
                  <strong class:status-hit-active={referenceDetails.dbsnp_rsids_normalized > 0} class:status-hit-empty={referenceDetails.dbsnp_rsids_normalized === 0}>
                    {referenceDetails.dbsnp_rsids_normalized.toLocaleString()}
                  </strong>
                {/if}
              </div>
              {#if referenceDetails.dbsnp_merged_raw_found && referenceDetails.dbsnp_merge_mappings_indexed === 0}
                <div class="status-warn">Archive on disk but 0 merge mappings indexed — import still needed (large job).</div>
              {/if}
              <div class="status-note">Offline dbSNP here is the NCBI <em>refsnp-merged</em> + withdrawn catalog (rsID merge map + dates/citations). Alleles, placements, and AF live in the huge per-chromosome <em>refsnp-chr*.json</em> files — we do not ingest those yet. Report AF chips use the local gnomAD cache when present (not this merge DB).</div>
            </div>

            <div class="status-group status-group-more-spaced">
              <div class="status-group-title">gnomAD allele frequencies</div>
              <div class="status-group-grid">
                <span>Status:</span>
                <strong class:status-ready={gnomadReadiness?.ready === true} class:status-pending={gnomadReadiness?.ready !== true && (gnomadReadiness?.indexes_cached ?? 0) > 0} class:status-missing={gnomadReadiness?.ready !== true && (gnomadReadiness?.indexes_cached ?? 0) === 0}>
                  {#if gnomadReadiness?.ready}
                    Ready · {gnomadReadiness.indexes_cached}/{gnomadReadiness.indexes_expected} indexes
                  {:else if gnomadReadiness}
                    {gnomadReadiness.indexes_cached}/{gnomadReadiness.indexes_expected} indexes · {gnomadReadiness.summary}
                  {:else}
                    Checking…
                  {/if}
                </strong>
              </div>
              <div class="status-note">Step 3 for AF chips on the report — separate from dbSNP rsID history. Manage Ollama/Qdrant under Advanced → Connections.</div>
              {#if gnomadReadiness && !gnomadReadiness.ready}
                <button
                  type="button"
                  class="btn btn-secondary btn-sm download-gnomad-button"
                  disabled={gnomadBusy || sweepRunning || !runtimeAvailable}
                  onclick={handleDownloadGnomadIndexes}
                >
                  {gnomadBusy ? 'Downloading…' : 'Download gnomAD indexes'}
                </button>
              {/if}
              {#if gnomadHint}
                <div class="status-note">{gnomadHint}</div>
              {/if}
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
    disabled={sweepRunning || !runtimeAvailable}
    {onBrowseFile}
    {onImportGenome}
  />

  <!-- Active Profiles -->
  <SampleList
    {samples}
    {selectedSample}
    disabled={sweepRunning || !runtimeAvailable}
    {onSelectSample}
    {onDeleteSample}
  />
</aside>
