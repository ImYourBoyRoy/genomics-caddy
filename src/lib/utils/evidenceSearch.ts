// ./src/lib/utils/evidenceSearch.ts
/**
 * Evidence library search orchestration (SQLite, Qdrant, workbench hybrid, corpus browse).
 */

import {
  searchEvidence,
  searchQdrantEvidence,
  searchQdrantTraitDiscovery,
  searchAssociationsHybrid,
  browseAssociations,
  type EvidenceRecord,
} from "../api/tauri";
import type { EvidenceCard, QdrantHit } from "../types/research";

export type EvidenceSearchSource = "sqlite" | "qdrant" | "workbench";
export type BrowsePreset = "actionable" | "clinical" | "gwas" | "unknown";

export interface EvidenceSearchParams {
  query: string;
  searchSource: EvidenceSearchSource;
  discoveryMode: "semantic" | "trait_index";
  traitCategory: string;
  sampleId: number | null;
  ollamaUrl: string;
  ollamaToken: string;
  wbDirectionFilter: string;
  wbMinDq: number;
  wbMinWellness?: number;
  wbEvidenceTier: string;
  browsePreset?: BrowsePreset;
  limit?: number;
  offset?: number;
}

export interface EvidenceSearchResult {
  results: EvidenceRecord[];
  qdrantResults: QdrantHit[];
  workbenchCards: EvidenceCard[];
  browseTotal?: number;
}

const DEFAULT_HYBRID_LIMIT = 50;
const DEFAULT_BROWSE_LIMIT = 50;

export async function runEvidenceBrowse(
  sampleId: number,
  preset: BrowsePreset,
  traitCategory?: string,
  limit = DEFAULT_BROWSE_LIMIT,
  offset = 0,
): Promise<{ cards: EvidenceCard[]; total: number }> {
  const result = await browseAssociations({
    sample_id: sampleId,
    preset,
    trait_category: traitCategory || undefined,
    limit,
    offset,
  });
  return { cards: result.cards, total: result.total_count };
}

export async function runEvidenceSearch(
  params: EvidenceSearchParams,
): Promise<EvidenceSearchResult> {
  const q = params.query.trim();
  const traitBrowse =
    params.searchSource === "qdrant" &&
    params.discoveryMode === "trait_index" &&
    !!params.traitCategory;
  const browsePreset = params.browsePreset;

  if (params.searchSource === "workbench" && browsePreset && params.sampleId != null) {
    const { cards, total } = await runEvidenceBrowse(
      params.sampleId,
      browsePreset,
      params.traitCategory || undefined,
      params.limit ?? DEFAULT_BROWSE_LIMIT,
      params.offset ?? 0,
    );
    return { results: [], qdrantResults: [], workbenchCards: cards, browseTotal: total };
  }

  if (!q && !traitBrowse) {
    return { results: [], qdrantResults: [], workbenchCards: [] };
  }

  if (params.searchSource === "workbench") {
    if (params.sampleId == null) {
      throw new Error("No genomic sample loaded. Please select a profile first.");
    }
    const workbenchCards = await searchAssociationsHybrid(
      {
        sample_id: params.sampleId,
        query: q,
        trait_category: params.traitCategory || undefined,
        evidence_tier: params.wbEvidenceTier || undefined,
        has_direction:
          params.wbDirectionFilter === "known"
            ? true
            : params.wbDirectionFilter === "unknown"
              ? false
              : undefined,
        min_data_quality: params.wbMinDq,
        min_wellness_actionability: params.wbMinWellness,
        limit: params.limit ?? DEFAULT_HYBRID_LIMIT,
      },
      params.ollamaUrl,
    );
    return { results: [], qdrantResults: [], workbenchCards };
  }

  if (params.searchSource === "qdrant") {
    if (params.sampleId == null) {
      throw new Error("No genomic sample loaded. Please select a profile first.");
    }
    const qdrantResults =
      params.discoveryMode === "trait_index" && params.traitCategory
        ? await searchQdrantTraitDiscovery(
            q || undefined,
            params.traitCategory,
            params.sampleId,
            params.ollamaUrl,
            params.limit ?? DEFAULT_HYBRID_LIMIT,
          )
        : await searchQdrantEvidence(
            q,
            params.ollamaUrl,
            params.sampleId,
            params.limit ?? DEFAULT_HYBRID_LIMIT,
            params.traitCategory || undefined,
          );
    return { results: [], qdrantResults, workbenchCards: [] };
  }

  const results = await searchEvidence(
    q,
    params.ollamaUrl || undefined,
    params.ollamaToken || undefined,
  );
  return { results, qdrantResults: [], workbenchCards: [] };
}
