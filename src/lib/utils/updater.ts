// ./src/lib/utils/updater.ts
/**
 * Desktop self-update helpers for Genomics Caddy.
 * Purpose: Quietly check GitHub Releases for a newer signed build, remember
 * dismissed versions, and apply only after an explicit confirm.
 * How to run: imported by AppUpdateHost in the Tauri webview; unit-tested via
 * `pnpm test`.
 * Key inputs: `@tauri-apps/plugin-updater` check result, localStorage dismissal,
 *          install layout (`installer` vs Windows portable zip).
 * Key outputs: banner state, progress callbacks, relaunch or in-place replace.
 * Notes: Never logs genotypes. The updater talks only to GitHub for latest.json
 * and the signed artifact. Windows portable copies replace themselves; NSIS
 * copies still run the signed installer. Web/Vite preview is a no-op.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';

export const DISMISSED_VERSION_KEY = 'genomics-caddy:dismissed-update-version';
const CHECK_TIMEOUT_MS = 10_000;

export type UpdateUiState = 'idle' | 'checking' | 'available' | 'current' | 'installing' | 'error';

export interface UpdateCheckResult {
  state: UpdateUiState;
  update?: Update;
  message: string;
}

export type UpdateInstallKind = 'installer' | 'portable';

export interface InstallUpdateOptions {
  confirmed: boolean;
  kind?: UpdateInstallKind;
  onProgress?: (percent: number) => void;
}

export interface InstallUpdateResult {
  installed: boolean;
  relaunched: boolean;
}

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(label)), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export function formatVersionLabel(version: string | undefined): string {
  const trimmed = version?.trim() ?? '';
  if (!trimmed) return '';
  return trimmed.startsWith('v') ? trimmed : `v${trimmed}`;
}

export function formatUpdateBannerCopy(input: {
  version: string;
  installing: boolean;
  progress: number;
  kind?: UpdateInstallKind;
}): string {
  if (!input.installing) return `Genomics Caddy ${input.version} is available`;
  if (input.kind === 'portable') {
    if (input.progress >= 100) return 'Replacing this portable copy…';
    if (input.progress > 0) return `Downloading signed portable zip… ${input.progress}%`;
    return 'Downloading signed portable zip…';
  }
  if (input.progress >= 100) return 'Installing signed update…';
  if (input.progress > 0) return `Downloading signed update… ${input.progress}%`;
  return 'Downloading signed update…';
}

export function getDismissedUpdateVersion(): string {
  try {
    return localStorage.getItem(DISMISSED_VERSION_KEY)?.trim() ?? '';
  } catch {
    return '';
  }
}

export function setDismissedUpdateVersion(version: string): void {
  try {
    localStorage.setItem(DISMISSED_VERSION_KEY, version.trim());
  } catch {
    // Restricted storage must not block dismissing the banner for this session.
  }
}

export function shouldShowUpdateBanner(version: string | undefined): boolean {
  const next = version?.trim() ?? '';
  if (!next) return false;
  return getDismissedUpdateVersion() !== next;
}

export async function checkForAppUpdate(options: {
  quiet?: boolean;
  previous?: Update;
} = {}): Promise<UpdateCheckResult> {
  const quiet = options.quiet === true;
  try {
    await options.previous?.close();
  } catch {
    // A stale updater resource must not block a fresh check.
  }

  try {
    const update = await withTimeout(check(), CHECK_TIMEOUT_MS, 'Update check timed out');
    if (!update) {
      return {
        state: 'current',
        message: quiet ? '' : 'You are on the latest signed Genomics Caddy release.',
      };
    }
    return {
      state: 'available',
      update,
      message: quiet
        ? ''
        : `Genomics Caddy ${formatVersionLabel(update.version)} is available.`,
    };
  } catch {
    return {
      state: 'error',
      message: quiet ? '' : 'Could not check for updates',
    };
  }
}

function progressPercent(event: DownloadEvent, downloaded: { bytes: number; total: number }): number {
  if (event.event === 'Started') {
    downloaded.bytes = 0;
    downloaded.total = event.data.contentLength ?? 0;
    return 0;
  }
  if (event.event === 'Progress') {
    downloaded.bytes += event.data.chunkLength;
    if (downloaded.total > 0) {
      return Math.min(99, Math.round((downloaded.bytes / downloaded.total) * 100));
    }
    return 0;
  }
  if (event.event === 'Finished') return 100;
  return 0;
}

export async function resolveUpdateInstallKind(): Promise<UpdateInstallKind> {
  try {
    const kind = await invoke<string>('get_update_channel');
    return kind === 'portable' ? 'portable' : 'installer';
  } catch {
    return 'installer';
  }
}

async function markUpdateInProgress(active: boolean): Promise<void> {
  try {
    await invoke('set_update_in_progress', { active });
  } catch {
    // Preview windows and missing commands must not block cancel or dismiss.
  }
}

export async function closeUpdateResource(update: Update | undefined): Promise<void> {
  if (!update) return;
  try {
    await update.close();
  } catch {
    // The installer may already have released the resource.
  }
}

async function replacePortableCopy(
  update: Update,
  onProgress?: (percent: number) => void,
): Promise<InstallUpdateResult> {
  const unlisten = await listen<{ percentage?: number }>('portable-update-progress', (event) => {
    onProgress?.(event.payload?.percentage ?? 0);
  });
  try {
    await invoke('apply_portable_update', { version: update.version });
    return { installed: true, relaunched: true };
  } finally {
    unlisten();
    await closeUpdateResource(update);
  }
}

export async function installAppUpdate(
  update: Update,
  options: InstallUpdateOptions,
): Promise<InstallUpdateResult> {
  if (!options.confirmed) {
    throw new Error('Confirm the update before downloading the signed installer.');
  }

  const kind = options.kind ?? 'installer';
  await markUpdateInProgress(true);
  let keepWindowUntilExit = false;
  try {
    if (kind === 'portable') {
      const result = await replacePortableCopy(update, options.onProgress);
      keepWindowUntilExit = true;
      return result;
    }

    const downloaded = { bytes: 0, total: 0 };
    try {
      await update.downloadAndInstall((event) => {
        options.onProgress?.(progressPercent(event, downloaded));
      });
    } finally {
      await closeUpdateResource(update);
    }

    try {
      await relaunch();
      return { installed: true, relaunched: true };
    } catch {
      return { installed: true, relaunched: false };
    }
  } finally {
    if (!keepWindowUntilExit) {
      await markUpdateInProgress(false);
    }
  }
}
