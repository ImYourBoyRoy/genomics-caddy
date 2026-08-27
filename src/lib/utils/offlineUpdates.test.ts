import { describe, expect, it } from 'vitest';
import { clearOfflineUpdate, listOfflineUpdates } from './offlineUpdates';
import type { OfflineUpdateCheck } from '../types/research';

function makeStatus(): OfflineUpdateCheck {
  return {
    total_updates_available: 2,
    indexed_summary: { gwas_rows: 0, clinvar_rows: 0, variant_locus_rows: 0 },
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
  it('lists only installed assets with a remote update flag', () => {
    const items = listOfflineUpdates(makeStatus());

    expect(items.map((item) => item.asset_id)).toEqual(['gwas_catalog']);
    expect(items[0]?.local_present).toBe(true);
  });

  it('clears a completed asset from the optimistic snapshot without touching other assets', () => {
    const status = makeStatus();
    const cleared = clearOfflineUpdate(status, 'gwas_catalog');

    expect(cleared?.total_updates_available).toBe(1);
    expect(cleared?.tiers[0]?.updates_available).toBe(1);
    expect(listOfflineUpdates(cleared).map((item) => item.asset_id)).toEqual([]);
    expect(cleared?.tiers[0]?.assets[1]?.update_available).toBe(true);
  });
});
