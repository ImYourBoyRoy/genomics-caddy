<!-- ./src/lib/components/research/OfflineDataPanel.svelte -->
<script lang="ts">
  import ActivityPulse from "../common/loading/ActivityPulse.svelte";
  import {
    checkOfflineDataUpdates,
    syncOfflineDataTier,
    syncAllOfflineData,
    buildOfflineTier2,
  } from "../../api/tauri";
  import type { OfflineUpdateCheck, OfflineSyncResult } from "../../types/research";

  interface Props {
    selectedSample: { id: number; name: string } | null;
    disabled?: boolean;
    onLog?: (msg: string) => void;
  }

  let { selectedSample = null, disabled = false, onLog }: Props = $props();

  let status = $state<OfflineUpdateCheck | null>(null);
  let loading = $state(false);
  let syncingTier = $state<number | null>(null);
  let forceSync = $state(false);

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function refresh() {
    loading = true;
    try {
      status = await checkOfflineDataUpdates();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      onLog?.(`Offline data status failed: ${msg}`);
      status = null;
    } finally {
      loading = false;
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
      <span class="update-badge" class:has-updates={status.total_updates_available > 0}>
        {status.total_updates_available > 0
          ? `${status.total_updates_available} update(s) available`
          : "Up to date"}
      </span>
    {/if}
  </div>

  <p class="offline-help">
  Tier 0: GWAS catalog, liftover chain, gnomAD manifest. Tier 1: ClinVar, PharmGKB, ClinGen, MANE.
  Tier 2: dbSNP lifecycle JSON + variant locus index from your genotypes. Downloads check remote size before fetching.
  {#if status?.indexed_summary}
    Indexed: {status.indexed_summary.gwas_rows.toLocaleString()} GWAS ·
    {status.indexed_summary.clinvar_rows.toLocaleString()} ClinVar ·
    {status.indexed_summary.variant_locus_rows.toLocaleString()} locus rows.
  {/if}
  </p>

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
      disabled={disabled || syncingTier !== null}
      onclick={() => runTier(0)}
    >
      {syncingTier === 0 ? "Syncing…" : "Sync Tier 0"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null}
      onclick={() => runTier(1)}
    >
      {syncingTier === 1 ? "Syncing…" : "Sync Tier 1"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null || !selectedSample}
      onclick={() => runTier(2)}
    >
      {syncingTier === 2 ? "Syncing…" : "Sync Tier 2"}
    </button>
    <button
      type="button"
      class="btn btn-primary btn-sm"
      disabled={disabled || syncingTier !== null}
      onclick={runAll}
    >
      {syncingTier === -1 ? "Syncing all…" : "Sync all tiers"}
    </button>
    <button
      type="button"
      class="btn btn-secondary btn-sm"
      disabled={disabled || syncingTier !== null || !selectedSample}
      onclick={rebuildTier2}
      title="Rebuild variant locus index from imported genotypes only"
    >
      Rebuild locus index
    </button>
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
                <span class="asset-label">{asset.label}</span>
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
