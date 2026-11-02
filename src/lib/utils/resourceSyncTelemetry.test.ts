// ./src/lib/utils/resourceSyncTelemetry.test.ts
/* Unit tests for the reference-sync reducer and its ledger/readout builders. Run: pnpm test */
import { describe, expect, it } from "vitest";
import {
  buildResourceSteps,
  resourceFocus,
  RESOURCE_VISIBLE_OPS,
} from "../components/common/bootstrap/bootstrapPipeline";
import {
  emptyResourceSync,
  humanizeAssetId,
  reduceResourceSync,
  type ResourceSyncEvent,
  type ResourceSyncTelemetry,
} from "./resourceSyncTelemetry";

const clock = { durationMs: () => 2000, stepElapsedMs: () => 500 };

function run(events: ResourceSyncEvent[]): ResourceSyncTelemetry {
  return events.reduce(reduceResourceSync, emptyResourceSync());
}

const tier: ResourceSyncEvent = {
  type: "phase",
  assetId: "__bulk__",
  phase: "bulk",
  message: "Sync · preparing tier 1 (3 asset(s))…",
  tier: 1,
  assetCount: 3,
};
const check = (assetId: string, label: string): ResourceSyncEvent => ({
  type: "phase",
  assetId,
  label,
  phase: "check",
  message: `Checking ${label}…`,
});

describe("reduceResourceSync", () => {
  it("tracks tier structure, checks, and skipped assets", () => {
    const state = run([
      tier,
      check("a", "Alpha"),
      { type: "phase", assetId: "a", label: "Alpha", phase: "skip", message: "Alpha up to date — skipping" },
      check("b", "Beta"),
    ]);
    expect(state.tier).toBe(1);
    expect(state.tierAssetCount).toBe(3);
    expect(state.tierAssetsSeen).toBe(2);
    expect(state.skipped).toBe(1);
    expect(state.checking).toEqual({ assetId: "b", label: "Beta" });
    expect(state.ops).toHaveLength(0);
  });

  it("builds a resume-aware download op from byte counters", () => {
    const state = run([
      tier,
      check("a", "Alpha"),
      { type: "phase", assetId: "a", label: "Alpha", phase: "download", message: "Step 1/2 · Downloading Alpha…" },
      { type: "download", assetId: "a", label: "Alpha", bytesDone: 5_000_000, totalBytes: 100_000_000, resumedBytes: 4_000_000, at: 1_000 },
      { type: "download", assetId: "a", label: "Alpha", bytesDone: 7_000_000, totalBytes: 100_000_000, resumedBytes: 4_000_000, at: 2_000 },
    ]);
    const [op] = state.ops;
    expect(state.checking).toBeNull();
    expect(op.kind).toBe("download");
    expect(op.state).toBe("active");
    // Only the 3 MB moved in this attempt count toward speed; resumed bytes never inflate it.
    expect(op.download?.transferBytes).toBe(3_000_000);
    expect(op.download?.speedBytesPerSecond).toBeCloseTo(2_000_000);
    expect(op.download?.percent).toBe(7);
  });

  it("never invents a download percentage for unknown sizes", () => {
    const state = run([
      { type: "download", assetId: "a", label: "Alpha", bytesDone: 1_000_000, totalBytes: 0, at: 1 },
    ]);
    expect(state.ops[0].download?.percent).toBe(-1);
    const [, ...rest] = buildResourceSteps({ message: "", clock, telemetry: state });
    expect(rest[0].fraction).toBeNull();
    expect(rest[0].meta).toBeNull();
  });

  it("finishes a download when the next asset is checked and indexes only on the running signal", () => {
    const state = run([
      tier,
      check("a", "Alpha"),
      { type: "phase", assetId: "a", label: "Alpha", phase: "download", message: "dl" },
      check("b", "Beta"),
      { type: "phase", assetId: "a", label: "Alpha", phase: "import", state: "queued", message: "queued" },
    ]);
    expect(state.ops[0].state).toBe("done");
    expect(state.queued).toEqual(["a"]);
    expect(state.ops.some((op) => op.kind === "index")).toBe(false);

    const running = reduceResourceSync(state, {
      type: "phase",
      assetId: "a",
      label: "Alpha",
      phase: "import",
      state: "running",
      message: "Step 2/2 · Indexing Alpha…",
    });
    expect(running.queued).toEqual([]);
    expect(running.ops.at(-1)?.kind).toBe("index");
    expect(running.ops.at(-1)?.state).toBe("active");
  });

  it("treats 0% before any rows as unknown and keeps row-index imports without ETA", () => {
    const state = run([
      { type: "import", assetId: "g", rowsProcessed: 0, percent: 0, etaSeconds: null, message: "CPU-bound" },
      { type: "import", assetId: "c", rowsProcessed: 1_200_000, percent: null, rowsPerSecond: 85_000, etaSeconds: null, message: "Indexing ClinVar" },
    ]);
    const [gwas, clinvar] = state.ops;
    expect(gwas.percent).toBeNull();
    expect(clinvar.percent).toBeNull();
    expect(clinvar.etaSeconds).toBeNull();
    expect(clinvar.rowsPerSecond).toBe(85_000);
    expect(resourceFocus(state, []).etaSeconds).toBeNull();
    expect(resourceFocus(state, []).etaUnknown).toBe(true);
  });

  it("falls back to a readable label when the backend sends none", () => {
    expect(humanizeAssetId("gwas_catalog")).toBe("Gwas catalog");
    const state = run([{ type: "phase", assetId: "gwas_catalog", phase: "check", message: "" }]);
    expect(state.checking?.label).toBe("Gwas catalog");
  });
});

