import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = readFileSync(new URL('./DownloadProgressDetails.svelte', import.meta.url), 'utf8');

describe('download progress details presentation', () => {
  it('explains unknown totals and retained bytes without fabricating an ETA', () => {
    expect(source).toContain('Total size unavailable · ETA unavailable');
    expect(source).toContain('Resumed · {formatTransferSize(progress.resumedBytes)} retained');
    expect(source).toContain('progress.etaSeconds == null');
    expect(source).toContain('Download percentage');
    expect(source).toContain('share of the known file size received so far');
    expect(source).toContain('finished download may still need verification and indexing');
  });

  it('makes the percentage explanation available to keyboard and screen-reader users', () => {
    expect(source).toContain('<Tooltip');
    expect(source).toContain('interactiveChildren');
    expect(source).toContain('type="button"');
    expect(source).toContain('aria-label={progress.percent >= 0 ? `Explain ${progress.percent}% downloaded`');
    expect(source).toContain('.download-progress-percent:focus-visible');
  });

  it('uses a compact accessible progress surface that can wrap in the sidebar', () => {
    expect(source).toContain('role="group" aria-label={label}');
    expect(source).toContain('<ProgressTrack');
    expect(source).toContain('min-width: 0');
    expect(source).toContain('overflow-wrap: anywhere');
  });
});
