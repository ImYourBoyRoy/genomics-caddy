import { writable } from 'svelte/store';
import type { UpdateUiState } from './updater';

export interface AppUpdateState {
  isDesktop: boolean;
  currentVersion: string;
  status: UpdateUiState;
  availableVersion: string | null;
  message: string;
  checkedAt: number | null;
  checkNow: (() => void) | null;
  installAvailable: (() => void) | null;
}

/** Shared by the global update banner and the Help & app info view. */
export const appUpdateState = writable<AppUpdateState>({
  isDesktop: false,
  currentVersion: '',
  status: 'idle',
  availableVersion: null,
  message: '',
  checkedAt: null,
  checkNow: null,
  installAvailable: null,
});
