// ./src/lib/utils/aiAssistantVectorDiagnostics.ts
/**
 * Vector research diagnostics loader for AiAssistantPanel.
 */

import { getVectorResearchDiagnostics } from "../api/tauri";
import type { VectorResearchDiagnostics } from "../types/research";
import type { GenomeSample } from "../types/genomics";

export async function fetchVectorResearchDiagnostics(
  selectedSample: GenomeSample | null,
): Promise<VectorResearchDiagnostics | null> {
  if (!selectedSample) return null;
  try {
    return await getVectorResearchDiagnostics(selectedSample.id);
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    return {
      connected: false,
      collection: "",
      collection_exists: false,
      qdrant_url: "",
      embedding_model: "",
      sweep_quality: "unknown",
      embedding_model_mismatch: false,
      error: message,
    };
  }
}
