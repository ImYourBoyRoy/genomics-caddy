import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./UpdateBanner.svelte', import.meta.url), 'utf8');
const host = readFileSync(new URL('./AppUpdateHost.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles/components/update-banner.css', import.meta.url), 'utf8');

describe('signed update banner', () => {
  it('is a live status strip with install and dismiss actions', () => {
    expect(source).toContain('role="status"');
    expect(source).toContain('aria-live="polite"');
    expect(source).toContain('class="app-update-banner no-print"');
    expect(source).toContain("kind === 'portable' ? 'Replace…' : 'Install…'");
    expect(source).toContain('Not now');
  });

  it('uses semantic theme tokens and keeps action targets large enough', () => {
    expect(styles).toContain('var(--status-info-soft-bg)');
    expect(styles).toContain('var(--status-info-border)');
    expect(styles).toContain('min-height: 44px;');
    expect(styles).not.toMatch(/(?:#[0-9a-f]{3,8}\b|rgba?\(|hsla?\()/i);
  });

  it('checks quietly on launch, exposes a manual check, and requires confirmation before download', () => {
    expect(host).toContain('checkForAppUpdate({ quiet, previous: pendingUpdate })');
    expect(host).toContain('getVersion()');
    expect(host).toContain('checkNow: () => { void checkManually(); }');
    expect(host).toContain('installAvailable: () => promptInstall()');
    expect(host).toContain('isTauri()');
    expect(host).toContain('dialogStore.confirm(');
    expect(host).toContain("installKind === 'portable' ? 'Replace this copy' : 'Install update'");
    expect(host).toContain('kind: installKind');
    expect(host).toContain("dialogStore.alert(");
    expect(host).toContain('Could not apply the signed update.');
    expect(host).not.toContain('throw error');
  });
});
