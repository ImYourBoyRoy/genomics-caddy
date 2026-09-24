export interface DownloadProgressSample {
  at: number;
  bytesDone: number;
  totalBytes: number;
  resumedBytes: number;
  transferBytes: number;
  speedBytesPerSecond: number | null;
  speedSamples: number;
  percent: number;
  etaSeconds: number | null;
}

/**
 * Build a download snapshot from backend byte counters. Rates use bytes moved
 * in this attempt (excluding a retained partial) and recent intervals only,
 * so a pause or resume cannot create a fictional speed or ETA.
 */
export function updateDownloadProgress(
  previous: DownloadProgressSample | undefined,
  input: { bytesDone: number; totalBytes: number; resumedBytes?: number },
  at: number,
): DownloadProgressSample {
  const bytesDone = Math.max(0, input.bytesDone);
  const totalBytes = Math.max(0, input.totalBytes);
  const resumedBytes = Math.min(bytesDone, Math.max(0, input.resumedBytes ?? 0));
  const transferBytes = Math.max(0, bytesDone - resumedBytes);
  const percent = totalBytes > 0
    ? Math.min(99, Math.max(0, Math.round((bytesDone / totalBytes) * 100)))
    : -1;

  let speedBytesPerSecond = previous?.speedBytesPerSecond ?? null;
  let speedSamples = previous?.speedSamples ?? 0;
  const previousTransferBytes = previous && previous.resumedBytes === resumedBytes
    ? previous.transferBytes
    : transferBytes;
  const elapsedSeconds = previous ? (at - previous.at) / 1000 : 0;
  const deltaBytes = transferBytes - previousTransferBytes;
  const transferEpochChanged = previous != null && previous.resumedBytes !== resumedBytes;

  if (!previous || transferEpochChanged || elapsedSeconds <= 0 || elapsedSeconds > 8 || deltaBytes < 0) {
    speedBytesPerSecond = null;
    speedSamples = 0;
  } else if (elapsedSeconds >= 0.4 && deltaBytes > 0) {
    const observedRate = deltaBytes / elapsedSeconds;
    speedBytesPerSecond = speedBytesPerSecond == null
      ? observedRate
      : speedBytesPerSecond * 0.65 + observedRate * 0.35;
    speedSamples += 1;
  }

  const remainingBytes = totalBytes > bytesDone ? totalBytes - bytesDone : 0;
  const etaSeconds = totalBytes > 0
    && remainingBytes > 0
    && speedSamples > 0
    && speedBytesPerSecond != null
    && speedBytesPerSecond >= 1024
    ? Math.ceil(remainingBytes / speedBytesPerSecond)
    : null;

  return {
    at,
    bytesDone,
    totalBytes,
    resumedBytes,
    transferBytes,
    speedBytesPerSecond,
    speedSamples,
    percent,
    etaSeconds,
  };
}

export function formatTransferSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(1)} kB`;
  return `${Math.round(bytes)} B`;
}

export function formatTransferEta(seconds: number): string {
  if (seconds < 60) return `About ${seconds}s left`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return `About ${minutes}m ${remainingSeconds}s left`;
  const hours = Math.floor(minutes / 60);
  return `About ${hours}h ${minutes % 60}m left`;
}
