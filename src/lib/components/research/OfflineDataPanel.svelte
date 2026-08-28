<!-- ./src/lib/components/research/OfflineDataPanel.svelte -->
<script lang="ts">
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import {
    checkOfflineDataUpdates,
    syncOfflineDataTier,
    syncAllOfflineData,
    syncSingleOfflineAsset,
    buildOfflineTier2,
  } from "../../api/tauri";
  import type { OfflineUpdateCheck, OfflineSyncResult } from "../../types/research";
  import { listOfflineUpdates } from "../../utils/offlineUpdates";
  import { isPrimaryCatalogId } from "../../utils/primaryCatalogs";
  import Tooltip from "../common/Tooltip.svelte";

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

  let updateAssets = $derived(listOfflineUpdates(status));

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function refresh(): Promise<boolean> {
    const generation = ++statusRefreshGeneration;
    loading = true;
    statusStale = true;
    try {
      const nextStatus = await checkOfflineDataUpdates();
      if (generation !== statusRefreshGeneration) return false;
      status = nextStatus;
      statusStale = false;
      statusError = '';
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

  function setOperationError(message: string, target: RetryTarget) {
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
          ? `${updateAssets.length} newer version(s) available`
          : "Up to date"}
      </span>
    {:else if status}
      <span class="update-badge status-stale">Status needs checking</span>
    {/if}
  </div>

  {#if statusError}
    <div class="offline-status-error" role="status">
      <span>{statusError}</span>
      <button type="button" class="btn btn-secondary btn-xs" disabled={disabled || loading} onclick={refresh}>
        {loading ? "Checking…" : "Retry"}
      </button>
    </div>
  {/if}

  <p class="offline-help">
  Tier 0: GWAS catalog, liftover chain, gnomAD manifest. Tier 1: ClinVar, PharmGKB, ClinGen, MANE.
  Tier 2: dbSNP lifecycle JSON + variant locus index from your genotypes. Downloads check remote size before fetching.
  Update badges here match the sidebar (all tier assets, not only the four primary catalogs).
  {#if status?.indexed_summary}
    Indexed: {status.indexed_summary.gwas_rows.toLocaleString()} GWAS ·
    {status.indexed_summary.clinvar_rows.toLocaleString()} ClinVar ·
    {status.indexed_summary.variant_locus_rows.toLocaleString()} locus rows.
  {/if}
  </p>

  {#if updateAssets.length > 0 && !statusStale}
    <div class="offline-update-list" role="status">
      <strong>Named updates ready:</strong>
      <ul class="offline-update-ul">
        {#each updateAssets as u (u.asset_id)}
          <li>
            <span>
              <strong>{u.label}</strong>
              <small>Tier {u.tier} · {u.primary ? "primary" : "supporting"} · {u.asset_id}</small>
            </span>
            <button
              type="button"
              class="btn btn-warning btn-xs"
              disabled={disabled || syncingTier !== null || syncingAsset !== null}
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
        disabled={disabled || syncingTier !== null || syncingAsset !== null}
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
    <input type="checkbox" bind:checked={forceSync} disabled={disabled} />
    <span>
      <strong>Force re-download</strong>
      <small>Replace local files even when present (re-import into SQLite).</small>
    </span>
  </label>

  <div class="offline-actions">
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null || syncingAsset !== null}
      onclick={() => runTier(0)}
    >
      {syncingTier === 0 ? "Syncing…" : "Sync Tier 0"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null || syncingAsset !== null}
      onclick={() => runTier(1)}
    >
      {syncingTier === 1 ? "Syncing…" : "Sync Tier 1"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null || syncingAsset !== null || !selectedSample}
      onclick={() => runTier(2)}
    >
      {syncingTier === 2 ? "Syncing…" : "Sync Tier 2"}
    </button>
    <button
      type="button"
      class="btn btn-primary btn-sm"
      disabled={disabled || syncingTier !== null || syncingAsset !== null}
      onclick={runAll}
    >
      {syncingTier === -1 ? "Syncing all…" : "Sync all tiers"}
    </button>
    <Tooltip interactiveChildren interactiveClickBehavior="dismiss" label="Rebuild locus index" description="Rebuilds the local variant-locus index from the imported genome only.">
      <button
        type="button"
        class="btn btn-secondary btn-sm"
        disabled={disabled || syncingTier !== null || syncingAsset !== null || !selectedSample}
        onclick={rebuildTier2}
      >
        Rebuild locus index
      </button>
    </Tooltip>
    <button type="button" class="btn btn-secondary btn-sm" disabled={disabled || loading} onclick={refresh}>
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
              <li class:update={asset.update_available && !statusStale}>
                <span class="asset-label">
                  {asset.label}
                  {#if isPrimaryCatalogId(asset.asset_id)}
                    <em class="primary-tag">primary</em>
                  {/if}
                </span>
                <span class="asset-meta">
                  {#if asset.row_count > 0}
                    {asset.row_count.toLocaleString()} rows
                  {:else if asset.local_present}
                    {formatBytes(asset.local_bytes)} on disk
                  {:else}
                    missing
                  {/if}
                  {#if asset.update_available && !statusStale}
                    · <em>update</em>
                    <button
                      type="button"
                      class="btn btn-warning btn-xs inline-update"
                      disabled={disabled || syncingTier !== null || syncingAsset !== null}
                      onclick={() => updateOne(asset.asset_id)}
                    >
                      {syncingAsset === asset.asset_id ? "…" : "Update"}
                    </button>
                  {/if}
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
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
</style>
