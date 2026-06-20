// ./src/lib/utils/pageBootstrap.ts
/**
 * Application startup bootstrap sequence for the main dashboard page.
 */

import { getAppBootstrap } from "../api/tauri";
import type { AppBootstrapStatus, AppPaths, GenomeSample } from "../types/genomics";
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

  let tickerIdx = 0;
  const ticker = setInterval(() => {
    tickerIdx = (tickerIdx + 1) % DB_TICKER_MESSAGES.length;
    callbacks.onPhase("db", DB_TICKER_MESSAGES[tickerIdx]);
  }, 2200);

  let pendingSample: GenomeSample | null = null;

  try {
    await yieldToUi();
    const status = await getAppBootstrap();
    clearInterval(ticker);

    callbacks.onStatus(status);
    callbacks.onPaths({
      data_dir: status.data_dir,
      db_path: status.db_path,
      chain_path: status.chain_path,
      env_path: status.env_path,
    });
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
