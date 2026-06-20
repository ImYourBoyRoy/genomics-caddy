// ./src/lib/types/research.ts
/*
  Module: research.ts
  Purpose: TypeScript types for the Qdrant research loop and autonomous marker enrichment.
  Responsibilities:
    - Define QdrantConfig for vector DB connection settings.
    - Define ResearchJob / ResearchProgress for async job tracking.
    - Define QdrantHit for semantic search results.
    - Define QdrantConnectionStatus for health checks.
    - Export utility constants and helper for detecting embed-capable models.
  Key Inputs: None (type declarations only).
  Key Outputs: Interfaces, enums, constants, and utility function exports.
  Operational Notes: Types must stay in sync with Rust backend structs in `src-tauri/src/research/types.rs`.
*/

export interface QdrantConfigPublic {
  url: string;
  collection: string;
  embedding_model: string;
  gwas_strict: boolean;
  auto_start: boolean;
  api_key_set: boolean;
  ncbi_api_key_set: boolean;
  research_scope: ResearchScopeConfig;
  named_vectors_enabled: boolean;
}

/** Partial save payload — omit secret fields to keep existing keyring values. */
export interface QdrantConfigUpdate {
  url: string;
  collection: string;
  embedding_model: string;
  gwas_strict: boolean;
  auto_start: boolean;
  api_key?: string;
  ncbi_api_key?: string;
  research_scope?: ResearchScopeConfig;
  named_vectors_enabled?: boolean;
}

export interface EnrichmentSourcesConfig {
  /** gnomAD population frequency (often slow on remote tabix). */
  gnomad?: boolean;
  /** Live ClinVar via NCBI E-utilities. */
  clinvar_live?: boolean;
  /** PubMed abstracts for trait context. */
  pubmed?: boolean;
  /** GTEx eQTL tissue associations. */
  gtex?: boolean;
  /** Ensembl VEP + dbSNP for gene symbols and coordinates. */
  vep_dbsnp?: boolean;
  /** PGS Catalog, Reactome, Open Targets, PharmGKB, OLS4. */
  secondary?: boolean;
  /** Re-run enabled sources on variants already indexed when data was skipped. */
  supplement_missing?: boolean;
}

export interface ResearchScopeConfig {
  curated: boolean;
  agent_discoveries: boolean;
  gwas_discovery: boolean;
  non_reference: boolean;
  non_reference_limit: number;
  gwas_discovery_limit: number;
  /** Legacy fast flag — synced when all live sources are off. */
  sweep_fast?: boolean;
  enrichment_sources?: EnrichmentSourcesConfig;
}

export interface PipelineTuningPublic {
  profile: string;
  prepare_concurrency: number;
  enrich_batch_size: number;
  gwas_api_concurrency: number;
  gnomad_concurrency: number;
  embed_parallel: number;
  pipeline_depth: number;
  prefetch_concurrency: number;
  fast_sweep: boolean;
  skip_gnomad: boolean;
  local_gwas_only: boolean;
  named_vectors_in_sweep: boolean;
}

export interface ResearchScopePreview {
  curated: number;
  agent_discoveries: number;
  gwas_discovery: number;
  non_reference: number;
  total_unique: number;
  genotype_total: number;
  gwas_reference_count: number;
  gwas_genome_overlap: number;
  /** GWAS overlap rsIDs excluded by gwas_discovery_limit cap */
  gwas_beyond_cap: number;
}

export interface ReferenceStatus {
  data_dir: string;
  gwas_rsid_count: number;
  gwas_file_present: boolean;
  gwas_gz_present: boolean;
}

export interface GwasSyncResult {
  rsid_count: number;
  downloaded: boolean;
  file_path: string;
  message: string;
}

export type ConnectionActivityPhase =
  | "idle"
  | "checking-qdrant"
  | "checking-ollama"
  | "loading-scope"
  | "ready"
  | "error";

export interface ConnectionActivity {
  phase: ConnectionActivityPhase;
  message: string;
}

export interface PurgeCollectionResult {
  job_reset: boolean;
  message: string;
}

export const DEFAULT_CONNECTION_ACTIVITY: ConnectionActivity = {
  phase: "idle",
  message: "Ready",
};

