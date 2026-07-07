// ./src/lib/utils/researchRunReadiness.test.ts
import { describe, expect, it } from "vitest";
import { computeRunReadiness } from "./researchRunReadiness";
import {
  DEFAULT_CONNECTION_ACTIVITY,
} from "../types/research";

const baseJob = {
  job_id: "j1",
  sample_id: 1,
  status: "idle" as const,
  total_markers: 100,
  enriched_count: 0,
  priority_complete: false,
  started_at: 0,
  last_updated: 0,
};

const readyConnection = {
  success: true,
  collection_exists: true,
};

const scopePreview = {
  curated: 10,
  agent_discoveries: 0,
  gwas_discovery: 0,
  non_reference: 0,
  total_unique: 10,
  genotype_total: 10,
  gwas_reference_count: 0,
  gwas_genome_overlap: 0,
  gwas_beyond_cap: 0,
};

describe("computeRunReadiness", () => {
  it("allows start when infrastructure is ready", () => {
    const r = computeRunReadiness({
      configUrl: "http://localhost:6333",
      connectionStatus: readyConnection,
      connectionActivity: DEFAULT_CONNECTION_ACTIVITY,
      ollamaStatus: "live",
      gnomadSweepReady: true,
      gnomadSourceEnabled: false,
      scopePreview,
      scopePreviewLoading: false,
      job: baseJob,
    });
    expect(r.canStart).toBe(true);
    expect(r.canPause).toBe(false);
  });

  it("blocks start on embedding model mismatch", () => {
    const r = computeRunReadiness({
      configUrl: "http://localhost:6333",
      connectionStatus: readyConnection,
      connectionActivity: DEFAULT_CONNECTION_ACTIVITY,
      ollamaStatus: "live",
      gnomadSweepReady: true,
      gnomadSourceEnabled: false,
      scopePreview,
      scopePreviewLoading: false,
      job: baseJob,
      embeddingModelMismatch: true,
      indexEmbeddingModel: "nomic-embed-text",
      configuredEmbeddingModel: "mxbai-embed-large",
    });
    expect(r.canStart).toBe(false);
    expect(r.primaryHint).toMatch(/Embedding model mismatch/);
  });

  it("detects interrupted sweep (running without loop)", () => {
    const r = computeRunReadiness({
      configUrl: "http://localhost:6333",
      connectionStatus: readyConnection,
      connectionActivity: DEFAULT_CONNECTION_ACTIVITY,
      ollamaStatus: "live",
      gnomadSweepReady: true,
      gnomadSourceEnabled: false,
      scopePreview,
      scopePreviewLoading: false,
      job: {
        ...baseJob,
        status: "running",
        loop_active: false,
      },
    });
    expect(r.sweepInterrupted).toBe(true);
    expect(r.canResume).toBe(true);
    expect(r.canPause).toBe(false);
    expect(r.primaryHint).toMatch(/interrupted/i);
  });

  it("requires gnomAD readiness when source enabled", () => {
    const r = computeRunReadiness({
      configUrl: "http://localhost:6333",
      connectionStatus: readyConnection,
      connectionActivity: DEFAULT_CONNECTION_ACTIVITY,
      ollamaStatus: "live",
      gnomadSweepReady: false,
      gnomadSourceEnabled: true,
      scopePreview,
      scopePreviewLoading: false,
      job: baseJob,
    });
    expect(r.canStart).toBe(false);
    expect(r.secondaryHint).toMatch(/gnomAD/);
  });

  it("ignores default scope when preview is loading", () => {
    const r = computeRunReadiness({
      configUrl: "http://localhost:6333",
      connectionStatus: readyConnection,
      connectionActivity: { phase: "loading-scope", message: "…" },
      ollamaStatus: "live",
      gnomadSweepReady: true,
      gnomadSourceEnabled: false,
      scopePreview: null,
      scopePreviewLoading: true,
      job: baseJob,
    });
    expect(r.canStart).toBe(false);
    expect(r.primaryHint).toMatch(/Counting sweep scope/);
  });
});
