import { beforeEach, describe, expect, it, vi } from 'vitest';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';
import {
  DISMISSED_VERSION_KEY,
  checkForAppUpdate,
  formatUpdateBannerCopy,
  formatVersionLabel,
  getDismissedUpdateVersion,
  installAppUpdate,
  setDismissedUpdateVersion,
  shouldShowUpdateBanner,
} from './updater';

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}));

function mockLocalStorage() {
  const store = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => {
      store.set(key, value);
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => {
      store.clear();
    },
  });
}

describe('updater helpers', () => {
  beforeEach(() => {
    mockLocalStorage();
    vi.mocked(check).mockReset();
    vi.mocked(relaunch).mockReset();
  });

  it('normalizes version labels and banner copy', () => {
    expect(formatVersionLabel('0.3.0')).toBe('v0.3.0');
    expect(formatVersionLabel('v0.3.0')).toBe('v0.3.0');
    expect(formatVersionLabel('')).toBe('');
    expect(formatUpdateBannerCopy({ version: 'v0.3.0', installing: false, progress: 0 }))
      .toBe('Genomics Caddy v0.3.0 is available');
    expect(formatUpdateBannerCopy({ version: 'v0.3.0', installing: true, progress: 0 }))
      .toBe('Downloading signed update…');
    expect(formatUpdateBannerCopy({ version: 'v0.3.0', installing: true, progress: 25 }))
      .toBe('Downloading signed update… 25%');
    expect(formatUpdateBannerCopy({ version: 'v0.3.0', installing: true, progress: 100 }))
      .toBe('Installing signed update…');
  });

  it('hides the banner after that exact version is dismissed', () => {
    expect(shouldShowUpdateBanner('0.3.0')).toBe(true);
    setDismissedUpdateVersion('0.3.0');
    expect(getDismissedUpdateVersion()).toBe('0.3.0');
    expect(localStorage.getItem(DISMISSED_VERSION_KEY)).toBe('0.3.0');
    expect(shouldShowUpdateBanner('0.3.0')).toBe(false);
    expect(shouldShowUpdateBanner('0.3.1')).toBe(true);
  });

  it('returns available state when GitHub has a newer signed build', async () => {
    const update = { version: '0.3.0', close: vi.fn() } as unknown as Update;
    vi.mocked(check).mockResolvedValue(update);

    await expect(checkForAppUpdate({ quiet: true })).resolves.toMatchObject({
      state: 'available',
      update,
      message: '',
    });
    await expect(checkForAppUpdate({ quiet: false })).resolves.toMatchObject({
      state: 'available',
      message: 'Genomics Caddy v0.3.0 is available.',
    });
  });

  it('returns current when GitHub has no newer signed build', async () => {
    vi.mocked(check).mockResolvedValue(null);

    await expect(checkForAppUpdate({ quiet: true })).resolves.toEqual({
      state: 'current',
      message: '',
    });
    await expect(checkForAppUpdate({ quiet: false })).resolves.toEqual({
      state: 'current',
      message: 'You are on the latest signed Genomics Caddy release.',
    });
  });

  it('swallows quiet check errors and surfaces them for a manual check', async () => {
    vi.mocked(check).mockRejectedValue(new Error('network unavailable'));

    await expect(checkForAppUpdate({ quiet: true })).resolves.toMatchObject({
      state: 'error',
      message: '',
    });
    await expect(checkForAppUpdate({ quiet: false })).resolves.toMatchObject({
      state: 'error',
      message: 'Could not check for updates',
    });
  });

  it('requires confirmation, reports progress, releases the update resource, and relaunches', async () => {
    const progress: number[] = [];
    const update = {
      close: vi.fn().mockResolvedValue(undefined),
      downloadAndInstall: vi.fn().mockImplementation(async (onEvent: (event: DownloadEvent) => void) => {
        onEvent({ event: 'Started', data: { contentLength: 200 } });
        onEvent({ event: 'Progress', data: { chunkLength: 50 } });
        onEvent({ event: 'Finished' });
      }),
    } as unknown as Update;
    vi.mocked(relaunch).mockResolvedValue(undefined);

    await expect(installAppUpdate(update, { confirmed: false })).rejects.toThrow('Confirm');
    expect(update.downloadAndInstall).not.toHaveBeenCalled();

    await expect(installAppUpdate(update, {
      confirmed: true,
      onProgress: (percent) => progress.push(percent),
    })).resolves.toEqual({ installed: true, relaunched: true });

    expect(progress).toEqual([0, 25, 100]);
    expect(update.downloadAndInstall).toHaveBeenCalledOnce();
    expect(update.close).toHaveBeenCalledOnce();
    expect(relaunch).toHaveBeenCalledOnce();
  });

  it('reports an installed update when automatic relaunch is unavailable', async () => {
    const update = {
      close: vi.fn().mockResolvedValue(undefined),
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    } as unknown as Update;
    vi.mocked(relaunch).mockRejectedValueOnce(new Error('restart unavailable'));

    await expect(installAppUpdate(update, { confirmed: true })).resolves.toEqual({
      installed: true,
      relaunched: false,
    });
  });
});