export const DEFAULT_ENRICHMENT_SOURCES: EnrichmentSourcesConfig = {
  gnomad: false,
  clinvar_live: false,
  pubmed: false,
  gtex: false,
  vep_dbsnp: false,
  secondary: false,
  supplement_missing: false,
};

export const FULL_ENRICHMENT_SOURCES: EnrichmentSourcesConfig = {
  gnomad: true,
  clinvar_live: true,
  pubmed: true,
  gtex: true,
  vep_dbsnp: true,
  secondary: true,
  supplement_missing: false,
};

export const DEFAULT_RESEARCH_SCOPE: ResearchScopeConfig = {
  curated: true,
  agent_discoveries: true,
  gwas_discovery: true,
  non_reference: true,
  non_reference_limit: 5000,
  gwas_discovery_limit: 10000,
  sweep_fast: true,
  enrichment_sources: { ...DEFAULT_ENRICHMENT_SOURCES },
};

/** @deprecated Use QdrantConfigPublic for reads and QdrantConfigUpdate for saves. */
export interface QdrantConfig {
  url: string;
  api_key?: string;
  collection: string;
  embedding_model: string;
  gwas_strict: boolean;
  ncbi_api_key?: string;
  auto_start: boolean;
  api_key_set?: boolean;
  ncbi_api_key_set?: boolean;
}

export interface ResearchJob {
  job_id: string;
  sample_id: number;
  status: 'running' | 'paused' | 'complete' | 'error' | 'idle';
  total_markers: number;
  enriched_count: number;
  priority_complete: boolean;
  current_rsid?: string;
  current_source?: string;
  started_at: number;    // unix timestamp
  last_updated: number;
  error_message?: string;
  /** True when the Rust sweep task is active in this app session. */
  loop_active?: boolean;
}

export interface PhaseMetricSnapshot {
  count: number;
  total_ms: number;
  avg_ms: number;
  cache_hits?: number;
  cache_lookups?: number;
  cache_hit_rate?: number | null;
}

export interface SweepPhaseMetricsSnapshot {
  gwas: PhaseMetricSnapshot;
  gnomad: PhaseMetricSnapshot;
  clinvar: PhaseMetricSnapshot;
  pubmed: PhaseMetricSnapshot;
  gtex: PhaseMetricSnapshot;
  vep: PhaseMetricSnapshot;
  embed: PhaseMetricSnapshot;
  qdrant: PhaseMetricSnapshot;
  gnomad_cache_hits: number;
  gnomad_cache_lookups: number;
  gnomad_cache_hit_rate?: number | null;
}

export interface ResearchProgress {
  job_id: string;
  status: string;
  enriched_count: number;
  total_markers: number;
  current_rsid?: string;
  current_source?: string;
  message: string;
  variants_per_sec?: number | null;
  recent_variants_per_sec?: number | null;
  phase_metrics?: SweepPhaseMetricsSnapshot | null;
  activity_phase?: string | null;
  batch_prepared?: number | null;
  batch_prefetch_done?: number | null;
  batch_total?: number | null;
  batch_elapsed_secs?: number | null;
}

export interface QdrantHit {
  rsid: string;
  gene?: string;
  category?: string;
  text: string;
  source: string;
  score: number;
  pmid?: string;
  gwas_trait?: string;
  gnomad_af?: number;
  gnomad_lookup_status?: string;
  gnomad_source_mode?: string;
  gnomad_release?: string;
  has_gnomad?: boolean;
  significance_score: number;
  genotype?: string;
  gene_confidence?: string;
  enrichment_version?: string;
  clinvar_significance?: string;
  chromosome?: string;
  position?: number;
  gwas_associations?: Record<string, unknown>[];
  gene_candidates?: Record<string, unknown>[];
  sources_provenance?: Record<string, unknown>;
  cross_refs?: Record<string, unknown>;
  trait_categories?: string[];
  consultation_modes?: string[];
  pack_refs?: Record<string, unknown>[];
  searchable_tags?: string[];
  association_summary?: Record<string, unknown>;
}

