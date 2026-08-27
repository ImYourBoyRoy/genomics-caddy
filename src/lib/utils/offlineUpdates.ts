// ./src/lib/utils/offlineUpdates.ts
/*
Purpose: Build a clear, named list of offline assets that need updates.
Used by Sidebar + OfflineDataPanel so "N updates" always names each asset.
*/

import { isPrimaryCatalogId } from "./primaryCatalogs";
import type { OfflineAssetStatus, OfflineUpdateCheck } from "../types/research";

export interface OfflineUpdateItem {
  asset_id: string;
  label: string;
  tier: number;
  primary: boolean;
  local_present: boolean;
  display_size: string;
  message: string;
  remote_content_length: number | null;
}

/** Every asset with a proven remote update (ETag / Last-Modified), primary first. */
export function listOfflineUpdates(status: OfflineUpdateCheck | null | undefined): OfflineUpdateItem[] {
  if (!status) return [];
  const out: OfflineUpdateItem[] = [];
  for (const tier of status.tiers) {
    for (const asset of tier.assets) {
      // A remote flag only represents an update for an asset that is already
      // installed locally. Missing assets belong in the download flow.
      if (!asset.local_present || !asset.update_available) continue;
      out.push(toUpdateItem(asset));
    }
  }
  out.sort((a, b) => {
    if (a.primary !== b.primary) return a.primary ? -1 : 1;
    if (a.tier !== b.tier) return a.tier - b.tier;
    return a.label.localeCompare(b.label);
  });
  return out;
}

function toUpdateItem(asset: OfflineAssetStatus): OfflineUpdateItem {
  return {
    asset_id: asset.asset_id,
    label: asset.label || asset.asset_id,
    tier: asset.tier,
    primary: isPrimaryCatalogId(asset.asset_id),
    local_present: asset.local_present,
    display_size: asset.display_size || "",
    message: asset.message || "",
    remote_content_length: asset.remote_content_length ?? null,
  };
}

export function formatUpdateSummary(items: OfflineUpdateItem[]): string {
  if (items.length === 0) return "";
  const names = items.map((i) => i.label);
  const primary = items.filter((i) => i.primary).length;
  const supporting = items.length - primary;
  const kind =
    supporting === 0
      ? "All are primary catalogs."
      : primary === 0
        ? "All are supporting assets (not the four main catalogs)."
        : `${primary} primary · ${supporting} supporting.`;
  return `${names.join(", ")}. ${kind}`;
}
