import { getVersion } from '@tauri-apps/api/app';
import { isTauri } from '@tauri-apps/api/core';
import packageJson from '../../../package.json';

/** Frontend fallback used by preview builds and report exports outside Tauri. */
export const PACKAGE_VERSION = packageJson.version;

export function normalizeAppVersion(version: string | null | undefined): string {
  const trimmed = version?.trim() ?? '';
  if (!trimmed) return PACKAGE_VERSION;
  return trimmed.startsWith('v') ? trimmed.slice(1) : trimmed;
}

/** Read the installed Tauri version when available, with the package version as a safe fallback. */
export async function getRuntimeAppVersion(): Promise<string> {
  if (!isTauri()) return PACKAGE_VERSION;
  try {
    return normalizeAppVersion(await getVersion());
  } catch {
    return PACKAGE_VERSION;
  }
}
