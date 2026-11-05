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
  // The ticker belongs to the database phase only: once real status arrives it
  // is stopped, otherwise it would drag the visible phase back to "db" while
  // the report is still generating (the startup screen appeared to restart).
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

    clearInterval(ticker);
    callbacks.onStatus(status);
    callbacks.onPaths(paths);
    callbacks.onChainPresent(status.chain_present);
    callbacks.onSamples(status.samples);

    callbacks.onPhase(
      "stats",
      `Indexed ${status.genotype_count.toLocaleString()} genotypes across your local database`
    );
    await yieldToUi();
    await bootstrapSleep(300);

    if (status.sample_count > 0) {
      pendingSample = status.samples[0];
      callbacks.onPhase("profile", `Loading profile: ${pendingSample.name}…`);
      await yieldToUi();
      await bootstrapSleep(180);

      if (status.chain_present) {
        callbacks.onPhase("report", "Analyzing genetic markers…");
        await yieldToUi();
        await callbacks.warmReport(pendingSample.id);
      }
    }

    callbacks.onPhase(
      "ready",
      status.sample_count > 0
        ? status.chain_present
          ? "All systems ready — welcome back."
          : "Install the liftover chain before opening or analyzing a profile."
        : status.chain_present
          ? "Ready — import a genome to begin."
          : "Install the liftover chain before importing DNA."
    );
    await yieldToUi();
    await bootstrapSleep(520);
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
