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
  LibraryStatus,
  AppBootstrapStatus,
  DiscoveredFindingSummary,
  GenomeSample,
  DbSnpRecord,
  GeneratedReport,
  NormalizedReport,
  ReportPayload,
  GenomeImportPreview,
} from "../types/genomics";
import { denormalizeReport } from "../utils/viewModels";
import { normalizeReportStatuses } from "../utils/reportStatuses";

export async function selectFile(): Promise<string | null> {
  return invoke<string | null>("select_file");
}

export async function selectDirectory(): Promise<string | null> {
  return invoke<string | null>("select_directory");
}

export async function saveReportJson(content: string, defaultFilename: string): Promise<boolean> {
  return invoke<boolean>("save_report_json", { content, defaultFilename });
}

export async function saveReportBundle(
  files: Array<{ filename: string; content: string }>,
  defaultFilename: string,
): Promise<boolean> {
  return invoke<boolean>('save_report_bundle', { files, defaultFilename });
}

export async function getAppPaths(): Promise<AppPaths> {
  return invoke<AppPaths>("get_app_paths");
}

export async function getLibraryStatus(): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("get_library_status");
}

export async function openLibraryDir(): Promise<void> {
  return invoke<void>("open_library_dir");
}

export async function setLibraryDir(dest: string, mode: "move" | "use"): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("set_library_dir", { dest, mode });
}

export async function resetLibraryDir(): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("reset_library_dir");
}

export async function eraseLibraryData(): Promise<void> {
  return invoke<void>("erase_library_data");
}

export async function getAppBootstrap(): Promise<AppBootstrapStatus> {
  return invoke<AppBootstrapStatus>("get_app_bootstrap");
}

export async function getDiscoveredFindingsSummary(
  sampleId: number
): Promise<DiscoveredFindingSummary[]> {
  return invoke<DiscoveredFindingSummary[]>("get_discovered_findings_summary", { sampleId });
}

export async function importGenome(
  filePath: string,
  sampleName: string,
  replaceExistingSampleId?: number,
): Promise<number> {
  return invoke<number>("import_genome", { filePath, sampleName, replaceExistingSampleId });
}

