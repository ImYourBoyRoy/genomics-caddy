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

  let updateAssets = $derived(listOfflineUpdates(status));

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function refresh() {
    const generation = ++statusRefreshGeneration;
    loading = true;
    try {
      const nextStatus = await checkOfflineDataUpdates();
      if (generation !== statusRefreshGeneration) return;
      status = nextStatus;
    } catch (e: unknown) {
      if (generation !== statusRefreshGeneration) return;
      const msg = e instanceof Error ? e.message : String(e);
      onLog?.(`Offline data status failed: ${msg}`);
      status = null;
    } finally {
      if (generation === statusRefreshGeneration) loading = false;
    }
  }

  async function runTier(tier: number) {
    syncingTier = tier;
    onLog?.(`Syncing offline Tier ${tier}${forceSync ? " (force)" : ""}…`);
    try {
      const result = await syncOfflineDataTier(
        tier,
        forceSync,
        tier === 2 ? selectedSample?.id : undefined
      );
      reportResult(result);
      await refresh();
    } catch (e: unknown) {
      onLog?.(`Tier ${tier} sync failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      syncingTier = null;
    }
  }

  async function runAll() {
    syncingTier = -1;
    onLog?.(`Syncing all offline tiers${forceSync ? " (force)" : ""}…`);
    try {
      const results = await syncAllOfflineData(forceSync, selectedSample?.id);
      for (const r of results) {
        reportResult(r);
      }
      await refresh();
    } catch (e: unknown) {
      onLog?.(`Full offline sync failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      syncingTier = null;
    }
  }

  async function updateOne(assetId: string, refreshAfter = true, allowBulk = false) {
    if (syncingAsset || (syncingTier !== null && !allowBulk)) return;
    syncingAsset = assetId;
    onLog?.(`Updating ${assetId}${forceSync ? " (force)" : ""}…`);
    try {
      const result = await syncSingleOfflineAsset(
        assetId,
        forceSync,
        assetId === "tier2_variant_locus" ? selectedSample?.id : undefined
      );
      reportResult(result);
      if (refreshAfter) await refresh();
    } catch (e: unknown) {
      onLog?.(`Update ${assetId} failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      syncingAsset = null;
    }
  }

  async function updateAllOutdated() {
    if (syncingAsset || syncingTier !== null || updateAssets.length === 0) return;
    syncingTier = -2;
    onLog?.(`Updating ${updateAssets.length} outdated asset(s)…`);
    try {
      for (const u of updateAssets) {
        await updateOne(u.asset_id, false, true);
      }
      await refresh();
    } catch (e: unknown) {
      onLog?.(`Update-all failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      syncingTier = null;
    }
  }

  async function rebuildTier2() {
    if (!selectedSample) {
      onLog?.("Select a DNA profile to build Tier 2 variant locus index.");
      return;
    }
    syncingTier = 2;
    try {
      const result = await buildOfflineTier2(selectedSample.id);
      reportResult(result);
      await refresh();
    } catch (e: unknown) {
      onLog?.(`Tier 2 build failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      syncingTier = null;
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
      <ActivityPulse message="Checking sources…" accent="#34d399" maxWidth="180px" />
    {:else if status}
      <span class="update-badge" class:has-updates={updateAssets.length > 0}>
        {updateAssets.length > 0
          ? `${updateAssets.length} newer version(s) available`
          : "Up to date"}
      </span>
    {/if}
  </div>

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

  {#if updateAssets.length > 0}
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
        onclick={updateAllOutdated}
      >
        {syncingTier === -2 ? "Updating all…" : `Update all ${updateAssets.length} outdated`}
      </button>
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
              <li class:update={asset.update_available}>
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
                  {#if asset.update_available}
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
    background: rgba(52, 211, 153, 0.15);
    color: #34d399;
  }
  .update-badge.has-updates {
    background: rgba(251, 191, 36, 0.15);
    color: #fbbf24;
  }
  .offline-update-list {
    margin: 0 0 0.85rem;
    padding: 0.65rem 0.75rem;
    border-radius: 8px;
    border: 1px solid rgba(251, 191, 36, 0.35);
    background: rgba(251, 191, 36, 0.08);
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
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 0.75rem;
    background: rgba(0, 0, 0, 0.15);
  }
  .tier-card.tier-ready {
    border-color: rgba(52, 211, 153, 0.35);
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
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  .asset-list li.update .asset-label {
    color: #fbbf24;
  }
  .asset-label {
    font-weight: 500;
  }
  .asset-meta {
    opacity: 0.75;
    font-size: 0.75rem;
  }
</style>
