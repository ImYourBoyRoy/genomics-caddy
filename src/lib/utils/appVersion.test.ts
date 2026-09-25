import { describe, expect, it, vi } from 'vitest';

const tauriState = vi.hoisted(() => ({ enabled: false }));
const getVersionMock = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => tauriState.enabled,
}));
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: getVersionMock,
}));

import { PACKAGE_VERSION, getRuntimeAppVersion, normalizeAppVersion } from './appVersion';

describe('runtime app version', () => {
  it('normalizes display labels without changing the package fallback', () => {
    expect(normalizeAppVersion('v2.4.1')).toBe('2.4.1');
    expect(normalizeAppVersion('  2.4.1  ')).toBe('2.4.1');
    expect(normalizeAppVersion('')).toBe(PACKAGE_VERSION);
  });

  it('uses the installed Tauri version when the desktop bridge is available', async () => {
    tauriState.enabled = true;
    getVersionMock.mockResolvedValueOnce('v0.2.5');

    await expect(getRuntimeAppVersion()).resolves.toBe('0.2.5');
  });

  it('falls back to the package version when the bridge cannot answer', async () => {
    tauriState.enabled = true;
    getVersionMock.mockRejectedValueOnce(new Error('bridge unavailable'));

    await expect(getRuntimeAppVersion()).resolves.toBe(PACKAGE_VERSION);
  });
});
