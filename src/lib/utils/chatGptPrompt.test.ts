// ./src/lib/utils/chatGptPrompt.test.ts
/* ChatGPT prompt budget tests. Run: pnpm test */
import { describe, expect, it, vi } from "vitest";

vi.mock("./markerPacksState.svelte", () => ({
  markerPacksStore: {
    manifest: { packs: [] },
    packs: {},
  },
}));

import type { GeneratedReport, GenomeSample } from "../types/genomics";
import {
  buildChatGptSystemPrompt,
  CHATGPT_CONTEXT_BUDGET_CHARS,
  CHATGPT_INSTRUCTIONS_MAX_CHARS,
} from "./chatGptPrompt";

function emptyProfile() {
  return {
    goals: "",
    challenges: "",
    relevantBodySystems: "",
    reproductiveHormoneContext: "",
    diet: "",
    supplements: "",
    medications: "",
    bloodwork: "",
    diagnoses: "",
    supportiveTests: "",
    injectProfile: false,
  };
}

describe("buildChatGptSystemPrompt", () => {
  it("downgrades developer raw JSON and emits compact context", () => {
    const hugeReport = {
      sections: [],
      // Simulate the multi-MB raw report field that blew the OpenAI cap.
      padding: "Z".repeat(2_000_000),
    } as unknown as GeneratedReport;

    const result = buildChatGptSystemPrompt({
      selectedSample: { id: 1, name: "Example", genetic_sex: "unknown" } as unknown as GenomeSample,
      generatedReport: hugeReport,
      selectedPacks: {},
      onlyActiveFindings: true,
      contextMode: "developer_raw_json",
      consultationMode: "general",
      userProfile: emptyProfile(),
      systemInstructions: "Be careful",
      laypersonMap: {},
    });

    expect(result.warnings.some((w) => /Developer raw JSON/i.test(w))).toBe(true);
    expect(result.prompt).toContain("[JSON CONTEXT]");
    expect(result.prompt).not.toContain("\n  \"app_context\"");
    expect(result.prompt).not.toContain("padding");
    expect(result.instructionsChars).toBeLessThan(CHATGPT_INSTRUCTIONS_MAX_CHARS);
    expect(result.contextChars).toBeLessThanOrEqual(CHATGPT_CONTEXT_BUDGET_CHARS);
  });
});
