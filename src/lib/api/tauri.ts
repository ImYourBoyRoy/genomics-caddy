// ./src/lib/api/tauri.ts
/*
Module Docstring:
Purpose: Tauri command invocation wrapper API layer.
Responsibilities:
- Encapsulate Tauri invoke calls into typed asynchronous functions.
- Handle type conversions between TypeScript and Tauri Rust payloads.
Key Inputs: Functional parameters matching Tauri commands.
Key Outputs: Strongly typed promises wrapping genomic and app state.
Operational Notes: Exposes clean promise interfaces for Svelte components.
*/

import { invoke } from "@tauri-apps/api/core";
import type {
  OllamaModelDetails,
  ActiveOllamaModel,
  McpToolDefinition,
  DbChatSession,
} from "../types/api";
import type { ChatMessage } from "../types/agent";
import type {
  AppPaths,
  AppBootstrapStatus,
  DiscoveredFindingSummary,
  GenomeSample,
  DbSnpRecord,
  GeneratedReport,
} from "../types/genomics";

export async function selectFile(): Promise<string | null> {
  return invoke<string | null>("select_file");
}

export async function saveReportJson(content: string, defaultFilename: string): Promise<boolean> {
  return invoke<boolean>("save_report_json", { content, defaultFilename });
}

export async function getAppPaths(): Promise<AppPaths> {
  return invoke<AppPaths>("get_app_paths");
}

export async function getAppBootstrap(): Promise<AppBootstrapStatus> {
  return invoke<AppBootstrapStatus>("get_app_bootstrap");
}

export async function getDiscoveredFindingsSummary(
  sampleId: number
): Promise<DiscoveredFindingSummary[]> {
  return invoke<DiscoveredFindingSummary[]>("get_discovered_findings_summary", { sampleId });
}

export async function importGenome(filePath: string, sampleName: string): Promise<number> {
  return invoke<number>("import_genome", { filePath, sampleName });
}

export async function getSamples(): Promise<GenomeSample[]> {
  return invoke<GenomeSample[]>("get_samples");
}

export async function queryRsids(sampleId: number, rsids: string[]): Promise<DbSnpRecord[]> {
  return invoke<DbSnpRecord[]>("query_rsids", { sampleId, rsids });
}

export async function queryRegion(
  sampleId: number,
  chromosome: string,
  start: number,
  end: number
): Promise<DbSnpRecord[]> {
  return invoke<DbSnpRecord[]>("query_region", { sampleId, chromosome, start, end });
}

export async function generateReport(sampleId: number, templateJson: string): Promise<GeneratedReport> {
  return invoke<GeneratedReport>("generate_report", { sampleId, templateJson });
}

export async function deleteSample(sampleId: number): Promise<void> {
  return invoke<void>("delete_sample", { sampleId });
}

export async function checkChainStatus(): Promise<boolean> {
  return invoke<boolean>("check_chain_status");
}

export async function downloadChain(): Promise<void> {
  return invoke<void>("download_chain_file");
}

export async function scanOllamaModels(url: string, token: string | undefined): Promise<string[]> {
  return invoke<string[]>("scan_ollama_models", { url, token });
}

export async function streamOllamaChat(
  streamId: string,
  url: string,
  token: string | undefined,
  model: string,
  messages: ChatMessage[],
  temperature?: number,
  numPredict?: number
): Promise<void> {
  return invoke<void>("stream_ollama_chat", {
    streamId,
    url,
    token,
    model,
    messages,
    temperature,
    numPredict,
  });
}

export async function cancelOllamaStream(): Promise<void> {
  return invoke<void>("cancel_ollama_stream");
}

export async function chatOllama(
  url: string,
  token: string | undefined,
  model: string,
  messages: ChatMessage[],
  temperature?: number
): Promise<string> {
  return invoke<string>("chat_ollama", { url, token, model, messages, temperature });
}

