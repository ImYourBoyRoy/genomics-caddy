// ./src/lib/utils/markerPacksState.svelte.ts
/**
 * Shared Svelte 5 reactive store for dynamically loaded DNA marker packs.
 *
 * Responsibilities:
 * - Invoke Tauri's `get_all_marker_packs` to load packs dynamically.
 * - Expose reactive `manifest` and `packs` objects for the frontend UI, report generation, and system prompt formatting.
 * - Cache loaded packs to prevent redundant IPC roundtrips.
 *
 * Key Inputs: Tauri command invocation `get_all_marker_packs`.
 * Key Outputs: Reactive `manifest` and `packs` store properties.
 * Operational Notes: Must be loaded during dashboard bootstrap (runPageBootstrap) to avoid race conditions.
 */

import { invoke } from "@tauri-apps/api/core";
import type { MarkerDefinition } from "../types/genomics";

export interface PackContent {
  name: string;
  markers: MarkerDefinition[];
}

export interface PackManifestInfo {
  id: string;
  label: string;
  description: string;
  default_enabled: boolean;
  requires_clinical_confirmation: boolean;
}

export interface ManifestData {
  packs: PackManifestInfo[];
}

export interface AllPacksPayload {
  manifest: ManifestData;
  packs: Record<string, PackContent>;
}

class MarkerPacksStore {
  // Svelte 5 reactive properties
  manifest = $state<ManifestData>({ packs: [] });
  packs = $state<Record<string, PackContent>>({});
  isLoaded = $state(false);
  error = $state("");

  async load() {
    if (this.isLoaded) return;
    await this.reload();
  }

  /** Force re-read packs from App/Data after merge. */
  async reload() {
    try {
      const data = await invoke<AllPacksPayload>("get_all_marker_packs");
      this.manifest = data.manifest;
      this.packs = data.packs;
      this.isLoaded = true;
      this.error = "";
    } catch (e: any) {
      this.error = String(e.message || e);
      console.error("Failed to load marker packs dynamically:", e);
    }
  }
}

export const markerPacksStore = new MarkerPacksStore();
