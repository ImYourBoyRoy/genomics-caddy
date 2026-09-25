import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./EmptyState.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles/components/empty-state.css', import.meta.url), 'utf8');

describe('welcome data-controls copy', () => {
  it('describes the responsive data-controls entry point without assuming a visible sidebar', () => {
    expect(source).toContain('Use Data controls for downloads, profiles, and progress while you work.');
    expect(source).not.toContain('The left sidebar stays available');
  });

  it('counts updates only for assets that are already installed locally', () => {
    expect(source).toContain('countFreshInstalledUpdates(offlineStatus, offlineStatusFresh, PRIMARY_CATALOG_IDS)');
    expect(source).not.toContain('if (asset?.update_available) updates += 1;');
  });

  it('keeps the welcome stack inside the scroll origin on short scaled viewports', () => {
    expect(styles).toContain('width: min(100%, 45rem);');
    expect(styles).toContain('box-sizing: border-box;');
    expect(styles).toContain('@media (max-height: 800px)');
    expect(styles).toContain('justify-content: flex-start;');
    expect(styles).toContain('min-height: auto;');
  });

  it('styles the help action in the static welcome sheet for the first WebKit paint', () => {
    expect(source).toContain('class="welcome-help-icon"');
    expect(source).toContain('class="welcome-help-arrow"');
    expect(styles).toContain('.welcome-help-link {');
    expect(styles).toContain('background: var(--status-accent-bg);');
    expect(styles).toContain('.welcome-help-link:focus-visible {');
    expect(source).not.toMatch(/<style>[\s\S]*?\.welcome-help-link/);
  });
});