export async function showOllamaModel(
  url: string,
  token: string | undefined,
  name: string
): Promise<OllamaModelDetails> {
  return invoke<OllamaModelDetails>("show_ollama_model", { url, token, name });
}

export async function getActiveOllamaModels(
  url: string,
  token: string | undefined
): Promise<{ models: ActiveOllamaModel[] }> {
  return invoke<{ models: ActiveOllamaModel[] }>("get_active_ollama_models", { url, token });
}

export async function getCurrentExe(): Promise<string> {
  return invoke<string>("get_current_exe");
}

export async function getMcpTools(): Promise<McpToolDefinition[]> {
  return invoke<McpToolDefinition[]>("get_mcp_tools");
}

export interface EvidenceRecord {
  rsid: string;
  gene: string;
  evidence_text: string;
  source_citation: string;
  has_embedding: boolean;
  similarity: number | null;
}

export async function listEvidenceSources(): Promise<string[]> {
  return invoke<string[]>("list_evidence_sources");
}

export async function getEvidenceForMarker(rsid: string): Promise<EvidenceRecord[]> {
  return invoke<EvidenceRecord[]>("get_evidence_for_marker", { rsid });
}

export async function searchEvidence(
  query: string,
  ollamaUrl?: string,
  ollamaToken?: string
): Promise<EvidenceRecord[]> {
  return invoke<EvidenceRecord[]>("search_evidence", {
    query,
    ollamaUrl: ollamaUrl || undefined,
    ollamaToken: ollamaToken || undefined,
  });
}

export async function getChatSessions(sampleId: number | null): Promise<DbChatSession[]> {
  return invoke<DbChatSession[]>("get_chat_sessions", { sampleId });
}

export async function saveChatSession(session: DbChatSession): Promise<void> {
  return invoke<void>("save_chat_session", { session });
}

export async function deleteChatSession(sessionId: string): Promise<void> {
  return invoke<void>("delete_chat_session", { sessionId });
}

export async function fetchExternalApi(
  url: string,
  apiKey?: string,
  ttlSecs?: number
): Promise<unknown> {
  return invoke<unknown>("fetch_external_api", {
    url,
    apiKey: apiKey || undefined,
    ttlSecs: ttlSecs || undefined,
  });
}

export async function getChromosomeCounts(sampleId: number): Promise<Record<string, number>> {
  return invoke<Record<string, number>>("get_chromosome_counts", { sampleId });
}

// ── Qdrant / Research ───────────────────────────────────────────────────────

import type {
  QdrantConfigPublic,
  QdrantConfigUpdate,
  QdrantConnectionStatus,
  ResearchJob,
  QdrantHit,
  VectorResearchDiagnostics,
  ResearchScopeConfig,
  ResearchScopePreview,
  ReferenceStatus,
  GwasSyncResult,
  PurgeCollectionResult,
  ConnectionActivity,
  EvidenceCard,
  HybridSearchParams,
  SimilarSearchParams,
  QualityDashboard,
  TraitClusterSummary,
  CandidateMarkerRow,
  EvidencePacket,
  MatchExplanation,
  BackfillResult,
  ReembedResult,
  ActionabilityPoint,
  ChromosomeTraitBand,
  PathwayFlowRow,
  AtlasPoint,
  VectorAtlasResult,
} from "../types/research";

export type {
  QdrantConfigPublic,
  QdrantConfigUpdate,
  QdrantConnectionStatus,
  ResearchJob,
  QdrantHit,
  VectorResearchDiagnostics,
  ResearchScopeConfig,
  ResearchScopePreview,
  ReferenceStatus,
  GwasSyncResult,
  PurgeCollectionResult,
  ConnectionActivity,
  EvidenceCard,
  HybridSearchParams,
  SimilarSearchParams,
  QualityDashboard,
  TraitClusterSummary,
  CandidateMarkerRow,
  EvidencePacket,
  MatchExplanation,
  BackfillResult,
  AtlasPoint,
  VectorAtlasResult,
};

