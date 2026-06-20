// ./src-tauri/src/research/evidence/types.rs
//! Normalized evidence models for vector research workbench.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const EVIDENCE_SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLedgerRow {
    pub source_name: String,
    pub trait_name: Option<String>,
    pub mapped_gene: Option<String>,
    pub p_value: Option<f64>,
    pub beta: Option<f64>,
    pub odds_ratio: Option<f64>,
    pub effect_allele: Option<String>,
    pub personal_dosage: Option<i32>,
    pub personal_direction: String,
    pub study_accession: Option<String>,
    pub pubmed_id: Option<String>,
    pub ancestry: Option<String>,
    pub sample_size: Option<i32>,
    pub fetched_at: Option<i64>,
    pub source_release: Option<String>,
    pub raw_payload_hash: Option<String>,
    pub quality_flags: Vec<String>,
    pub evidence_tier: String,
    pub association_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExplanation {
    pub vector_score: f32,
    pub hybrid_score: f32,
    pub shared_traits: Vec<String>,
    pub shared_genes: Vec<String>,
    pub shared_categories: Vec<String>,
    pub shared_sources: Vec<String>,
    pub matched_tags: Vec<String>,
    pub boost_reasons: Vec<String>,
    pub missing_metadata_warnings: Vec<String>,
    pub similarity_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCard {
    pub rsid: String,
    pub genotype: Option<String>,
    pub chromosome: Option<String>,
    pub position_grch38: Option<i64>,
    pub gene_symbol: Option<String>,
    pub gene_confidence: Option<String>,
    pub primary_trait: Option<String>,
    pub trait_category: Option<String>,
    pub trait_categories: Vec<String>,
    pub evidence_tier: Option<String>,
    pub association_type: Option<String>,
    pub personal_direction: String,
    pub directionality_confidence: f32,
    pub directionality_label: String,
    pub has_direction: bool,
    pub p_value_min: Option<f64>,
    pub gwas_best_pvalue: Option<f64>,
    pub association_strength_score: f32,
    pub personal_match_score: f32,
    pub clinical_actionability_score: f32,
    pub wellness_actionability_score: f32,
    pub data_quality_score: f32,
    pub novelty_score: f32,
    pub source_names: Vec<String>,
    pub primary_source: Option<String>,
    pub source_count: u32,
    pub conflict_count: u32,
    pub missing_field_count: u32,
    pub quality_flags: Vec<String>,
    pub missing_fields: Vec<String>,
    pub has_gwas: bool,
    pub has_clinvar: bool,
    #[serde(default)]
    pub has_pgs: bool,
    #[serde(default)]
    pub has_pharmgkb: bool,
    #[serde(default)]
    pub has_reactome: bool,
    pub clinvar_significance: Option<String>,
    pub gnomad_af: Option<f64>,
    pub enrichment_version: Option<String>,
    pub schema_version: Option<String>,
    pub stale: bool,
    pub semantic_score: f32,
    pub synthesis: String,
    pub association_summary: Option<Value>,
    pub gene_candidates: Option<Vec<Value>>,
    pub sources_provenance: Option<Value>,
    pub cross_refs: Option<Value>,
    pub evidence_ledger: Vec<EvidenceLedgerRow>,
    pub match_explanation: Option<MatchExplanation>,
    pub verification_ideas: Vec<String>,
    pub prohibited_claims: Vec<String>,
    pub qdrant_point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchParams {
    pub sample_id: i64,
    pub query: String,
    pub trait_category: Option<String>,
    pub evidence_tier: Option<String>,
    pub has_direction: Option<bool>,
    pub min_data_quality: Option<f32>,
    pub min_wellness_actionability: Option<f32>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarSearchParams {
    pub sample_id: i64,
    pub rsid: Option<String>,
    pub qdrant_point_id: Option<u64>,
    pub similarity_mode: String,
    pub limit: u32,
    pub include_self: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDashboard {
    pub sample_id: i64,
    pub total_genotypes: u64,
    pub vectorized_variants: Option<u64>,
    pub association_fact_count: u64,
    pub variants_with_gwas: u64,
    pub variants_with_clinvar: u64,
    pub known_direction_count: u64,
    pub unknown_direction_count: u64,
    pub missing_gene_count: u64,
    pub missing_effect_allele_count: u64,
    pub missing_study_accession_count: u64,
    pub stale_vector_count: u64,
    pub source_conflict_count: u64,
    pub schema_mismatch_count: u64,
    pub schema_v1_count: u64,
    pub enrichment_v41_count: u64,
    pub candidate_unreviewed_count: u64,
    pub avg_data_quality_score: f32,
    pub source_record_count: u64,
    pub cache_stale_count: u64,
    pub variants_with_pgs: u64,
    pub variants_with_gtex: u64,
    pub variants_with_pubmed: u64,
    pub source_coverage: SourceCoverageSummary,
    pub collection: String,
    pub enrichment_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceCoverageSummary {
    pub gwas: u64,
    pub clinvar: u64,
    pub gtex: u64,
    pub pubmed: u64,
    pub pgs: u64,
    pub pharmgkb: u64,
    pub reactome: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitClusterSummary {
    pub cluster_id: String,
    pub cluster_title: String,
    pub trait_category: Option<String>,
    pub top_traits: Vec<String>,
    pub top_genes: Vec<String>,
    pub top_rsids: Vec<String>,
    pub association_count: u32,
    pub source_count: u32,
    pub known_direction_count: u32,
    pub unknown_direction_count: u32,
    pub data_quality_score: f32,
    pub wellness_actionability_score: f32,
    pub clinical_actionability_score: f32,
    pub missing_field_count: u32,
    pub verification_ideas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMarkerRow {
    pub candidate_id: String,
    pub rsid: String,
    pub gene_symbol: Option<String>,
    pub trait_name: Option<String>,
    pub trait_category: Option<String>,
    pub status: String,
    pub data_quality_score: f32,
    pub association_strength_score: f32,
    pub reason_surfaced: Option<String>,
    pub best_evidence_tier: Option<String>,
    #[serde(default)]
    pub source_count: i32,
    #[serde(default)]
    pub known_direction_count: i32,
    pub best_p_value: Option<f64>,
    #[serde(default)]
    pub has_gwas: bool,
    #[serde(default)]
    pub has_clinvar: bool,
    #[serde(default)]
    pub has_gtex: bool,
    #[serde(default)]
    pub has_pubmed: bool,
    #[serde(default)]
    pub has_pgs: bool,
    #[serde(default)]
    pub has_pharmgkb: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePacket {
    pub packet_id: String,
    pub generated_at: i64,
    pub sample_id: i64,
    pub rsid: Option<String>,
    pub cluster_id: Option<String>,
    pub genotype: Option<String>,
    pub association_facts: Vec<Value>,
    pub evidence_cards: Vec<EvidenceCard>,
    pub similar_associations: Vec<EvidenceCard>,
    pub quality_flags: Vec<String>,
    pub missing_fields: Vec<String>,
    pub conflicts: Vec<String>,
    pub scores: Value,
    pub verification_ideas: Vec<String>,
    pub prohibited_claims: Vec<String>,
    pub research_questions: Vec<String>,
    #[serde(default)]
    pub source_records: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionabilityPoint {
    pub rsid: String,
    pub gene_symbol: Option<String>,
    pub trait_category: Option<String>,
    pub clinical_actionability_score: f32,
    pub wellness_actionability_score: f32,
    pub data_quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromosomeTraitBand {
    pub chromosome: String,
    pub trait_category: String,
    pub association_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathwayFlowRow {
    pub gene_symbol: String,
    pub pathway_name: String,
    pub trait_name: Option<String>,
    pub rsid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCandidateStatusRequest {
    pub candidate_id: String,
    pub status: String,
    pub reviewer_note: Option<String>,
}
