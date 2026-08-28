import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/tauri', () => ({
  deleteSample: vi.fn(),
  generateReport: vi.fn(),
  getSamples: vi.fn(),
  importGenome: vi.fn(),
  queryRegion: vi.fn(),
  queryRsids: vi.fn(),
}));

vi.mock('./reportTemplate', () => ({
  mergedReportTemplateJson: vi.fn(() => '{}'),
}));

import { reloadReport } from './pageSampleHandlers';

describe('reloadReport', () => {
  it('rebuilds the selected profile even when an existing report is present', async () => {
    const warmReportFn = vi.fn().mockResolvedValue(undefined);
    const selectedSample = {
      id: 7,
      name: 'Fixture profile',
      genetic_sex: 'Female',
      imported_at: '2026-08-27T00:00:00Z',
    };

    await reloadReport({ selectedSample, warmReportFn });

    expect(warmReportFn).toHaveBeenCalledOnce();
    expect(warmReportFn).toHaveBeenCalledWith(7);
  });

  it('does not request a report when no profile is selected', async () => {
    const warmReportFn = vi.fn().mockResolvedValue(undefined);

    await reloadReport({ selectedSample: null, warmReportFn });

    expect(warmReportFn).not.toHaveBeenCalled();
  });
});
