// ./src/lib/utils/consultationContextPack.test.ts
/* Ask-scoped consultation context packer tests. Run: pnpm test */
import { describe, expect, it, vi } from "vitest";

vi.mock("./markerPacksState.svelte", () => ({
  markerPacksStore: {
    manifest: {
      packs: [
        { id: "metabolism", label: "Metabolism" },
        { id: "hormones_reproductive", label: "Hormone & Reproductive Context" },
      ],
    },
    packs: {},
  },
}));

vi.mock("./supportResourceContext", () => ({
  buildSupportResourceContext: () => ({ note: "support" }),
}));

vi.mock("./conditionEvidence", () => ({
  buildConditionEvidenceSummaries: () => [],
  buildCatalogAssociationSummaries: () => [],
  buildConditionCoverageSummaries: () => [],
  getConditionCoverageGaps: () => [],
}));

vi.mock("./qdrantRag", () => ({
  buildVectorResearchBlock: (_hits: unknown, meta: { query: string }) => ({
    query: meta.query,
    hits: [],
  }),
}));

import type { EvaluatedMarker, GeneratedReport, GenomeSample } from "../types/genomics";
import { buildCandidatePool, calculateContextStats } from "./aiPrompt";
import {
  buildConsultationPack,
  CONSULTATION_BUDGETS,
  rankCandidates,
  toProviderMessages,
} from "./consultationContextPack";

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

function marker(partial: Partial<EvaluatedMarker> & Pick<EvaluatedMarker, "rsid" | "gene">): EvaluatedMarker {
  return {
    link_id: partial.link_id ?? `link:${partial.rsid}`,
    rsid: partial.rsid,
    gene: partial.gene,
    variant_name: partial.variant_name ?? `${partial.gene} marker`,
    user_genotype: partial.user_genotype ?? "AG",
    effect_allele: partial.effect_allele ?? "A",
    effect_count: partial.effect_count ?? 1,
    effect_direction: partial.effect_direction ?? "risk",
    evidence_tier: partial.evidence_tier ?? "B_replicated",
    severity_class: partial.severity_class ?? "moderate",
    assertion_status: partial.assertion_status ?? "Verified",
    interpretation_allowed: partial.interpretation_allowed ?? true,
    impact: partial.impact ?? "Example",
    interpretation: partial.interpretation ?? "Association context.",
    do_not_claim: partial.do_not_claim ?? [],
    confirm_with: partial.confirm_with ?? [],
    clinical_confirmation_required: partial.clinical_confirmation_required ?? false,
    sources: partial.sources ?? [],
  } as EvaluatedMarker;
}

function sampleReport(): GeneratedReport {
  return {
    sections: [
      {
        name: "Metabolism",
        markers: [
          marker({ rsid: "rs1801133", gene: "MTHFR", effect_count: 2, clinical_confirmation_required: true }),
          marker({ rsid: "rs4680", gene: "COMT", effect_count: 0, severity_class: "benign" }),
          marker({ rsid: "rs1057910", gene: "CYP2C9", effect_count: 1 }),
        ],
      },
    ],
  } as unknown as GeneratedReport;
}

const sample = { id: 1, name: "Example", genetic_sex: "unknown" } as unknown as GenomeSample;

describe("candidate pool parity", () => {
  it("matches calculateContextStats included count for active_findings", () => {
    const report = sampleReport();
    const packs = { metabolism: true };
    const pool = buildCandidatePool(report, packs, "active_findings", {});
    const stats = calculateContextStats(report, packs, "active_findings");
    expect(pool.length).toBe(stats.included);
    expect(pool.every((row) => (row.payload.effect_count ?? 0) > 0)).toBe(true);
  });
});

describe("rankCandidates", () => {
  it("boosts gene/rsid ask overlap and vector hits", () => {
    const report = sampleReport();
    const pool = buildCandidatePool(report, { metabolism: true }, "full_selected", {});
    const ranked = rankCandidates(pool, "Tell me about MTHFR and rs1801133", [
      { rsid: "rs1801133", gene: "MTHFR", score: 0.9 } as never,
    ]);
    expect(ranked[0].payload.rsid).toBe("rs1801133");
    expect(ranked[0].rankReasons).toEqual(
      expect.arrayContaining([expect.stringMatching(/term|vector/)]),
    );
    expect(ranked[0].score).toBeGreaterThan(ranked[ranked.length - 1].score);
  });
});

