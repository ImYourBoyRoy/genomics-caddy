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

/** A remote update is actionable only when the corresponding local asset exists. */
export function hasProvenOfflineUpdate(
  asset: Pick<OfflineAssetStatus, "local_present" | "update_available"> | null | undefined,
): boolean {
  return Boolean(asset?.local_present && asset.update_available);
}

/** A visible update requires both a proven asset flag and a fresh status probe. */
export function hasFreshProvenOfflineUpdate(
  statusFresh: boolean,
  asset: Pick<OfflineAssetStatus, "local_present" | "update_available"> | null | undefined,
): boolean {
  return statusFresh && hasProvenOfflineUpdate(asset);
}

/** Every asset with a proven remote update (ETag / Last-Modified), primary first. */
export function listOfflineUpdates(status: OfflineUpdateCheck | null | undefined): OfflineUpdateItem[] {
  if (!status) return [];
  const out: OfflineUpdateItem[] = [];
  for (const tier of status.tiers) {
    for (const asset of tier.assets) {
      // A remote flag only represents an update for an asset that is already
      // installed locally. Missing assets belong in the download flow.
      if (!hasProvenOfflineUpdate(asset)) continue;
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

/**
 * Remove a completed asset from the optimistic UI snapshot while the backend
 * performs its authoritative post-sync probe. The probe can add the asset
 * back only when it proves that a newer remote identity still exists.
 */
export function clearOfflineUpdate(
  status: OfflineUpdateCheck | null | undefined,
  assetId: string,
): OfflineUpdateCheck | null | undefined {
  if (!status) return status;

  let changed = false;
  const tiers = status.tiers.map((tier) => {
    const assets = tier.assets.map((asset) => {
      if (asset.asset_id !== assetId || !hasProvenOfflineUpdate(asset)) return asset;
      changed = true;
      return { ...asset, update_available: false };
    });
    const updates_available = assets.filter((asset) => hasProvenOfflineUpdate(asset)).length;
    return { ...tier, assets, updates_available };
  });

  if (!changed) return status;
  const total_updates_available = tiers.reduce((total, tier) => total + tier.updates_available, 0);
  return { ...status, tiers, total_updates_available };
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
