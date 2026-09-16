import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/tauri', () => ({
  deleteSample: vi.fn(),
  generateReport: vi.fn(),
  getSamples: vi.fn(),
  inspectGenome: vi.fn(),
  importGenome: vi.fn(),
  queryRegion: vi.fn(),
  queryRsids: vi.fn(),
}));

vi.mock('./reportTemplate', () => ({
  mergedReportTemplateJson: vi.fn(() => '{}'),
}));

import { deleteSample as apiDeleteSample, importGenome as apiImportGenome, inspectGenome } from '../api/tauri';
import { deleteSampleWithConfirm, reloadReport, runImportGenome } from './pageSampleHandlers';

beforeEach(() => {
  vi.clearAllMocks();
});

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

describe('runImportGenome', () => {
  it('does not write when the pre-import preview fails', async () => {
    vi.mocked(inspectGenome).mockRejectedValueOnce(new Error('invalid fixture'));
    const onState = vi.fn();

    await runImportGenome(new Event('submit'), {
      filePath: '/private/bad-fixture.txt',
      sampleNameInput: 'Rejected fixture',
      existingSamples: [],
      onConfirmationRequired: vi.fn(),
      onState,
      refreshSamples: vi.fn().mockResolvedValue([]),
      selectSample: vi.fn(),
    });

    expect(apiImportGenome).not.toHaveBeenCalled();
    expect(onState).toHaveBeenCalledWith(expect.objectContaining({
      importError: expect.stringContaining('No data was written'),
    }));
  });

  it('requires explicit confirmation before replacing a duplicate profile', async () => {
    vi.mocked(apiImportGenome).mockResolvedValueOnce(7);
    vi.mocked(inspectGenome).mockResolvedValueOnce({
      source_file_name: 'fixture.txt',
      diagnostics: {
        format: '23andMe',
        vendor: '23andMe',
        delimiter: 'TSV',
        source_build: 'GRCh37',
        coordinate_system: '1-based inclusive input; GRCh37 database coordinates',
        allele_orientation: 'Unknown (vendor strand not stated)',
        total_rows: 1,
        accepted_rows: 1,
        malformed_rows: 0,
        duplicate_rows: 0,
        warnings: [],
      },
      liftover_available: true,
    });
    const pending = { confirm: null as (() => void | Promise<void>) | null };
    const onConfirmationRequired = vi.fn((callback: () => void | Promise<void>) => {
      pending.confirm = callback;
    });
    const onState = vi.fn();
    const refreshSamples = vi.fn().mockResolvedValue([{
      id: 7,
      name: 'Fixture PROFILE',
      genetic_sex: 'Female',
      imported_at: '2026-08-27T00:00:00Z',
    }]);
    const selectSample = vi.fn();

    await runImportGenome(new Event('submit'), {
      filePath: '/private/fixture.txt',
      sampleNameInput: ' fixture profile ',
      existingSamples: [{
        id: 7,
        name: 'Fixture profile',
        genetic_sex: 'Female',
        imported_at: '2026-08-27T00:00:00Z',
      }],
      onConfirmationRequired,
      onState,
      refreshSamples,
      selectSample,
    });

    expect(onConfirmationRequired).toHaveBeenCalledOnce();
    expect(onConfirmationRequired).toHaveBeenCalledWith(expect.any(Function), true);
    expect(inspectGenome).toHaveBeenCalledWith('/private/fixture.txt');
    expect(apiImportGenome).not.toHaveBeenCalled();
    expect(onState).toHaveBeenCalledWith(expect.objectContaining({ importPhase: 'awaiting-confirmation' }));

    await pending.confirm?.();

    expect(apiImportGenome).toHaveBeenCalledWith('/private/fixture.txt', 'fixture profile', 7);
    await vi.waitFor(() => expect(refreshSamples).toHaveBeenCalledOnce());
    expect(selectSample).toHaveBeenCalledWith(expect.objectContaining({ id: 7 }));
  });

  it('imports a new profile without replacement when the name is unique', async () => {
    vi.mocked(apiImportGenome).mockResolvedValueOnce(8);
    vi.mocked(inspectGenome).mockResolvedValueOnce({
      source_file_name: 'fixture.txt',
      diagnostics: {
        format: '23andMe',
        vendor: '23andMe',
        delimiter: 'TSV',
        source_build: 'GRCh37',
        coordinate_system: '1-based inclusive input; GRCh37 database coordinates',
        allele_orientation: 'Unknown (vendor strand not stated)',
        total_rows: 1,
        accepted_rows: 1,
        malformed_rows: 0,
        duplicate_rows: 0,
        warnings: [],
      },
      liftover_available: true,
    });
    const onState = vi.fn();
    const pending = { confirm: null as (() => void | Promise<void>) | null };
    const onConfirmationRequired = vi.fn((callback: () => void | Promise<void>) => {
      pending.confirm = callback;
    });
    const refreshSamples = vi.fn().mockResolvedValue([{
      id: 8,
      name: 'New profile',
      genetic_sex: 'Unknown',
      imported_at: '2026-08-27T00:00:00Z',
    }]);
    const selectSample = vi.fn();

    await runImportGenome(new Event('submit'), {
      filePath: '/private/fixture.txt',
      sampleNameInput: ' New profile ',
      existingSamples: [],
      onConfirmationRequired,
      onState,
      refreshSamples,
      selectSample,
    });

    expect(apiImportGenome).not.toHaveBeenCalled();
    expect(onConfirmationRequired).toHaveBeenCalledOnce();
    expect(onConfirmationRequired).toHaveBeenCalledWith(expect.any(Function), false);
    await pending.confirm?.();
    await vi.waitFor(() => expect(apiImportGenome).toHaveBeenCalledWith('/private/fixture.txt', 'New profile', undefined));
    expect(refreshSamples).toHaveBeenCalledOnce();
    expect(selectSample).toHaveBeenCalledWith(expect.objectContaining({ id: 8, name: 'New profile' }));
    expect(onState).toHaveBeenCalledWith(expect.objectContaining({ importSuccess: 'Successfully imported profile as ID: 8.' }));
  });

  it('allows an explicitly GRCh38 export after preview instead of rejecting it', async () => {
    vi.mocked(apiImportGenome).mockResolvedValueOnce(9);
    vi.mocked(inspectGenome).mockResolvedValueOnce({
      source_file_name: 'grch38-fixture.txt',
      diagnostics: {
        format: '23andMe',
        vendor: '23andMe',
        delimiter: 'TSV',
        source_build: 'GRCh38',
        coordinate_system: '1-based inclusive GRCh38 source coordinates; GRCh37 is retained when inverse liftover maps',
        allele_orientation: 'Unknown (vendor strand not stated)',
        total_rows: 1,
        accepted_rows: 1,
        malformed_rows: 0,
        duplicate_rows: 0,
        warnings: [],
      },
      liftover_available: true,
    });
    const pending = { confirm: null as (() => void | Promise<void>) | null };
    const onConfirmationRequired = vi.fn((callback: () => void | Promise<void>) => {
      pending.confirm = callback;
    });
    const refreshSamples = vi.fn().mockResolvedValue([{
      id: 9,
      name: 'GRCh38 profile',
      genetic_sex: 'Unknown',
      imported_at: '2026-09-03T00:00:00Z',
    }]);

    await runImportGenome(new Event('submit'), {
      filePath: '/private/grch38-fixture.txt',
      sampleNameInput: 'GRCh38 profile',
      existingSamples: [],
      onConfirmationRequired,
      onState: vi.fn(),
      refreshSamples,
      selectSample: vi.fn(),
    });

    expect(onConfirmationRequired).toHaveBeenCalledOnce();
    await pending.confirm?.();
    await vi.waitFor(() => expect(apiImportGenome).toHaveBeenCalledWith('/private/grch38-fixture.txt', 'GRCh38 profile', undefined));
  });
});

