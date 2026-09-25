import { describe, expect, it } from 'vitest';
import {
  formatTransferEta,
  formatTransferSize,
  updateDownloadProgress,
} from './offlineDownloadProgress';

describe('offline download progress', () => {
  it('does not invent a total, rate, or ETA before the server reports enough data', () => {
    const first = updateDownloadProgress(undefined, { bytesDone: 1024, totalBytes: 0 }, 1000);
    expect(first.percent).toBe(-1);
    expect(first.speedBytesPerSecond).toBeNull();
    expect(first.etaSeconds).toBeNull();
  });

  it('calculates a smoothed rate and ETA from transfer bytes, not resumed bytes', () => {
    const initial = updateDownloadProgress(undefined, {
      bytesDone: 80 * 1024 * 1024,
      totalBytes: 100 * 1024 * 1024,
      resumedBytes: 80 * 1024 * 1024,
    }, 1000);
    const next = updateDownloadProgress(initial, {
      bytesDone: 82 * 1024 * 1024,
      totalBytes: 100 * 1024 * 1024,
      resumedBytes: 80 * 1024 * 1024,
    }, 2000);
    expect(next.speedBytesPerSecond).toBe(2 * 1024 * 1024);
    expect(next.etaSeconds).toBe(9);
  });

  it('drops stale speed after a long pause or counter reset', () => {
    const initial = updateDownloadProgress(undefined, { bytesDone: 0, totalBytes: 1000 }, 1000);
    const active = updateDownloadProgress(initial, { bytesDone: 500, totalBytes: 1000 }, 2000);
    expect(active.speedBytesPerSecond).not.toBeNull();
    const afterPause = updateDownloadProgress(active, { bytesDone: 500, totalBytes: 1000 }, 12000);
    expect(afterPause.speedBytesPerSecond).toBeNull();
    expect(afterPause.etaSeconds).toBeNull();
  });

  it('drops the previous attempt rate when an unsafe resume restarts from zero', () => {
    const resumed = updateDownloadProgress(undefined, {
      bytesDone: 500,
      totalBytes: 1000,
      resumedBytes: 400,
    }, 1000);
    const active = updateDownloadProgress(resumed, {
      bytesDone: 600,
      totalBytes: 1000,
      resumedBytes: 400,
    }, 2000);
    const restarted = updateDownloadProgress(active, {
      bytesDone: 0,
      totalBytes: 1000,
      resumedBytes: 0,
    }, 2500);

    expect(restarted.speedBytesPerSecond).toBeNull();
    expect(restarted.etaSeconds).toBeNull();
  });

  it('formats compact file sizes and ETA strings', () => {
    expect(formatTransferSize(1024 ** 2)).toBe('1.0 MB');
    expect(formatTransferEta(89)).toBe('About 1m 29s left');
    expect(formatTransferEta(3661)).toBe('About 1h 1m left');
  });
});
