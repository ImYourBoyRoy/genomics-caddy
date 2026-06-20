// ./src/lib/utils/evidenceSearch.ts
/**
 * Evidence library search orchestration (SQLite, Qdrant, workbench hybrid).
 */

import {
  searchEvidence,
  searchQdrantEvidence,
  searchQdrantTraitDiscovery,
  searchAssociationsHybrid,
  type EvidenceRecord,
} from "../api/tauri";
import type { EvidenceCard, QdrantHit } from "../types/research";

export type EvidenceSearchSource = "sqlite" | "qdrant" | "workbench";

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
}

export interface EvidenceSearchResult {
  results: EvidenceRecord[];
  qdrantResults: QdrantHit[];
  workbenchCards: EvidenceCard[];
}

export async function runEvidenceSearch(
  params: EvidenceSearchParams
): Promise<EvidenceSearchResult> {
  const q = params.query.trim();
  const traitBrowse =
    params.searchSource === "qdrant" &&
    params.discoveryMode === "trait_index" &&
    !!params.traitCategory;

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
        limit: 25,
      },
      params.ollamaUrl
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
            50
          )
        : await searchQdrantEvidence(
            q,
            params.ollamaUrl,
            params.sampleId,
            25,
            params.traitCategory || undefined
          );
    return { results: [], qdrantResults, workbenchCards: [] };
  }

  const results = await searchEvidence(
    q,
    params.ollamaUrl || undefined,
    params.ollamaToken || undefined
  );
  return { results, qdrantResults: [], workbenchCards: [] };
}
