import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const sidebarSource = readFileSync(resolve(process.cwd(), 'src/lib/components/sidebar/Sidebar.svelte'), 'utf8');

describe('Sidebar update-state ordering', () => {
  it('ignores stale status probes and waits for one final bulk-update refresh', () => {
    expect(sidebarSource).toContain('let statusRefreshGeneration = 0;');
    expect(sidebarSource).toContain('const generation = ++statusRefreshGeneration;');
    expect(sidebarSource).toContain('if (generation !== statusRefreshGeneration) return;');
    expect(sidebarSource).toContain('async function handleSyncAsset(assetId: string, force: boolean, refreshAfter = true)');
    expect(sidebarSource).toContain('if (!await handleSyncAsset(item.asset_id, forceRedownload, false)) failed = true;');
    expect(sidebarSource).toContain('await refreshStatusInBackground();');
  });

  it('keeps an explicit retry path for update failures without adding persistent warning copy', () => {
    expect(sidebarSource).toContain('type UpdateRetryTarget =');
    expect(sidebarSource).toContain('let updateRetry = $state<UpdateRetryTarget | null>(null);');
    expect(sidebarSource).toContain("{ kind: 'status' }");
    expect(sidebarSource).toContain("{ kind: 'asset', assetId, force }");
    expect(sidebarSource).toContain("{ kind: 'missing' }");
    expect(sidebarSource).toContain('function handleRetryUpdate()');
    expect(sidebarSource).toContain("{#if updatePhase === 'error' && updateRetry}");
    expect(sidebarSource).toContain('aria-live="polite"');
    expect(sidebarSource).toContain('resource-update-retry');
    expect(sidebarSource).toContain('if (failureMessage && failureRetry) setUpdateState');
    expect(sidebarSource).toContain('if (!await handleSyncAsset(item.asset_id, forceRedownload, false)) failed = true;');
    expect(sidebarSource).toContain("{ kind: 'outdated' }");
  });
});
