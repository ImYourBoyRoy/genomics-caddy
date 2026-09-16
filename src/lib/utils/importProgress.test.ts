import { describe, expect, it } from "vitest";
import {
  importPhaseForProgress,
  importStepState,
  importTimelineProgress,
} from "./importProgress";

describe("DNA import progress waterfall", () => {
  it("maps native checkpoints to the matching visual phase", () => {
    expect(importPhaseForProgress("Loading file...", 5)).toBe("reading");
    expect(importPhaseForProgress("Parsing genetic records...", 15)).toBe("parsing");
    expect(importPhaseForProgress("Initializing liftover database...", 45)).toBe("liftover");
    expect(importPhaseForProgress("Liftover & Ingesting SNPs: 50,000/434,818...", 55)).toBe("ingesting");
    expect(importPhaseForProgress("Building variant placement index...", 96)).toBe("indexing");
  });

  it("keeps completed steps behind the active import phase", () => {
    expect(importStepState("ingesting", "validate")).toBe("done");
    expect(importStepState("ingesting", "parse")).toBe("done");
    expect(importStepState("ingesting", "liftover")).toBe("done");
    expect(importStepState("ingesting", "store")).toBe("active");
    expect(importStepState("ingesting", "index")).toBe("pending");
    expect(importStepState("error", "parse", "parse")).toBe("error");
    expect(importStepState("error", "validate", "parse")).toBe("done");
  });

  it("completes the visual timeline only after profile/report loading", () => {
    expect(importTimelineProgress("report")).toBeLessThan(100);
    expect(importTimelineProgress("ready")).toBe(100);
  });

  it("shows validation as complete and no import work active while awaiting confirmation", () => {
    expect(importStepState("awaiting-confirmation", "validate")).toBe("done");
    expect(importStepState("awaiting-confirmation", "parse")).toBe("pending");
    expect(importTimelineProgress("awaiting-confirmation")).toBe(14);
  });
});
