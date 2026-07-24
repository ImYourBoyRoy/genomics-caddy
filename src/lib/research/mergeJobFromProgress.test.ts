// ./src/lib/research/mergeJobFromProgress.test.ts
import { describe, expect, it } from "vitest";
import { mergeJobFromProgress } from "./mergeJobFromProgress";
import type { ResearchJob, ResearchProgress } from "../types/research";

function job(partial: Partial<ResearchJob>): ResearchJob {
  return {
    job_id: "job_1_1",
    sample_id: 1,
    status: "running",
    total_markers: 100,
    enriched_count: 10,
    priority_complete: false,
    started_at: 1,
    last_updated: 1,
    loop_active: true,
    ...partial,
  };
}

function progress(partial: Partial<ResearchProgress>): ResearchProgress {
  return {
    job_id: "job_1_1",
    status: "running",
    enriched_count: 11,
    total_markers: 100,
    message: "working",
    ...partial,
  };
}

describe("mergeJobFromProgress", () => {
  it("does not resurrect idle cancelled jobs from stale running pulses", () => {
    const current = job({
      status: "idle",
      error_message: "Sweep cancelled by user.",
      loop_active: false,
    });
    const merged = mergeJobFromProgress(current, progress({ status: "running" }), 1);
    expect(merged.status).toBe("idle");
    expect(merged.loop_active).toBe(false);
  });

  it("clears loop_active when progress reports idle", () => {
    const current = job({ status: "running", loop_active: true });
    const merged = mergeJobFromProgress(
      current,
      progress({ status: "idle", message: "Sweep cancelled by user." }),
      1,
    );
    expect(merged.status).toBe("idle");
    expect(merged.loop_active).toBe(false);
  });

  it("does not resurrect idle cancelled jobs from stale paused pulses", () => {
    const current = job({
      status: "idle",
      error_message: "Sweep cancelled by user.",
      loop_active: false,
    });
    const merged = mergeJobFromProgress(current, progress({ status: "paused" }), 1);
    expect(merged.status).toBe("idle");
    expect(merged.loop_active).toBe(false);
  });
});