describe("resource ledger builders", () => {
  it("starts with an active scan row and uses the tier count as its meta", () => {
    const state = run([tier, check("a", "Alpha")]);
    const steps = buildResourceSteps({ message: "Checking Alpha…", clock, telemetry: state });
    expect(steps[0]).toMatchObject({ id: "scan", state: "active", meta: "1 / 3" });
    expect(steps[0].detail).toContain("Alpha");
  });

  it("shows real sizes, speed, and ETA for the active download", () => {
    const state = run([
      tier,
      check("a", "Alpha"),
      { type: "download", assetId: "a", label: "Alpha", bytesDone: 1_000_000, totalBytes: 50_000_000, at: 1_000 },
      { type: "download", assetId: "a", label: "Alpha", bytesDone: 3_000_000, totalBytes: 50_000_000, at: 2_000 },
    ]);
    const steps = buildResourceSteps({ message: "", clock, telemetry: state });
    const active = steps.find((step) => step.state === "active");
    expect(active?.label).toBe("Download Alpha");
    expect(active?.detail).toMatch(/3\.0 MB of 50\.0 MB · 2\.0 MB\/s/);
    expect(active?.meta).toBe("6%");
    const focus = resourceFocus(state, steps);
    expect(focus.etaSeconds).toBe(24);
    expect(focus.etaCaption).toBe("left to download");
    expect(focus.stepTitle).toBe("Tier 1 asset");
    expect(focus.stepCount).toBe(3);
  });

  it("keeps only recent operations visible and drops the scan row once they overflow", () => {
    const events: ResourceSyncEvent[] = [tier];
    for (let i = 0; i < RESOURCE_VISIBLE_OPS + 2; i += 1) {
      events.push(check(`a${i}`, `Asset ${i}`));
      events.push({ type: "phase", assetId: `a${i}`, label: `Asset ${i}`, phase: "download", message: "dl" });
    }
    const steps = buildResourceSteps({ message: "", clock, telemetry: run(events) });
    expect(steps.some((step) => step.id === "scan")).toBe(false);
    expect(steps).toHaveLength(RESOURCE_VISIBLE_OPS);
    expect(steps.filter((step) => step.state === "active")).toHaveLength(1);
  });
});
