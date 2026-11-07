<!-- ./src/lib/components/research/OfflineDataPanel.svelte -->
<script lang="ts">
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import {
    checkOfflineDataUpdates,
    syncOfflineDataTier,
    syncAllOfflineData,
    syncSingleOfflineAsset,
    buildOfflineTier2,
    removeOfflineDownload,
    deleteOfflineDatabase,
    purgeOfflineDownloads,
  } from "../../api/tauri";
  import type { OfflineAssetStatus, OfflineUpdateCheck, OfflineSyncResult } from "../../types/research";
  import { clearOfflineUpdate, hasProvenOfflineUpdate, isCompleteOfflineStatus, listOfflineUpdates, offlineStatusFailureMessage } from "../../utils/offlineUpdates";
  import { isPrimaryCatalogId } from "../../utils/primaryCatalogs";
  import Tooltip from "../common/Tooltip.svelte";
  import { dialogStore } from "../../utils/dialogState.svelte";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    disabled?: boolean;
    onLog?: (msg: string) => void;
  }

  let { selectedSample = null, disabled = false, onLog }: Props = $props();

  let status = $state<OfflineUpdateCheck | null>(null);
  let loading = $state(false);
  let syncingTier = $state<number | null>(null);
  let syncingAsset = $state<string | null>(null);
  let deletingAsset = $state<string | null>(null);
  let deletingDatabase = $state<string | null>(null);
  let purgingDownloads = $state(false);
  let forceSync = $state(false);
  let statusRefreshGeneration = 0;
  let statusStale = $state(false);
  let statusError = $state('');
  let operationError = $state('');
  type RetryTarget =
    | { kind: 'tier'; tier: number; force: boolean }
    | { kind: 'all'; force: boolean }
    | { kind: 'asset'; assetId: string; force: boolean }
    | { kind: 'outdated'; assetIds: string[]; force: boolean }
    | { kind: 'rebuild'; sampleId: number };
  let retryTarget = $state<RetryTarget | null>(null);

  const offlineDatabases = [
    { id: "gwas", label: "GWAS Catalog", assets: ["gwas_catalog"] },
    { id: "clinvar", label: "ClinVar variants", assets: ["clinvar_variant_summary"] },
    { id: "clinvar_submissions", label: "ClinVar submissions", assets: ["clinvar_submission_summary"] },
    { id: "clinpgx", label: "ClinPGx", assets: ["pharmgkb_clinical_variants", "pharmgkb_genes"] },
    { id: "clingen", label: "ClinGen", assets: ["clingen_gene_validity"] },
    { id: "mane", label: "MANE Select", assets: ["mane_select_summary"] },
    { id: "dbsnp", label: "dbSNP", assets: ["dbsnp_merged_json", "dbsnp_withdrawn_json"] },
  ];

  let updateAssets = $derived(isCompleteOfflineStatus(status) ? listOfflineUpdates(status) : []);

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  function formatTimestamp(seconds: number | null | undefined): string {
    if (!seconds || !Number.isFinite(seconds)) return "not recorded";
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(seconds * 1000));
  }

  function findAssetById(assetId: string) {
    return status?.tiers.flatMap((tier) => tier.assets).find((asset) => asset.asset_id === assetId);
  }

  function indexedCountFor(database: (typeof offlineDatabases)[number]): number {
    return database.assets.reduce((total, assetId) => total + (findAssetById(assetId)?.row_count ?? 0), 0);
  }

  function canManageResources(): boolean {
    return !disabled && syncingTier === null && syncingAsset === null && !loading
      && !deletingAsset && !deletingDatabase && !purgingDownloads;
  }

  function confirmRemoveDownload(asset: OfflineAssetStatus) {
    const isGnomad = asset.asset_id === "gnomad_index_manifest";
    dialogStore.confirm(
      isGnomad
        ? "Clear downloaded gnomAD tabix index files, release metadata, and cached frequency lookups? This does not delete local VCF files, DNA profiles, or genotype databases."
        : `Remove the downloaded ${asset.label} file and any partial copy? Its already indexed reference data will remain available. Your DNA profiles and genotype databases are not affected.`,
      async () => {
        clearOperationError();
        deletingAsset = asset.asset_id;
        let failure = "";
        try {
          const result = await removeOfflineDownload(asset.asset_id);
          onLog?.(`Removed ${result.files_removed} file(s) and ${result.records_removed.toLocaleString()} cached record(s) (${formatBytes(result.bytes_freed)}).`);
        } catch (e: unknown) {
          failure = `Could not finish removing the ${asset.label} download. Review its status below; if it is still present, try again.`;
          onLog?.(`Removing ${asset.label} failed: ${e instanceof Error ? e.message : String(e)}`);
        } finally {
          deletingAsset = null;
          await refresh();
          if (failure) setOperationError(failure, null);
        }
      },
      isGnomad ? "Clear gnomAD downloads" : "Remove downloaded file",
    );
  }

  function confirmDeleteDatabase(database: (typeof offlineDatabases)[number]) {
    dialogStore.confirm(
      `Delete the local ${database.label} database and its indexed records? Downloaded source files remain so you can rebuild it later. DNA profiles and genotype databases are not affected.`,
      async () => {
        clearOperationError();
        deletingDatabase = database.id;
        let failure = "";
        try {
          const result = await deleteOfflineDatabase(database.id);
          onLog?.(`Deleted the ${database.label} database, removed ${result.records_removed.toLocaleString()} indexed record(s), and freed ${formatBytes(result.bytes_freed)}.`);
        } catch (e: unknown) {
          failure = `Could not finish deleting the ${database.label} database. Review its status below; if it remains, close any operation using it and try again.`;
          onLog?.(`Deleting ${database.label} database failed: ${e instanceof Error ? e.message : String(e)}`);
        } finally {
          deletingDatabase = null;
          await refresh();
          if (failure) setOperationError(failure, null);
        }
      },
      `Delete ${database.label} database`,
    );
  }

  function confirmPurgeDownloads() {
    dialogStore.confirm(
      "Remove all app-managed reference downloads and catalog databases, including the coordinate map and gnomAD index downloads? This keeps DNA profiles, genotype databases, reports, exports, local VCF files, and app settings. Files outside known app-managed locations are left alone.",
      async () => {
        clearOperationError();
        purgingDownloads = true;
        let failure = "";
        try {
          const result = await purgeOfflineDownloads();
          onLog?.(`Removed ${result.files_removed} reference file(s), ${result.databases_removed} catalog database(s), and ${result.records_removed.toLocaleString()} indexed record(s); freed ${formatBytes(result.bytes_freed)}.`);
        } catch (e: unknown) {
          failure = "Could not finish purging reference data. Some items may already be removed; review the status below before trying again.";
          onLog?.(`Purging reference data failed: ${e instanceof Error ? e.message : String(e)}`);
        } finally {
          purgingDownloads = false;
          await refresh();
          if (failure) setOperationError(failure, null);
        }
      },
      "Purge reference downloads",
    );
  }

  async function refresh(): Promise<boolean> {
    const generation = ++statusRefreshGeneration;
    loading = true;
    statusStale = true;
    try {
      const nextStatus = await checkOfflineDataUpdates();
      if (generation !== statusRefreshGeneration) return false;
      status = nextStatus;
      statusStale = !isCompleteOfflineStatus(nextStatus);
      statusError = statusStale ? offlineStatusFailureMessage(nextStatus) : '';
      return true;
    } catch (e: unknown) {
      if (generation !== statusRefreshGeneration) return false;
      const msg = e instanceof Error ? e.message : String(e);
      onLog?.(`Offline data status failed: ${msg}`);
      statusStale = true;
      statusError = 'Could not verify the latest resource status.';
      return false;
    } finally {
      if (generation === statusRefreshGeneration) loading = false;
    }
  }

  function clearOperationError() {
    operationError = '';
    retryTarget = null;
  }

  function setOperationError(message: string, target: RetryTarget | null) {
    operationError = message;
    retryTarget = target;
  }

  async function runTier(tier: number) {
    if (syncingTier !== null || syncingAsset) return;
    const requestedForce = forceSync;
    syncingTier = tier;
    clearOperationError();
    onLog?.(`Syncing offline Tier ${tier}${requestedForce ? " (force)" : ""}…`);
    let failure = '';
    try {
      const result = await syncOfflineDataTier(
        tier,
        requestedForce,
        tier === 2 ? selectedSample?.id : undefined
      );
      reportResult(result);
      // Remove completed assets from the visible snapshot immediately. The
      // final refresh remains authoritative and can add an asset back only if
      // the server still proves that a newer identity exists.
      for (const syncedAssetId of result.assets_synced) {
        status = clearOfflineUpdate(status, syncedAssetId) ?? status;
      }
      if (result.errors?.length) {
        failure = `Tier ${tier} could not be fully synced. Review the error details and retry.`;
      }
    } catch (e: unknown) {
      failure = `Tier ${tier} sync failed. The last good local resources remain available.`;
      onLog?.(`Tier ${tier} sync failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      await refresh();
      syncingTier = null;
      if (failure) setOperationError(failure, { kind: 'tier', tier, force: requestedForce });
    }
  }

  async function runAll() {
    if (syncingTier !== null || syncingAsset) return;
    const requestedForce = forceSync;
    syncingTier = -1;
    clearOperationError();
    onLog?.(`Syncing all offline tiers${requestedForce ? " (force)" : ""}…`);
    let failure = '';
    try {
      const results = await syncAllOfflineData(requestedForce, selectedSample?.id);
      for (const r of results) {
        reportResult(r);
      }
      if (results.some((result) => result.errors?.length)) {
        failure = 'Some offline resources could not be synced. Review the error details and retry.';
      }
    } catch (e: unknown) {
      failure = 'Offline sync failed. The last good local resources remain available.';
      onLog?.(`Full offline sync failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      await refresh();
      syncingTier = null;
      if (failure) setOperationError(failure, { kind: 'all', force: requestedForce });
    }
  }

  async function updateOne(assetId: string, refreshAfter = true, allowBulk = false) {
    if (syncingAsset || (syncingTier !== null && !allowBulk)) return;
    const requestedForce = forceSync;
    syncingAsset = assetId;
    if (refreshAfter) clearOperationError();
    onLog?.(`Updating ${assetId}${requestedForce ? " (force)" : ""}…`);
    let failure = '';
    try {
      const result = await syncSingleOfflineAsset(
        assetId,
        requestedForce,
        assetId === "tier2_variant_locus" ? selectedSample?.id : undefined
      );
      reportResult(result);
      if (result.errors?.length) {
        failure = `${assetId} could not be fully updated. Review the error details and retry.`;
      }
    } catch (e: unknown) {
      failure = `${assetId} update failed. The last good local resource remains available.`;
      onLog?.(`Update ${assetId} failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      if (refreshAfter) await refresh();
      syncingAsset = null;
      if (failure && refreshAfter) setOperationError(failure, { kind: 'asset', assetId, force: requestedForce });
    }
    return !failure;
  }

  async function updateAllOutdated(assetIds = updateAssets.map((asset) => asset.asset_id), requestedForce = forceSync) {
    if (syncingAsset || syncingTier !== null || assetIds.length === 0) return;
    syncingTier = -2;
    clearOperationError();
    onLog?.(`Updating ${assetIds.length} outdated asset(s)…`);
    const failedAssetIds: string[] = [];
    try {
      for (const assetId of assetIds) {
        const previousForce = forceSync;
        forceSync = requestedForce;
        const succeeded = await updateOne(assetId, false, true);
        forceSync = previousForce;
        if (!succeeded) failedAssetIds.push(assetId);
      }
    } catch (e: unknown) {
      failedAssetIds.push(...assetIds.filter((assetId) => !failedAssetIds.includes(assetId)));
      onLog?.(`Update-all failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      await refresh();
      syncingTier = null;
      if (failedAssetIds.length > 0) {
        setOperationError(
          `${failedAssetIds.length} resource update(s) failed. Review the error details and retry.`,
          { kind: 'outdated', assetIds: failedAssetIds, force: requestedForce },
        );
      }
    }
  }

  async function rebuildTier2() {
    if (!selectedSample) {
      onLog?.("Select a DNA profile to build Tier 2 variant locus index.");
      return;
    }
    const sampleId = selectedSample.id;
    syncingTier = 2;
    clearOperationError();
    let failure = '';
    try {
      const result = await buildOfflineTier2(sampleId);
      reportResult(result);
      if (result.errors?.length) {
        failure = 'The locus index could not be rebuilt. Review the error details and retry.';
      }
    } catch (e: unknown) {
      failure = 'The locus index could not be rebuilt. The previous index remains available.';
      onLog?.(`Tier 2 build failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      await refresh();
      syncingTier = null;
      if (failure) setOperationError(failure, { kind: 'rebuild', sampleId });
    }
  }

  async function retryLastOperation() {
    const target = retryTarget;
    if (!target) return;
    retryTarget = null;
    operationError = '';
    switch (target.kind) {
      case 'tier':
        forceSync = target.force;
        await runTier(target.tier);
        break;
      case 'all':
        forceSync = target.force;
        await runAll();
        break;
      case 'asset':
        forceSync = target.force;
        await updateOne(target.assetId);
        break;
      case 'outdated':
        await updateAllOutdated(target.assetIds, target.force);
        break;
      case 'rebuild':
        await rebuildTier2();
        break;
    }
  }

  function reportResult(result: OfflineSyncResult) {
    for (const msg of result.messages) {
      onLog?.(`[Tier ${result.tier}] ${msg}`);
    }
    for (const err of result.errors) {
      onLog?.(`[Tier ${result.tier} error] ${err}`);
    }
    if (result.assets_synced.length > 0) {
      onLog?.(
        `Tier ${result.tier} complete: ${result.assets_synced.join(", ")}`
      );
    }
  }

  $effect(() => {
    void refresh();
  });
</script>

<section class="offline-data-panel scope-subcard scope-subcard--full">
  <div class="offline-header">
    <h3 class="subcard-title">Offline reference data</h3>
    {#if loading}
      <ActivityPulse message="Checking sources…" accent="var(--status-success-text)" maxWidth="180px" />
    {:else if status && !statusStale}
      <span class="update-badge" class:has-updates={updateAssets.length > 0}>
        {updateAssets.length > 0
          ? `${updateAssets.length} source update(s) available`
          : "Up to date"}
      </span>
    {:else if status}
      <span class="update-badge status-stale">Status needs checking</span>
    {/if}
  </div>

  {#if statusError}
    <div class="offline-status-error offline-status-attention" role="status">
      <span>{statusError}</span>
      <button type="button" class="btn btn-secondary btn-xs" disabled={disabled || loading} onclick={refresh}>
        {loading ? "Checking…" : "Retry"}
      </button>
    </div>
  {/if}

  <p class="offline-help">
  Tier 0: GWAS catalog, liftover chain, gnomAD manifest. Tier 1: ClinVar, ClinPGx, ClinGen, MANE.
  Tier 2: dbSNP lifecycle JSON + variant locus index from your genotypes. Downloads check remote size before fetching.
  Update badges here match the sidebar (all tier assets, not only the four primary catalogs).
  Updates use server file identifiers or revision metadata when available. Some sources do not provide a stable revision, so a check cannot always prove that file contents are unchanged; use Force re-download to fetch a fresh copy.
  {#if status?.indexed_summary}
    Indexed: {status.indexed_summary.gwas_rows.toLocaleString()} GWAS ·
    {status.indexed_summary.clinvar_rows.toLocaleString()} ClinVar ·
    {status.indexed_summary.variant_locus_rows.toLocaleString()} locus rows.
  {/if}
  </p>

  {#if updateAssets.length > 0 && !statusStale}
    <div class="offline-update-list" role="status">
      <strong>Updates ready:</strong>
      <ul class="offline-update-ul">
        {#each updateAssets as u (u.asset_id)}
          <li>
            <span>
              <strong>{u.label}</strong>
              <small>Tier {u.tier} · {u.primary ? "primary" : "supporting"}</small>
            </span>
            <button
              type="button"
              class="btn btn-warning btn-xs"
              disabled={!canManageResources()}
              onclick={() => updateOne(u.asset_id)}
            >
              {syncingAsset === u.asset_id ? "Updating…" : "Update"}
            </button>
          </li>
        {/each}
      </ul>
      <button
        type="button"
        class="btn btn-primary btn-sm"
        disabled={!canManageResources()}
        onclick={() => updateAllOutdated()}
      >
        {syncingTier === -2 ? "Updating all…" : `Update all ${updateAssets.length} outdated`}
      </button>
    </div>
  {/if}

  {#if operationError}
    <div class="offline-operation-error" role="alert">
      <span>{operationError}</span>
      {#if retryTarget}
        <button type="button" class="btn btn-secondary btn-xs" disabled={disabled || loading || syncingTier !== null || syncingAsset !== null} onclick={retryLastOperation}>
          Retry
        </button>
      {/if}
    </div>
  {/if}

  <label class="scope-option force-option">
    <input type="checkbox" bind:checked={forceSync} disabled={!canManageResources()} />
    <span>
      <strong>Force re-download</strong>
      <small>Replace local files even when present (re-import into SQLite).</small>
    </span>
  </label>

  <div class="offline-actions">
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={!canManageResources()}
      onclick={() => runTier(0)}
    >
      {syncingTier === 0 ? "Syncing…" : "Sync Tier 0"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={!canManageResources()}
      onclick={() => runTier(1)}
    >
      {syncingTier === 1 ? "Syncing…" : "Sync Tier 1"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={!canManageResources() || !selectedSample}
      onclick={() => runTier(2)}
    >
      {syncingTier === 2 ? "Syncing…" : "Sync Tier 2"}
    </button>
    <button
      type="button"
      class="btn btn-primary btn-sm"
      disabled={!canManageResources()}
      onclick={runAll}
    >
      {syncingTier === -1 ? "Syncing all…" : "Sync all tiers"}
    </button>
    <Tooltip interactiveChildren interactiveClickBehavior="dismiss" label="Rebuild locus index" description="Rebuilds the local variant-locus index from the imported genome only.">
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={!canManageResources() || !selectedSample}
        onclick={rebuildTier2}
      >
        Rebuild locus index
      </button>
    </Tooltip>
    <button type="button" class="btn btn-secondary btn-sm" disabled={!canManageResources()} onclick={refresh}>
      Check updates
    </button>
  </div>

  {#if status}
    <div class="tier-grid">
      {#each status.tiers as tier}
        <div class="tier-card" class:tier-ready={tier.ready}>
          <h4>Tier {tier.tier}</h4>
          <ul class="asset-list">
            {#each tier.assets as asset}
              <li class:update={hasProvenOfflineUpdate(asset) && !statusStale}>
                <div class="asset-main">
                  <span class="asset-label">
                    {asset.label}
                    {#if isPrimaryCatalogId(asset.asset_id)}
                      <em class="primary-tag">primary</em>
                    {/if}
                  </span>
                  <span class="asset-meta">
                    {#if asset.row_count > 0 && asset.local_present}
                      {asset.row_count.toLocaleString()} indexed rows
                    {:else if asset.row_count > 0}
                      {asset.row_count.toLocaleString()} indexed rows; source file removed
                    {:else if asset.local_present}
                      {formatBytes(asset.local_bytes)} downloaded; not indexed yet
                    {:else}
                      not downloaded
                    {/if}
                    {#if hasProvenOfflineUpdate(asset) && !statusStale}
                      · <em>source update</em>
                    {/if}
                  </span>
                  <span class="asset-provenance">
                    {#if asset.version_label}
                      Installed file details: {asset.version_label}
                    {:else}
                      No version label provided by the server
                    {/if}
                    {#if asset.local_present && asset.downloaded_at}
                      · Downloaded {formatTimestamp(asset.downloaded_at)}
                    {/if}
                    {#if asset.indexed_at}
                      · Last indexed {formatTimestamp(asset.indexed_at)}
                    {/if}
                    {#if asset.last_checked_at}
                      · Checked {formatTimestamp(asset.last_checked_at)}
                    {/if}
                    {#if asset.remote_version_label === "File identifier changed"}
                      · The server's file identifier changed since download
                    {:else if asset.remote_version_label && asset.remote_version_label !== asset.version_label}
                      · Server file marker: {asset.remote_version_label}
                    {/if}
                    {#if asset.source_url}
                      · <a href={asset.source_url} target="_blank" rel="noreferrer">Source file</a>
                    {/if}
                  </span>
                  {#if hasProvenOfflineUpdate(asset) && !statusStale}
                    <span class="asset-change-note">
                      A newer source file is available. Update to replace the local copy.
                    </span>
                  {/if}
                  {#if asset.local_present && asset.row_count > 0 && asset.downloaded_at && (!asset.indexed_at || asset.downloaded_at > asset.indexed_at)}
                    <span class="asset-change-note">
                      The downloaded file is newer than the indexed database.
                    </span>
                  {/if}
                  {#if asset.source_changed_at}
                    <span class="asset-change-note">
                      Downloaded file bytes changed locally {formatTimestamp(asset.source_changed_at)}
                      {#if asset.previous_row_count !== null}
                        · indexed rows {asset.previous_row_count.toLocaleString()} → {asset.row_count.toLocaleString()}
                      {/if}
                      {#if asset.previous_version_label}
                        · previous file revision: {asset.previous_version_label}
                      {/if}
                      . Row totals show database size, not record-by-record differences.
                    </span>
                  {/if}
                </div>
                <div class="asset-row-actions">
                  {#if hasProvenOfflineUpdate(asset) && !statusStale}
                    <button
                      type="button"
                      class="btn btn-warning btn-xs inline-update"
                      disabled={!canManageResources()}
                      onclick={() => updateOne(asset.asset_id)}
                    >
                      {syncingAsset === asset.asset_id ? "Updating…" : "Update"}
                    </button>
                  {/if}
                  {#if asset.local_present || asset.asset_id === "gnomad_index_manifest"}
                    <button
                      type="button"
                      class="btn btn-secondary btn-xs"
                      disabled={!canManageResources()}
                      onclick={() => confirmRemoveDownload(asset)}
                    >
                      {deletingAsset === asset.asset_id
                        ? "Removing…"
                        : asset.asset_id === "gnomad_index_manifest" ? "Clear gnomAD downloads" : "Remove download"}
                    </button>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {/if}

  <details class="offline-manage">
    <summary>Manage reference downloads and databases</summary>
    <p class="offline-manage-help">
      Remove a download to free its source file while keeping its indexed data. Delete a database to remove the imported index while keeping source files. Both actions leave DNA profiles alone.
    </p>
    <div class="database-manage-list">
      {#each offlineDatabases as database (database.id)}
        <div class="database-manage-row">
          <span>
            <strong>{database.label}</strong>
            <small>{indexedCountFor(database).toLocaleString()} indexed rows</small>
          </span>
          <button
            type="button"
            class="btn btn-secondary btn-xs"
            disabled={!canManageResources()}
            onclick={() => confirmDeleteDatabase(database)}
          >
            {deletingDatabase === database.id ? "Deleting…" : "Delete database"}
          </button>
        </div>
      {/each}
    </div>
    <div class="offline-purge-row">
      <span>
        <strong>Start over with reference data</strong>
        <small>Removes all app-managed catalog downloads and imported reference databases. DNA profiles remain.</small>
      </span>
      <button
        type="button"
        class="btn btn-danger btn-sm"
        disabled={!canManageResources()}
        onclick={confirmPurgeDownloads}
      >
        {purgingDownloads ? "Purging…" : "Purge all downloads"}
      </button>
    </div>
  </details>

  {#if status?.tiers.flatMap((tier) => tier.assets).some((asset) => asset.label.startsWith("ClinPGx"))}
    <p class="clinpgx-guidance">
      The ClinPGx API is still under development. Its API schema version is not a data release number; these downloaded files show the server's file details and when this app downloaded or indexed them. CPIC publishes its database/API releases separately. Structured data may omit guideline-specific caveats, so for medication decisions consult the full
      <a href="https://api.clinpgx.org/" target="_blank" rel="noreferrer">ClinPGx API notes</a> and the relevant
      <a href="https://www.clinpgx.org/page/cpicResources" target="_blank" rel="noreferrer">CPIC guideline</a>. See the
      <a href="https://github.com/cpicpgx/cpic-data/releases" target="_blank" rel="noreferrer">CPIC database/API release history</a>; its version is separate from the file markers above.
    </p>
  {/if}
</section>

<style>
  .offline-data-panel {
    margin-top: 1rem;
  }
  .offline-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .offline-help {
    font-size: 0.85rem;
    opacity: 0.85;
    margin: 0.5rem 0 0.75rem;
    line-height: 1.4;
  }
  .update-badge {
    font-size: 0.8rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: var(--status-success-bg);
    color: var(--status-success-text);
  }
  .update-badge.has-updates {
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
  }
  .update-badge.status-stale {
    background: var(--status-info-bg);
    color: var(--status-info-text);
  }
  .offline-update-list {
    margin: 0 0 0.85rem;
    padding: 0.65rem 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--status-warning-border);
    background: var(--status-warning-bg);
  }
  .offline-status-error,
  .offline-operation-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-top: 0.65rem;
    padding: 0.55rem 0.7rem;
    border: 1px solid var(--status-danger-border);
    border-radius: 8px;
    background: var(--status-danger-bg);
    color: var(--status-danger-text);
    font-size: 0.8rem;
  }
  .offline-status-attention {
    border-color: var(--status-warning-border);
    background: var(--status-warning-bg);
    color: var(--status-warning-text);
  }
  .offline-update-ul {
    list-style: none;
    margin: 0.4rem 0 0.6rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .offline-update-ul li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    align-items: center;
  }
  .offline-update-ul small {
    display: block;
    opacity: 0.75;
    font-size: 0.72rem;
  }
  .primary-tag {
    font-size: 0.65rem;
    font-style: normal;
    margin-left: 0.35rem;
    opacity: 0.7;
  }
  .inline-update {
    margin-left: 0.35rem;
  }
  .force-option {
    margin-bottom: 0.75rem;
  }
  .offline-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  .tier-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0.75rem;
  }
  .tier-card {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 0.75rem;
    background: var(--surface-subtle);
  }
  .tier-card.tier-ready {
    border-color: var(--status-success-border);
  }
  .tier-card h4 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
  }
  .asset-list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.8rem;
  }
  .asset-list li {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.3rem;
    padding: 0.35rem 0;
    border-bottom: 1px solid var(--border-color);
  }
  .asset-list li.update .asset-label {
    color: var(--status-warning-text);
  }
  .asset-label {
    font-weight: 500;
  }
  .asset-meta {
    opacity: 0.75;
    font-size: 0.75rem;
  }
  .asset-main {
    display: grid;
    gap: 0.12rem;
    min-width: 0;
    width: 100%;
  }
  .asset-provenance,
  .asset-change-note {
    display: block;
    color: var(--text-muted);
    font-size: 0.72rem;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .asset-provenance a {
    color: var(--accent-text, var(--status-info-text));
    text-decoration: underline;
    text-underline-offset: 0.12em;
  }
  .asset-change-note {
    color: var(--status-info-text);
  }
  .asset-row-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
  }
  .offline-manage {
    margin-top: 0.9rem;
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: var(--surface-subtle);
  }
  .offline-manage summary {
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .offline-manage-help,
  .clinpgx-guidance {
    margin: 0.55rem 0 0.7rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    line-height: 1.45;
  }
  .clinpgx-guidance a {
    color: var(--accent-text, var(--status-info-text));
    text-decoration: underline;
    text-underline-offset: 0.12em;
  }
  .database-manage-list {
    display: grid;
    gap: 0.4rem;
  }
  .database-manage-row,
  .offline-purge-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-width: 0;
    padding: 0.55rem 0;
    border-top: 1px solid var(--border-color);
  }
  .database-manage-row > span,
  .offline-purge-row > span {
    display: grid;
    gap: 0.15rem;
    min-width: 0;
  }
  .database-manage-row small,
  .offline-purge-row small {
    color: var(--text-muted);
    font-size: 0.72rem;
    line-height: 1.35;
  }
  .database-manage-row button,
  .offline-purge-row button {
    flex: 0 0 auto;
  }
  .offline-purge-row {
    margin-top: 0.3rem;
    padding-top: 0.7rem;
  }
  @media (max-width: 480px) {
    .database-manage-row,
    .offline-purge-row {
      align-items: flex-start;
      flex-direction: column;
    }
    .database-manage-row button,
    .offline-purge-row button {
      width: 100%;
    }
  }
</style>
