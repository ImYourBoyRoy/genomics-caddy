// ./src/lib/utils/loadingTelemetry.test.ts
/* Unit tests for the loading telemetry formatters, ETA smoother, and step builders. Run: pnpm test */
import { describe, expect, it } from "vitest";
import {
  EtaSmoother,
  clampFraction,
  formatClock,
  formatRemaining,
  formatStepDuration,
  parseMessageProgress,
} from "./loadingTelemetry";
import { buildImportSteps, buildResourceSteps, stepStates } from "../components/common/bootstrap/bootstrapPipeline";
import { overallPercent, segmentsFromSteps } from "../components/common/loading/pipelineTypes";

const clock = { durationMs: () => 1200, stepElapsedMs: () => 300 };

describe("loading telemetry formatting", () => {
  it("formats a fixed-width stopwatch", () => {
    expect(formatClock(0)).toBe("0:00");
    expect(formatClock(7_400)).toBe("0:07");
    expect(formatClock(723_000)).toBe("12:03");
    expect(formatClock(3_723_000)).toBe("1:02:03");
    expect(formatClock(Number.NaN)).toBe("0:00");
  });

  it("formats step durations and remaining estimates", () => {
    expect(formatStepDuration(400)).toBe("0.4s");
    expect(formatStepDuration(12_000)).toBe("12s");
    expect(formatStepDuration(65_000)).toBe("1m 05s");
    expect(formatRemaining(2)).toBe("<5s");
    expect(formatRemaining(42)).toBe("~42s");
    expect(formatRemaining(80)).toBe("~1m 20s");
    expect(formatRemaining(-1)).toBe("");
  });

  it("clamps fractions and parses (n of m) counters", () => {
    expect(clampFraction(2)).toBe(1);
    expect(clampFraction(Number.NaN)).toBe(0);
    expect(parseMessageProgress("Importing pack (3 of 12)")?.fraction).toBeCloseTo(0.25);
    expect(parseMessageProgress("no counter")).toBeNull();
    expect(parseMessageProgress("bad (1 of 0)")).toBeNull();
  });
});

describe("EtaSmoother", () => {
  it("stays quiet without signal and never reports negative time", () => {
    const smoother = new EtaSmoother();
    expect(smoother.update(null)).toBeNull();
    expect(smoother.update({ fraction: 0.001, elapsedMs: 100 })).toBeNull();
    const eta = smoother.update({ fraction: 0.5, elapsedMs: 20_000 });
    expect(eta === null || eta >= 0).toBe(true);
  });
});

describe("pipeline builders", () => {
  it("builds import steps with a single live detail line", () => {
    const steps = buildImportSteps({ phase: "ingesting", status: "Writing genotypes", clock });
    expect(steps.filter((step) => step.state === "active")).toHaveLength(1);
    const active = steps.find((step) => step.state === "active");
    expect(active?.detail).toBe("Writing genotypes");
    expect(active?.elapsedMs).toBe(300);
    expect(steps.filter((step) => step.state === "done").every((step) => step.durationMs === 1200)).toBe(true);
    expect(Object.keys(stepStates(steps))).toHaveLength(steps.length);
  });

  it("derives overall progress from equal-weight segments", () => {
    const steps = buildImportSteps({ phase: "ingesting", status: "", clock });
    const percent = overallPercent(steps);
    expect(percent).toBeGreaterThan(0);
    expect(percent).toBeLessThan(100);
    expect(segmentsFromSteps(steps)).toHaveLength(steps.length);
  });
});
