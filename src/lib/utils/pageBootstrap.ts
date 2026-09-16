// ./src/lib/utils/pageBootstrap.ts
/**
 * Application startup bootstrap sequence for the main dashboard page.
 */

import { getAppBootstrap, getAppPaths } from "../api/tauri";
import type { AppBootstrapStatus, AppPaths, GenomeSample } from "../types/genomics";
import { markerPacksStore } from "./markerPacksState.svelte";
import {
  DB_TICKER_MESSAGES,
  sleep as bootstrapSleep,
  yieldToUi,
  type BootstrapPhase,
} from "../components/common/bootstrap/bootstrapPhases";

export interface BootstrapCallbacks {
  onPhase: (phase: BootstrapPhase, message: string) => void;
  onStatus: (status: AppBootstrapStatus) => void;
  onPaths: (paths: AppPaths) => void;
  onChainPresent: (present: boolean) => void;
  onSamples: (samples: GenomeSample[]) => void;
  onError: (message: string) => void;
  warmReport: (sampleId: number) => Promise<void>;
}

export interface BootstrapResult {
  pendingSample: GenomeSample | null;
}

export async function runPageBootstrap(callbacks: BootstrapCallbacks): Promise<BootstrapResult> {
  callbacks.onPhase("db", DB_TICKER_MESSAGES[0]);

  // The native database emits detailed progress events, but a warm database
  // can still leave the webview visually unchanged while IPC/configuration
  // work is pending. Keep the last visible DB checkpoint moving until the
  // authoritative status arrives; real backend events continue to replace it.
  let tickerIdx = 0;
  const ticker = setInterval(() => {
    tickerIdx = (tickerIdx + 1) % DB_TICKER_MESSAGES.length;
    callbacks.onPhase("db", DB_TICKER_MESSAGES[tickerIdx]);
  }, 2200);

  let pendingSample: GenomeSample | null = null;

  try {
    await yieldToUi();
    callbacks.onPhase("db", "Loading marker pack configurations…");
    await markerPacksStore.load();
    await yieldToUi();
    const [status, paths] = await Promise.all([getAppBootstrap(), getAppPaths()]);

    callbacks.onStatus(status);
    callbacks.onPaths(paths);
    callbacks.onChainPresent(status.chain_present);
    callbacks.onSamples(status.samples);

    callbacks.onPhase(
      "stats",
      `Indexed ${status.genotype_count.toLocaleString()} genotypes across your local database`
    );
    await yieldToUi();
    await bootstrapSleep(450);

    if (status.sample_count > 0) {
      pendingSample = status.samples[0];
      callbacks.onPhase("profile", `Loading profile: ${pendingSample.name}…`);
      await yieldToUi();
      await bootstrapSleep(400);

      callbacks.onPhase("report", "Analyzing genetic markers…");
      await yieldToUi();
      await callbacks.warmReport(pendingSample.id);
      await bootstrapSleep(350);
    }

    callbacks.onPhase(
      "ready",
      status.sample_count > 0 ? "All systems ready — welcome back." : "Ready — import a genome to begin."
    );
    await yieldToUi();
    await bootstrapSleep(700);
  } catch (e: unknown) {
    clearInterval(ticker);
    const message = e instanceof Error ? e.message : String(e);
    callbacks.onPhase("error", "Startup failed");
    callbacks.onError(message);
    console.error("Bootstrap failed", e);
  } finally {
    clearInterval(ticker);
  }

  return { pendingSample };
}