describe("buildConsultationPack", () => {
  it("keeps ChatGPT instructions tiny and genomic JSON in turn/session blocks", () => {
    const pack = buildConsultationPack({
      askText: "What does my MTHFR finding mean?",
      selectedSample: sample,
      generatedReport: sampleReport(),
      selectedPacks: { metabolism: true },
      contextMode: "active_findings",
      consultationMode: "general",
      userProfile: emptyProfile(),
      systemInstructions: "Be careful with claims.",
      laypersonMap: {},
      history: [],
      provider: "chatgpt",
    });

    expect(pack.instructions.length).toBeLessThan(CONSULTATION_BUDGETS.chatgpt.instructionsMaxChars);
    expect(pack.instructions).not.toContain("[TURN CONTEXT]");
    expect(pack.turnBlock).toContain("[TURN CONTEXT]");
    expect(pack.sessionBlock).toContain("[SESSION CONTEXT]");
    expect(pack.turnBlock.length).toBeLessThanOrEqual(CONSULTATION_BUDGETS.chatgpt.turnContextChars);
    expect(pack.report.sentFindings).toBeGreaterThan(0);
    expect(pack.report.poolSize).toBeGreaterThanOrEqual(pack.report.sentFindings);

    const messages = toProviderMessages(pack);
    expect(messages[0].role).toBe("system");
    expect(messages[0].content).toBe(pack.instructions);
    expect(messages.some((m) => m.content.includes("[SESSION CONTEXT]"))).toBe(true);
    expect(messages.some((m) => m.content.includes("[TURN CONTEXT]"))).toBe(true);
    expect(messages[messages.length - 1].content).toBe("What does my MTHFR finding mean?");
  });

  it("never sends developer raw_report to ChatGPT and remaps with a notice", () => {
    const hugeReport = {
      ...sampleReport(),
      padding: "Z".repeat(50_000),
    } as unknown as GeneratedReport;

    const pack = buildConsultationPack({
      askText: "summarize",
      selectedSample: sample,
      generatedReport: hugeReport,
      selectedPacks: { metabolism: true },
      contextMode: "developer_raw_json",
      consultationMode: "general",
      userProfile: emptyProfile(),
      systemInstructions: "Be careful",
      laypersonMap: {},
      history: [],
      provider: "chatgpt",
    });

    expect(pack.report.effectiveMode).toBe("active_findings");
    expect(pack.report.notices.some((n) => /Developer raw JSON/i.test(n))).toBe(true);
    expect(pack.turnBlock).not.toContain("raw_report");
    expect(pack.turnBlock).not.toContain("padding");
    expect(pack.instructions.length).toBeLessThan(CONSULTATION_BUDGETS.chatgpt.instructionsMaxChars);
  });

  it("fits findings atomically under the turn budget", () => {
    const manyMarkers = Array.from({ length: 80 }, (_, i) =>
      marker({
        rsid: `rs${1000 + i}`,
        gene: i % 2 === 0 ? "MTHFR" : `GENE${i}`,
        effect_count: 1,
        link_id: `link:${i}`,
      }),
    );
    const report = {
      sections: [{ name: "Metabolism", markers: manyMarkers }],
    } as unknown as GeneratedReport;

    const pack = buildConsultationPack({
      askText: "MTHFR related findings",
      selectedSample: sample,
      generatedReport: report,
      selectedPacks: { metabolism: true },
      contextMode: "full_selected",
      consultationMode: "general",
      userProfile: emptyProfile(),
      systemInstructions: "Be careful",
      laypersonMap: {},
      history: [],
      provider: "chatgpt",
    });

    expect(pack.turnBlock.length).toBeLessThanOrEqual(CONSULTATION_BUDGETS.chatgpt.turnContextChars);
    expect(pack.report.sentFindings).toBeGreaterThan(0);
    // No mid-truncated finding JSON: turn block must parse after the marker prefix.
    const json = pack.turnBlock.replace(/^\[TURN CONTEXT\]\n/, "");
    expect(() => JSON.parse(json)).not.toThrow();
  });
});