export async function testQdrantConnection(
  url: string,
  apiKey?: string,
  collection?: string
): Promise<QdrantConnectionStatus> {
  return invoke<QdrantConnectionStatus>("test_qdrant_connection", {
    url,
    apiKey: apiKey || undefined,
    collection: collection || undefined,
  });
}

export async function getQdrantConfig(): Promise<QdrantConfigPublic> {
  return invoke<QdrantConfigPublic>("get_qdrant_config");
}

export async function saveQdrantConfig(config: QdrantConfigUpdate): Promise<void> {
  return invoke<void>("save_qdrant_config", { update: config });
}

export async function createQdrantCollection(ollamaUrl: string): Promise<QdrantConnectionStatus> {
  return invoke<QdrantConnectionStatus>("create_qdrant_collection", { ollamaUrl });
}

export async function purgeQdrantCollection(
  sampleId?: number
): Promise<PurgeCollectionResult> {
  return invoke<PurgeCollectionResult>("purge_qdrant_collection", {
    sampleId: sampleId ?? null,
  });
}

export async function getResearchJobStatus(sampleId: number): Promise<ResearchJob | null> {
  return invoke<ResearchJob | null>("get_research_job_status", { sampleId });
}

export async function startResearchJob(
  sampleId: number,
  ollamaUrl: string,
  scope?: ResearchScopeConfig,
  forceReenrich?: boolean
): Promise<void> {
  return invoke<void>("start_research_job", { sampleId, ollamaUrl, scope, forceReenrich });
}

export async function pauseResearchJob(sampleId: number): Promise<ResearchJob | null> {
  return invoke<ResearchJob | null>("pause_research_job", { sampleId });
}

export async function cancelResearchJob(sampleId: number): Promise<ResearchJob | null> {
  return invoke<ResearchJob | null>("cancel_research_job", { sampleId });
}

export async function resumeResearchJob(sampleId: number, ollamaUrl: string): Promise<void> {
  return invoke<void>("resume_research_job", { sampleId, ollamaUrl });
}

export async function searchQdrantEvidence(
  query: string,
  ollamaUrl: string,
  sampleId?: number,
  limit?: number,
  traitCategory?: string
): Promise<QdrantHit[]> {
  return invoke<QdrantHit[]>("search_qdrant_evidence", {
    query,
    ollamaUrl,
    sampleId,
    limit,
    traitCategory,
  });
}

export async function searchQdrantTraitDiscovery(
  query: string | undefined,
  traitCategory: string | undefined,
  sampleId: number,
  ollamaUrl: string,
  limit?: number
): Promise<QdrantHit[]> {
  return invoke<QdrantHit[]>("search_qdrant_trait_discovery", {
    query: query || null,
    traitCategory: traitCategory || null,
    sampleId,
    ollamaUrl,
    limit,
  });
}

export async function getVectorPromotedFindings(
  sampleId: number
): Promise<import("../types/genomics").VectorPromotedFinding[]> {
  return invoke("get_vector_promoted_findings", { sampleId });
}

export async function getVectorResearchDiagnostics(
  sampleId?: number
): Promise<VectorResearchDiagnostics> {
  return invoke<VectorResearchDiagnostics>("get_vector_research_diagnostics", {
    sampleId,
  });
}

// ── Evidence workbench ──────────────────────────────────────────────────────

export async function searchAssociationsHybrid(
  params: HybridSearchParams,
  ollamaUrl: string
): Promise<EvidenceCard[]> {
  return invoke<EvidenceCard[]>("search_associations_hybrid", { params, ollamaUrl });
}

export async function getSimilarAssociations(
  params: SimilarSearchParams
): Promise<EvidenceCard[]> {
  return invoke<EvidenceCard[]>("get_similar_associations", { params });
}

export async function getVariantEvidenceCard(
  sampleId: number,
  rsid: string
): Promise<EvidenceCard | null> {
  return invoke<EvidenceCard | null>("get_variant_evidence_card", { sampleId, rsid });
}