describe('deleteSampleWithConfirm', () => {
  it('leaves the profile untouched when deletion is canceled', async () => {
    const confirm = vi.fn();
    const refreshSamples = vi.fn().mockResolvedValue([]);

    await deleteSampleWithConfirm({
      id: 7,
      selectedSample: null,
      confirm,
      alert: vi.fn(),
      onState: vi.fn(),
      refreshSamples,
      selectSample: vi.fn(),
      removeSampleFromList: vi.fn(),
    });

    expect(confirm).toHaveBeenCalledOnce();
    expect(apiDeleteSample).not.toHaveBeenCalled();
    expect(refreshSamples).not.toHaveBeenCalled();
  });

  it('deletes the native profile and clears selected profile state after confirmation', async () => {
    vi.mocked(apiDeleteSample).mockResolvedValueOnce(undefined);
    const confirm = vi.fn();
    const onState = vi.fn();
    const nextProfile = {
      id: 8,
      name: 'Next fixture profile',
      genetic_sex: 'Unknown',
      imported_at: '2026-08-27T00:00:00Z',
    };
    const refreshSamples = vi.fn().mockResolvedValue([nextProfile]);
    const selectSample = vi.fn();
    const removeSampleFromList = vi.fn();

    await deleteSampleWithConfirm({
      id: 7,
      selectedSample: {
        id: 7,
        name: 'Fixture profile',
        genetic_sex: 'Female',
        imported_at: '2026-08-27T00:00:00Z',
      },
      confirm,
      alert: vi.fn(),
      onState,
      refreshSamples,
      selectSample,
      removeSampleFromList,
    });

    expect(confirm).toHaveBeenCalledOnce();
    expect(confirm.mock.calls[0][0]).toContain('Context/Diary');
    await confirm.mock.calls[0][1]();

    expect(apiDeleteSample).toHaveBeenCalledWith(7);
    expect(onState).toHaveBeenCalledWith({ selectedSample: null, generatedReport: null });
    expect(refreshSamples).toHaveBeenCalledOnce();
    expect(selectSample).toHaveBeenCalledWith(nextProfile);
    expect(removeSampleFromList).toHaveBeenCalledWith(7);
  });

  it('reconciles a partially completed deletion against the refreshed profile list', async () => {
    vi.mocked(apiDeleteSample).mockRejectedValueOnce(new Error('cleanup interrupted'));
    const confirm = vi.fn();
    const alert = vi.fn();
    const onState = vi.fn();

    await deleteSampleWithConfirm({
      id: 7,
      selectedSample: { id: 7, name: 'Fixture profile', genetic_sex: 'Female', imported_at: '2026-08-27T00:00:00Z' },
      confirm,
      alert,
      onState,
      refreshSamples: vi.fn().mockResolvedValue([]),
      selectSample: vi.fn(),
      removeSampleFromList: vi.fn(),
    });

    await confirm.mock.calls[0][1]();

    expect(onState).toHaveBeenCalledWith({ selectedSample: null, generatedReport: null });
    expect(alert).toHaveBeenCalledWith(expect.stringContaining('no longer active'));
  });

  it('keeps profile state intact when the deletion fails and the profile remains listed', async () => {
    vi.mocked(apiDeleteSample).mockRejectedValueOnce(new Error('permission denied'));
    const confirm = vi.fn();
    const alert = vi.fn();
    const onState = vi.fn();
    const profile = { id: 7, name: 'Fixture profile', genetic_sex: 'Female', imported_at: '2026-08-27T00:00:00Z' };

    await deleteSampleWithConfirm({
      id: 7,
      selectedSample: profile,
      confirm,
      alert,
      onState,
      refreshSamples: vi.fn().mockResolvedValue([profile]),
      selectSample: vi.fn(),
      removeSampleFromList: vi.fn(),
    });

    await confirm.mock.calls[0][1]();

    expect(onState).not.toHaveBeenCalled();
    expect(alert).toHaveBeenCalledWith(expect.stringContaining('permission denied'));
  });
});
