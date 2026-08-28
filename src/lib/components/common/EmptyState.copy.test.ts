import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./EmptyState.svelte', import.meta.url), 'utf8');

describe('welcome data-controls copy', () => {
  it('describes the responsive data-controls entry point without assuming a visible sidebar', () => {
    expect(source).toContain('Use Data controls for downloads, profiles, and progress while you work.');
    expect(source).not.toContain('The left sidebar stays available');
  });

  it('counts updates only for assets that are already installed locally', () => {
    expect(source).toContain('isCompleteOfflineStatus(offlineStatus) && asset?.local_present && asset.update_available');
    expect(source).not.toContain('if (asset?.update_available) updates += 1;');
  });
});