export async function explainVectorMatch(
  query: string,
  hitPayload: Record<string, unknown>,
  vectorScore: number
): Promise<MatchExplanation> {
  return invoke<MatchExplanation>("explain_vector_match_cmd", {
    query,
    hitPayload,
    vectorScore,
  });
}

export async function backfillEvidencePayloads(
  sampleId: number,
  limit?: number
): Promise<BackfillResult> {
  return invoke<BackfillResult>("backfill_evidence_payloads_cmd", { sampleId, limit });
}

export async function reembedStaleVectors(
  sampleId: number,
  ollamaUrl: string,
  limit?: number
): Promise<ReembedResult> {
  return invoke<ReembedResult>("reembed_stale_vectors_cmd", { sampleId, ollamaUrl, limit });
}

export async function getActionabilityMatrix(
  sampleId: number,
  limit?: number
): Promise<ActionabilityPoint[]> {
  return invoke<ActionabilityPoint[]>("get_actionability_matrix", { sampleId, limit });
}

export async function getChromosomeTraitOverlay(
  sampleId: number
): Promise<ChromosomeTraitBand[]> {
  return invoke<ChromosomeTraitBand[]>("get_chromosome_trait_overlay", { sampleId });
}

export async function getPathwayFlowRows(
  sampleId: number,
  limit?: number
): Promise<PathwayFlowRow[]> {
  return invoke<PathwayFlowRow[]>("get_pathway_flow_rows", { sampleId, limit });
}

export async function getQualityDashboard(sampleId: number): Promise<QualityDashboard> {
  return invoke<QualityDashboard>("get_quality_dashboard", { sampleId });
}

export async function buildTraitClusters(
  sampleId: number,
  traitCategory?: string,
  minDataQuality?: number,
  limit?: number
): Promise<TraitClusterSummary[]> {
  return invoke<TraitClusterSummary[]>("build_trait_clusters_cmd", {
    sampleId,
    traitCategory: traitCategory || null,
    minDataQuality,
    limit,
  });
}

export async function exportEvidencePacket(
  sampleId: number,
  rsid?: string,
  clusterId?: string,
  associationIds?: string[]
): Promise<EvidencePacket> {
  return invoke<EvidencePacket>("export_evidence_packet_cmd", {
    sampleId,
    rsid: rsid || null,
    clusterId: clusterId || null,
    associationIds: associationIds || null,
  });
}

export async function listCandidateMarkers(limit?: number): Promise<CandidateMarkerRow[]> {
  return invoke<CandidateMarkerRow[]>("list_candidate_markers", { limit });
}

export async function updateCandidateMarkerStatus(request: {
  candidate_id: string;
  status: string;
  reviewer_note?: string;
}): Promise<void> {
  return invoke<void>("update_candidate_marker_status", { request });
}

export async function ensureQdrantPayloadIndexes(): Promise<string[]> {
  return invoke<string[]>("ensure_qdrant_payload_indexes");
}

export async function buildVectorAtlas(
  sampleId: number,
  limit?: number
): Promise<VectorAtlasResult> {
  return invoke<VectorAtlasResult>("build_vector_atlas_cmd", { sampleId, limit });
}

export async function getVectorAtlasCached(
  sampleId: number,
  limit?: number
): Promise<AtlasPoint[]> {
  return invoke<AtlasPoint[]>("get_vector_atlas_cached", { sampleId, limit });
}

export async function enableNamedVectorsCollection(ollamaUrl: string): Promise<string> {
  return invoke<string>("enable_named_vectors_collection", { ollamaUrl });
}

export async function getOllamaToken(): Promise<string | null> {
  return invoke<string | null>("get_ollama_token");
}

export async function saveOllamaToken(token?: string): Promise<void> {
  return invoke<void>("save_ollama_token", { token: token || undefined });
}

export async function purgeDatabaseCache(): Promise<number> {
  return invoke<number>("purge_database_cache");
}

