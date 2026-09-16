import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const sidebarSource = readFileSync(resolve(process.cwd(), 'src/lib/components/sidebar/Sidebar.svelte'), 'utf8');

describe('Sidebar update-state ordering', () => {
  it('uses the installed-asset predicate for every visible update state', () => {
    expect(sidebarSource).toContain('hasFreshProvenOfflineUpdate');
    expect(sidebarSource).toContain('hasFreshProvenOfflineUpdate(offlineStatusFresh, findAsset(db.tierNum, db.assetId))');
    expect(sidebarSource).toContain('hasFreshProvenOfflineUpdate(offlineStatusFresh, companion)');
    expect(sidebarSource).not.toContain('hasProvenOfflineUpdate(findAsset(db.tierNum, db.assetId))');
    expect(sidebarSource).not.toContain('hasProvenOfflineUpdate(companion)');
    expect(sidebarSource).not.toContain('if (asset.update_available)');
    expect(sidebarSource).not.toContain('if (companion?.update_available)');
  });

  it('does not advertise a missing collapsed data panel as an aria-controls target', () => {
    expect(sidebarSource).toContain('aria-controls={isPanelCollapsed ? undefined : "data-updates-panel"}');
  });

  it('ignores stale status probes and waits for one final bulk-update refresh', () => {
    expect(sidebarSource).toContain('let statusRefreshGeneration = 0;');
    expect(sidebarSource).toContain('let offlineStatusFresh = $state(false);');
    expect(sidebarSource).toContain("type StatusRefreshResult = 'ready' | 'stale' | 'error';");
    expect(sidebarSource).toContain('offlineStatusFresh = false;');
    expect(sidebarSource).toContain('offlineStatusFresh = isCompleteOfflineStatus(status);');
    expect(sidebarSource).toContain('offlineStatusFailureMessage(status)');
    expect(sidebarSource).toContain('offlineStatusFresh ? listOfflineUpdates(offlineStatus) : []');
    expect(sidebarSource).toContain('listOfflineUpdates(offlineStatusFresh ? offlineStatus : null)');
    expect(sidebarSource).toContain('async function refreshStatusInBackground(): Promise<StatusRefreshResult>');
    expect(sidebarSource).toContain("if (!failureMessage && finalStatus === 'error')");
    expect(sidebarSource).toContain("failureRetry = { kind: 'status' };");
    expect(sidebarSource).toContain('const generation = ++statusRefreshGeneration;');
    expect(sidebarSource).toContain('if (generation !== statusRefreshGeneration) return;');
    expect(sidebarSource).toContain('async function handleSyncAsset(assetId: string, force: boolean, refreshAfter = true)');
    expect(sidebarSource).toContain('if (!await handleSyncAsset(item.asset_id, forceRedownload, false)) failed = true;');
    expect(sidebarSource).toContain('await refreshStatusInBackground();');
    expect(sidebarSource).toContain('onOfflineStatusChange?.(offlineStatus, offlineStatusFresh);');
  });

  it('does not leave row actions enabled while a fresh status probe is running', () => {
    expect(sidebarSource).toContain('isCheckingStatus || !offlineStatus');
    expect(sidebarSource).toContain('isCheckingStatus || !offlineStatus || sweepRunning');
  });

  it('shows a concise collapsed-row status after the authoritative probe settles', () => {
    expect(sidebarSource).toContain("if (isCheckingStatus || updatePhase === 'checking') return 'Checking…';");
    expect(sidebarSource).toContain("if (updatePhase === 'attention' && updateRetry?.kind === 'status') return 'Remote check incomplete';");
    expect(sidebarSource).toContain("if (updatePhase === 'error') return 'Update issue';");
    expect(sidebarSource).toContain("if (offlineStatusFresh && updatesAvailable === 0 && missingPrimaryCount === 0) return 'Current';");
    expect(sidebarSource).toContain('class="resource-status-pill"');
    expect(sidebarSource).toContain('class:resource-status-current={collapsedStatusLabel === \'Current\'}');
    expect(sidebarSource).toContain('class:resource-status-incomplete={collapsedStatusLabel === \'Remote check incomplete\'}');
    expect(sidebarSource).toContain('class:resource-status-error={collapsedStatusLabel === \'Update issue\'}');
    expect(sidebarSource).toContain("'attention',\n          offlineStatusFailureMessage(status)");
  });

  it('distinguishes local catalog availability from remote freshness', () => {
    expect(sidebarSource).toContain('let localPrimarySummary = $derived.by(() => {');
    expect(sidebarSource).toContain('<strong>Local catalogs:</strong>');
    expect(sidebarSource).toContain('Imported DNA profiles and remote freshness are checked separately.');
    expect(sidebarSource).not.toContain('asset.local_bytes < 64 * 1024');
    expect(sidebarSource).toContain('Downloaded · not indexed yet (${sizeStr})');
  });

  it('keeps an explicit retry path for update failures without adding persistent warning copy', () => {
    expect(sidebarSource).toContain('type UpdateRetryTarget =');
    expect(sidebarSource).toContain('let updateRetry = $state<UpdateRetryTarget | null>(null);');
    expect(sidebarSource).toContain("{ kind: 'status' }");
    expect(sidebarSource).toContain("{ kind: 'asset', assetId, force }");
    expect(sidebarSource).toContain("{ kind: 'missing' }");
    expect(sidebarSource).toContain('function handleRetryUpdate()');
    expect(sidebarSource).toContain("{#if (updatePhase === 'attention' || updatePhase === 'error') && updateRetry}");
    expect(sidebarSource).toContain('aria-live="polite"');
    expect(sidebarSource).toContain('resource-update-retry');
    expect(sidebarSource).toContain('if (failureMessage && failureRetry) setUpdateState');
    expect(sidebarSource).toContain('if (!await handleSyncAsset(item.asset_id, forceRedownload, false)) failed = true;');
    expect(sidebarSource).toContain("{ kind: 'outdated' }");
  });

  it('allows each installed phase to paint before the next update phase replaces it', () => {
    expect(sidebarSource).toContain('async function allowUpdateStatePaint(): Promise<void>');
    expect(sidebarSource).toContain('requestAnimationFrame(() => requestAnimationFrame(() => resolve()))');
    expect(sidebarSource.match(/await allowUpdateStatePaint\(\);/g)?.length).toBe(4);
  });
});
