import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./DownloadProgressDetails.svelte', import.meta.url), 'utf8');

describe('download progress details presentation', () => {
  it('explains unknown totals and retained bytes without fabricating an ETA', () => {
    expect(source).toContain('Total size unavailable · ETA unavailable');
    expect(source).toContain('Resumed · {formatTransferSize(progress.resumedBytes)} retained');
    expect(source).toContain('progress.etaSeconds == null');
  });

  it('uses a compact accessible progress surface that can wrap in the sidebar', () => {
    expect(source).toContain('role="group" aria-label={label}');
    expect(source).toContain('<ProgressTrack');
    expect(source).toContain('min-width: 0');
    expect(source).toContain('overflow-wrap: anywhere');
  });
});