export async function selectSavePath(defaultFilename: string): Promise<string | null> {
  return invoke<string | null>("select_save_path", { defaultFilename });
}

export async function getResearchScope(): Promise<ResearchScopeConfig> {
  return invoke<ResearchScopeConfig>("get_research_scope");
}

export async function saveResearchScope(scope: ResearchScopeConfig): Promise<void> {
  return invoke<void>("save_research_scope", { scope });
}

export async function previewResearchScope(
  sampleId: number,
  scope: ResearchScopeConfig
): Promise<ResearchScopePreview> {
  return invoke<ResearchScopePreview>("preview_research_scope", { sampleId, scope });
}

export async function previewPipelineTuning(
  ollamaUrl: string,
  qdrantUrl: string,
  sweepFast?: boolean
): Promise<import("../types/research").PipelineTuningPublic> {
  return invoke("preview_pipeline_tuning", {
    ollamaUrl,
    qdrantUrl,
    sweepFast: sweepFast ?? null,
  });
}

export async function getResearchDebugLog(): Promise<boolean> {
  return invoke<boolean>("get_research_debug_log");
}

export async function setResearchDebugLog(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_research_debug_log", { enabled });
}

export async function getReferenceStatus(): Promise<ReferenceStatus> {
  return invoke<ReferenceStatus>("get_reference_status");
}

export async function syncGwasReference(): Promise<GwasSyncResult> {
  return invoke<GwasSyncResult>("sync_gwas_reference");
}

// ── Agent / Evidence ────────────────────────────────────────────────────────

import type { VariantEvidence, SafetyCheckResult } from "../types/agent";

export type { VariantEvidence, SafetyCheckResult };

export async function getVariantEvidence(
  sampleId: number,
  rsid: string,
  ncbiApiKey?: string,
  customExportPath?: string
): Promise<VariantEvidence> {
  return invoke<VariantEvidence>("get_variant_evidence", {
    sampleId,
    rsid,
    ncbiApiKey: ncbiApiKey || undefined,
    customExportPath: customExportPath || undefined,
  });
}

export async function getDbDiscoveredFindings(sampleId: number): Promise<VariantEvidence[]> {
  return invoke<VariantEvidence[]>("get_db_discovered_findings", { sampleId });
}

export async function runSafetyAudit(
  reportText: string,
  variantEvidenceList: VariantEvidence[]
): Promise<SafetyCheckResult> {
  return invoke<SafetyCheckResult>("run_safety_audit", { reportText, variantEvidenceList });
}

export async function exportDiscoveredFindings(
  sampleId: number,
  exportPath: string
): Promise<number> {
  return invoke<number>("export_discovered_findings", { sampleId, exportPath });
}

export async function getGnomadConfig(): Promise<import("../types/research").GnomadConfig> {
  return invoke("get_gnomad_config_cmd");
}

export async function saveGnomadConfig(
  config: import("../types/research").GnomadConfig
): Promise<void> {
  return invoke("save_gnomad_config_cmd", { config });
}

export async function testGnomadSourceUrls(): Promise<
  import("../types/research").GnomadSourceTestResult
> {
  return invoke("test_gnomad_source_urls_cmd");
}

export async function clearGnomadCache(): Promise<number> {
  return invoke("clear_gnomad_cache_cmd");
}

export async function getGnomadReadiness(): Promise<import("../types/research").GnomadReadinessStatus> {
  return invoke("get_gnomad_readiness_cmd");
}

export async function downloadGnomadIndexes(): Promise<import("../types/research").GnomadIndexSyncResult> {
  return invoke("download_gnomad_indexes_cmd");
}

export async function selectGnomadLocalDir(): Promise<string | null> {
  return invoke<string | null>("select_gnomad_local_dir_cmd");
}

export async function getGnomadContext(
  request: Record<string, unknown>
): Promise<import("../types/research").GnomadContext> {
  return invoke("get_gnomad_context_cmd", { request });
}
