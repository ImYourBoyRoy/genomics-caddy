// ./src/lib/dev/loadingLabScenarios.test.ts
/* Unit tests for the permanent loading-lab fixtures. Run: pnpm test */
import { describe, expect, it } from "vitest";
import {
  LOADING_LAB_DEFAULT_SCENARIO,
  LOADING_LAB_SCENARIO_IDS,
  loadingLabScenario,
} from "./loadingLabScenarios";

describe("loadingLabScenarios", () => {
  it("exposes the five canonical visual states", () => {
    expect(LOADING_LAB_SCENARIO_IDS.sort()).toEqual(
      ["import", "res-download", "res-index", "res-scan", "startup"].sort(),
    );
  });

  it("defaults unknown ids to the download scenario", () => {
    expect(loadingLabScenario("nope").id).toBe(LOADING_LAB_DEFAULT_SCENARIO);
  });

  it("keeps resource scenarios free of genotype-shaped fields", () => {
    for (const id of ["res-scan", "res-download", "res-index"]) {
      const scenario = loadingLabScenario(id);
      expect(scenario.mode).toBe("resources");
      expect(scenario.resourceSync).not.toBeNull();
      expect(scenario.importProgress).toBeNull();
      expect(scenario.reportProgress).toBeNull();
    }
  });

  it("builds a resume-aware download detail for the download fixture", () => {
    const ops = loadingLabScenario("res-download").resourceSync?.ops ?? [];
    const active = ops.find((op) => op.state === "active");
    expect(active?.kind).toBe("download");
    expect(active?.download?.resumedBytes).toBe(10_000_000);
    expect(active?.download?.percent).toBe(22);
  });

  it("marks ClinVar indexing as unknown total (no fabricated percent)", () => {
    const ops = loadingLabScenario("res-index").resourceSync?.ops ?? [];
    const active = ops.find((op) => op.state === "active");
    expect(active?.kind).toBe("index");
    expect(active?.percent).toBeNull();
    expect(active?.rowsProcessed).toBe(1_240_000);
  });
});
