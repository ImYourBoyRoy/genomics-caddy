// ./src-tauri/src/research/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QdrantConfig {
    pub url: String,               // e.g. "http://192.168.1.5:6333"
    pub api_key: Option<String>,
    pub collection: String,        // default: "genomics_evidence"
    pub embedding_model: String,   // default: "mxbai-embed-large"
    pub gwas_strict: bool,         // true = p<5e-8, false = p<1e-5
    pub ncbi_api_key: Option<String>,
    pub auto_start: bool,
    #[serde(default)]
    pub named_vectors_enabled: bool,
}

/// Settings returned to the UI — never includes raw secret values.
#[derive(Debug, Serialize, Clone)]
pub struct QdrantConfigPublic {
    pub url: String,
    pub collection: String,
    pub embedding_model: String,
    pub gwas_strict: bool,
    pub auto_start: bool,
    pub api_key_set: bool,
    pub ncbi_api_key_set: bool,
    pub research_scope: ResearchScopeConfig,
    pub named_vectors_enabled: bool,
}

/// Partial update from the UI — omit secret fields to leave keyring values unchanged.
#[derive(Debug, Deserialize, Clone)]
pub struct QdrantConfigUpdate {
    pub url: String,
    pub collection: String,
    pub embedding_model: String,
    pub gwas_strict: bool,
    pub auto_start: bool,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub ncbi_api_key: Option<String>,
    #[serde(default)]
    pub research_scope: Option<ResearchScopeConfig>,
    #[serde(default)]
    pub named_vectors_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResearchJob {
    pub job_id: String,
    pub sample_id: i64,
    pub status: String,            // "running" | "paused" | "complete" | "error" | "idle"
    pub total_markers: i64,
    pub enriched_count: i64,
    pub priority_complete: bool,
    pub current_rsid: Option<String>,
    pub current_source: Option<String>,
    pub started_at: i64,
    pub last_updated: i64,
    pub error_message: Option<String>,
    #[serde(default)]
    pub scope_json: Option<String>,
    /// In-memory sweep loop active (may differ briefly from DB status during pause/resume).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResearchProgress {
    pub job_id: String,
    pub status: String,
    pub enriched_count: i64,
    pub total_markers: i64,
    pub current_rsid: Option<String>,
    pub current_source: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants_per_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_variants_per_sec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_metrics: Option<super::sweep_metrics::SweepPhaseMetricsSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_prepared: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_prefetch_done: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_total: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_elapsed_secs: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QdrantHit {
    pub rsid: String,
    pub gene: Option<String>,
    pub category: Option<String>,
    pub text: String,
    pub source: String,
    pub score: f32,
    pub pmid: Option<String>,
    pub gwas_trait: Option<String>,
    pub gnomad_af: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gnomad_lookup_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gnomad_source_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gnomad_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_gnomad: Option<bool>,
    pub significance_score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genotype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gene_confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrichment_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clinvar_significance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chromosome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gwas_associations: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gene_candidates: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources_provenance: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_refs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_categories: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consultation_modes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_refs: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub searchable_tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association_summary: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VectorResearchDiagnostics {
    pub connected: bool,
    pub collection: String,
    pub collection_exists: bool,
    pub qdrant_url: String,
    pub embedding_model: String,
    pub total_vectors: Option<u64>,
    pub sample_vectors: Option<u64>,
    pub enrichment_enriched: Option<u32>,
    pub enrichment_total: Option<u32>,
    pub enrichment_status: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub named_vectors_enabled: bool,
    /// `fast`, `full`, or `unknown` — reflects last sweep / scope posture.
    #[serde(default)]
    pub sweep_quality: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_embedding_model: Option<String>,
    #[serde(default)]
    pub embedding_model_mismatch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_vector_count: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QdrantConnectionStatus {
    pub success: bool,
    pub vectors_count: Option<u64>,
    pub collection_exists: bool,
    pub collections: Option<Vec<String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMarker {
    pub(crate) rsid: String,
    pub(crate) gene: Option<String>,
    pub(crate) chromosome: String,
    pub(crate) allele1: String,
    pub(crate) allele2: String,
    pub(crate) significance_score: f32,
    pub(crate) clinvar_sig: Option<String>,
}

/// Per-source toggles for enrichment during vector research sweeps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentSourcesConfig {
    #[serde(default = "default_true")]
    pub gnomad: bool,
    #[serde(default = "default_true")]
    pub clinvar_live: bool,
    #[serde(default = "default_true")]
    pub pubmed: bool,
    #[serde(default = "default_true")]
    pub gtex: bool,
    #[serde(default = "default_true")]
    pub vep_dbsnp: bool,
    #[serde(default = "default_true")]
    pub secondary: bool,
    /// Re-run enabled sources on variants already indexed when data was skipped.
    #[serde(default)]
    pub supplement_missing: bool,
}

fn default_true() -> bool {
    true
}

impl EnrichmentSourcesConfig {
    pub fn fast_index() -> Self {
        Self {
            gnomad: false,
            clinvar_live: false,
            pubmed: false,
            gtex: false,
            vep_dbsnp: false,
            secondary: false,
            supplement_missing: false,
        }
    }

    pub fn full() -> Self {
        Self {
            gnomad: true,
            clinvar_live: true,
            pubmed: true,
            gtex: true,
            vep_dbsnp: true,
            secondary: true,
            supplement_missing: false,
        }
    }

    pub fn from_sweep_fast(fast: bool) -> Self {
        if fast {
            Self::fast_index()
        } else {
            Self::full()
        }
    }

    pub fn is_fast_index(&self) -> bool {
        !self.gnomad
            && !self.clinvar_live
            && !self.pubmed
            && !self.gtex
            && !self.vep_dbsnp
            && !self.secondary
    }
}

impl Default for EnrichmentSourcesConfig {
    fn default() -> Self {
        Self::full()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchScopeConfig {
    pub curated: bool,
    pub agent_discoveries: bool,
    pub gwas_discovery: bool,
    pub non_reference: bool,
    pub non_reference_limit: u32,
    pub gwas_discovery_limit: u32,
    #[serde(default)]
    pub sweep_fast: bool,
    #[serde(default)]
    pub enrichment_sources: EnrichmentSourcesConfig,
}

impl Default for ResearchScopeConfig {
    fn default() -> Self {
        Self {
            curated: true,
            agent_discoveries: true,
            gwas_discovery: true,
            non_reference: true,
            non_reference_limit: 5000,
            gwas_discovery_limit: 10_000,
            sweep_fast: false,
            enrichment_sources: EnrichmentSourcesConfig::full(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResearchScopePreview {
    pub curated: u64,
    pub agent_discoveries: u64,
    pub gwas_discovery: u64,
    pub non_reference: u64,
    pub total_unique: u64,
    pub genotype_total: u64,
    pub gwas_reference_count: u64,
    pub gwas_genome_overlap: u64,
    /// GWAS overlap rsIDs not included because gwas_discovery_limit caps the queue.
    pub gwas_beyond_cap: u64,
}
