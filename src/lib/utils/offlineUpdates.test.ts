import { describe, expect, it } from 'vitest';
import { clearOfflineUpdate, hasFreshProvenOfflineUpdate, hasProvenOfflineUpdate, isCompleteOfflineStatus, listOfflineUpdates, offlineStatusFailureMessage } from './offlineUpdates';
import type { OfflineUpdateCheck } from '../types/research';

function makeStatus(): OfflineUpdateCheck {
  return {
    total_updates_available: 2,
    indexed_summary: { gwas_rows: 0, clinvar_rows: 0, variant_locus_rows: 0 },
    remote_check: { assets_checked: 1, assets_failed: 0, timed_out: false },
    tiers: [
      {
        tier: 0,
        ready: true,
        updates_available: 2,
        assets: [
          {
            asset_id: 'gwas_catalog',
            tier: 0,
            label: 'GWAS Catalog',
            local_present: true,
            local_bytes: 100,
            row_count: 10,
            synced_at: 1,
            update_available: true,
            remote_content_length: 200,
            version_label: null,
            message: 'Update available on server',
            display_size: '200 B',
          },
          {
            asset_id: 'clinvar_variant_summary',
            tier: 0,
            label: 'ClinVar',
            local_present: false,
            local_bytes: 0,
            row_count: 0,
            synced_at: null,
            update_available: true,
            remote_content_length: 300,
            version_label: null,
            message: 'Not downloaded',
            display_size: '300 B',
          },
        ],
      },
    ],
  };
}

describe('listOfflineUpdates', () => {
  it('only treats a completed remote probe as authoritative', () => {
    const complete = makeStatus();
    expect(isCompleteOfflineStatus(complete)).toBe(true);
    expect(isCompleteOfflineStatus({
      ...complete,
      remote_check: { assets_checked: 1, assets_failed: 1, timed_out: false },
    })).toBe(false);
    expect(isCompleteOfflineStatus({
      ...complete,
      remote_check: { assets_checked: 0, assets_failed: 0, timed_out: true },
    })).toBe(false);
    expect(offlineStatusFailureMessage({
      ...complete,
      remote_check: { assets_checked: 3, assets_failed: 3, timed_out: false },
    })).toContain('3 remote resources could not be verified');
    expect(offlineStatusFailureMessage({
      ...complete,
      remote_check: { assets_checked: 0, assets_failed: 0, timed_out: true },
    })).toContain('timed out');
  });

  it('requires a local asset before treating a remote flag as an update', () => {
    expect(hasProvenOfflineUpdate({ local_present: true, update_available: true })).toBe(true);
    expect(hasProvenOfflineUpdate({ local_present: false, update_available: true })).toBe(false);
    expect(hasProvenOfflineUpdate(null)).toBe(false);
  });

  it('requires a fresh status probe before exposing a visible update', () => {
    const asset = { local_present: true, update_available: true };

    expect(hasFreshProvenOfflineUpdate(false, asset)).toBe(false);
    expect(hasFreshProvenOfflineUpdate(true, asset)).toBe(true);
    expect(hasFreshProvenOfflineUpdate(true, { local_present: false, update_available: true })).toBe(false);
  });

  it('lists only installed assets with a remote update flag', () => {
    const items = listOfflineUpdates(makeStatus());

    expect(items.map((item) => item.asset_id)).toEqual(['gwas_catalog']);
    expect(items[0]?.local_present).toBe(true);
  });

  it('clears a completed asset from the optimistic snapshot without touching other assets', () => {
    const status = makeStatus();
    const cleared = clearOfflineUpdate(status, 'gwas_catalog');

    expect(cleared?.total_updates_available).toBe(0);
    expect(cleared?.tiers[0]?.updates_available).toBe(0);
    expect(listOfflineUpdates(cleared).map((item) => item.asset_id)).toEqual([]);
    expect(cleared?.tiers[0]?.assets[1]?.update_available).toBe(true);
  });
});