export async function inspectGenome(filePath: string): Promise<GenomeImportPreview> {
  return invoke<GenomeImportPreview>("inspect_genome", { filePath });
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

export async function generateReport(sampleId: number, templateJson: string): Promise<ReportPayload> {
  const wireReport = await invoke<NormalizedReport>("generate_report", { sampleId, templateJson });
  const raw = normalizeReportStatuses(wireReport);
  const report = denormalizeReport(raw);
  return { report, raw };
}

export async function deleteSample(sampleId: number): Promise<void> {
  return invoke<void>("delete_sample", { sampleId });
}

export async function logJsError(
  message: string,
  source: string,
  line: number | null,
  col: number | null,
  stack: string | null
): Promise<void> {
  return invoke<void>("log_js_error", { message, source, line, col, stack });
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
  OfflineUpdateCheck,
  OfflineSyncResult,
  DatabaseCachePurgeResult,
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
  EvidenceCorpusSummary,
  BrowseAssociationsParams,
  BrowseAssociationsResult,
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
  OfflineUpdateCheck,
  OfflineSyncResult,
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

export async function getRecentFindingPreviews(
  sampleId: number,
  limit?: number
): Promise<import("../types/research").ResearchFindingPreview[]> {
  return invoke("get_recent_finding_previews", { sampleId, limit });
}

export interface VectorBrowsePage {
  provider: string;
  points: import("../types/research").QdrantHit[];
  next_offset?: unknown | null;
  total_hint?: number | null;
  note?: string | null;
}

export async function browseVectorStore(
  sampleId: number,
  ollamaUrl: string,
  limit?: number,
  offset?: unknown,
  rsid?: string
): Promise<VectorBrowsePage> {
  return invoke<VectorBrowsePage>("browse_vector_store", {
    sampleId,
    ollamaUrl,
    limit,
    offset: offset ?? null,
    rsid: rsid || null,
  });
}

export interface PackDraftExportResult {
  path: string;
  candidate_count: number;
  already_in_packs: number;
  from_promoted: number;
  from_vector_browse: number;
  message: string;
}

export async function exportPackDraftFromVectors(
  sampleId: number,
  sampleName: string,
  ollamaUrl: string,
  minScore?: number
): Promise<PackDraftExportResult> {
  return invoke<PackDraftExportResult>("export_pack_draft_from_vectors", {
    sampleId,
    sampleName,
    ollamaUrl,
    minScore: minScore ?? null,
  });
}

export async function listPackDraftExports(): Promise<string[]> {
  return invoke<string[]>("list_pack_draft_exports");
}

export async function listRuntimeMarkerPackIds(): Promise<string[]> {
  return invoke<string[]>("list_runtime_marker_pack_ids");
}

export interface PackMergeResult {
  target_pack_id: string;
  target_pack_path: string;
  added: number;
  skipped_existing: number;
  skipped_incomplete: number;
  backup_path?: string | null;
  message: string;
}

export async function mergePackDraftIntoPack(
  draftPath: string,
  targetPackId: string,
  onlyRsids?: string[],
  requireComplete?: boolean
): Promise<PackMergeResult> {
  return invoke<PackMergeResult>("merge_pack_draft_into_pack", {
    draftPath,
    targetPackId,
    onlyRsids: onlyRsids ?? null,
    requireComplete: requireComplete ?? false,
  });
}

export async function reloadMarkerPacks(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("reload_marker_packs");
}

export interface ResearchFoundPackView {
  name: string;
  markers: Array<Record<string, unknown>>;
  enabled_in_report: boolean;
  path: string;
}

export interface AlleleFillResult {
  updated: number;
  unchanged: number;
  message: string;
}

export interface ResearchFoundMarkerPatch {
  rsid: string;
  gene?: string | null;
  effect_allele?: string | null;
  effect_direction?: string | null;
  evidence_tier?: string | null;
  impact?: string | null;
  interpretation?: string | null;
  variant_name?: string | null;
  clinical_confirmation_required?: boolean | null;
}

export async function getResearchFoundPack(): Promise<ResearchFoundPackView> {
  return invoke<ResearchFoundPackView>("get_research_found_pack");
}

export async function fillResearchFoundAlleles(): Promise<AlleleFillResult> {
  return invoke<AlleleFillResult>("fill_research_found_alleles");
}

export async function setResearchFoundEnabled(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_research_found_enabled", { enabled });
}

export async function updateResearchFoundMarker(
  patch: ResearchFoundMarkerPatch
): Promise<ResearchFoundPackView> {
  return invoke<ResearchFoundPackView>("update_research_found_marker", { patch });
}

export async function deleteResearchFoundMarkers(
  rsids: string[]
): Promise<ResearchFoundPackView> {
  return invoke<ResearchFoundPackView>("delete_research_found_markers", { rsids });
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

export async function getEvidenceCorpusSummary(
  sampleId: number,
  enrichmentEnriched?: number,
  enrichmentTotal?: number,
): Promise<EvidenceCorpusSummary> {
  return invoke<EvidenceCorpusSummary>("get_evidence_corpus_summary", {
    sampleId,
    enrichmentEnriched: enrichmentEnriched ?? null,
    enrichmentTotal: enrichmentTotal ?? null,
  });
}

export async function browseAssociations(
  params: BrowseAssociationsParams,
): Promise<BrowseAssociationsResult> {
  return invoke<BrowseAssociationsResult>("browse_associations_cmd", { params });
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

export interface OllamaServiceConfig {
  url: string;
  from_env: boolean;
  token_set: boolean;
  configured: boolean;
  env_url?: string | null;
}

export async function getOllamaServiceConfig(): Promise<OllamaServiceConfig> {
  return invoke<OllamaServiceConfig>("get_ollama_service_config");
}

export async function saveOllamaToken(token?: string): Promise<void> {
  return invoke<void>("save_ollama_token", { token: token || undefined });
}

export async function saveOllamaUrl(url: string): Promise<void> {
  return invoke<void>("save_ollama_url", { url });
}

export interface InferenceHostProfile {
  platform: string;
  arch: string;
  is_local_url: boolean;
  unified_memory: boolean;
  accel_backends: string[];
  accel_bytes?: number | null;
  system_ram_bytes?: number | null;
  observed_vram_in_use_bytes?: number | null;
  posture: string;
  notes: string[];
}

export interface OllamaModelInsight {
  name: string;
  role: string;
  size_bytes?: number | null;
  parameter_size?: string | null;
  quantization?: string | null;
  family?: string | null;
  currently_loaded: boolean;
  size_vram_bytes?: number | null;
  load_hint: string;
  rationale: string;
}

export interface OllamaDiscoveryReport {
  host: InferenceHostProfile;
  models: OllamaModelInsight[];
  ollama_reachable: boolean;
  error?: string | null;
}

export async function probeInferenceHost(ollamaUrl?: string): Promise<InferenceHostProfile> {
  return invoke<InferenceHostProfile>("probe_inference_host", {
    ollamaUrl: ollamaUrl || undefined,
  });
}

export async function discoverOllamaModels(
  url: string,
  token?: string
): Promise<OllamaDiscoveryReport> {
  return invoke<OllamaDiscoveryReport>("discover_ollama_models", {
    url,
    token: token || undefined,
  });
}

export interface LocalhostServiceStatus {
  ollama_url: string;
  qdrant_url: string;
  ollama_reachable: boolean;
  qdrant_reachable: boolean;
  ollama_version?: string | null;
  qdrant_version?: string | null;
  ollama_error?: string | null;
  qdrant_error?: string | null;
  notes: string[];
}

export interface ServiceUpdateCheck {
  service: string;
  installed_version?: string | null;
  latest_version?: string | null;
  update_available?: boolean | null;
  notes: string[];
}

export async function probeLocalhostServices(): Promise<LocalhostServiceStatus> {
  return invoke<LocalhostServiceStatus>("probe_localhost_services");
}

export async function getOllamaVersion(url: string, token?: string): Promise<{ version?: string }> {
  return invoke("get_ollama_version", { url, token: token || undefined });
}

export async function getQdrantVersion(url: string, apiKey?: string): Promise<{ version?: string; title?: string }> {
  return invoke("get_qdrant_version", { url, apiKey: apiKey || undefined });
}

export async function checkOllamaUpdate(url: string, token?: string): Promise<ServiceUpdateCheck> {
  return invoke<ServiceUpdateCheck>("check_ollama_update", { url, token: token || undefined });
}

export async function checkQdrantUpdate(url: string, apiKey?: string): Promise<ServiceUpdateCheck> {
  return invoke<ServiceUpdateCheck>("check_qdrant_update", { url, apiKey: apiKey || undefined });
}

export async function pullOllamaModel(url: string, name: string, token?: string): Promise<unknown> {
  return invoke("pull_ollama_model", { url, name, token: token || undefined });
}

export async function deleteOllamaModel(url: string, name: string, token?: string): Promise<void> {
  return invoke("delete_ollama_model", { url, name, token: token || undefined });
}

export async function probeVectorProvider(
  provider: string,
  url: string,
  apiKey?: string
): Promise<{
  provider: string;
  reachable: boolean;
  research_supported: boolean;
  collection_exists?: boolean;
  vectors_count?: number;
  collections?: string[];
  capabilities?: Record<string, unknown>;
  note?: string;
  error?: string;
  http_status?: number;
  info?: unknown;
}> {
  return invoke("probe_vector_provider", {
    provider,
    url,
    apiKey: apiKey || undefined,
  });
}

export async function purgeDatabaseCache(): Promise<DatabaseCachePurgeResult> {
  return invoke<DatabaseCachePurgeResult>("purge_database_cache");
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

export async function checkOfflineDataUpdates(): Promise<OfflineUpdateCheck> {
  return invoke<OfflineUpdateCheck>("check_offline_data_updates");
}

export interface ReferenceStatusDetails {
  clinvar_raw_found: boolean;
  clinvar_indexed_rows: number;
  clinvar_rsid_hits: number;
  clinvar_last_indexed: number | null;
  dbsnp_merged_raw_found: boolean;
  dbsnp_merge_mappings_indexed: number;
  dbsnp_rsids_normalized: number;
  dbsnp_merge_index_available: boolean;
  dbsnp_placement_index_available: boolean;
  orientation_verification_available: boolean;
}

export async function getOfflineReferenceStatus(
  reportRsids?: string[] | null
): Promise<ReferenceStatusDetails> {
  return invoke<ReferenceStatusDetails>("get_offline_reference_status", {
    reportRsids: reportRsids ?? null,
  });
}

export async function syncOfflineDataTier(
  tier: number,
  force = false,
  sampleId?: number
): Promise<OfflineSyncResult> {
  return invoke<OfflineSyncResult>('sync_offline_data_tier', {
    tier,
    force,
    sampleId: sampleId ?? null,
  });
}

/** Sync a single asset by its asset_id. force=true re-downloads even if the file exists. */
export async function syncSingleOfflineAsset(
  assetId: string,
  force = false,
  sampleId?: number
): Promise<OfflineSyncResult> {
  return invoke<OfflineSyncResult>('sync_single_offline_asset', {
    assetId,
    force,
    sampleId: sampleId ?? null,
  });
}

/** Download all missing/outdated assets across all tiers (no force re-download). */
export async function syncAllOfflineMissing(sampleId?: number): Promise<OfflineSyncResult[]> {
  return invoke<OfflineSyncResult[]>('sync_all_offline_missing', {
    sampleId: sampleId ?? null,
  });
}

export async function syncAllOfflineData(
  force = false,
  sampleId?: number
): Promise<OfflineSyncResult[]> {
  return invoke<OfflineSyncResult[]>('sync_all_offline_data', {
    force,
    sampleId: sampleId ?? null,
  });
}

export async function buildOfflineTier2(sampleId: number): Promise<OfflineSyncResult> {
  return invoke<OfflineSyncResult>('build_offline_tier2', { sampleId });
}

/** Cancel an in-progress catalog import (cooperative). */
export async function cancelOfflineImport(): Promise<void> {
  return invoke<void>('cancel_offline_import');
}

/** Export pack coverage + genome×catalog findings JSON under App/Data/exports/. */
export async function exportDiscoveryFindings(sampleId: number): Promise<{
  pack_coverage_path: string;
  full_findings_path: string;
  findings_in_packs: number;
  findings_beyond_packs: number;
  genotype_rsid_count: number;
  pack_rsid_count: number;
}> {
  return invoke('export_discovery_findings', { sampleId });
}

export interface DiscoveryFindingItem {
  rsid: string;
  genotype?: string;
  in_marker_packs?: boolean;
  clinvar?: {
    clinical_significance?: string;
    gene?: string;
    phenotypes?: string;
    review_status?: string;
    variation_id?: string;
  } | null;
  clinvar_annotations?: Array<{
    clinical_significance?: string;
    gene?: string;
    phenotypes?: string;
    review_status?: string;
    variation_id?: string;
  }>;
  gwas?: {
    top_trait?: string;
    best_pvalue?: number | null;
    primary_gene?: string;
  } | null;
  pharmgkb?: {
    gene?: string;
    drug?: string;
    phenotype?: string;
    evidence_level?: string;
  } | null;
  pharmgkb_annotations?: Array<{
    gene?: string;
    drug?: string;
    phenotype?: string;
    evidence_level?: string;
  }>;
}

export interface DiscoveryQueryResult {
  sample_id: number;
  genotype_rsid_count: number;
  pack_rsid_count: number;
  findings_in_packs: number;
  total_matched: number;
  beyond_packs_only: boolean;
  source_filter: string | null;
  query: string | null;
  limit: number;
  offset: number;
  cached?: boolean;
  items: DiscoveryFindingItem[];
}

/** Browse ranked genome×catalog associations (default: beyond marker packs). */
export async function queryDiscoveryFindings(
  sampleId: number,
  opts?: {
    beyondPacksOnly?: boolean;
    sourceFilter?: string | null;
    query?: string | null;
    limit?: number;
    offset?: number;
  }
): Promise<DiscoveryQueryResult> {
  return invoke('query_discovery_findings', {
    sampleId,
    beyondPacksOnly: opts?.beyondPacksOnly ?? true,
    sourceFilter: opts?.sourceFilter ?? null,
    query: opts?.query ?? null,
    limit: opts?.limit ?? 100,
    offset: opts?.offset ?? 0,
  });
}

export async function cancelDiscoveryQuery(): Promise<void> {
  return invoke('cancel_discovery_query');
}

export async function getCustomDownloadDir(): Promise<string | null> {
  return invoke<string | null>('get_custom_download_dir');
}

export async function setCustomDownloadDir(path: string | null): Promise<void> {
  return invoke<void>('set_custom_download_dir', { path });
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

export async function refreshGnomadFrequencyCache(
  sampleId: number,
  rsids: string[],
): Promise<import("../types/research").GnomadBatchProgress> {
  return invoke("refresh_gnomad_frequency_cache_cmd", { sampleId, rsids });
}

export async function selectGnomadLocalDir(): Promise<string | null> {
  return invoke<string | null>("select_gnomad_local_dir_cmd");
}

export async function getGnomadContext(
  request: Record<string, unknown>
): Promise<import("../types/research").GnomadContext> {
  return invoke("get_gnomad_context_cmd", { request });
}

export async function getAllMarkerPacks(): Promise<any> {
  return invoke("get_all_marker_packs");
}

export async function queryLocalReferenceDb(
  table: string,
  searchQuery?: string,
  limit?: number,
  offset?: number
): Promise<{ rows: any[]; total: number }> {
  return invoke<{ rows: any[]; total: number }>("query_local_reference_db", {
    table,
    searchQuery: searchQuery || null,
    limit: limit ?? null,
    offset: offset ?? null,
  });
}
