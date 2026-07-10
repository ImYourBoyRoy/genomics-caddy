// ./src/lib/utils/variantNavigation.ts
/**
 * Cross-tab variant navigation helpers for the main dashboard.
 */

import type { VariantNavTarget } from "../constants/traitCategories";
import type { DbSnpRecord, GenomeSample } from "../types/genomics";
import { queryRsids } from "../api/tauri";

export interface VariantNavState {
  activeTab: string;
  highlightRsid: string;
  mapFocusRsid: string;
  searchRsid: string;
  browserResults: DbSnpRecord[];
  isBrowsing: boolean;
  aiInitialSearchQuery: string;
  aiActiveView: "chat" | "evidence";
}

export interface VariantNavCallbacks {
  getSelectedSample: () => GenomeSample | null;
  setState: (patch: Partial<VariantNavState>) => void;
  onExploreResearch: (rsid: string) => void;
  onSearchError: (message: string) => void;
}

export function navigateToVariant(
  rsid: string,
  target: VariantNavTarget,
  callbacks: VariantNavCallbacks
): void {
  const normalized = rsid.trim().toLowerCase();
  if (!normalized) return;

  callbacks.setState({ highlightRsid: normalized });

  if (target === "evidence") {
    callbacks.onExploreResearch(normalized);
    return;
  }

  if (target === "browser") {
    callbacks.setState({ searchRsid: normalized, activeTab: "browser" });
    queueMicrotask(() => {
      void (async () => {
        const sample = callbacks.getSelectedSample();
        if (!sample) return;
        callbacks.setState({ isBrowsing: true });
        try {
          const browserResults = await queryRsids(sample.id, [normalized]);
          callbacks.setState({ browserResults });
        } catch (e: unknown) {
          callbacks.onSearchError(e instanceof Error ? e.message : String(e));
        } finally {
          callbacks.setState({ isBrowsing: false });
        }
      })();
    });
    return;
  }

  if (target === "map") {
    callbacks.setState({ mapFocusRsid: normalized, activeTab: "map" });
    return;
  }

  callbacks.setState({ activeTab: "report" });
  queueMicrotask(() => {
    const curated = document.getElementById(`variant-${normalized}`);
    if (curated) {
      curated.scrollIntoView({ behavior: "smooth", block: "center" });
      return;
    }
    const vectorCard = document.getElementById(`vector-promoted-${normalized}`);
    if (vectorCard) {
      vectorCard.scrollIntoView({ behavior: "smooth", block: "center" });
      return;
    }
    callbacks.onExploreResearch(normalized);
  });
}