export interface VectorResearchDiagnostics {
  connected: boolean;
  collection: string;
  collection_exists: boolean;
  qdrant_url: string;
  embedding_model: string;
  total_vectors?: number;
  sample_vectors?: number;
  enrichment_enriched?: number;
  enrichment_total?: number;
  enrichment_status?: string;
  error?: string;
}

export interface QdrantConnectionStatus {
  success: boolean;
  vectors_count?: number;
  collection_exists: boolean;
  collections?: string[];
  error?: string;
}

export const DEFAULT_QDRANT_CONFIG: QdrantConfigPublic = {
  url: 'http://localhost:6333',
  collection: 'genomics_evidence',
  embedding_model: 'mxbai-embed-large',
  gwas_strict: true,
  auto_start: false,
  api_key_set: false,
  ncbi_api_key_set: false,
  research_scope: { ...DEFAULT_RESEARCH_SCOPE },
  named_vectors_enabled: false,
};

// Known embed-capable model name patterns (for filtering Ollama model list)
export const EMBED_MODEL_PATTERNS = ['embed', 'nomic', 'bge', 'e5-', 'gte-', 'mxbai', 'qwen3-embedding'];

export function isEmbedModel(name?: string): boolean {
  if (!name || typeof name !== 'string') return false;
  const lower = name.toLowerCase();
  return EMBED_MODEL_PATTERNS.some(p => lower.includes(p));
}

// ── Evidence workbench (Qdrant + SQLite normalized facts) ───────────────────

export interface EvidenceLedgerRow {
  source_name: string;
  trait_name?: string;
  mapped_gene?: string;
  p_value?: number;
  beta?: number;
  odds_ratio?: number;
  effect_allele?: string;
  personal_dosage?: number;
  personal_direction: string;
  study_accession?: string;
  pubmed_id?: string;
  ancestry?: string;
  sample_size?: number;
  quality_flags: string[];
  evidence_tier: string;
  association_type: string;
}

export interface MatchExplanation {
  vector_score: number;
  hybrid_score: number;
  shared_traits: string[];
  shared_genes: string[];
  shared_categories: string[];
  shared_sources: string[];
  matched_tags: string[];
  boost_reasons: string[];
  missing_metadata_warnings: string[];
  similarity_mode?: string;
}

export interface EvidenceCard {
  rsid: string;
  genotype?: string;
  chromosome?: string;
  position_grch38?: number;
  gene_symbol?: string;
  gene_confidence?: string;
  primary_trait?: string;
  trait_category?: string;
  trait_categories: string[];
  evidence_tier?: string;
  association_type?: string;
  personal_direction: string;
  directionality_confidence: number;
  directionality_label: string;
  has_direction: boolean;
  p_value_min?: number;
  gwas_best_pvalue?: number;
  association_strength_score: number;
  personal_match_score: number;
  clinical_actionability_score: number;
  wellness_actionability_score: number;
  data_quality_score: number;
  novelty_score: number;
  source_names: string[];
  primary_source?: string;
  source_count: number;
  conflict_count: number;
  missing_field_count: number;
  quality_flags: string[];
  missing_fields: string[];
  has_gwas: boolean;
  has_clinvar: boolean;
  has_pgs?: boolean;
  has_pharmgkb?: boolean;
  has_reactome?: boolean;
  clinvar_significance?: string;
  gnomad_af?: number;
  enrichment_version?: string;
  schema_version?: string;
  stale: boolean;
  semantic_score: number;
  synthesis: string;
  association_summary?: Record<string, unknown>;
  gene_candidates?: Record<string, unknown>[];
  sources_provenance?: Record<string, unknown>;
  cross_refs?: Record<string, unknown>;
  evidence_ledger: EvidenceLedgerRow[];
  match_explanation?: MatchExplanation;
  verification_ideas: string[];
  prohibited_claims: string[];
  qdrant_point_id?: string;
}

export interface HybridSearchParams {
  sample_id: number;
  query: string;
  trait_category?: string;
  evidence_tier?: string;
  has_direction?: boolean;
  min_data_quality?: number;
  min_wellness_actionability?: number;
  limit: number;
}

