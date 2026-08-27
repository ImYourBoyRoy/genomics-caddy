import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const panelSource = readFileSync(resolve(process.cwd(), 'src/lib/components/research/OfflineDataPanel.svelte'), 'utf8');

describe('OfflineDataPanel update-state ordering', () => {
  it('invalidates stale refreshes and performs one final refresh after bulk updates', () => {
    expect(panelSource).toContain('let statusRefreshGeneration = 0;');
    expect(panelSource).toContain('const generation = ++statusRefreshGeneration;');
    expect(panelSource).toContain('if (generation !== statusRefreshGeneration) return;');
    expect(panelSource).toContain('async function updateOne(assetId: string, refreshAfter = true, allowBulk = false)');
    expect(panelSource).toContain('syncingTier !== null && !allowBulk');
    expect(panelSource).toContain('await updateOne(u.asset_id, false, true);');
    expect(panelSource).toContain('await refresh();');
  });
});
