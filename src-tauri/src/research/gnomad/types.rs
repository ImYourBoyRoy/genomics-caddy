// ./src-tauri/src/research/gnomad/types.rs
use serde::{Deserialize, Serialize};

pub const PARSER_VERSION: &str = "gnomad_vcf_1";
pub const DEFAULT_RELEASE: &str = "4.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GnomadSourceMode {
    GraphQlInteractive,
    #[default]
    RemoteIndexedVcfHttps,
    LocalIndexedVcf,
    PythonToolboxSidecar,
}


impl GnomadSourceMode {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "graphql_interactive" | "graphql" => Self::GraphQlInteractive,
            "local_indexed_vcf" | "local" => Self::LocalIndexedVcf,
            "python_toolbox_sidecar" | "toolbox" => Self::PythonToolboxSidecar,
            _ => Self::RemoteIndexedVcfHttps,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::GraphQlInteractive => "graphql_interactive",
            Self::RemoteIndexedVcfHttps => "remote_indexed_vcf_https",
            Self::LocalIndexedVcf => "local_indexed_vcf",
            Self::PythonToolboxSidecar => "python_toolbox_sidecar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GnomadHttpsProvider {
    #[default]
    Aws,
    Google,
}


impl GnomadHttpsProvider {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "google" | "gcs" | "gcp" => Self::Google,
            _ => Self::Aws,
        }
    }

    pub fn base_url(self) -> &'static str {
        match self {
            Self::Aws => "https://gnomad-public-us-east-1.s3.amazonaws.com",
            Self::Google => "https://storage.googleapis.com/gcp-public-data--gnomad",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GnomadDataset {
    Exomes,
    Genomes,
    #[default]
    Combined,
}


impl GnomadDataset {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "exomes" | "exome" => Self::Exomes,
            "genomes" | "genome" => Self::Genomes,
            _ => Self::Combined,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GnomadLookupStatus {
    NotQueried,
    CacheHit,
    RemoteVcfHit,
    LocalVcfHit,
    GraphQlHit,
    NoRecordAtPosition,
    AlleleMismatch,
    MissingCoordinate,
    MissingRefAlt,
    AmbiguousLiftover,
    NetworkError,
    IndexMissing,
    SourceUnavailable,
    UnsupportedContig,
    ParserError,
    StaleRelease,
}

impl GnomadLookupStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotQueried => "not_queried",
            Self::CacheHit => "cache_hit",
            Self::RemoteVcfHit => "remote_vcf_hit",
            Self::LocalVcfHit => "local_vcf_hit",
            Self::GraphQlHit => "graphql_hit",
            Self::NoRecordAtPosition => "no_record_at_position",
            Self::AlleleMismatch => "allele_mismatch",
            Self::MissingCoordinate => "missing_coordinate",
            Self::MissingRefAlt => "missing_ref_alt",
            Self::AmbiguousLiftover => "ambiguous_liftover",
            Self::NetworkError => "network_error",
            Self::IndexMissing => "index_missing",
            Self::SourceUnavailable => "source_unavailable",
            Self::UnsupportedContig => "unsupported_contig",
            Self::ParserError => "parser_error",
            Self::StaleRelease => "stale_release",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "cache_hit" => Self::CacheHit,
            "remote_vcf_hit" => Self::RemoteVcfHit,
            "local_vcf_hit" => Self::LocalVcfHit,
            "graphql_hit" => Self::GraphQlHit,
            "no_record_at_position" => Self::NoRecordAtPosition,
            "allele_mismatch" => Self::AlleleMismatch,
            "missing_coordinate" => Self::MissingCoordinate,
            "missing_ref_alt" => Self::MissingRefAlt,
            "ambiguous_liftover" => Self::AmbiguousLiftover,
            "network_error" => Self::NetworkError,
            "index_missing" => Self::IndexMissing,
            "source_unavailable" => Self::SourceUnavailable,
            "unsupported_contig" => Self::UnsupportedContig,
            "parser_error" => Self::ParserError,
            "stale_release" => Self::StaleRelease,
            _ => Self::NotQueried,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GnomadDatasetPolicy {
    /// Query exomes first; genomes only when exomes miss (recommended for sweeps).
    #[default]
    Auto,
    /// Always query both exomes and genomes.
    Combined,
    ExomesOnly,
    GenomesOnly,
}


impl GnomadDatasetPolicy {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "combined" | "both" => Self::Combined,
            "exomes_only" | "exomes" => Self::ExomesOnly,
            "genomes_only" | "genomes" => Self::GenomesOnly,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Combined => "combined",
            Self::ExomesOnly => "exomes_only",
            Self::GenomesOnly => "genomes_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadConfig {
    pub enabled: bool,
    pub source_mode: GnomadSourceMode,
    pub release: String,
    pub provider: GnomadHttpsProvider,
    pub local_vcf_dir: Option<String>,
    pub max_remote_concurrent_files: usize,
    pub max_queries_per_second: f64,
    pub graphql_enabled_for_sweep: bool,
    /// GraphQL fallback after VCF miss for priority variants (association/GWAS hits).
    pub graphql_fallback_enabled: bool,
    pub dataset_policy: GnomadDatasetPolicy,
    pub exome_template: String,
    pub genome_template: String,
}

impl Default for GnomadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            source_mode: GnomadSourceMode::RemoteIndexedVcfHttps,
            release: DEFAULT_RELEASE.to_string(),
            provider: GnomadHttpsProvider::Aws,
            local_vcf_dir: None,
            max_remote_concurrent_files: 2,
            max_queries_per_second: 4.0,
            graphql_enabled_for_sweep: false,
            graphql_fallback_enabled: true,
            dataset_policy: GnomadDatasetPolicy::Auto,
            exome_template: "release/4.1/vcf/exomes/gnomad.exomes.v4.1.sites.chr{chrom}.vcf.bgz"
                .to_string(),
            genome_template: "release/4.1/vcf/genomes/gnomad.genomes.v4.1.sites.chr{chrom}.vcf.bgz"
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadContext {
    pub lookup_status: String,
    pub release: String,
    pub dataset: String,
    pub variant_id: Option<String>,
    pub rsids: Vec<String>,
    pub ac: Option<i64>,
    pub an: Option<i64>,
    pub af: Option<f64>,
    pub ac_exomes: Option<i64>,
    pub an_exomes: Option<i64>,
    pub af_exomes: Option<f64>,
    pub ac_genomes: Option<i64>,
    pub an_genomes: Option<i64>,
    pub af_genomes: Option<f64>,
    pub popmax: Option<f64>,
    pub popmax_population: Option<String>,
    pub faf95_popmax: Option<f64>,
    pub faf95_popmax_population: Option<String>,
    pub homozygote_count: Option<i64>,
    pub hemizygote_count: Option<i64>,
    pub filters: Vec<String>,
    pub flags: Vec<String>,
    pub population_frequencies: serde_json::Value,
    pub source_url: Option<String>,
    pub source_mode: String,
    pub fetched_at: Option<i64>,
    pub warnings: Vec<String>,
    pub user_allele_match_status: Option<String>,
}

impl GnomadContext {
    pub fn empty(status: GnomadLookupStatus) -> Self {
        Self {
            lookup_status: status.as_str().to_string(),
            release: DEFAULT_RELEASE.to_string(),
            dataset: "combined".to_string(),
            variant_id: None,
            rsids: Vec::new(),
            ac: None,
            an: None,
            af: None,
            ac_exomes: None,
            an_exomes: None,
            af_exomes: None,
            ac_genomes: None,
            an_genomes: None,
            af_genomes: None,
            popmax: None,
            popmax_population: None,
            faf95_popmax: None,
            faf95_popmax_population: None,
            homozygote_count: None,
            hemizygote_count: None,
            filters: Vec::new(),
            flags: Vec::new(),
            population_frequencies: serde_json::json!({}),
            source_url: None,
            source_mode: GnomadSourceMode::default().as_str().to_string(),
            fetched_at: None,
            warnings: Vec::new(),
            user_allele_match_status: None,
        }
    }

    pub fn best_af(&self) -> Option<f64> {
        [self.af, self.af_exomes, self.af_genomes, self.popmax]
            .into_iter()
            .flatten()
            .filter(|v| *v > 0.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadLookupRequest {
    pub sample_id: i64,
    pub rsid: String,
    pub chrom_grch38: Option<String>,
    pub pos_grch38: Option<i64>,
    pub reference_allele: Option<String>,
    pub alternate_allele: Option<String>,
    pub genotype: Option<String>,
    pub dataset: Option<String>,
    pub release: Option<String>,
    pub source_mode: Option<String>,
    pub force_refresh: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadBatchRequest {
    pub sample_id: i64,
    pub rsids: Option<Vec<String>>,
    pub association_ids: Option<Vec<String>>,
    pub trait_category: Option<String>,
    pub priority_mode: Option<String>,
    pub source_mode: Option<String>,
    pub release: Option<String>,
    pub limit: Option<usize>,
    pub force_refresh: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadBatchProgress {
    pub total_candidates: usize,
    pub cache_hits: usize,
    pub remote_queries: usize,
    pub local_queries: usize,
    pub allele_matches: usize,
    pub allele_mismatches: usize,
    pub missing_coordinates: usize,
    pub errors: usize,
    pub current_chromosome: Option<String>,
    pub estimated_remaining: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadSourceTestResult {
    pub release: String,
    pub provider: String,
    pub exome_vcf_url: String,
    pub exome_index_url: String,
    pub genome_vcf_url: String,
    pub genome_index_url: String,
    pub exome_vcf_ok: bool,
    pub exome_index_ok: bool,
    pub genome_vcf_ok: bool,
    pub genome_index_ok: bool,
    pub smoke_test_ok: bool,
    pub smoke_test_message: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadMissingItem {
    pub id: String,
    pub kind: String,
    pub chrom: Option<String>,
    pub dataset: Option<String>,
    pub label: String,
    pub url: Option<String>,
    pub fix_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadReadinessStatus {
    pub ready: bool,
    pub enabled: bool,
    pub source_mode: String,
    pub release: String,
    pub provider: String,
    pub remote_urls_ok: bool,
    pub indexes_expected: u32,
    pub indexes_cached: u32,
    pub cache_dir: String,
    pub local_dir: Option<String>,
    pub missing_items: Vec<GnomadMissingItem>,
    pub summary: String,
    pub primary_action: Option<String>,
    pub downloads_page_url: String,
    pub manifest_exome_contigs: Vec<String>,
    pub manifest_genome_contigs: Vec<String>,
    pub unsupported_contigs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomadIndexSyncResult {
    pub downloaded: u32,
    pub skipped: u32,
    pub failed: u32,
    pub message: String,
    pub errors: Vec<String>,
}