export interface SimilarSearchParams {
  sample_id: number;
  rsid?: string;
  qdrant_point_id?: number;
  similarity_mode: string;
  limit: number;
  include_self?: boolean;
}

export interface QualityDashboard {
  sample_id: number;
  total_genotypes: number;
  vectorized_variants?: number;
  association_fact_count: number;
  variants_with_gwas: number;
  variants_with_clinvar: number;
  known_direction_count: number;
  unknown_direction_count: number;
  missing_gene_count: number;
  missing_effect_allele_count: number;
  missing_study_accession_count: number;
  stale_vector_count: number;
  source_conflict_count: number;
  schema_mismatch_count: number;
  schema_v1_count: number;
  enrichment_v41_count: number;
  candidate_unreviewed_count: number;
  avg_data_quality_score: number;
  source_record_count: number;
  cache_stale_count: number;
  variants_with_pgs: number;
  variants_with_gtex: number;
  variants_with_pubmed: number;
  source_coverage: SourceCoverageSummary;
  collection: string;
  enrichment_status?: string;
}

export interface SourceCoverageSummary {
  gwas: number;
  clinvar: number;
  gtex: number;
  pubmed: number;
  pgs: number;
  pharmgkb: number;
  reactome: number;
}

export interface BackfillResult {
  scanned: number;
  updated_payload: number;
  skipped_current: number;
  needs_reembed: number;
  errors: number;
}

export interface ReembedResult {
  scanned: number;
  reembedded: number;
  skipped_fresh: number;
  errors: number;
}

export interface ActionabilityPoint {
  rsid: string;
  gene_symbol?: string;
  trait_category?: string;
  clinical_actionability_score: number;
  wellness_actionability_score: number;
  data_quality_score: number;
}

export interface ChromosomeTraitBand {
  chromosome: string;
  trait_category: string;
  association_count: number;
}

export interface PathwayFlowRow {
  gene_symbol: string;
  pathway_name: string;
  trait_name?: string;
  rsid?: string;
}

export interface AtlasPoint {
  rsid: string;
  qdrant_point_id?: string;
  x: number;
  y: number;
  trait_category?: string;
  gene_symbol?: string;
  data_quality_score: number;
}

export interface VectorAtlasResult {
  sample_id: number;
  point_count: number;
  projection_method: string;
  points: AtlasPoint[];
  warning: string;
}

export interface TraitClusterSummary {
  cluster_id: string;
  cluster_title: string;
  trait_category?: string;
  top_traits: string[];
  top_genes: string[];
  top_rsids: string[];
  association_count: number;
  source_count: number;
  known_direction_count: number;
  unknown_direction_count: number;
  data_quality_score: number;
  wellness_actionability_score: number;
  clinical_actionability_score: number;
  missing_field_count: number;
  verification_ideas: string[];
}

export interface CandidateMarkerRow {
  candidate_id: string;
  rsid: string;
  gene_symbol?: string;
  trait_name?: string;
  trait_category?: string;
  status: string;
  data_quality_score: number;
  association_strength_score: number;
  reason_surfaced?: string;
  best_evidence_tier?: string;
  source_count?: number;
  known_direction_count?: number;
  best_p_value?: number;
  has_gwas?: boolean;
  has_clinvar?: boolean;
  has_gtex?: boolean;
  has_pubmed?: boolean;
  has_pgs?: boolean;
  has_pharmgkb?: boolean;
}

export interface EvidencePacket {
  packet_id: string;
  generated_at: number;
  sample_id: number;
  rsid?: string;
  cluster_id?: string;
  genotype?: string;
  association_facts: Record<string, unknown>[];
  evidence_cards: EvidenceCard[];
  similar_associations: EvidenceCard[];
  quality_flags: string[];
  missing_fields: string[];
  conflicts: string[];
  scores: Record<string, unknown>;
  verification_ideas: string[];
  prohibited_claims: string[];
  research_questions: string[];
  source_records?: Record<string, unknown>[];
}

