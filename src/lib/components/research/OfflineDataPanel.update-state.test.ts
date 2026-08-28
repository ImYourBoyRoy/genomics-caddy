import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const panelSource = readFileSync(resolve(process.cwd(), 'src/lib/components/research/OfflineDataPanel.svelte'), 'utf8');

describe('OfflineDataPanel update-state ordering', () => {
  it('does not render an update state for a missing local asset', () => {
    expect(panelSource).toContain('hasProvenOfflineUpdate');
    expect(panelSource).toContain('offlineStatusFailureMessage(nextStatus)');
    expect(panelSource).toContain('hasProvenOfflineUpdate(asset) && !statusStale');
    expect(panelSource).not.toContain('asset.update_available && !statusStale');
  });

  it('invalidates stale refreshes and performs one final refresh after bulk updates', () => {
    expect(panelSource).toContain('let statusRefreshGeneration = 0;');
    expect(panelSource).toContain('const generation = ++statusRefreshGeneration;');
    expect(panelSource).toContain('if (generation !== statusRefreshGeneration) return false;');
    expect(panelSource).toContain('async function updateOne(assetId: string, refreshAfter = true, allowBulk = false)');
    expect(panelSource).toContain('syncingTier !== null && !allowBulk');
    expect(panelSource).toContain('const succeeded = await updateOne(assetId, false, true);');
    expect(panelSource).toContain('await refresh();');
    expect(panelSource).toContain('let statusError = $state(\'\');');
    expect(panelSource).toContain('let statusStale = $state(false);');
    expect(panelSource).toContain('let retryTarget = $state<RetryTarget | null>(null);');
    expect(panelSource).toContain('async function retryLastOperation()');
    expect(panelSource).toContain('class="offline-status-error" role="status"');
    expect(panelSource).toContain('class="offline-operation-error" role="alert"');
    expect(panelSource).toContain('kind: \'outdated\'; assetIds: string[]; force: boolean');
    expect(panelSource).toContain('statusError = \'Could not verify the latest resource status.\';');
    expect(panelSource).toContain('clearOfflineUpdate(status, syncedAssetId)');
    expect(panelSource).toContain('for (const syncedAssetId of result.assets_synced)');
    expect(panelSource).toContain('if (failure) setOperationError(failure, { kind: \'tier\', tier, force: requestedForce });');
    expect(panelSource).toContain('if (failedAssetIds.length > 0)');
    expect(panelSource).toContain('forceSync = target.force;');
    expect(panelSource).toContain('Status needs checking');
    expect(panelSource).toContain('updateAssets.length > 0 && !statusStale');
    expect(panelSource).toContain('hasProvenOfflineUpdate(asset) && !statusStale');
  });
});
