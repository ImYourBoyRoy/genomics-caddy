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
    expect(sidebarSource).toContain('await handleSyncAsset(item.asset_id, forceRedownload, false);');
    expect(sidebarSource).toContain('await refreshStatusInBackground();');
  });
});