export const SIMILARITY_MODES = [
  { id: 'trait_similarity', label: 'Similar traits' },
  { id: 'gene_mechanism_similarity', label: 'Similar genes' },
  { id: 'evidence_similarity', label: 'Similar evidence' },
  { id: 'higher_quality_similar', label: 'Higher quality' },
  { id: 'clinically_stronger', label: 'Clinically stronger' },
  { id: 'wellness_actionable', label: 'Wellness actionable' },
  { id: 'same_trait_different_gene', label: 'Same trait, different gene' },
  { id: 'same_gene_different_trait', label: 'Same gene, different trait' },
] as const;

export type GnomadSourceMode =
  | "remote_indexed_vcf_https"
  | "local_indexed_vcf"
  | "graphql_interactive";

export type GnomadHttpsProvider = "aws" | "google";

export type GnomadDatasetPolicy = "auto" | "combined" | "exomes_only" | "genomes_only";

export interface GnomadConfig {
  enabled: boolean;
  source_mode: GnomadSourceMode;
  release: string;
  provider: GnomadHttpsProvider;
  local_vcf_dir?: string | null;
  max_remote_concurrent_files: number;
  max_queries_per_second: number;
  graphql_enabled_for_sweep: boolean;
  graphql_fallback_enabled: boolean;
  dataset_policy: GnomadDatasetPolicy;
  exome_template: string;
  genome_template: string;
}

export interface GnomadContext {
  lookup_status: string;
  release: string;
  dataset: string;
  variant_id?: string | null;
  rsids: string[];
  ac?: number | null;
  an?: number | null;
  af?: number | null;
  ac_exomes?: number | null;
  an_exomes?: number | null;
  af_exomes?: number | null;
  ac_genomes?: number | null;
  an_genomes?: number | null;
  af_genomes?: number | null;
  popmax?: number | null;
  popmax_population?: string | null;
  faf95_popmax?: number | null;
  faf95_popmax_population?: string | null;
  homozygote_count?: number | null;
  hemizygote_count?: number | null;
  filters: string[];
  flags: string[];
  population_frequencies: Record<string, unknown>;
  source_url?: string | null;
  source_mode: string;
  fetched_at?: number | null;
  warnings: string[];
  user_allele_match_status?: string | null;
}

export interface GnomadSourceTestResult {
  release: string;
  provider: string;
  exome_vcf_url: string;
  exome_index_url: string;
  genome_vcf_url: string;
  genome_index_url: string;
  exome_vcf_ok: boolean;
  exome_index_ok: boolean;
  genome_vcf_ok: boolean;
  genome_index_ok: boolean;
  smoke_test_ok: boolean;
  smoke_test_message?: string | null;
  errors: string[];
}

export interface GnomadMissingItem {
  id: string;
  kind: string;
  chrom?: string | null;
  dataset?: string | null;
  label: string;
  url?: string | null;
  fix_action: string;
}

export interface GnomadReadinessStatus {
  ready: boolean;
  enabled: boolean;
  source_mode: string;
  release: string;
  provider: string;
  remote_urls_ok: boolean;
  indexes_expected: number;
  indexes_cached: number;
  cache_dir: string;
  local_dir?: string | null;
  missing_items: GnomadMissingItem[];
  summary: string;
  primary_action?: string | null;
  downloads_page_url: string;
  manifest_exome_contigs: string[];
  manifest_genome_contigs: string[];
  unsupported_contigs: string[];
}

export interface GnomadIndexSyncResult {
  downloaded: number;
  skipped: number;
  failed: number;
  message: string;
  errors: string[];
}

export const DEFAULT_GNOMAD_CONFIG: GnomadConfig = {
  enabled: true,
  source_mode: "remote_indexed_vcf_https",
  release: "4.1",
  provider: "aws",
  local_vcf_dir: null,
  max_remote_concurrent_files: 2,
  max_queries_per_second: 4,
  graphql_enabled_for_sweep: false,
  graphql_fallback_enabled: true,
  dataset_policy: "auto",
  exome_template: "release/4.1/vcf/exomes/gnomad.exomes.v4.1.sites.chr{chrom}.vcf.bgz",
  genome_template: "release/4.1/vcf/genomes/gnomad.genomes.v4.1.sites.chr{chrom}.vcf.bgz",
};
