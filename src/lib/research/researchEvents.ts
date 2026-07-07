// ./src/lib/research/researchEvents.ts
/*
  Central Tauri event bridge for vector research progress + debug logs.
*/

import { listen } from "@tauri-apps/api/event";
import type { ResearchFindingPreview, ResearchProgress } from "../types/research";
import {
  handleResearchDebugEvent,
  handleResearchFindingEvent,
  handleResearchProgressEvent,
  normalizeResearchProgress,
} from "./liveProgress.svelte";

type DebugPayload = { tag: string; message: string; ts: number };

let started = false;
let unlistenProgress: (() => void) | undefined;
let unlistenDebug: (() => void) | undefined;
let unlistenFinding: (() => void) | undefined;

export async function startResearchEventListeners(): Promise<void> {
  if (started || typeof window === "undefined" || !(window as any).__TAURI_INTERNALS__) {
    return;
  }
  started = true;

  unlistenProgress = await listen<ResearchProgress>("research:progress", (event) => {
    const payload = normalizeResearchProgress(event.payload);
    handleResearchProgressEvent(payload);
  });

  unlistenDebug = await listen<DebugPayload>("research:debug", (event) => {
    handleResearchDebugEvent(event.payload.tag, event.payload.message);
  });

  unlistenFinding = await listen<ResearchFindingPreview>("research:finding", (event) => {
    handleResearchFindingEvent(event.payload);
  });
}

export function stopResearchEventListeners() {
  unlistenProgress?.();
  unlistenDebug?.();
  unlistenFinding?.();
  unlistenProgress = undefined;
  unlistenDebug = undefined;
  unlistenFinding = undefined;
  started = false;
}
