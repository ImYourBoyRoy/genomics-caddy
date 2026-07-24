// ./src/lib/utils/researchJobMetrics.bootstrap.test.ts
import { describe, expect, it } from "vitest";
import {
  batchCounterDone,
  batchCounterLabel,
  isBootstrapPhase,
  pipelineStepIndex,
} from "./researchJobMetrics";

describe("bootstrap progress helpers", () => {
  it("detects bootstrap phase", () => {
    expect(isBootstrapPhase("Qdrant bootstrap")).toBe(true);
    expect(isBootstrapPhase("qdrant")).toBe(false);
  });

  it("uses prepared chunk count for bootstrap, not total-as-done", () => {
    expect(batchCounterDone("Qdrant bootstrap", 48, 0, 210)).toBe(48);
    expect(batchCounterDone("qdrant", 48, 0, 210)).toBe(210);
    expect(batchCounterLabel("Qdrant bootstrap")).toBe("chunks scanned");
    expect(batchCounterLabel("qdrant")).toBe("upserting");
  });

  it("does not mark Upsert pipeline step active during bootstrap", () => {
    expect(pipelineStepIndex("Qdrant bootstrap")).toBe(-1);
    expect(pipelineStepIndex("qdrant")).toBeGreaterThanOrEqual(0);
  });
});
