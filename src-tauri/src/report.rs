// ./src-tauri/src/report.rs
/*
Module Docstring:
Purpose: Direction-aware, template-based report generator for genetic trait profiling.
Responsibilities:
- Evaluate effect alleles for custom marker profiles defined in JSON.
- Classify each marker's severity based on effect_direction, evidence_tier, effect_count,
  and clinical_confirmation_required — not just raw allele count.
- Compute direction-aware section summaries with separate tallies for risk, protective,
  trait, and context-dependent markers.
- Only risk-direction markers contribute to percent signal scores.
- Sections where all markers require clinical confirmation suppress percent display.
- Enrich each evaluated marker with ClinVar / GWAS data from the local reference DB
  and gnomAD allele frequencies from the local variant cache.
Key Inputs: SQLite connection, sample ID, and report template JSON.
Key Outputs: Generated report JSON containing evaluated markers, severity classes, section summaries, and DB enrichment.
Operational Notes: Designed for consumer-grade raw DNA, not clinical diagnostics.
*/

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::offline::schema::schema_attached;

#[derive(Debug, Deserialize)]
struct ExportDisclosures {
    privacy_warning: String,
    raw_genotype_section_title: String,
    raw_genotype_notice: String,
    import_provenance_notice: String,
}

static EXPORT_DISCLOSURES: LazyLock<ExportDisclosures> = LazyLock::new(|| {
    let policy = include_str!("../../src/lib/marker-packs/ai_prompt_policy.json");
    serde_json::from_str::<serde_json::Value>(policy)
        .ok()
        .and_then(|value| value.get("export_disclosures").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .expect("ai_prompt_policy.json must contain valid export disclosures")
});

/// The source callability resource is the canonical variant-type policy for
/// both the TypeScript context and the Rust evaluator. Keeping the scoring
/// decision in that resource prevents a non-SNP assertion with a one-letter
/// label from entering the SNP allele counter.
static CALLABILITY_SCORING_POLICIES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let resource = include_str!("../../src/lib/marker-packs/callability_rules.json");
    let value: serde_json::Value = serde_json::from_str(resource)
        .expect("callability_rules.json must contain valid JSON");
    value
        .get("variant_type_registry")
        .and_then(serde_json::Value::as_object)
        .map(|registry| {
            registry
                .iter()
                .filter_map(|(variant_type, policy)| {
                    policy
                        .get("scoring_policy")
                        .and_then(serde_json::Value::as_str)
                        .map(|scoring_policy| (variant_type.to_ascii_lowercase(), scoring_policy.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
});

static CALLABILITY_ASSAY_REQUIREMENTS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let resource = include_str!("../../src/lib/marker-packs/callability_rules.json");
    let value: serde_json::Value = serde_json::from_str(resource)
        .expect("callability_rules.json must contain valid JSON");
    value
        .get("variant_type_registry")
        .and_then(serde_json::Value::as_object)
        .map(|registry| {
            registry
                .iter()
                .filter_map(|(variant_type, policy)| {
                    policy
                        .get("assay_requirement")
                        .and_then(serde_json::Value::as_str)
                        .map(|requirement| {
                            (variant_type.to_ascii_lowercase(), requirement.to_string())
                        })
                })
                .collect()
        })
        .unwrap_or_default()
});

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectDirection {
    Risk,
    Protective,
    ContextDependent,
    Trait,
    Unknown,
    NotApplicable,
    NoClaim,
}

// ---------------------------------------------------------------------------
// Input structs (deserialized from JSON template)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerSource {
    pub name: String,
    pub url: Option<String>,
    pub accessed: Option<String>,
    pub evidence_type: Option<String>,
    pub conflict_of_interest: Option<String>,
    pub notes: Option<String>,
}

/// A citation derived from the local reference database (ClinVar, GWAS, evidence_library).
/// These supplement — never replace — the marker pack's own sources.
#[derive(Debug, Serialize, Clone, Default)]
pub struct EnrichedSource {
    /// "ClinVar", "GWAS", "PharmGKB", "EvidenceLibrary"
    pub source_type: String,
    /// Human-readable citation text, e.g. "ClinVar: Pathogenic (expert panel)"
    pub citation: String,
    /// Optional additional detail, e.g. associated trait name or conditions
    pub details: Option<String>,
    /// Link to the canonical database entry
    pub url: Option<String>,
}

/// Stable, deduplicated source metadata exposed alongside every generated report.
///
/// The identity contract intentionally mirrors `src/lib/utils/reportReferences.ts`:
/// URLs are preferred, while URL-less sources use their descriptive fields. The
/// UTF-16 hash keeps IDs stable across the Rust report generator and TypeScript
/// exports, including for non-ASCII source text.
#[derive(Debug, Serialize, Clone)]
pub struct ReportReference {
    pub id: String,
    pub title: String,
    pub organization: String,
    pub date: String,
    pub evidence_role: String,
    pub url: Option<String>,
}

#[derive(Clone, Copy)]
enum ReferenceSource<'a> {
    Marker(&'a MarkerSource),
    Enriched(&'a EnrichedSource),
}

fn clean_reference(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn normalized_reference_url(value: Option<&str>) -> String {
    let trimmed = value.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.strip_suffix('/').unwrap_or(trimmed).to_lowercase()
}

fn report_reference_key(source: ReferenceSource<'_>) -> String {
    let url = match source {
        ReferenceSource::Marker(marker) => normalized_reference_url(marker.url.as_deref()),
        ReferenceSource::Enriched(enriched) => normalized_reference_url(enriched.url.as_deref()),
    };
    if !url.is_empty() {
        return format!("url:{url}");
    }

    match source {
        ReferenceSource::Marker(marker) => [
            clean_reference(Some(&marker.name), "Not recorded"),
            clean_reference(marker.evidence_type.as_deref(), "Not recorded"),
            clean_reference(marker.notes.as_deref(), "Not recorded"),
        ]
        .join("|")
        .to_lowercase(),
        ReferenceSource::Enriched(enriched) => [
            clean_reference(Some(&enriched.source_type), "Not recorded"),
            clean_reference(Some(&enriched.citation), "Not recorded"),
            clean_reference(enriched.details.as_deref(), "Not recorded"),
        ]
        .join("|")
        .to_lowercase(),
    }
}

fn report_reference_id(key: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for code_unit in key.encode_utf16() {
        hash ^= u32::from(code_unit);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("REF-{hash:08X}")
}

fn report_reference_from_source(source: ReferenceSource<'_>, id: String) -> ReportReference {
    match source {
        ReferenceSource::Marker(marker) => ReportReference {
            id,
            title: clean_reference(Some(&marker.name), "Reference"),
            organization: clean_reference(Some(&marker.name), "Not specified"),
            date: clean_reference(marker.accessed.as_deref(), "Access date not recorded"),
            evidence_role: clean_reference(
                marker.evidence_type.as_deref(),
                "Marker-pack reference",
            ),
            url: marker.url.as_ref().map(|url| url.trim().to_string()),
        },
        ReferenceSource::Enriched(enriched) => ReportReference {
            id,
            title: clean_reference(Some(&enriched.citation), "Catalog reference"),
            organization: clean_reference(Some(&enriched.source_type), "Local reference catalog"),
            date: "Local catalog record".to_string(),
            evidence_role: clean_reference(enriched.details.as_deref(), "Catalog evidence"),
            url: enriched.url.as_ref().map(|url| url.trim().to_string()),
        },
    }
}

#[derive(Default)]
struct ReportReferenceRegistryBuilder {
    references: Vec<ReportReference>,
    ids_by_key: HashMap<String, String>,
}

impl ReportReferenceRegistryBuilder {
    fn register(&mut self, source: ReferenceSource<'_>) -> String {
        let key = report_reference_key(source);
        if let Some(id) = self.ids_by_key.get(&key) {
            return id.clone();
        }

        let id = report_reference_id(&key);
        let reference = report_reference_from_source(source, id.clone());
        self.ids_by_key.insert(key, id.clone());
        self.references.push(reference);
        id
    }

    fn register_marker_sources(&mut self, sources: &[MarkerSource]) -> Vec<String> {
        sources
            .iter()
            .map(|source| self.register(ReferenceSource::Marker(source)))
            .collect()
    }

    fn register_enriched_sources(&mut self, sources: &[EnrichedSource]) -> Vec<String> {
        sources
            .iter()
            .map(|source| self.register(ReferenceSource::Enriched(source)))
            .collect()
    }
}

/// Local reference DB data fetched per-rsID during report generation.
#[derive(Debug, Default)]
struct LocalEnrichment {
    pub clinvar_annotations: Vec<ClinVarAnnotation>,
    pub gwas_associations: Vec<serde_json::Value>,
    pub clinvar_significance: Option<String>,
    pub clinvar_conditions: Option<String>,
    pub clinvar_review_status: Option<String>,
    pub gwas_top_trait: Option<String>,
    pub gwas_best_pvalue: Option<f64>,
    pub gwas_association_count: Option<i64>,
    pub population_af: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingInterpretationClass {
    SusceptibilityContext,
    CarrierPossibility,
    ClinicallyActionableVariant,
    ResearchContext,
    ProtectiveContext,
    TraitContext,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingInheritanceModel {
    AutosomalDominant,
    AutosomalRecessive,
    XLinked,
    YLinked,
    Mitochondrial,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingClinicalState {
    ClinicallyConfirmed,
    CarrierPossibility,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClinicalSemantics {
    pub condition_label: Option<String>,
    pub interpretation_class: Option<FindingInterpretationClass>,
    pub inheritance_model: Option<FindingInheritanceModel>,
    pub clinical_state: Option<FindingClinicalState>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MarkerDefinition {
    pub rsid: String,
    pub gene: String,
    pub variant_name: Option<String>,
    #[serde(alias = "risk_allele")]
    pub effect_allele: String,
    pub impact: String,
    pub evidence_tier: String,
    pub interpretation: String,
    pub do_not_claim: Vec<String>,
    pub confirm_with: Vec<String>,
    pub effect_direction: EffectDirection,
    pub raw_dna_limitation: Option<String>,
    pub clinical_confirmation_required: Option<bool>,
    /// Optional biological applicability hint; never inferred as gender or anatomy.
    pub sex_scope: Option<String>,
    /// Explicit biological/life-stage context tags used for compact UI and handoffs.
    pub context_tags: Option<Vec<String>>,
    pub sources: Option<Vec<MarkerSource>>,
    pub variant_type: Option<String>,
    pub expected_plus_alleles: Option<Vec<String>>,
    pub strand: Option<String>,
    pub source_build: Option<String>,
    pub hgvs: Option<String>,
    pub allele_orientation_verified: Option<bool>,
    pub orientation_source: Option<String>,
    pub interpretation_blocked_if_unverified: Option<bool>,
    /// Explicit disease/inheritance semantics; never inferred from a gene name.
    pub clinical_semantics: Option<ClinicalSemantics>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SectionDefinition {
    pub name: String,
    pub markers: Vec<MarkerDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReportTemplate {
    pub title: String,
    pub description: String,
    pub sections: Vec<SectionDefinition>,
}

// ---------------------------------------------------------------------------
// Output structs (serialized to JSON for the frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    Found,
    NoData,
    NotInRawFile,
    AmbiguousRawCall,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionStatus {
    Verified,
    NoData,
    NotInRawFile,
    BlockedRawCall,
    UnverifiedOrientation,
    OrientationMismatch,
    AmbiguousAlleles,
    NotEvaluated,
}

/// Explains whether the current report can evaluate the assertion without
/// overloading `AssertionStatus`, which also carries raw-data and orientation
/// outcomes. This is intentionally orthogonal: a SNP can be callable but not
/// present in the imported file, while an HLA/panel assertion can be present
/// as a row but not callable by this evaluator.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CallabilityState {
    Callable,
    NotCallable,
    NotPresent,
    Blocked,
    Unknown,
}

/// Explicit strand/orientation outcome, separate from assertion status and
/// callability so consumers can explain why normalization was or was not
/// applied without interpreting a missing call as a mismatch.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrientationState {
    Verified,
    NotRequired,
    Unverified,
    Mismatch,
    Unknown,
}

#[derive(Debug, Serialize, Clone)]
pub struct CanonicalVariant {
    pub rsid: String,
    pub gene: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromosome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_grch37: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_grch38: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_type: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserCall {
    pub user_genotype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_genotype: Option<String>,
    pub call_status: CallStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_build: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VariantCategoryLink {
    pub link_id: String, // format: "{pack_id}:{category_id}:{rsid}:{stable_assertion_hash}"
    /// Explicit assertion identity, separate from Simple-mode presentation grouping.
    pub assertion_key: String,
    pub rsid: String,
    pub gene: String,
    pub variant_name: Option<String>,
    pub variant_type: Option<String>,
    pub source_build: Option<String>,
    pub hgvs: Option<String>,
    pub orientation_source: Option<String>,
    pub category_id: String,
    pub category_label: String,
    pub impact: String,
    pub evidence_tier: String,
    pub interpretation: String,
    pub effect_direction: EffectDirection,
    pub effect_allele: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_plus_alleles: Option<Vec<String>>,
    pub severity_class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_count: Option<u8>,
    pub assertion_status: AssertionStatus,
    pub callability_state: CallabilityState,
    pub orientation_state: OrientationState,
    pub requires_orientation_verification: bool,
    pub interpretation_allowed: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub do_not_claim: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub confirm_with: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_dna_limitation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinical_confirmation_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation_blocked_if_unverified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinical_semantics: Option<ClinicalSemantics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<MarkerSource>,
    /// Stable IDs into the report-level reference registry.
    pub reference_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClinVarAnnotation {
    pub clinical_significance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allele_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rcv_accession: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene_symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phenotype_ids: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assembly: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromosome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_evaluated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_submitters: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_allele: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternate_allele: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GwasHit {
    pub trait_name: String,
    pub p_value_string: String,
    pub neg_log10_p: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub association_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p_value_underflow: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DbsnpAnnotation {
    pub rsid: String,
    pub chromosome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_grch37: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_grch38: Option<i64>,
    pub ref_allele: String,
    pub alt_alleles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plus_strand_alleles: Option<Vec<String>>,
    /// Release / build label from the allele source (typically gnomAD, not offline dbSNP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_build: Option<String>,
    /// Explicit provenance: offline dbSNP is merge aliases only; allele/AF chips use gnomAD cache.
    #[serde(default = "default_allele_source")]
    pub allele_source: String,
}

#[allow(dead_code)] // referenced by serde `default = "default_allele_source"`
fn default_allele_source() -> String {
    "gnomAD".to_string()
}

#[derive(Debug, Serialize, Clone)]
pub struct PopulationAnnotation {
    pub allele_frequency: f64,
    pub rarity_bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popmax: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popmax_population: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faf95_popmax: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homozygote_count: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PharmGkbAnnotation {
    pub drug: String,
    pub phenotype: String,
    pub evidence_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gene: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClinGenAnnotation {
    pub gene_symbol: String,
    pub disease_label: String,
    pub classification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hgnc_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode_of_inheritance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ManeAnnotation {
    pub gene_symbol: String,
    pub ensembl_transcript: String,
    pub refseq_transcript: String,
    pub mane_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grch38_coordinates: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VariantEnrichment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinvar: Option<ClinVarAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clinvar_annotations: Vec<ClinVarAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gwas_hits: Vec<GwasHit>,
    /// Individual local GWAS Catalog records retained for condition/trait linking.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gwas_associations: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dbsnp: Option<DbsnpAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<PopulationAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pharmgkb: Option<PharmGkbAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pharmgkb_annotations: Vec<PharmGkbAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clingen: Option<ClinGenAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clingen_annotations: Vec<ClinGenAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mane: Option<ManeAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mane_annotations: Vec<ManeAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub db_enriched_sources: Vec<EnrichedSource>,
    /// Stable IDs into the report-level reference registry.
    pub reference_ids: Vec<String>,
}

/// Direction-aware summary statistics for a report section.
#[derive(Debug, Serialize, Clone)]
pub struct SectionSummary {
    pub risk_effect_count: u16,
    pub risk_possible: u16,
    pub protective_effect_count: u16,
    pub protective_possible: u16,
    pub trait_count: u16,
    pub context_dependent_count: u16,
    pub no_data_count: u16,
    pub not_evaluated_count: u16,
    pub confirmation_required_count: u16,
    pub total_markers: u16,
    pub show_percent_score: bool,
    pub all_require_confirmation: bool,
    pub active_marker_count: u16,
    pub active_risk_marker_count: u16,
    pub active_protective_marker_count: u16,
    pub active_trait_marker_count: u16,
    pub active_context_marker_count: u16,
    pub blocked_unverified_count: u16,
    pub benign_modifier_count: u16,
}

#[derive(Debug, Serialize, Clone)]
pub struct NormalizedSection {
    pub section_id: String,
    pub name: String,
    pub link_ids: Vec<String>,
    pub section_signal_score: f64,
    pub summary: SectionSummary,
}

#[derive(Debug, Serialize, Clone)]
pub struct GeneratedReport {
    pub schema_version: String,
    pub export_format: String,
    pub generated_at: String,
    pub title: String,
    pub description: String,
    pub overall_signal_score: f64,
    pub variants: HashMap<String, CanonicalVariant>,
    pub user_calls: HashMap<String, UserCall>,
    pub category_links: HashMap<String, VariantCategoryLink>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub enrichment: HashMap<String, VariantEnrichment>,
    pub sections: Vec<NormalizedSection>,
    /// Deduplicated source metadata referenced by link/enrichment IDs.
    pub references: Vec<ReportReference>,
    /// Explicit catalog readiness notes (never silent when ClinVar/dbSNP expected but missing).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub catalog_warnings: Vec<String>,
    /// Local, all-imported-variant ClinVar condition matches. Raw alleles are
    /// evaluated in-process and are never serialized into this summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genomewide_clinvar: Option<GenomeWideClinVarDiscovery>,
    /// Source-file and coordinate provenance for the genotype database used
    /// to produce this report. This contains metadata, not raw genotype rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_provenance: Option<crate::parser::ImportProvenance>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct GenomeWideClinVarDiscovery {
    pub local_index_ready: bool,
    pub genotypes_scanned: u64,
    pub exact_variant_count: u64,
    pub association_count: u64,
    pub omitted_association_count: u64,
    pub allele_orientation: String,
    pub source_asset_ids: Vec<String>,
    pub associations: Vec<GenomeWideClinVarAssociation>,
}

#[derive(Debug, Serialize, Clone)]
pub struct GenomeWideClinVarAssociation {
    pub association_is: String,
    pub association_scope: String,
    pub condition: String,
    pub rsid: String,
    pub gene_symbol: Option<String>,
    pub variation_id: String,
    pub scv_accession: String,
    pub clinical_significance: String,
    pub variant_summary_clinical_significance: String,
    pub review_status: String,
    pub last_evaluated: Option<String>,
    pub allele_match: String,
    pub origin_status: String,
    pub variant_summary_conflict: bool,
    pub source_url: String,
}

// ---------------------------------------------------------------------------
// Severity classification
// ---------------------------------------------------------------------------

/// Determine the visual severity class for a single marker.
fn compute_severity_class(
    effect_count: u8,
    direction: &EffectDirection,
    evidence_tier: &str,
    clinical_confirmation_required: Option<bool>,
    is_missing: bool,
) -> String {
    // No data
    if is_missing {
        return "no_data".to_string();
    }

    // No effect alleles detected → benign presentation regardless of direction
    if effect_count == 0 {
        return "benign".to_string();
    }

    // Clinical confirmation takes precedence over direction styling
    if clinical_confirmation_required == Some(true) {
        return "confirmation_required".to_string();
    }

    // Direction-based classification
    match direction {
        EffectDirection::Protective => "protective".to_string(),
        EffectDirection::Trait => "trait".to_string(),
        EffectDirection::ContextDependent => "context_dependent".to_string(),
        EffectDirection::Unknown => "context_dependent".to_string(),
        EffectDirection::NotApplicable => "trait".to_string(),
        EffectDirection::NoClaim => "trait".to_string(),
        EffectDirection::Risk => {
            // Evidence tier modulates severity:
            // Tier A/B → full severity classification
            // Tier C   → capped at moderate_risk (no high_risk for unvalidated markers)
            // Tier D/E → low_risk ceiling (weak hypothesis, inform-only)
            let is_strong = evidence_tier.starts_with('A') || evidence_tier.starts_with('B');
            let is_weak = evidence_tier.starts_with('D') || evidence_tier.starts_with('E');
            match (effect_count, is_strong, is_weak) {
                (2, true, _) => "high_risk".to_string(),
                (2, false, false) => "moderate_risk".to_string(), // Tier C, homozygous → moderate
                (2, _, true) => "low_risk".to_string(),           // Tier D/E, any → low
                (1, _, true) => "low_risk".to_string(),           // Tier D/E single copy
                (1, _, _) => "moderate_risk".to_string(),         // Tier A/B/C, single copy
                _ => "benign".to_string(),
            }
        }
    }
}

/// Return whether the curated assertion currently defines an allele that the
/// consumer-array report evaluator can compare to the stored raw call.
///
/// This deliberately accepts only a single canonical nucleotide. Prose such
/// as `study_reported_allele`, `N/A`, panel classifications, and multi-locus
/// score labels must not fall through to `count_effect_alleles`, where a zero
/// count would otherwise look like a benign result. Indels, haplotypes, CNVs,
/// HLA alleles, diplotypes, and PRS models require their own callability
/// policy and remain outside this SNP counter until one exists.
fn is_matchable_effect_allele(effect_allele: &str) -> bool {
    let allele = effect_allele.trim().to_ascii_uppercase();
    allele.chars().count() == 1
        && allele
            .chars()
            .next()
            .is_some_and(|base| matches!(base, 'A' | 'C' | 'G' | 'T'))
}

fn is_matchable_snp_assertion(variant_type: Option<&str>, effect_allele: &str) -> bool {
    // Existing programmatic report templates may omit variant_type. Preserve
    // that API by treating an omitted type as the legacy SNP path; curated
    // marker packs are required to classify every row as a concrete type.
    let policy_type = variant_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("snp")
        .to_ascii_lowercase();
    CALLABILITY_SCORING_POLICIES
        .get(&policy_type)
        .is_some_and(|policy| policy == "snp_allele_count")
        && is_matchable_effect_allele(effect_allele)
}

fn callability_state_for_result(
    variant_type: Option<&str>,
    assertion_status: AssertionStatus,
    is_missing: bool,
) -> CallabilityState {
    let policy_type = variant_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("snp")
        .to_ascii_lowercase();
    let evaluator_can_score = CALLABILITY_SCORING_POLICIES
        .get(&policy_type)
        .is_some_and(|policy| policy == "snp_allele_count");

    if !evaluator_can_score || assertion_status == AssertionStatus::NotEvaluated {
        return CallabilityState::NotCallable;
    }
    if is_missing {
        return CallabilityState::NotPresent;
    }
    if matches!(
        assertion_status,
        AssertionStatus::BlockedRawCall
            | AssertionStatus::UnverifiedOrientation
            | AssertionStatus::OrientationMismatch
            | AssertionStatus::AmbiguousAlleles
    ) {
        return CallabilityState::Blocked;
    }
    if assertion_status == AssertionStatus::Verified {
        return CallabilityState::Callable;
    }
    CallabilityState::Unknown
}

fn callability_assay_requirement(variant_type: Option<&str>) -> String {
    let policy_type = variant_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("snp")
        .to_ascii_lowercase();
    CALLABILITY_ASSAY_REQUIREMENTS
        .get(&policy_type)
        .or_else(|| CALLABILITY_ASSAY_REQUIREMENTS.get("unspecified"))
        .cloned()
        .unwrap_or_else(|| "variant type must be classified before interpretation".to_string())
}

fn orientation_state_for_result(
    assertion_status: AssertionStatus,
    requires_orientation_verification: bool,
    marker_orientation_evidence: bool,
    orientation_mismatch_detected: bool,
    is_missing: bool,
) -> OrientationState {
    if orientation_mismatch_detected {
        return OrientationState::Mismatch;
    }
    match assertion_status {
        AssertionStatus::OrientationMismatch => OrientationState::Mismatch,
        AssertionStatus::UnverifiedOrientation => OrientationState::Unverified,
        AssertionStatus::Verified => {
            if requires_orientation_verification || marker_orientation_evidence {
                OrientationState::Verified
            } else {
                OrientationState::NotRequired
            }
        }
        AssertionStatus::NoData
        | AssertionStatus::NotInRawFile
        | AssertionStatus::BlockedRawCall
        | AssertionStatus::AmbiguousAlleles
        | AssertionStatus::NotEvaluated => {
            if requires_orientation_verification {
                if is_missing {
                    OrientationState::Unknown
                } else {
                    OrientationState::Unverified
                }
            } else {
                OrientationState::NotRequired
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Enrichment helpers
// ---------------------------------------------------------------------------

/// Returns a plain-language population rarity bucket for an allele frequency.
fn classify_population_rarity(af: f64) -> &'static str {
    if af >= 0.05 {
        "Common (>5%)"
    } else if af >= 0.01 {
        "Uncommon (1\u{2013}5%)"
    } else if af >= 0.001 {
        "Rare (<1%)"
    } else {
        "Very Rare (<0.1%)"
    }
}

/// Build a gnomAD population frequency URL for an rsID.
fn gnomad_url(rsid: &str) -> String {
    format!("https://gnomad.broadinstitute.org/variant/{}", rsid)
}

/// Fetch ClinVar and GWAS reference data for a batch of rsIDs in a single SQL query.
/// Returns a map keyed by rsID. Missing rsIDs will simply be absent from the map.
/// Resolves a batch of rsIDs dynamically using recursive Common Table Expressions (CTEs)
/// to look up their ultimate merge targets in the `rsid_aliases` table.
/// Returns a map from original_rsid -> current_rsid.
pub fn resolve_normalized_rsids(conn: &Connection, rsids: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if rsids.is_empty() {
        return map;
    }

    // SQLite recursive CTE to trace rsID merges up to 10 levels deep.
    // Exact case matching is used here (dbSNP entries are normalized to lowercase rsNNN)
    // to utilize the primary key index on rsid_aliases(rsid).
    let sql = "
        WITH RECURSIVE merge_chain(rsid, depth) AS (
            VALUES(?, 0)
            UNION ALL
            SELECT alias.merged_into, merge_chain.depth + 1
            FROM rsid_aliases alias
            JOIN merge_chain ON alias.rsid = merge_chain.rsid
            WHERE alias.merged_into IS NOT NULL AND merge_chain.depth < 10
        )
        SELECT rsid FROM merge_chain ORDER BY depth DESC LIMIT 1;
    ";

    if let Ok(mut stmt) = conn.prepare(sql) {
        for original in rsids {
            let orig_lower = original.to_lowercase();
            let normalized = stmt
                .query_row([orig_lower], |row| row.get::<_, String>(0))
                .unwrap_or_else(|_| original.to_lowercase());
            if normalized != original.to_lowercase() {
                map.insert(original.clone(), normalized);
            }
        }
    }
    map
}

fn catalog_table_available(conn: &Connection, schema: &str, table: &str) -> bool {
    if !schema_attached(conn, schema) {
        return false;
    }
    let sql = format!(
        "SELECT 1 FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1"
    );
    conn.query_row(&sql, [table], |_| Ok(true)).is_ok()
}

fn table_column_available(
    conn: &Connection,
    schema: Option<&str>,
    table: &str,
    column: &str,
) -> bool {
    let pragma = match schema {
        Some(schema) => format!("PRAGMA {schema}.table_info({table})"),
        None => format!("PRAGMA table_info({table})"),
    };
    conn.prepare(&pragma)
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))
                .map(|rows| rows.flatten().any(|name| name.eq_ignore_ascii_case(column)))
        })
        .unwrap_or(false)
}

fn non_empty_option(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

fn first_non_empty(primary: Option<String>, fallback: Option<String>) -> Option<String> {
    non_empty_option(primary).or_else(|| non_empty_option(fallback))
}

/// Fetch ClinVar and GWAS reference data for a batch of rsIDs in a single SQL query.
/// Returns a map keyed by rsID. Missing rsIDs will simply be absent from the map.
fn fetch_local_enrichment(conn: &Connection, rsids: &[String]) -> HashMap<String, LocalEnrichment> {
    if rsids.is_empty() {
        return HashMap::new();
    }

    // 1. Resolve normalized/current rsIDs
    let normalized_map = resolve_normalized_rsids(conn, rsids);
    let mut current_rsids = Vec::new();
    for rsid in rsids {
        let curr = normalized_map.get(rsid).unwrap_or(rsid).to_lowercase();
        if !current_rsids.contains(&curr) {
            current_rsids.push(curr);
        }
    }

    let mut map: HashMap<String, LocalEnrichment> = HashMap::new();
    let placeholders: String = current_rsids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(",");

    let params: Vec<&dyn rusqlite::types::ToSql> = current_rsids
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let mut clinvar_data: HashMap<String, Vec<ClinVarAnnotation>> = HashMap::new();
    // ClinVar is optional until the user downloads it. The report-level
    // catalog_warnings field already explains that state, so do not emit a
    // duplicate stderr warning for an expected missing table.
    if catalog_table_available(conn, "clinvar", "clinvar_reference") {
        let rcv_accession = if table_column_available(
            conn,
            Some("clinvar"),
            "clinvar_reference",
            "rcv_accession",
        ) {
            "rcv_accession"
        } else {
            "'' AS rcv_accession"
        };
        let sql_clinvar = format!(
            "SELECT rsid, allele_id, variation_id, {rcv_accession}, name, gene_symbol,
                    clinical_significance, clin_sig_simple, phenotype_ids,
                    phenotype_list, conditions, review_status, assembly,
                    chromosome, start, stop, last_evaluated, number_submitters,
                    reference_allele_vcf, alternate_allele_vcf
             FROM clinvar.clinvar_reference
             WHERE LOWER(rsid) IN ({})
             ORDER BY LOWER(rsid), variation_id, allele_id, gene_symbol,
                      clinical_significance",
            placeholders
        );
        if let Ok(mut stmt) = conn.prepare(&sql_clinvar)
            && let Ok(rows) = stmt.query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    ClinVarAnnotation {
                        clinical_significance: row.get::<_, String>(6)?,
                        conditions: first_non_empty(
                            row.get::<_, Option<String>>(9)?,
                            row.get::<_, Option<String>>(10)?,
                        ),
                        review_status: non_empty_option(row.get(11)?),
                        rsid: non_empty_option(row.get(0)?),
                        allele_id: non_empty_option(row.get(1)?),
                        variation_id: non_empty_option(row.get(2)?),
                        rcv_accession: non_empty_option(row.get(3)?),
                        gene_symbol: non_empty_option(row.get(5)?),
                        name: non_empty_option(row.get(4)?),
                        phenotype_ids: non_empty_option(row.get(8)?),
                        assembly: non_empty_option(row.get(12)?),
                        chromosome: non_empty_option(row.get(13)?),
                        start: row.get(14)?,
                        stop: row.get(15)?,
                        last_evaluated: non_empty_option(row.get(16)?),
                        number_submitters: row.get(17)?,
                        reference_allele: non_empty_option(row.get(18)?),
                        alternate_allele: non_empty_option(row.get(19)?),
                    },
                ))
            })
        {
            for r in rows.flatten() {
                clinvar_data.entry(r.0.to_lowercase()).or_default().push(r.1);
            }
        }
    }

    // Query GWAS Catalog
    let associations_json = if table_column_available(conn, None, "gwas_reference", "associations_json") {
        "associations_json"
    } else {
        "'[]' AS associations_json"
    };
    let sql_gwas = format!(
        "SELECT rsid, top_trait, best_pvalue, association_count, {associations_json}
         FROM gwas_reference WHERE rsid IN ({})",
        placeholders
    );
    let mut gwas_data = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql_gwas) {
        if let Ok(rows) = stmt.query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        }) {
            for r in rows.flatten() {
                gwas_data.insert(r.0.to_lowercase(), r);
            }
        }
    }

    // 3. Map back to original rsIDs
    for rsid in rsids {
        let rsid_lower = rsid.to_lowercase();
        let curr = normalized_map.get(rsid).unwrap_or(rsid).to_lowercase();

        let mut entry = LocalEnrichment::default();
        let mut has_data = false;

        if let Some(c_rows) = clinvar_data.get(&curr) {
            entry.clinvar_annotations = c_rows.clone();
            if let Some(c_row) = c_rows.first() {
                entry.clinvar_significance = Some(c_row.clinical_significance.clone());
                entry.clinvar_conditions = c_row.conditions.clone();
                entry.clinvar_review_status = c_row.review_status.clone();
            }
            has_data = true;
        }

        if let Some(g_row) = gwas_data.get(&curr) {
            entry.gwas_top_trait = g_row.1.clone();
            entry.gwas_best_pvalue = g_row.2;
            entry.gwas_association_count = g_row.3;
            entry.gwas_associations = g_row
                .4
                .as_deref()
                .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(json).ok())
                .unwrap_or_default()
                .into_iter()
                .map(|mut association| {
                    if let Some(fields) = association.as_object_mut() {
                        fields.entry("association_is").or_insert_with(|| {
                            serde_json::json!("variant_trait_statistical_association")
                        });
                    }
                    association
                })
                .collect();
            has_data = true;
        }

        if has_data {
            map.insert(rsid_lower, entry);
        }
    }

    map
}

/// Build enriched sources from a LocalEnrichment record for a specific rsID.
fn gnomad_source(rsid: &str, af: f64, record: Option<&RawDbsnpGnomad>) -> EnrichedSource {
    let citation = match record {
        Some(record) if !record.dataset.is_empty() => format!(
            "gnomAD {} AF: {:.4} ({})",
            record.dataset,
            af,
            classify_population_rarity(af)
        ),
        _ => format!("gnomAD AF: {:.4} ({})", af, classify_population_rarity(af)),
    };
    let details = record.map(|record| {
        let mut details = format!(
            "Release {}; source mode {}",
            record.release, record.source_mode
        );
        if let Some(popmax) = record.popmax {
            details.push_str(&format!("; popmax {:.4}", popmax));
            if let Some(population) = &record.popmax_population {
                details.push_str(&format!(" ({population})"));
            }
        }
        details
    });
    EnrichedSource {
        source_type: "gnomAD".to_string(),
        citation,
        details,
        url: Some(gnomad_url(rsid)),
    }
}

fn build_enriched_sources(
    rsid: &str,
    enr: &LocalEnrichment,
    gnomad: Option<&RawDbsnpGnomad>,
) -> Vec<EnrichedSource> {
    let mut sources = Vec::new();

    // ClinVar
    for annotation in &enr.clinvar_annotations {
        let review = annotation
            .review_status
            .as_deref()
            .unwrap_or("review status unavailable");
        let condition = annotation.conditions.as_deref().unwrap_or("");
        let identifier = annotation.variation_id.as_deref().unwrap_or(rsid);
        sources.push(EnrichedSource {
            source_type: "ClinVar".to_string(),
            citation: format!("ClinVar: {} ({})", annotation.clinical_significance, review),
            details: if condition.is_empty() {
                annotation.name.clone()
            } else {
                Some(condition.to_string())
            },
            url: Some(format!(
                "https://www.ncbi.nlm.nih.gov/clinvar/?term={}%5BVariant+ID%5D",
                identifier
            )),
        });
    }

    // GWAS: retain per-study provenance when available, with a legacy fallback.
    if !enr.gwas_associations.is_empty() {
        for association in &enr.gwas_associations {
            // Internal discovery seeds share this table but carry a placeholder
            // p-value; they are not published GWAS evidence for report sources.
            if association["source"].as_str() == Some("discovery_catalog_fallback") {
                continue;
            }
            let trait_name = association["trait_name"]
                .as_str()
                .or_else(|| association["trait"]["trait"].as_str())
                .filter(|value| !value.trim().is_empty());
            let Some(trait_name) = trait_name else { continue };
            let pval_str = association["pvalue"]
                .as_f64()
                .map(|value| format!("{value:.2e}"))
                .unwrap_or_else(|| "not recorded".to_string());
            let accession = association["study_accession"].as_str();
            let mapped_gene = association["mapped_gene"].as_str();
            let effect_allele = association["effect_allele"].as_str();
            let mut details = Vec::new();
            if let Some(accession) = accession {
                details.push(format!("Study {accession}"));
            }
            if let Some(gene) = mapped_gene {
                details.push(format!("mapped gene {gene}"));
            }
            if let Some(allele) = effect_allele {
                details.push(format!("reported effect allele {allele}"));
            }
            sources.push(EnrichedSource {
                source_type: "GWAS".to_string(),
                citation: format!("GWAS Catalog: {trait_name} (p={pval_str})"),
                details: (!details.is_empty()).then(|| details.join(" · ")),
                url: Some(accession
                    .map(|accession| format!("https://www.ebi.ac.uk/gwas/studies/{accession}"))
                    .unwrap_or_else(|| format!("https://www.ebi.ac.uk/gwas/variants/{rsid}"))),
            });
        }
    } else if let Some(ref trait_name) = enr.gwas_top_trait
        && enr.gwas_best_pvalue.is_some_and(|p| p < 1e-5)
    {
        let pval_str = enr
            .gwas_best_pvalue
            .map(|p| format!("{:.2e}", p))
            .unwrap_or_else(|| "N/A".to_string());
        let study_count = enr.gwas_association_count.unwrap_or(0);
        sources.push(EnrichedSource {
            source_type: "GWAS".to_string(),
            citation: format!(
                "GWAS Catalog: {} (p={}, {} studies)",
                trait_name, pval_str, study_count
            ),
            details: None,
            url: Some(format!("https://www.ebi.ac.uk/gwas/variants/{}", rsid)),
        });
    }

    // gnomAD population frequency
    if let Some(af) = enr.population_af {
        sources.push(gnomad_source(rsid, af, gnomad));
    }

    sources
}

// ---------------------------------------------------------------------------
// Report generation
// ---------------------------------------------------------------------------

fn complement(allele: &str) -> String {
    allele
        .chars()
        .map(|c| match c {
            'A' => 'T',
            'T' => 'A',
            'C' => 'G',
            'G' => 'C',
            'a' => 't',
            't' => 'a',
            'c' => 'g',
            'g' => 'c',
            other => other,
        })
        .collect()
}

fn stable_hash(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:08x}", hasher.finish())[0..6].to_string()
}

fn count_effect_alleles(genotype: &str, effect_allele: &str) -> u8 {
    let genotype_clean = genotype.trim().to_uppercase();
    let effect_clean = effect_allele.trim().to_uppercase();

    if effect_clean.is_empty()
        || genotype_clean.is_empty()
        || genotype_clean == "-"
        || genotype_clean == "--"
    {
        return 0;
    }

    // If there's a separator, split and count exact matches
    if genotype_clean.contains('/') || genotype_clean.contains('|') {
        let parts: Vec<&str> = if genotype_clean.contains('/') {
            genotype_clean.split('/').collect()
        } else {
            genotype_clean.split('|').collect()
        };
        let mut count = 0;
        for part in parts {
            if part.trim() == effect_clean {
                count += 1;
            }
        }
        return count;
    }

    // No separator:
    if effect_clean.len() == 1 {
        let effect_char = effect_clean.chars().next().unwrap_or('-');
        let mut count = 0;
        for c in genotype_clean.chars() {
            if c == effect_char {
                count += 1;
            }
        }
        count
    } else {
        // Multi-char allele (indel)
        if genotype_clean == effect_clean {
            1
        } else if genotype_clean == effect_clean.repeat(2) {
            2
        } else if genotype_clean.contains(&effect_clean) {
            1
        } else {
            0
        }
    }
}

fn current_iso_8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let secs = since_the_epoch.as_secs();

    let days = secs / 86400;
    let mut year = 1970;
    let mut days_remaining = days;

    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_remaining < days_in_year {
            break;
        }
        days_remaining -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut month_days = vec![31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_leap {
        month_days[1] = 29;
    }

    let mut month = 1;
    for &d in &month_days {
        if days_remaining < d {
            break;
        }
        days_remaining -= d;
        month += 1;
    }

    let day = days_remaining + 1;
    let seconds_of_day = secs % 86400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

struct RawDbsnpGnomad {
    rsid: String,
    chrom: String,
    pos: i64,
    ref_allele: String,
    alt_allele: String,
    af: Option<f64>,
    release: String,
    source_mode: String,
    dataset: String,
    popmax: Option<f64>,
    popmax_population: Option<String>,
    faf95_popmax: Option<f64>,
    homozygote_count: Option<i64>,
}

fn fetch_gnomad_dbsnp_metadata(
    conn: &Connection,
    rsids: &[String],
    current_release: Option<&str>,
) -> HashMap<String, RawDbsnpGnomad> {
    if rsids.is_empty() {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    // Offline gnomAD VCF hits use `remote_vcf_hit`; older API paths used `found`.
    let sql = "SELECT chrom, pos, ref, alt, af, release, source_mode, dataset,
                      popmax, popmax_population, faf95_popmax, homozygote_count, rsids_json
               FROM reference.gnomad_variant_cache 
               WHERE (?1 IS NULL OR release = ?1)
                 AND lookup_status IN ('found', 'remote_vcf_hit', 'local_vcf_hit', 'graphql_hit', 'cache_hit')";
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map(rusqlite::params![current_release], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<f64>>(4).ok().flatten(),
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<f64>>(8).ok().flatten(),
                row.get::<_, Option<String>>(9).ok().flatten(),
                row.get::<_, Option<f64>>(10).ok().flatten(),
                row.get::<_, Option<i64>>(11).ok().flatten(),
                row.get::<_, Option<String>>(12)?,
            ))
        }) {
            for row_res in rows.flatten() {
                if let (
                    chrom,
                    pos,
                    ref_allele,
                    alt_allele,
                    af,
                    release,
                    source_mode,
                    dataset,
                    popmax,
                    popmax_population,
                    faf95_popmax,
                    homozygote_count,
                    Some(rsids_json),
                ) = row_res
                {
                    if let Ok(rsid_list) = serde_json::from_str::<Vec<String>>(&rsids_json) {
                        for r in rsid_list {
                            let r_lower = r.to_lowercase();
                            map.insert(
                                r_lower,
                                RawDbsnpGnomad {
                                    rsid: r.clone(),
                                    chrom: chrom.clone(),
                                    pos,
                                    ref_allele: ref_allele.clone(),
                                    alt_allele: alt_allele.clone(),
                                    af,
                                    release: release.clone(),
                                    source_mode: source_mode.clone(),
                                    dataset: dataset.clone(),
                                    popmax,
                                    popmax_population: popmax_population.clone(),
                                    faf95_popmax,
                                    homozygote_count,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    let normalized_map = resolve_normalized_rsids(conn, rsids);
    let mut filtered_map = HashMap::new();
    for rsid in rsids {
        let rsid_lower = rsid.to_lowercase();
        let curr = normalized_map.get(rsid).unwrap_or(rsid).to_lowercase();
        if let Some(meta) = map.get(&curr).or_else(|| map.get(&rsid_lower)) {
            filtered_map.insert(
                rsid.clone(),
                RawDbsnpGnomad {
                    rsid: rsid.clone(),
                    chrom: meta.chrom.clone(),
                    pos: meta.pos,
                    ref_allele: meta.ref_allele.clone(),
                    alt_allele: meta.alt_allele.clone(),
                    af: meta.af,
                    release: meta.release.clone(),
                    source_mode: meta.source_mode.clone(),
                    dataset: meta.dataset.clone(),
                    popmax: meta.popmax,
                    popmax_population: meta.popmax_population.clone(),
                    faf95_popmax: meta.faf95_popmax,
                    homozygote_count: meta.homozygote_count,
                },
            );
        }
    }
    filtered_map
}

fn build_clinvar_annotation(enr: &LocalEnrichment) -> Option<ClinVarAnnotation> {
    enr.clinvar_annotations.first().cloned().or_else(|| {
        enr.clinvar_significance
            .as_ref()
            .map(|sig| ClinVarAnnotation {
                clinical_significance: sig.clone(),
                conditions: enr.clinvar_conditions.clone(),
                review_status: enr.clinvar_review_status.clone(),
                rsid: None,
                allele_id: None,
                variation_id: None,
                rcv_accession: None,
                gene_symbol: None,
                name: None,
                phenotype_ids: None,
                assembly: None,
                chromosome: None,
                start: None,
                stop: None,
                last_evaluated: None,
                number_submitters: None,
                reference_allele: None,
                alternate_allele: None,
            })
    })
}

fn build_gwas_hits(enr: &LocalEnrichment) -> Vec<GwasHit> {
    let mut hits = Vec::new();
    if let Some(ref trait_name) = enr.gwas_top_trait {
        if let Some(p) = enr.gwas_best_pvalue {
            if p < 1e-5 {
                let p_val_str = format!("{:.2e}", p);
                let (neg_log10_p, underflow) = if p <= 0.0 {
                    (324.0, Some(true))
                } else {
                    (-p.log10(), None)
                };
                hits.push(GwasHit {
                    trait_name: trait_name.clone(),
                    p_value_string: p_val_str,
                    neg_log10_p,
                    association_count: enr.gwas_association_count,
                    p_value_underflow: underflow,
                });
            }
        }
    }
    hits
}

fn is_palindromic_alleles(alleles: &[String]) -> bool {
    if alleles.len() != 2 {
        return false;
    }
    let a1 = alleles[0].to_uppercase();
    let a2 = alleles[1].to_uppercase();
    (a1 == "A" && a2 == "T")
        || (a1 == "T" && a2 == "A")
        || (a1 == "C" && a2 == "G")
        || (a1 == "G" && a2 == "C")
}

fn is_gene_on_negative_strand(conn: &Connection, gene: &str) -> bool {
    let sql = "SELECT grch38_coordinates FROM mane_transcripts WHERE UPPER(gene_symbol) = UPPER(?) LIMIT 1";
    if let Ok(coords) = conn.query_row(sql, [gene], |r| r.get::<_, String>(0)) {
        return coords.contains("(-)");
    }
    false
}

fn fetch_pharmgkb_enrichment(
    conn: &Connection,
    rsids: &[String],
) -> HashMap<String, Vec<PharmGkbAnnotation>> {
    if rsids.is_empty() {
        return HashMap::new();
    }

    // Resolve normalized/current rsIDs
    let normalized_map = resolve_normalized_rsids(conn, rsids);
    let mut current_rsids = Vec::new();
    for rsid in rsids {
        let curr = normalized_map.get(rsid).unwrap_or(rsid);
        if !current_rsids.contains(curr) {
            current_rsids.push(curr.clone());
        }
    }

    let placeholders = current_rsids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT LOWER(rsid), gene, drug, phenotype, evidence_level
         FROM pharmgkb_clinical_variants
         WHERE LOWER(rsid) IN ({})
         ORDER BY LOWER(rsid), drug, phenotype, evidence_level",
        placeholders
    );
    let params: Vec<&dyn rusqlite::types::ToSql> = current_rsids
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let mut query_map: HashMap<String, Vec<PharmGkbAnnotation>> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let rsid: String = row.get(0).unwrap_or_default();
                let annotation = PharmGkbAnnotation {
                    gene: non_empty_option(row.get(1).unwrap_or(None)),
                    drug: row.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default(),
                    phenotype: row.get::<_, Option<String>>(3).unwrap_or(None).unwrap_or_default(),
                    evidence_level: row.get::<_, Option<String>>(4).unwrap_or(None).unwrap_or_default(),
                };
                let annotations = query_map.entry(rsid.to_lowercase()).or_default();
                if !annotations.iter().any(|existing| {
                    existing.gene == annotation.gene
                        && existing.drug == annotation.drug
                        && existing.phenotype == annotation.phenotype
                        && existing.evidence_level == annotation.evidence_level
                }) {
                    annotations.push(annotation);
                }
            }
        }
    }

    // Map back to original rsIDs
    let mut map = HashMap::new();
    for rsid in rsids {
        let curr = normalized_map.get(rsid).unwrap_or(rsid).to_lowercase();
        if let Some(ann) = query_map.get(&curr) {
            map.insert(rsid.to_lowercase(), ann.clone());
        }
    }
    map
}

fn fetch_clingen_enrichment(
    conn: &Connection,
    genes: &[String],
) -> HashMap<String, Vec<ClinGenAnnotation>> {
    if genes.is_empty() {
        return HashMap::new();
    }
    let placeholders = genes
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT LOWER(gene_symbol), disease_label, classification, hgnc_id, moi, report_url
         FROM clingen_gene_validity
         WHERE LOWER(gene_symbol) IN ({})
         ORDER BY LOWER(gene_symbol), disease_label, classification",
        placeholders
    );
    let normalized_genes: Vec<String> = genes.iter().map(|gene| gene.to_lowercase()).collect();
    let params: Vec<&dyn rusqlite::types::ToSql> = normalized_genes
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let mut map: HashMap<String, Vec<ClinGenAnnotation>> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let gene_symbol: String = row.get(0).unwrap_or_default();
                let disease_label: String = row.get(1).unwrap_or_default();
                let annotation = ClinGenAnnotation {
                    gene_symbol: gene_symbol.clone(),
                    disease_label,
                    classification: row.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default(),
                    hgnc_id: non_empty_option(row.get(3).unwrap_or(None)),
                    mode_of_inheritance: non_empty_option(row.get(4).unwrap_or(None)),
                    report_url: non_empty_option(row.get(5).unwrap_or(None)),
                };
                let annotations = map.entry(gene_symbol).or_default();
                if !annotations.iter().any(|existing| {
                    existing.disease_label == annotation.disease_label
                        && existing.classification == annotation.classification
                }) {
                    annotations.push(annotation);
                }
            }
        }
    }
    map
}

fn fetch_mane_enrichment(conn: &Connection, genes: &[String]) -> HashMap<String, Vec<ManeAnnotation>> {
    if genes.is_empty() {
        return HashMap::new();
    }
    let placeholders = genes
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT LOWER(gene_symbol), ensembl_transcript, refseq_transcript, mane_status,
                grch38_coordinates
         FROM mane_transcripts
         WHERE LOWER(gene_symbol) IN ({})
         ORDER BY LOWER(gene_symbol)",
        placeholders
    );
    let normalized_genes: Vec<String> = genes.iter().map(|gene| gene.to_lowercase()).collect();
    let params: Vec<&dyn rusqlite::types::ToSql> = normalized_genes
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let mut map: HashMap<String, Vec<ManeAnnotation>> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let gene_symbol: String = row.get(0).unwrap_or_default();
                let annotation = ManeAnnotation {
                    gene_symbol: gene_symbol.clone(),
                    ensembl_transcript: row.get::<_, Option<String>>(1).unwrap_or(None).unwrap_or_default(),
                    refseq_transcript: row.get::<_, Option<String>>(2).unwrap_or(None).unwrap_or_default(),
                    mane_status: row.get::<_, Option<String>>(3).unwrap_or(None).unwrap_or_default(),
                    grch38_coordinates: non_empty_option(row.get(4).unwrap_or(None)),
                };
                map.entry(gene_symbol).or_default().push(annotation);
            }
        }
    }
    map
}

const MAX_GENOMEWIDE_CLINVAR_ASSOCIATIONS: usize = 500;

fn clinvar_submission_kind(classification: &str) -> Option<&'static str> {
    let value = classification.trim().to_ascii_lowercase();
    if value.is_empty()
        || value.contains("conflict")
        || value.contains("uncertain")
        || value.contains("benign")
        || value.contains("not pathogenic")
    {
        return None;
    }
    if value.contains("pathogenic") {
        Some("variant_condition_summary")
    } else if value.contains("risk allele") || value.contains("risk factor") {
        Some("variant_risk_factor_summary")
    } else {
        None
    }
}

fn orient_dna_base(value: &str, reverse_strand: bool) -> Option<char> {
    let normalized = value.trim().to_ascii_uppercase();
    let mut bases = normalized.chars();
    let base = bases.next()?;
    if bases.next().is_some() || !matches!(base, 'A' | 'C' | 'G' | 'T') {
        return None;
    }
    if !reverse_strand {
        return Some(base);
    }
    match base {
        'A' => Some('T'),
        'C' => Some('G'),
        'G' => Some('C'),
        'T' => Some('A'),
        _ => None,
    }
}

fn review_status_has_criteria(value: &str) -> bool {
    let status = value.to_ascii_lowercase();
    !status.trim().is_empty()
        && !status.contains("no assertion criteria")
        && !status.contains("no classification provided")
}

fn clinvar_field_is_present(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "-" | "na" | "n/a" | "not provided" | "not specified"
        )
}

fn review_status_rank(value: &str) -> u8 {
    let status = value.to_ascii_lowercase();
    if status.contains("practice guideline") {
        5
    } else if status.contains("expert panel") {
        4
    } else if status.contains("multiple submitters") && !status.contains("conflict") {
        3
    } else if status.contains("single submitter") {
        2
    } else {
        1
    }
}

/// Cross-match every imported genotype against locally indexed, condition-specific
/// ClinVar submissions. Exact allele matches are evaluated in-process; genotype
/// bases are not included in the report payload.
fn fetch_genomewide_clinvar_discovery(
    conn: &Connection,
    sample_id: i64,
    provenance: Option<&crate::parser::ImportProvenance>,
) -> Result<GenomeWideClinVarDiscovery, String> {
    let genotypes_scanned = conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes WHERE sample_id = ?",
            [sample_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    let orientation = provenance
        .map(|item| item.diagnostics.allele_orientation.to_ascii_lowercase())
        .unwrap_or_default();
    let reverse_strand = orientation.contains("reverse") || orientation.contains("minus strand");
    let forward_strand = orientation.contains("forward") || orientation.contains("plus strand");
    let orientation_label = if reverse_strand {
        "Reverse strand converted to reference orientation"
    } else if forward_strand {
        "Forward strand"
    } else {
        "Unverified; exact allele matching unavailable"
    };

    let mut discovery = GenomeWideClinVarDiscovery {
        genotypes_scanned,
        allele_orientation: orientation_label.to_string(),
        ..GenomeWideClinVarDiscovery::default()
    };
    if !catalog_table_available(conn, "clinvar", "clinvar_reference")
        || !catalog_table_available(conn, "clinvar_submissions", "clinvar_submissions")
    {
        return Ok(discovery);
    }

    discovery.local_index_ready = true;
    discovery.source_asset_ids = vec![
        "clinvar_variant_summary".to_string(),
        "clinvar_submission_summary".to_string(),
    ];
    if !forward_strand && !reverse_strand {
        return Ok(discovery);
    }

    let source_build = provenance
        .map(|item| item.diagnostics.source_build.to_ascii_uppercase())
        .unwrap_or_default();
    let assembly = if source_build.contains("38") {
        "GRCh38"
    } else if source_build.contains("37") || source_build.contains("19") {
        "GRCh37"
    } else {
        discovery.allele_orientation =
            "Unverified assembly; exact ClinVar matching unavailable".to_string();
        return Ok(discovery);
    };

    let mut statement = conn
        .prepare(
            "SELECT g.rsid, g.allele1, g.allele2,
                    c.variation_id, c.gene_symbol, c.reference_allele_vcf,
                    c.alternate_allele_vcf, c.clinical_significance,
                    s.scv, s.clinical_significance, s.reported_phenotype_info,
                    s.submitted_phenotype_info, s.review_status, s.date_last_evaluated,
                    s.origin_counts, s.somatic_clinical_impact, s.oncogenicity
             FROM genotypes AS g
             JOIN clinvar.clinvar_reference AS c ON c.rsid = g.rsid
             JOIN clinvar_submissions.clinvar_submissions AS s
               ON s.variation_id = c.variation_id
             WHERE g.sample_id = ?1 AND c.assembly = ?2
             ORDER BY g.rsid, s.scv",
        )
        .map_err(|error| format!("Prepare local ClinVar full-genome scan: {error}"))?;
    let mut rows = statement
        .query(rusqlite::params![sample_id, assembly])
        .map_err(|error| format!("Run local ClinVar full-genome scan: {error}"))?;
    let mut seen = HashSet::new();
    let mut matched_variants = HashSet::new();
    let mut matches = Vec::new();

    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let rsid: String = row.get(0).unwrap_or_default();
        let allele1: String = row.get(1).unwrap_or_default();
        let allele2: String = row.get(2).unwrap_or_default();
        let variation_id: String = row.get(3).unwrap_or_default();
        let gene: String = row.get(4).unwrap_or_default();
        let reference: String = row.get(5).unwrap_or_default();
        let alternate: String = row.get(6).unwrap_or_default();
        let aggregate_classification: String = row.get(7).unwrap_or_default();
        let scv: String = row.get(8).unwrap_or_default();
        let clinical_significance: String = row.get(9).unwrap_or_default();
        let reported_phenotype: String = row.get(10).unwrap_or_default();
        let submitted_phenotype: String = row.get(11).unwrap_or_default();
        let review_status: String = row.get(12).unwrap_or_default();
        let last_evaluated: String = row.get(13).unwrap_or_default();
        let origin_counts: String = row.get(14).unwrap_or_default();
        let somatic_clinical_impact: String = row.get(15).unwrap_or_default();
        let oncogenicity: String = row.get(16).unwrap_or_default();

        let Some(association_is) = clinvar_submission_kind(&clinical_significance) else {
            continue;
        };
        if !review_status_has_criteria(&review_status) {
            continue;
        }
        if origin_counts.to_ascii_lowercase().contains("somatic")
            || clinvar_field_is_present(&somatic_clinical_impact)
            || clinvar_field_is_present(&oncogenicity)
        {
            continue;
        }

        // ClinVar's variant summary alleles are reference-oriented already; only
        // the imported sample calls need conversion when the source is reverse-strand.
        let Some(reference_base) = orient_dna_base(&reference, false) else {
            continue;
        };
        let Some(alternate_base) = orient_dna_base(&alternate, false) else {
            continue;
        };
        let Some(allele1) = orient_dna_base(&allele1, reverse_strand) else {
            continue;
        };
        let Some(allele2) = orient_dna_base(&allele2, reverse_strand) else {
            continue;
        };
        if allele1 != reference_base
            && allele1 != alternate_base
            || allele2 != reference_base && allele2 != alternate_base
            || (allele1 != alternate_base && allele2 != alternate_base)
        {
            continue;
        }

        let condition = if clinvar_field_is_present(&submitted_phenotype) {
            submitted_phenotype
        } else {
            reported_phenotype
        };
        if !clinvar_field_is_present(&condition) || variation_id.is_empty() || scv.is_empty() {
            continue;
        }
        let unique_key = format!("{rsid}|{variation_id}|{scv}|{condition}");
        if !seen.insert(unique_key) {
            continue;
        }
        matched_variants.insert(rsid.clone());
        discovery.association_count += 1;
        matches.push(GenomeWideClinVarAssociation {
            association_is: association_is.to_string(),
            association_scope: "variant".to_string(),
            condition: condition.trim().to_string(),
            rsid: rsid.clone(),
            gene_symbol: (!gene.trim().is_empty()).then_some(gene),
            variation_id: variation_id.clone(),
            scv_accession: scv.clone(),
            clinical_significance,
            variant_summary_clinical_significance: aggregate_classification.clone(),
            review_status,
            last_evaluated: (!last_evaluated.trim().is_empty()).then_some(last_evaluated),
            allele_match: "matched".to_string(),
            origin_status: if origin_counts.to_ascii_lowercase().contains("germline") {
                "Germline observation reported".to_string()
            } else {
                "Origin not specified".to_string()
            },
            variant_summary_conflict: aggregate_classification
                .to_ascii_lowercase()
                .contains("conflict"),
            source_url: format!(
                "https://www.ncbi.nlm.nih.gov/clinvar/?term={scv}"
            ),
        });
    }

    matches.sort_by(|left, right| {
        review_status_rank(&right.review_status)
            .cmp(&review_status_rank(&left.review_status))
            .then_with(|| left.condition.cmp(&right.condition))
            .then_with(|| left.rsid.cmp(&right.rsid))
            .then_with(|| left.scv_accession.cmp(&right.scv_accession))
    });
    discovery.exact_variant_count = matched_variants.len() as u64;
    discovery.omitted_association_count = matches
        .len()
        .saturating_sub(MAX_GENOMEWIDE_CLINVAR_ASSOCIATIONS)
        as u64;
    matches.truncate(MAX_GENOMEWIDE_CLINVAR_ASSOCIATIONS);
    discovery.associations = matches;
    Ok(discovery)
}

/// Evaluates a template against a user's database records.
pub fn generate_report(
    conn: &Connection,
    sample_id: i64,
    template: &ReportTemplate,
) -> Result<GeneratedReport, String> {
    let import_provenance = conn
        .query_row(
            "SELECT import_id, source_file_name, source_file_sha256, source_format,
                    source_vendor, delimiter, source_build, coordinate_system, allele_orientation,
                    total_rows, accepted_rows, malformed_rows, duplicate_rows,
                    liftover_mapped_rows, liftover_unmapped_rows
             FROM import_provenance ORDER BY imported_at DESC LIMIT 1",
            [],
            |row| {
                Ok(crate::parser::ImportProvenance {
                    import_id: row.get(0)?,
                    source_file_name: row.get(1)?,
                    source_file_sha256: row.get(2)?,
                    diagnostics: crate::parser::ParseDiagnostics {
                        format: row.get(3)?,
                        vendor: row.get(4)?,
                        delimiter: row.get(5)?,
                        source_build: row.get(6)?,
                        coordinate_system: row.get(7)?,
                        allele_orientation: row.get(8)?,
                        total_rows: row.get::<_, i64>(9)?.max(0) as usize,
                        accepted_rows: row.get::<_, i64>(10)?.max(0) as usize,
                        malformed_rows: row.get::<_, i64>(11)?.max(0) as usize,
                        duplicate_rows: row.get::<_, i64>(12)?.max(0) as usize,
                        warnings: Vec::new(),
                    },
                    liftover_mapped_rows: row.get::<_, i64>(13)?.max(0) as usize,
                    liftover_unmapped_rows: row.get::<_, i64>(14)?.max(0) as usize,
                })
            },
        )
        .ok();
    let import_source_build = import_provenance
        .as_ref()
        .map(|provenance| provenance.diagnostics.source_build.clone());
    let mut catalog_warnings = Vec::new();
    let clinvar_attached = schema_attached(conn, "clinvar");
    let dbsnp_attached = schema_attached(conn, "dbsnp");
    if !clinvar_attached {
        catalog_warnings.push(
            "ClinVar catalog is not attached (App/Data/clinvar.db missing or empty). Clinical significance enrichment is skipped — download/import ClinVar, then Re-sync."
                .to_string(),
        );
    } else {
        let clinvar_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clinvar.clinvar_reference",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if clinvar_rows == 0 {
            catalog_warnings.push(
                "ClinVar database is attached but has 0 indexed rows. Raw download may exist — use Re-sync to import."
                    .to_string(),
            );
        }
    }
    if !dbsnp_attached {
        catalog_warnings.push(
            "dbSNP merge map is not attached (App/Data/dbsnp.db missing). rsID merge remapping is skipped — download/import dbSNP References."
                .to_string(),
        );
    } else {
        let dbsnp_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM dbsnp.rsid_aliases", [], |r| r.get(0))
            .unwrap_or(0);
        if dbsnp_rows == 0 {
            catalog_warnings.push(
                "dbSNP database is attached but has 0 merge mappings. Use Re-sync on dbSNP References."
                    .to_string(),
            );
        }
    }

    // 1. Collect all rsIDs and genes to query in a single batch
    let mut rsids = Vec::new();
    let mut genes = Vec::new();
    for sec in &template.sections {
        for m in &sec.markers {
            rsids.push(m.rsid.clone());
            genes.push(m.gene.clone());
        }
    }

    // Resolve normalized/current rsIDs
    let normalized_map = resolve_normalized_rsids(conn, &rsids);

    // Construct query rsIDs: template rsIDs + their normalized versions
    let mut query_rsids = rsids.clone();
    for curr in normalized_map.values() {
        if !query_rsids.contains(curr) {
            query_rsids.push(curr.clone());
        }
    }

    // 2. Query user genotypes from database
    let user_variants = crate::db::query_by_rsids(conn, sample_id, &query_rsids)
        .map_err(|e| format!("Database query error: {}", e))?;

    let mut genotype_map = HashMap::new();
    for v in user_variants {
        let clean_allele1 = if v.allele1.is_empty() {
            "-".to_string()
        } else {
            v.allele1
        };
        let clean_allele2 = if v.allele2.is_empty() {
            "-".to_string()
        } else {
            v.allele2
        };
        let genotype = format!("{}{}", clean_allele1, clean_allele2);

        // Put under genotype's own rsid
        genotype_map.insert(v.rsid.clone(), genotype.clone());

        // Bidirectionally map so the evaluator can find it under either original or normalized rsid
        for (orig, curr) in &normalized_map {
            if orig.eq_ignore_ascii_case(&v.rsid) {
                genotype_map.insert(curr.clone(), genotype.clone());
            }
            if curr.eq_ignore_ascii_case(&v.rsid) {
                genotype_map.insert(orig.clone(), genotype.clone());
            }
        }
    }

    // Coordinate fallback: check clinvar_reference to map rsID to chromosome/position,
    // then query by coordinates if the rsID itself was missing in the raw genotypes.
    let mut missing_rsids = Vec::new();
    for rsid in &query_rsids {
        if !genotype_map.contains_key(rsid) {
            missing_rsids.push(rsid.clone());
        }
    }

    if !missing_rsids.is_empty() {
        let placeholders = missing_rsids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT LOWER(rsid), chromosome, start, assembly 
             FROM clinvar_reference 
             WHERE rsid IN ({}) AND start IS NOT NULL AND chromosome != ''",
            placeholders
        );
        let params: Vec<&dyn rusqlite::types::ToSql> = missing_rsids
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok(mut rows) = stmt.query(params.as_slice()) {
                while let Ok(Some(row)) = rows.next() {
                    let rsid: String = row.get(0).unwrap_or_default();
                    let chrom: String = row.get(1).unwrap_or_default();
                    let pos: i64 = row.get(2).unwrap_or(0);
                    let assembly: String = row.get(3).unwrap_or_default();

                    let chrom_clean = chrom.trim_start_matches("chr").to_uppercase();

                    let q_sql = if assembly.contains("38") {
                        "SELECT allele1, allele2 FROM genotypes 
                         WHERE sample_id = ? AND chromosome = ? AND position_grch38 = ? 
                         LIMIT 1"
                    } else {
                        "SELECT allele1, allele2 FROM genotypes 
                         WHERE sample_id = ? AND chromosome = ? AND position_grch37 = ? 
                         LIMIT 1"
                    };

                    if let Ok((a1, a2)) = conn.query_row(
                        q_sql,
                        rusqlite::params![
                            &sample_id as &dyn rusqlite::types::ToSql,
                            &chrom_clean as &dyn rusqlite::types::ToSql,
                            &pos as &dyn rusqlite::types::ToSql
                        ],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                    ) {
                        let clean_allele1 = if a1.is_empty() { "-".to_string() } else { a1 };
                        let clean_allele2 = if a2.is_empty() { "-".to_string() } else { a2 };
                        let genotype = format!("{}{}", clean_allele1, clean_allele2);

                        genotype_map.insert(rsid.clone(), genotype.clone());
                        for (orig, curr) in &normalized_map {
                            if orig.to_lowercase() == rsid {
                                genotype_map.insert(orig.clone(), genotype.clone());
                                genotype_map.insert(curr.clone(), genotype.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Batch-fetch local reference enrichment and dbSNP metadata
    let enrichment_map = fetch_local_enrichment(conn, &rsids);
    let current_gnomad_release = conn
        .query_row(
            "SELECT release FROM reference.gnomad_config WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    let dbsnp_data = fetch_gnomad_dbsnp_metadata(conn, &rsids, current_gnomad_release.as_deref());
    let pharmgkb_map = fetch_pharmgkb_enrichment(conn, &rsids);
    let clingen_map = fetch_clingen_enrichment(conn, &genes);
    let mane_map = fetch_mane_enrichment(conn, &genes);

    // Initialize mappings
    let mut variants_map: HashMap<String, CanonicalVariant> = HashMap::new();
    let mut user_calls_map: HashMap<String, UserCall> = HashMap::new();
    let mut category_links_map: HashMap<String, VariantCategoryLink> = HashMap::new();
    let mut enrichment_map_out: HashMap<String, VariantEnrichment> = HashMap::new();
    let mut reference_registry = ReportReferenceRegistryBuilder::default();

    let pack_id = template.title.to_lowercase().replace(' ', "_");

    // 4. Evaluate each section with direction-aware scoring
    let mut evaluated_sections = Vec::new();
    let mut total_risk_possible: u16 = 0;
    let mut total_risk_effects: u16 = 0;
    let mut evaluated_rsids = std::collections::HashSet::new();

    for sec in &template.sections {
        let mut link_ids = Vec::new();
        let category_id = sec.name.to_lowercase().replace(' ', "_");

        // Section-level summary accumulators
        let mut risk_effect_count: u16 = 0;
        let mut risk_possible: u16 = 0;
        let mut protective_effect_count: u16 = 0;
        let mut protective_possible: u16 = 0;
        let mut trait_count: u16 = 0;
        let mut context_dependent_count: u16 = 0;
        let mut no_data_count: u16 = 0;
        let mut not_evaluated_count: u16 = 0;
        let mut confirmation_required_count: u16 = 0;
        let mut all_require_confirmation = true;

        let mut active_marker_count: u16 = 0;
        let mut active_risk_marker_count: u16 = 0;
        let mut active_protective_marker_count: u16 = 0;
        let mut active_trait_marker_count: u16 = 0;
        let mut active_context_marker_count: u16 = 0;
        let mut blocked_unverified_count: u16 = 0;
        let mut benign_modifier_count: u16 = 0;

        for m in &sec.markers {
            // Build/insert CanonicalVariant
            variants_map.entry(m.rsid.clone()).or_insert_with(|| {
                let mut chromosome = m.strand.clone();
                let mut position_grch37 = None;
                let mut position_grch38 = None;

                if let Some(db) = dbsnp_data.get(&m.rsid) {
                    chromosome = Some(db.chrom.clone());
                    let is_37 = db.release.to_lowercase().contains("37");
                    if is_37 {
                        position_grch37 = Some(db.pos);
                    } else {
                        position_grch38 = Some(db.pos);
                    }
                }

                CanonicalVariant {
                    rsid: m.rsid.clone(),
                    gene: m.gene.clone(),
                    variant_name: m.variant_name.clone(),
                    chromosome,
                    position_grch37,
                    position_grch38,
                    variant_type: m.variant_type.clone(),
                }
            });

            // Build/insert UserCall
            let raw_call = genotype_map
                .get(&m.rsid)
                .cloned()
                .unwrap_or_else(|| "--".to_string());
            let is_missing = raw_call == "--"
                || raw_call.contains('-')
                || raw_call.contains('0')
                || raw_call.contains('?')
                || raw_call.contains('N');
            let has_known_base = raw_call
                .chars()
                .any(|base| matches!(base, 'A' | 'C' | 'G' | 'T'));
            let has_ambiguous_base = raw_call.contains('?')
                || (raw_call.contains('N') && has_known_base);
            let call_status = if is_missing {
                if has_ambiguous_base {
                    CallStatus::AmbiguousRawCall
                } else if genotype_map.contains_key(&m.rsid) {
                    CallStatus::NoData
                } else {
                    CallStatus::NotInRawFile
                }
            } else {
                CallStatus::Found
            };

            user_calls_map
                .entry(m.rsid.clone())
                .or_insert_with(|| UserCall {
                    user_genotype: raw_call.clone(),
                    normalized_genotype: None,
                    call_status,
                    source_build: import_source_build.clone().or_else(|| m.source_build.clone()),
                });

            // Evaluate Assertion details
            let dbsnp_rec = dbsnp_data.get(&m.rsid);
            let dbsnp_alleles =
                dbsnp_rec.map(|db| vec![db.ref_allele.clone(), db.alt_allele.clone()]);

            let is_palindromic = m
                .expected_plus_alleles
                .as_ref()
                .map(|v| is_palindromic_alleles(v))
                .unwrap_or(false)
                || dbsnp_alleles
                    .as_ref()
                    .map(|v| is_palindromic_alleles(v))
                    .unwrap_or(false);

            let requires_orientation_verification =
                m.interpretation_blocked_if_unverified.unwrap_or(false) || is_palindromic;
            let expected_plus_alleles = m
                .expected_plus_alleles
                .clone()
                .or_else(|| dbsnp_rec.map(|db| vec![db.ref_allele.clone(), db.alt_allele.clone()]));
            let marker_orientation_evidence = m.allele_orientation_verified == Some(true)
                && m.orientation_source.as_deref().is_some_and(|source| !source.trim().is_empty());
            let imported_file_orientation_evidence = import_provenance
                .as_ref()
                .map(|provenance| !provenance.diagnostics.allele_orientation.starts_with("Unknown"))
                .unwrap_or(false);
            // A dbSNP/gnomAD row describes the reference allele set; it does
            // not prove that this consumer file used the same strand. For a
            // current import, both the marker resource and source-file
            // orientation evidence are required before complementing a call.
            let orientation_can_be_normalized = marker_orientation_evidence
                && (imported_file_orientation_evidence
                    || (import_provenance.is_none() && dbsnp_data.contains_key(&m.rsid)));

            let assertion_status;
            let mut interpretation_allowed = true;
            let mut effect_count: Option<u8> = None;
            let mut interpretation = m.interpretation.clone();
            let mut impact = m.impact.clone();
            let mut orientation_mismatch_detected = false;
            let severity_class;

            if is_missing {
                assertion_status = if call_status == CallStatus::NotInRawFile {
                    AssertionStatus::NotInRawFile
                } else if call_status == CallStatus::AmbiguousRawCall {
                    AssertionStatus::BlockedRawCall
                } else {
                    AssertionStatus::NoData
                };
                interpretation_allowed = false;
                severity_class = "no_data".to_string();
                if call_status == CallStatus::AmbiguousRawCall {
                    interpretation = "⚠️ Interpretation blocked: the raw export contains an ambiguous call at this marker. Confirm the source file or use a validated clinical assay.".to_string();
                }
                no_data_count += 1;
            } else if !is_matchable_snp_assertion(m.variant_type.as_deref(), &m.effect_allele) {
                assertion_status = AssertionStatus::NotEvaluated;
                interpretation_allowed = false;
                severity_class = "not_evaluated".to_string();
                interpretation = "Interpretation not evaluated: this assertion does not define a matchable raw-DNA allele. No genotype-based conclusion was produced.".to_string();
                not_evaluated_count += 1;
            } else {
                active_marker_count += 1;
                let mut current_genotype = raw_call.clone();
                let mut orientation_warning = false;

                // Check expected plus alleles
                if let Some(ref expected) = expected_plus_alleles {
                    let mut mismatch = false;
                    for c in current_genotype.chars() {
                        if !expected.contains(&c.to_string()) {
                            mismatch = true;
                        }
                    }

                    if mismatch {
                        let comp_genotype = complement(&current_genotype);
                        let mut comp_mismatch = false;
                        for c in comp_genotype.chars() {
                            if !expected.contains(&c.to_string()) {
                                comp_mismatch = true;
                            }
                        }

                        if !comp_mismatch {
                            if orientation_can_be_normalized {
                                current_genotype = comp_genotype;
                                if let Some(uc) = user_calls_map.get_mut(&m.rsid) {
                                    uc.normalized_genotype = Some(current_genotype.clone());
                                }
                            } else {
                                orientation_warning = true;
                            }
                        } else {
                            orientation_warning = true;
                        }
                    }
                }

                // Check orientation validation
                let direct_expected_match = expected_plus_alleles.as_ref().is_some_and(|expected| {
                    current_genotype
                        .chars()
                        .all(|allele| expected.contains(&allele.to_string()))
                });
                let has_metadata = direct_expected_match || orientation_can_be_normalized;
                let is_unverified = !has_metadata || orientation_warning;
                orientation_mismatch_detected = orientation_warning;

                if requires_orientation_verification && is_unverified {
                    assertion_status = AssertionStatus::UnverifiedOrientation;
                    interpretation_allowed = false;
                    interpretation = "⚠️ Clinical interpretation blocked: Allele orientation has not been verified for this chip build/strand configuration. Confirm genotype with clinical assay.".to_string();
                    impact = "Interpretation Blocked (Unverified Strand)".to_string();
                    severity_class = "confirmation_required".to_string();
                    blocked_unverified_count += 1;
                } else if orientation_warning {
                    assertion_status = AssertionStatus::OrientationMismatch;
                    interpretation_allowed = false;
                    interpretation = format!(
                        "⚠️ WARNING: Orientation mismatch detected. {}",
                        interpretation
                    );
                    severity_class = "confirmation_required".to_string();
                    blocked_unverified_count += 1;
                } else {
                    assertion_status = AssertionStatus::Verified;
                    let is_neg_strand = is_gene_on_negative_strand(conn, &m.gene);
                    let eff_allele = if is_neg_strand {
                        complement(&m.effect_allele)
                    } else {
                        m.effect_allele.clone()
                    };
                    let count = count_effect_alleles(&current_genotype, &eff_allele);
                    effect_count = Some(count);
                    severity_class = compute_severity_class(
                        count,
                        &m.effect_direction,
                        &m.evidence_tier,
                        m.clinical_confirmation_required,
                        false,
                    );

                    if count == 0 {
                        benign_modifier_count += 1;
                    } else {
                        match m.effect_direction {
                            EffectDirection::Risk => {
                                risk_possible += 2;
                                risk_effect_count += count as u16;
                                if evaluated_rsids.insert(m.rsid.clone()) {
                                    total_risk_possible += 2;
                                    total_risk_effects += count as u16;
                                }
                                active_risk_marker_count += 1;
                            }
                            EffectDirection::Protective => {
                                protective_possible += 2;
                                protective_effect_count += count as u16;
                                active_protective_marker_count += 1;
                            }
                            EffectDirection::Trait
                            | EffectDirection::NotApplicable
                            | EffectDirection::NoClaim => {
                                trait_count += 1;
                                active_trait_marker_count += 1;
                            }
                            EffectDirection::ContextDependent | EffectDirection::Unknown => {
                                context_dependent_count += 1;
                                active_context_marker_count += 1;
                            }
                        }
                    }
                }
            }

            // Track confirmation requirements
            let requires_confirmation = m.clinical_confirmation_required == Some(true);
            if requires_confirmation {
                if !is_missing && assertion_status != AssertionStatus::NotEvaluated {
                    confirmation_required_count += 1;
                }
            } else {
                all_require_confirmation = false;
            }

            let sources = m.sources.clone().unwrap_or_default();
            let marker_reference_ids = reference_registry.register_marker_sources(&sources);

            // Populate Enrichment
            let rsid_lower = m.rsid.to_lowercase();
            let gene_lower = m.gene.to_lowercase();
            let clinvar = enrichment_map
                .get(&rsid_lower)
                .and_then(build_clinvar_annotation);
            let gwas_hits = enrichment_map
                .get(&rsid_lower)
                .map(build_gwas_hits)
                .unwrap_or_default();

            let dbsnp = dbsnp_data.get(&m.rsid).map(|db| {
                let is_37 = db.release.to_lowercase().contains("37");
                DbsnpAnnotation {
                    rsid: db.rsid.clone(),
                    chromosome: db.chrom.clone(),
                    position_grch37: if is_37 { Some(db.pos) } else { None },
                    position_grch38: if !is_37 { Some(db.pos) } else { None },
                    ref_allele: db.ref_allele.clone(),
                    alt_alleles: vec![db.alt_allele.clone()],
                    plus_strand_alleles: Some(vec![db.ref_allele.clone(), db.alt_allele.clone()]),
                    source_build: Some(format!("gnomAD {}", db.release)),
                    allele_source: "gnomAD cache (not offline dbSNP alleles)".to_string(),
                }
            });

            let population = dbsnp_data.get(&m.rsid).and_then(|db| {
                db.af.map(|af| PopulationAnnotation {
                    allele_frequency: af,
                    rarity_bucket: classify_population_rarity(af).to_string(),
                    source_mode: Some(db.source_mode.clone()),
                    dataset: Some(db.dataset.clone()),
                    source_build: Some(db.release.clone()),
                    popmax: db.popmax,
                    popmax_population: db.popmax_population.clone(),
                    faf95_popmax: db.faf95_popmax,
                    homozygote_count: db.homozygote_count,
                })
            });

            let pharmgkb_annotations = pharmgkb_map
                .get(&rsid_lower)
                .cloned()
                .unwrap_or_default();
            let pharmgkb = pharmgkb_annotations.first().cloned();
            let clingen_annotations = clingen_map
                .get(&gene_lower)
                .cloned()
                .unwrap_or_default();
            let clingen = clingen_annotations.first().cloned();
            let mane_annotations = mane_map.get(&gene_lower).cloned().unwrap_or_default();
            let mane = mane_annotations.first().cloned();

            let enr_with_af = enrichment_map.get(&rsid_lower).map(|e| LocalEnrichment {
                clinvar_annotations: e.clinvar_annotations.clone(),
                clinvar_significance: e.clinvar_significance.clone(),
                clinvar_conditions: e.clinvar_conditions.clone(),
                clinvar_review_status: e.clinvar_review_status.clone(),
                gwas_top_trait: e.gwas_top_trait.clone(),
                gwas_best_pvalue: e.gwas_best_pvalue,
                gwas_association_count: e.gwas_association_count,
                gwas_associations: e.gwas_associations.clone(),
                population_af: dbsnp_data.get(&m.rsid).and_then(|d| d.af),
            });

            let mut db_enriched_sources = if let Some(ref e) = enr_with_af {
                build_enriched_sources(&m.rsid, e, dbsnp_data.get(&m.rsid))
            } else if let Some(ref pop) = population {
                vec![gnomad_source(
                    &m.rsid,
                    pop.allele_frequency,
                    dbsnp_data.get(&m.rsid),
                )]
            } else {
                Vec::new()
            };

            for pkb in &pharmgkb_annotations {
                let drug = if pkb.drug.is_empty() {
                    "clinical variant annotation"
                } else {
                    &pkb.drug
                };
                db_enriched_sources.push(EnrichedSource {
                    source_type: "ClinPGx".to_string(),
                    citation: format!(
                        "ClinPGx: {} (Level {}, {})",
                        drug,
                        if pkb.evidence_level.is_empty() {
                            "evidence unavailable"
                        } else {
                            &pkb.evidence_level
                        },
                        pkb.gene.as_deref().unwrap_or("gene unavailable")
                    ),
                    details: non_empty_option(Some(pkb.phenotype.clone())),
                    url: Some(format!(
                        "https://api.clinpgx.org/v1/variant?name={}",
                        m.rsid
                    )),
                });
            }

            for cg in &clingen_annotations {
                db_enriched_sources.push(EnrichedSource {
                    source_type: "ClinGen".to_string(),
                    citation: format!(
                        "ClinGen Gene Validity: {} (Gene: {})",
                        cg.classification, cg.gene_symbol
                    ),
                    details: Some(match cg.mode_of_inheritance.as_deref() {
                        Some(moi) => format!("{} · inheritance: {moi}", cg.disease_label),
                        None => cg.disease_label.clone(),
                    }),
                    url: cg.report_url.clone().or_else(|| {
                        Some(format!(
                            "https://search.clinicalgenome.org/kb/genes/{}",
                            cg.gene_symbol
                        ))
                    }),
                });
            }

            for mn in &mane_annotations {
                db_enriched_sources.push(EnrichedSource {
                    source_type: "MANE".to_string(),
                    citation: format!(
                        "MANE Transcript: {} / {}",
                        mn.refseq_transcript, mn.ensembl_transcript
                    ),
                    details: mn.grch38_coordinates.clone().or_else(|| {
                        non_empty_option(Some(format!("Status: {}", mn.mane_status)))
                    }),
                    url: None,
                });
            }

            if clinvar.is_some()
                || !enrichment_map
                    .get(&rsid_lower)
                    .map(|e| e.clinvar_annotations.is_empty())
                    .unwrap_or(true)
                || !gwas_hits.is_empty()
                || enrichment_map
                    .get(&rsid_lower)
                    .is_some_and(|e| !e.gwas_associations.is_empty())
                || dbsnp.is_some()
                || population.is_some()
                || !pharmgkb_annotations.is_empty()
                || !clingen_annotations.is_empty()
                || !mane_annotations.is_empty()
                || !db_enriched_sources.is_empty()
            {
                let enrichment_reference_ids =
                    reference_registry.register_enriched_sources(&db_enriched_sources);
                enrichment_map_out.insert(
                    m.rsid.clone(),
                    VariantEnrichment {
                        clinvar,
                        clinvar_annotations: enrichment_map
                            .get(&rsid_lower)
                            .map(|e| e.clinvar_annotations.clone())
                            .unwrap_or_default(),
                        gwas_hits,
                        gwas_associations: enrichment_map
                            .get(&rsid_lower)
                            .map(|e| e.gwas_associations.clone())
                            .unwrap_or_default(),
                        dbsnp,
                        population,
                        pharmgkb,
                        pharmgkb_annotations,
                        clingen,
                        clingen_annotations,
                        mane,
                        mane_annotations,
                        db_enriched_sources,
                        reference_ids: enrichment_reference_ids,
                    },
                );
            }

            let enrichment_reference_ids = enrichment_map_out
                .get(&m.rsid)
                .map(|enrichment| enrichment.reference_ids.as_slice())
                .unwrap_or_default();
            let mut reference_ids = marker_reference_ids.clone();
            for reference_id in enrichment_reference_ids {
                if !reference_ids.contains(reference_id) {
                    reference_ids.push(reference_id.clone());
                }
            }

            // Construct an explicit assertion identity. rsID remains the lookup
            // key for one raw call, while this additive key separates the
            // biomedical assertion from Simple-mode presentation grouping.
            let assertion_key = serde_json::json!({
                "version": 1,
                "locus": {
                    "rsid": m.rsid,
                    "gene": m.gene,
                    "source_build": m.source_build,
                    "hgvs": m.hgvs,
                },
                "allele_definition": {
                    "variant_type": m.variant_type.as_deref().unwrap_or("snp"),
                    "effect_allele": m.effect_allele,
                    "expected_plus_alleles": expected_plus_alleles.clone(),
                },
                "condition_or_trait": {
                    "category_id": category_id,
                    "label": m
                        .clinical_semantics
                        .as_ref()
                        .and_then(|semantics| semantics.condition_label.clone())
                        .or_else(|| m.variant_name.clone()),
                    "interpretation_class": m
                        .clinical_semantics
                        .as_ref()
                        .and_then(|semantics| semantics.interpretation_class.clone()),
                    "inheritance_model": m
                        .clinical_semantics
                        .as_ref()
                        .and_then(|semantics| semantics.inheritance_model.clone()),
                    "clinical_state": m
                        .clinical_semantics
                        .as_ref()
                        .and_then(|semantics| semantics.clinical_state.clone()),
                },
                "population_context": {
                    "sex_scope": m.sex_scope,
                    "context_tags": m.context_tags,
                },
                "assay_requirement": callability_assay_requirement(m.variant_type.as_deref()),
                "source_assertion": reference_ids.clone(),
            })
            .to_string();

            // Construct a semantic link identity. rsID remains the lookup
            // key for one raw call, while the link identity distinguishes
            // gene, allele, direction, variant type, build, clinical
            // semantics, and source references when a marker is reused.
            let assertion_text = serde_json::json!({
                "rsid": m.rsid,
                "gene": m.gene,
                "variant_name": m.variant_name,
                "variant_type": m.variant_type,
                "effect_allele": m.effect_allele,
                "effect_direction": m.effect_direction,
                "impact": impact,
                "interpretation": interpretation,
                "evidence_tier": m.evidence_tier,
                "source_build": m.source_build,
                "hgvs": m.hgvs,
                "expected_plus_alleles": expected_plus_alleles,
                "clinical_semantics": m.clinical_semantics,
                "reference_ids": reference_ids,
            })
            .to_string();
            let link_id = format!(
                "{}:{}:{}:{}",
                pack_id,
                category_id,
                m.rsid,
                stable_hash(&assertion_text)
            );

            let link = VariantCategoryLink {
                link_id: link_id.clone(),
                assertion_key,
                rsid: m.rsid.clone(),
                gene: m.gene.clone(),
                variant_name: m.variant_name.clone(),
                variant_type: m.variant_type.clone(),
                source_build: m.source_build.clone(),
                hgvs: m.hgvs.clone(),
                orientation_source: m.orientation_source.clone(),
                category_id: category_id.clone(),
                category_label: sec.name.clone(),
                impact,
                evidence_tier: m.evidence_tier.clone(),
                interpretation,
                effect_direction: m.effect_direction.clone(),
                effect_allele: m.effect_allele.clone(),
                expected_plus_alleles: expected_plus_alleles.clone(),
                severity_class,
                effect_count,
                assertion_status,
                callability_state: callability_state_for_result(
                    m.variant_type.as_deref(),
                    assertion_status,
                    is_missing,
                ),
                orientation_state: orientation_state_for_result(
                    assertion_status,
                    requires_orientation_verification,
                    marker_orientation_evidence,
                    orientation_mismatch_detected,
                    is_missing,
                ),
                requires_orientation_verification,
                interpretation_allowed,
                do_not_claim: m.do_not_claim.clone(),
                confirm_with: m.confirm_with.clone(),
                raw_dna_limitation: m.raw_dna_limitation.clone(),
                clinical_confirmation_required: m.clinical_confirmation_required,
                sex_scope: m.sex_scope.clone(),
                context_tags: m.context_tags.clone(),
                interpretation_blocked_if_unverified: m.interpretation_blocked_if_unverified,
                clinical_semantics: m.clinical_semantics.clone(),
                sources,
                reference_ids,
            };

            category_links_map.insert(link_id.clone(), link);
            link_ids.push(link_id);
        }

        let show_percent_score = !all_require_confirmation && risk_possible > 0;
        let section_signal_score = if risk_possible > 0 {
            (risk_effect_count as f64 / risk_possible as f64) * 100.0
        } else {
            0.0
        };

        let summary = SectionSummary {
            risk_effect_count,
            risk_possible,
            protective_effect_count,
            protective_possible,
            trait_count,
            context_dependent_count,
            no_data_count,
            not_evaluated_count,
            confirmation_required_count,
            total_markers: sec.markers.len() as u16,
            show_percent_score,
            all_require_confirmation,
            active_marker_count,
            active_risk_marker_count,
            active_protective_marker_count,
            active_trait_marker_count,
            active_context_marker_count,
            blocked_unverified_count,
            benign_modifier_count,
        };

        evaluated_sections.push(NormalizedSection {
            section_id: category_id,
            name: sec.name.clone(),
            link_ids,
            section_signal_score,
            summary,
        });
    }

    let overall_signal_score = if total_risk_possible > 0 {
        (total_risk_effects as f64 / total_risk_possible as f64) * 100.0
    } else {
        0.0
    };
    let genomewide_clinvar = match fetch_genomewide_clinvar_discovery(
        conn,
        sample_id,
        import_provenance.as_ref(),
    ) {
        Ok(discovery) => Some(discovery),
        Err(error) => {
            eprintln!("Local full-genome ClinVar discovery was unavailable: {error}");
            catalog_warnings.push(
                "Local ClinVar condition discovery could not complete. The curated marker report is still available; check the local ClinVar indexes and re-sync if needed.".to_string(),
            );
            None
        }
    };

    Ok(GeneratedReport {
        schema_version: "2.0.0".to_string(),
        export_format: "normalized_sparse".to_string(),
        generated_at: current_iso_8601(),
        title: template.title.clone(),
        description: template.description.clone(),
        overall_signal_score,
        variants: variants_map,
        user_calls: user_calls_map,
        category_links: category_links_map,
        enrichment: enrichment_map_out,
        sections: evaluated_sections,
        references: reference_registry.references,
        catalog_warnings,
        genomewide_clinvar,
        import_provenance,
    })
}

// ---------------------------------------------------------------------------
// Markdown renderer
// ---------------------------------------------------------------------------

/// Helper to render the generated report as a readable markdown string.
pub fn render_markdown(report: &GeneratedReport) -> String {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", report.title));
    md.push_str(&format!(">{}\n\n", report.description));
    md.push_str(&format!("{}\n\n", EXPORT_DISCLOSURES.privacy_warning));
    md.push_str(&format!(
        "## {}\n\n{}\n\n",
        EXPORT_DISCLOSURES.raw_genotype_section_title,
        EXPORT_DISCLOSURES.raw_genotype_notice
    ));
    if let Some(provenance) = &report.import_provenance {
        md.push_str("## Import provenance\n\n");
        md.push_str(&format!("{}\n\n", EXPORT_DISCLOSURES.import_provenance_notice));
        md.push_str(&format!(
            "- Import ID: `{}`\n- Source file: `{}`\n- Source SHA-256: `{}`\n- Format/vendor: `{}` / `{}` ({})\n- Source build: `{}`\n- Coordinate system: `{}`\n- Allele orientation: `{}`\n- Rows: {} accepted; {} malformed; {} duplicate\n- Liftover: {} mapped; {} unmapped\n\n",
            provenance.import_id,
            provenance.source_file_name,
            provenance.source_file_sha256,
            provenance.diagnostics.format,
            provenance.diagnostics.vendor,
            provenance.diagnostics.delimiter,
            provenance.diagnostics.source_build,
            provenance.diagnostics.coordinate_system,
            provenance.diagnostics.allele_orientation,
            provenance.diagnostics.accepted_rows,
            provenance.diagnostics.malformed_rows,
            provenance.diagnostics.duplicate_rows,
            provenance.liftover_mapped_rows,
            provenance.liftover_unmapped_rows,
        ));
    }
    md.push_str(&format!(
        "**Matched allele load**: {:.1}%\n\n",
        report.overall_signal_score
    ));
    md.push_str("> *Share of association-direction alleles among curated pack markers. Not a disease probability. Protective/trait markers are tallied separately.*\n\n");
    md.push_str("---\n\n");

    for sec in &report.sections {
        md.push_str(&format!("## {}\n", sec.name));

        if sec.summary.show_percent_score {
            md.push_str(&format!(
                "*Matched alleles*: {:.1}%\n\n",
                sec.section_signal_score
            ));
        } else {
            md.push_str("*Score suppressed — clinical confirmation required for all markers in this section.*\n\n");
        }

        let s = &sec.summary;
        md.push_str(&format!(
            "Summary: {} markers | {} association alleles/{} possible | {} protective | {} trait | {} context-dependent | {} no-data | {} not-evaluated | {} confirmation-required\n\n",
            s.total_markers, s.risk_effect_count, s.risk_possible,
            s.protective_effect_count, s.trait_count,
            s.context_dependent_count, s.no_data_count, s.not_evaluated_count,
            s.confirmation_required_count
        ));

        md.push_str("| Marker | Gene | Raw genotype | Normalized genotype | Effect Allele | Severity | Direction | Tier | Interpretation |\n");
        md.push_str("|---|---|---|---|---|---|---|---|---|\n");

        for link_id in &sec.link_ids {
            if let Some(link) = report.category_links.get(link_id) {
                let (raw_genotype, normalized_genotype) = report
                    .user_calls
                    .get(&link.rsid)
                    .map(|uc| (
                        uc.user_genotype.as_str(),
                        uc.normalized_genotype.as_deref().unwrap_or("--"),
                    ))
                    .unwrap_or(("--", "--"));

                let dir_str = match link.effect_direction {
                    EffectDirection::Risk => "Risk",
                    EffectDirection::Protective => "Protective",
                    EffectDirection::ContextDependent => "Context-dependent",
                    EffectDirection::Trait => "Trait",
                    EffectDirection::Unknown => "Unknown",
                    EffectDirection::NotApplicable => "Not applicable",
                    EffectDirection::NoClaim => "No claim",
                };

                let gene = report
                    .variants
                    .get(&link.rsid)
                    .map(|v| v.gene.as_str())
                    .unwrap_or("");

                md.push_str(&format!(
                    "| **{}** | **{}** | `{}` | `{}` | `{}` | {} | {} | `{}` | *{}* |\n",
                    link.rsid,
                    gene,
                    raw_genotype,
                    normalized_genotype,
                    link.effect_allele,
                    link.severity_class,
                    dir_str,
                    link.evidence_tier,
                    link.impact
                ));
            }
        }
        md.push_str("\n---\n\n");
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE samples (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                genetic_sex TEXT DEFAULT 'Unknown',
                imported_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE genotypes (
                sample_id INTEGER,
                rsid TEXT NOT NULL,
                chromosome TEXT NOT NULL,
                position_grch37 INTEGER NOT NULL,
                position_grch38 INTEGER,
                allele1 TEXT NOT NULL,
                allele2 TEXT NOT NULL,
                PRIMARY KEY (sample_id, rsid)
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO samples (id, name) VALUES (1, 'Test Sample')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn genomewide_clinvar_scan_keeps_condition_provenance_and_filters_somatic_rows() {
        let conn = setup_test_db();
        conn.execute("ATTACH DATABASE ':memory:' AS clinvar", [])
            .unwrap();
        conn.execute("ATTACH DATABASE ':memory:' AS clinvar_submissions", [])
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE clinvar.clinvar_reference (
                rsid TEXT, variation_id TEXT, gene_symbol TEXT,
                reference_allele_vcf TEXT, alternate_allele_vcf TEXT,
                clinical_significance TEXT, assembly TEXT
            );
            CREATE TABLE clinvar_submissions.clinvar_submissions (
                scv TEXT, variation_id TEXT, clinical_significance TEXT,
                reported_phenotype_info TEXT, submitted_phenotype_info TEXT,
                review_status TEXT, date_last_evaluated TEXT, origin_counts TEXT,
                somatic_clinical_impact TEXT, oncogenicity TEXT
            );
            INSERT INTO genotypes
                (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
            VALUES (1, 'rsClinvarTest', '1', 101, 201, 'A', 'G'),
                   (1, 'rsSomaticTest', '1', 102, 202, 'T', 'C');
            INSERT INTO clinvar.clinvar_reference VALUES
                ('rsClinvarTest', '1001', 'TEST1', 'A', 'G', 'Pathogenic', 'GRCh37'),
                ('rsSomaticTest', '1002', 'TEST2', 'T', 'C', 'Pathogenic', 'GRCh37');
            INSERT INTO clinvar_submissions VALUES
                ('SCV000000001.1', '1001', 'Pathogenic', 'MedGen:C1 (Synthetic condition)', 'Synthetic condition', 'criteria provided, multiple submitters, no conflicts', '2026-01-01', 'germline:2', 'NA', '-'),
                ('SCV000000002.1', '1002', 'Pathogenic', 'Synthetic cancer', 'Synthetic cancer', 'criteria provided, single submitter', '2026-01-01', 'somatic:1', 'Tier I', 'Oncogenic');",
        )
        .unwrap();

        let provenance = crate::parser::ImportProvenance::new(
            "synthetic-import".into(),
            "synthetic.txt".into(),
            "synthetic-sha".into(),
            crate::parser::ParseDiagnostics {
                format: "23andMe".into(),
                vendor: "synthetic".into(),
                delimiter: "tab".into(),
                source_build: "GRCh37".into(),
                coordinate_system: "1-based-inclusive".into(),
                allele_orientation: "forward-strand (synthetic test)".into(),
                total_rows: 2,
                accepted_rows: 2,
                malformed_rows: 0,
                duplicate_rows: 0,
                warnings: Vec::new(),
            },
        );

        let discovery = fetch_genomewide_clinvar_discovery(&conn, 1, Some(&provenance)).unwrap();
        assert!(discovery.local_index_ready);
        assert_eq!(discovery.genotypes_scanned, 2);
        assert_eq!(discovery.exact_variant_count, 1);
        assert_eq!(discovery.association_count, 1);
        assert_eq!(discovery.associations[0].condition, "Synthetic condition");
        assert_eq!(discovery.associations[0].origin_status, "Germline observation reported");
        let serialized = serde_json::to_string(&discovery).unwrap();
        assert!(!serialized.contains("A/G"));
        assert!(!serialized.contains("allele1"));
        assert!(!serialized.contains("rsSomaticTest"));

        conn.execute(
            "UPDATE genotypes SET allele1 = 'T', allele2 = 'C' WHERE rsid = 'rsClinvarTest'",
            [],
        )
        .unwrap();
        let mut reverse_provenance = provenance.clone();
        reverse_provenance.diagnostics.allele_orientation =
            "reverse-strand (synthetic test)".into();
        let reverse_discovery =
            fetch_genomewide_clinvar_discovery(&conn, 1, Some(&reverse_provenance)).unwrap();
        assert_eq!(reverse_discovery.exact_variant_count, 1);
        assert!(reverse_discovery.allele_orientation.starts_with("Reverse strand"));

        let mut unknown_provenance = provenance;
        unknown_provenance.diagnostics.allele_orientation = "Unknown".into();
        let unknown_discovery =
            fetch_genomewide_clinvar_discovery(&conn, 1, Some(&unknown_provenance)).unwrap();
        assert_eq!(unknown_discovery.exact_variant_count, 0);
        assert!(unknown_discovery.allele_orientation.starts_with("Unverified"));
    }

    #[test]
    fn missing_optional_catalog_table_is_detected_without_querying_it() {
        let conn = setup_test_db();
        assert!(!catalog_table_available(
            &conn,
            "clinvar",
            "clinvar_reference"
        ));

        conn.execute("ATTACH DATABASE ':memory:' AS clinvar", [])
            .unwrap();
        assert!(!catalog_table_available(
            &conn,
            "clinvar",
            "clinvar_reference"
        ));
        conn.execute(
            "CREATE TABLE clinvar.clinvar_reference (rsid TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        assert!(catalog_table_available(
            &conn,
            "clinvar",
            "clinvar_reference"
        ));
    }

    #[test]
    fn catalog_enrichment_supports_older_schemas_without_new_association_columns() {
        let conn = setup_test_db();
        conn.execute("ATTACH DATABASE ':memory:' AS clinvar", [])
            .unwrap();
        conn.execute(
            "CREATE TABLE clinvar.clinvar_reference (
                rsid TEXT, allele_id TEXT, variation_id TEXT, name TEXT, gene_symbol TEXT,
                clinical_significance TEXT, clin_sig_simple TEXT, phenotype_ids TEXT,
                phenotype_list TEXT, conditions TEXT, review_status TEXT, assembly TEXT,
                chromosome TEXT, start INTEGER, stop INTEGER, last_evaluated TEXT,
                number_submitters INTEGER, reference_allele_vcf TEXT, alternate_allele_vcf TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clinvar.clinvar_reference
             (rsid, variation_id, name, gene_symbol, clinical_significance, conditions)
             VALUES ('rs321', '54321', 'Older row', 'GENE2', 'Pathogenic', 'Synthetic condition')",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE gwas_reference (
                rsid TEXT PRIMARY KEY, top_trait TEXT, best_pvalue REAL, association_count INTEGER
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gwas_reference (rsid, top_trait, best_pvalue, association_count)
             VALUES ('rs321', 'Older GWAS trait', 1e-8, 1)",
            [],
        )
        .unwrap();

        let local = fetch_local_enrichment(&conn, &["rs321".to_string()]);
        assert_eq!(local["rs321"].clinvar_annotations.len(), 1);
        assert_eq!(local["rs321"].clinvar_annotations[0].rcv_accession, None);
        assert_eq!(local["rs321"].gwas_top_trait.as_deref(), Some("Older GWAS trait"));
        assert!(local["rs321"].gwas_associations.is_empty());
    }

    #[test]
    fn test_report_reference_registry_is_stable_and_deduplicated() {
        let source = MarkerSource {
            name: "Shared evidence source".to_string(),
            url: Some("https://example.test/evidence".to_string()),
            accessed: Some("2026-08-27".to_string()),
            evidence_type: Some("Research context".to_string()),
            conflict_of_interest: None,
            notes: None,
        };
        let source_with_trailing_slash = MarkerSource {
            url: Some("HTTPS://EXAMPLE.TEST/EVIDENCE/".to_string()),
            ..source.clone()
        };
        let mut registry = ReportReferenceRegistryBuilder::default();

        let first_id = registry.register(ReferenceSource::Marker(&source));
        let second_id = registry.register(ReferenceSource::Marker(&source_with_trailing_slash));

        assert_eq!(first_id, "REF-5AE32D29");
        assert_eq!(first_id, second_id);
        assert_eq!(registry.references.len(), 1);
        assert_eq!(registry.references[0].evidence_role, "Research context");
    }

    #[test]
    fn local_catalog_enrichment_preserves_one_to_many_annotations() {
        let conn = setup_test_db();
        conn.execute(
            "CREATE TABLE pharmgkb_clinical_variants (
                rsid TEXT NOT NULL,
                gene TEXT,
                drug TEXT,
                phenotype TEXT,
                evidence_level TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pharmgkb_clinical_variants (rsid, gene, drug, phenotype, evidence_level)
             VALUES
                ('rs123', 'GENE1', 'Drug A', 'Phenotype A', '1A'),
                ('rs123', 'GENE1', 'Drug B', 'Phenotype B', '2A')",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE clingen_gene_validity (
                hgnc_id TEXT,
                gene_symbol TEXT NOT NULL,
                disease_label TEXT NOT NULL,
                classification TEXT,
                moi TEXT,
                report_url TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clingen_gene_validity
             (hgnc_id, gene_symbol, disease_label, classification, moi, report_url)
             VALUES
                ('HGNC:1', 'GENE1', 'Condition A', 'Definitive', 'Autosomal dominant', 'https://example.test/a'),
                ('HGNC:1', 'GENE1', 'Condition B', 'Limited', 'Autosomal recessive', 'https://example.test/b')",
            [],
        )
        .unwrap();

        conn.execute("ATTACH DATABASE ':memory:' AS clinvar", [])
            .unwrap();
        conn.execute(
            "CREATE TABLE clinvar.clinvar_reference (
                rsid TEXT,
                allele_id TEXT,
                variation_id TEXT,
                rcv_accession TEXT,
                name TEXT,
                gene_symbol TEXT,
                clinical_significance TEXT,
                clin_sig_simple TEXT,
                phenotype_ids TEXT,
                phenotype_list TEXT,
                conditions TEXT,
                review_status TEXT,
                assembly TEXT,
                chromosome TEXT,
                start INTEGER,
                stop INTEGER,
                last_evaluated TEXT,
                number_submitters INTEGER,
                reference_allele_vcf TEXT,
                alternate_allele_vcf TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clinvar.clinvar_reference
             (rsid, allele_id, variation_id, rcv_accession, name, gene_symbol, clinical_significance,
              conditions, review_status, assembly, chromosome, start, stop)
             VALUES
                ('rs123', '1', '101', 'RCV000000001', 'Variant A', 'GENE1', 'Pathogenic', 'Condition A', 'reviewed', 'GRCh38', '1', 10, 10),
                ('rs123', '2', '102', 'RCV000000002', 'Variant B', 'GENE1', 'Benign', 'Condition B', 'criteria provided', 'GRCh38', '1', 11, 11)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE gwas_reference (
                rsid TEXT PRIMARY KEY,
                top_trait TEXT,
                best_pvalue REAL,
                association_count INTEGER,
                associations_json TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gwas_reference
             (rsid, top_trait, best_pvalue, association_count, associations_json)
             VALUES ('rs123', 'Synthetic trait', 1e-9, 2,
               '[{\"trait_name\":\"Synthetic trait\",\"pvalue\":1e-9,\"study_accession\":\"GCST000001\"},
                 {\"trait_name\":\"Second synthetic trait\",\"pvalue\":2e-8,\"study_accession\":\"GCST000002\"},
                 {\"trait_name\":\"Internal routing seed\",\"pvalue\":1e-10,\"source\":\"discovery_catalog_fallback\"}]')",
            [],
        )
        .unwrap();

        let pharmgkb = fetch_pharmgkb_enrichment(&conn, &["rs123".to_string()]);
        assert_eq!(pharmgkb["rs123"].len(), 2);
        assert_eq!(pharmgkb["rs123"][0].drug, "Drug A");
        assert_eq!(pharmgkb["rs123"][1].phenotype, "Phenotype B");

        let clingen = fetch_clingen_enrichment(&conn, &["GENE1".to_string()]);
        assert_eq!(clingen["gene1"].len(), 2);
        assert_eq!(clingen["gene1"][0].mode_of_inheritance.as_deref(), Some("Autosomal dominant"));
        assert_eq!(clingen["gene1"][1].disease_label, "Condition B");

        let local = fetch_local_enrichment(&conn, &["rs123".to_string()]);
        assert_eq!(local["rs123"].clinvar_annotations.len(), 2);
        assert_eq!(local["rs123"].clinvar_annotations[0].variation_id.as_deref(), Some("101"));
        assert_eq!(local["rs123"].clinvar_annotations[0].rcv_accession.as_deref(), Some("RCV000000001"));
        assert_eq!(local["rs123"].clinvar_annotations[1].clinical_significance, "Benign");
        assert_eq!(local["rs123"].gwas_associations.len(), 3);
        assert_eq!(
            local["rs123"].gwas_associations[0]["association_is"],
            "variant_trait_statistical_association"
        );
        assert_eq!(
            local["rs123"].gwas_associations[1]["study_accession"],
            "GCST000002"
        );
        let sources = build_enriched_sources("rs123", &local["rs123"], None);
        assert!(sources.iter().any(|source| {
            source.source_type == "GWAS"
                && source.citation.contains("Second synthetic trait")
                && source.details.as_deref().is_some_and(|details| details.contains("GCST000002"))
        }));
        assert!(sources.iter().any(|source| {
            source.source_type == "GWAS"
                && source.url.as_deref() == Some("https://www.ebi.ac.uk/gwas/studies/GCST000001")
        }));
        assert!(!sources.iter().any(|source| {
            source.citation.contains("Internal routing seed")
        }));
    }

    #[test]
    fn test_dpyd_rs55886062_safety_gate() {
        let conn = setup_test_db();
        let sample_id = 1;

        // Insert homozygous A/A genotype (which is Ref/Ref on minus)
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs55886062",
                "1",
                "97000000",
                "97000000",
                "A",
                "A",
            ],
        )
        .unwrap();
        let marker = MarkerDefinition {
            rsid: "rs55886062".to_string(),
            gene: "DPYD".to_string(),
            variant_name: Some("rs55886062 / I560S".to_string()),
            effect_allele: "C".to_string(), // correct effect allele
            impact: "Severe toxicity".to_string(),
            evidence_tier: "A".to_string(),
            interpretation: "Carriers of C allele have severe toxicity risk.".to_string(),
            do_not_claim: vec![],
            confirm_with: vec![],
            effect_direction: EffectDirection::Risk,
            raw_dna_limitation: None,
            clinical_confirmation_required: Some(true),
            sex_scope: None,
            context_tags: None,
            sources: None,
            variant_type: Some("snp".to_string()),
            expected_plus_alleles: Some(vec!["A".to_string(), "C".to_string()]),
            strand: Some("minus".to_string()),
            source_build: Some("GRCh38".to_string()),
            hgvs: Some("c.1679T>G".to_string()),
            allele_orientation_verified: Some(true),
            orientation_source: Some("dbSNP".to_string()),
            interpretation_blocked_if_unverified: Some(true),
            clinical_semantics: Some(ClinicalSemantics {
                condition_label: Some("DPYD fluoropyrimidine toxicity context".to_string()),
                interpretation_class: Some(FindingInterpretationClass::ClinicallyActionableVariant),
                inheritance_model: Some(FindingInheritanceModel::AutosomalRecessive),
                clinical_state: Some(FindingClinicalState::Unknown),
            }),
        };

        let template = ReportTemplate {
            title: "Test".to_string(),
            description: "Test".to_string(),
            sections: vec![SectionDefinition {
                name: "PGx".to_string(),
                markers: vec![marker.clone()],
            }],
        };

        let report = generate_report(&conn, sample_id, &template).unwrap();
        let link_id = &report.sections[0].link_ids[0];
        let evaluated = report.category_links.get(link_id).unwrap();
        let call = report.user_calls.get("rs55886062").unwrap();

        // Genotype AA is benign because effect allele is C (0 count)
        assert_eq!(call.user_genotype, "AA");
        assert_eq!(evaluated.effect_count, Some(0));
        assert_eq!(evaluated.severity_class, "benign");
        assert_eq!(evaluated.orientation_state, OrientationState::Verified);
        let assertion_key = evaluated.assertion_key.clone();
        let assertion_identity: serde_json::Value =
            serde_json::from_str(&assertion_key).expect("assertion key JSON");
        assert_eq!(assertion_identity["version"], 1);
        assert_eq!(assertion_identity["locus"]["rsid"], "rs55886062");
        assert_eq!(assertion_identity["allele_definition"]["effect_allele"], "C");
        assert_eq!(
            assertion_identity["condition_or_trait"]["interpretation_class"],
            "clinically_actionable_variant"
        );
        assert_eq!(
            assertion_identity["condition_or_trait"]["inheritance_model"],
            "autosomal_recessive"
        );
        assert!(assertion_key.contains("assay_requirement"));
        assert!(!assertion_key.contains("AA"));

        // Now test genotype GG (G is not in expected alleles [A, C])
        conn.execute("DELETE FROM genotypes", []).unwrap();
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs55886062",
                "1",
                "97000000",
                "97000000",
                "G",
                "G",
            ],
        )
        .unwrap();

        let report2 = generate_report(&conn, sample_id, &template).unwrap();
        let link_id2 = &report2.sections[0].link_ids[0];
        let evaluated2 = report2.category_links.get(link_id2).unwrap();
        let call2 = report2.user_calls.get("rs55886062").unwrap();

        // Genotype GG triggers orientation warning and safety gate block
        assert_eq!(call2.user_genotype, "GG");
        assert_eq!(evaluated2.severity_class, "confirmation_required");
        assert_eq!(evaluated2.orientation_state, OrientationState::Mismatch);
        assert!(evaluated2.interpretation.contains("blocked"));
        assert_eq!(evaluated2.assertion_key, assertion_key);

        // Adding an unrelated source-backed assertion must not migrate the
        // existing assertion identity. This protects downstream chat/export
        // consumers from row-order or source-table churn.
        let mut expanded_template = template.clone();
        let mut unrelated_marker = marker.clone();
        unrelated_marker.rsid = "rs-unrelated-test".to_string();
        unrelated_marker.gene = "UNRELATED".to_string();
        unrelated_marker.variant_name = Some("Unrelated test assertion".to_string());
        unrelated_marker.sources = Some(vec![MarkerSource {
            name: "Unrelated test source".to_string(),
            url: Some("https://example.test/unrelated".to_string()),
            accessed: None,
            evidence_type: Some("test".to_string()),
            conflict_of_interest: None,
            notes: None,
        }]);
        expanded_template.sections[0].markers.push(unrelated_marker);

        let report3 = generate_report(&conn, sample_id, &expanded_template).unwrap();
        let expanded_link_id = report3.sections[0]
            .link_ids
            .iter()
            .find(|candidate| {
                report3
                    .category_links
                    .get(*candidate)
                    .map(|link| link.rsid == "rs55886062")
                    .unwrap_or(false)
            })
            .expect("original assertion retained after unrelated row addition");
        let expanded_evaluated = report3
            .category_links
            .get(expanded_link_id)
            .expect("expanded original assertion link");
        assert_eq!(expanded_link_id, link_id2);
        assert_eq!(expanded_evaluated.assertion_key, assertion_key);
    }

    #[test]
    fn test_non_matchable_effect_allele_is_not_evaluated() {
        let conn = setup_test_db();
        let sample_id = 1;
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs-study-label",
                "1",
                "100",
                "100",
                "A",
                "A",
            ],
        )
        .unwrap();

        let template = ReportTemplate {
            title: "Non-matchable assertion test".to_string(),
            description: "Test".to_string(),
            sections: vec![SectionDefinition {
                name: "Research".to_string(),
                markers: vec![MarkerDefinition {
                    rsid: "rs-study-label".to_string(),
                    gene: "GENE1".to_string(),
                    variant_name: Some("Study-defined label".to_string()),
                    effect_allele: "study_reported_allele".to_string(),
                    impact: "Research context".to_string(),
                    evidence_tier: "C_candidate_OR_mechanistic".to_string(),
                    interpretation: "Context only".to_string(),
                    do_not_claim: vec![],
                    confirm_with: vec![],
                    effect_direction: EffectDirection::Risk,
                    raw_dna_limitation: None,
                    clinical_confirmation_required: None,
                    sex_scope: None,
                    context_tags: None,
                    sources: None,
                    variant_type: Some("snp".to_string()),
                    expected_plus_alleles: Some(vec!["A".to_string(), "G".to_string()]),
                    strand: None,
                    source_build: Some("GRCh38".to_string()),
                    hgvs: None,
                    allele_orientation_verified: Some(true),
                    orientation_source: Some("test".to_string()),
                    interpretation_blocked_if_unverified: Some(false),
                    clinical_semantics: None,
                }],
            }],
        };

        let report = generate_report(&conn, sample_id, &template).unwrap();
        let link = report
            .category_links
            .get(&report.sections[0].link_ids[0])
            .unwrap();

        assert_eq!(link.assertion_status, AssertionStatus::NotEvaluated);
        assert_eq!(link.callability_state, CallabilityState::NotCallable);
        assert_eq!(link.effect_count, None);
        assert_eq!(link.severity_class, "not_evaluated");
        assert!(!link.interpretation_allowed);
        assert_eq!(report.sections[0].summary.not_evaluated_count, 1);
        assert_eq!(report.sections[0].summary.risk_effect_count, 0);
    }

    #[test]
    fn test_non_snp_variant_type_cannot_enter_snp_counter() {
        let conn = setup_test_db();
        let sample_id = 1;
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rare-variant-label",
                "1",
                "100",
                "100",
                "A",
                "A",
            ],
        )
        .unwrap();

        let report = generate_report(
            &conn,
            sample_id,
            &ReportTemplate {
                title: "Variant-type registry test".to_string(),
                description: "Test".to_string(),
                sections: vec![SectionDefinition {
                    name: "Rare variant".to_string(),
                    markers: vec![MarkerDefinition {
                        rsid: "rare-variant-label".to_string(),
                        gene: "GENE1".to_string(),
                        variant_name: Some("Rare variant label".to_string()),
                        effect_allele: "A".to_string(),
                        impact: "Confirmation required".to_string(),
                        evidence_tier: "A_rare_high_effect_when_confirmed".to_string(),
                        interpretation: "Requires a variant-specific assay".to_string(),
                        do_not_claim: vec!["A consumer SNP call is not sufficient".to_string()],
                        confirm_with: vec!["Clinical sequencing".to_string()],
                        effect_direction: EffectDirection::Risk,
                        raw_dna_limitation: None,
                        clinical_confirmation_required: Some(true),
                        sex_scope: None,
                        context_tags: None,
                        sources: None,
                        variant_type: Some("rare_variant".to_string()),
                        expected_plus_alleles: Some(vec!["A".to_string(), "G".to_string()]),
                        strand: None,
                        source_build: Some("GRCh38".to_string()),
                        hgvs: None,
                        allele_orientation_verified: Some(true),
                        orientation_source: Some("test".to_string()),
                        interpretation_blocked_if_unverified: Some(false),
                        clinical_semantics: None,
                    }],
                }],
            },
        )
        .unwrap();
        let link = report
            .category_links
            .get(&report.sections[0].link_ids[0])
            .unwrap();

        assert_eq!(link.assertion_status, AssertionStatus::NotEvaluated);
        assert_eq!(link.callability_state, CallabilityState::NotCallable);
        assert_eq!(link.effect_count, None);
        assert_eq!(link.severity_class, "not_evaluated");
        assert!(!link.interpretation_allowed);
        assert_eq!(report.sections[0].summary.risk_effect_count, 0);
    }

    #[test]
    fn test_effect_allele_matchability_is_conservative() {
        assert!(is_matchable_effect_allele("A"));
        assert!(is_matchable_effect_allele(" t "));
        assert!(!is_matchable_effect_allele("study_reported_allele"));
        assert!(!is_matchable_effect_allele("N/A"));
        assert!(!is_matchable_effect_allele("TA"));
        assert!(!is_matchable_effect_allele("I"));
        assert!(is_matchable_snp_assertion(Some("snp"), "A"));
        assert!(is_matchable_snp_assertion(Some("pharmacogenomic_snp"), "C"));
        assert!(!is_matchable_snp_assertion(Some("rare_variant"), "A"));
        assert!(!is_matchable_snp_assertion(Some("hla_tag"), "A"));
        assert_eq!(
            callability_state_for_result(Some("snp"), AssertionStatus::Verified, false),
            CallabilityState::Callable
        );
        assert_eq!(
            callability_state_for_result(Some("snp"), AssertionStatus::NotInRawFile, true),
            CallabilityState::NotPresent
        );
        assert_eq!(
            callability_state_for_result(Some("snp"), AssertionStatus::OrientationMismatch, false),
            CallabilityState::Blocked
        );
        assert_eq!(
            callability_state_for_result(Some("hla_tag"), AssertionStatus::NotInRawFile, true),
            CallabilityState::NotCallable
        );
    }

    #[test]
    fn test_normalized_report_deduplication_and_no_call_omission() {
        let conn = setup_test_db();
        let sample_id = 1;

        // Insert rs12345 (Genotype: CC) and rs67890 (NoData: --)
        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs12345",
                "1",
                "1000",
                "1000",
                "C",
                "C",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO genotypes (sample_id, rsid, chromosome, position_grch37, position_grch38, allele1, allele2)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            [
                &sample_id.to_string(),
                "rs67890",
                "1",
                "2000",
                "2000",
                "?",
                "A",
            ],
        )
        .unwrap();

        // Template containing duplicate rs12345 in two different sections, and rs67890 (which is missing)
        let m1 = MarkerDefinition {
            rsid: "rs12345".to_string(),
            gene: "GENEA".to_string(),
            variant_name: None,
            effect_allele: "C".to_string(),
            impact: "Impact A".to_string(),
            evidence_tier: "A".to_string(),
            interpretation: "Interp A".to_string(),
            do_not_claim: vec![],
            confirm_with: vec![],
            effect_direction: EffectDirection::Risk,
            raw_dna_limitation: None,
            clinical_confirmation_required: None,
            sex_scope: None,
            context_tags: None,
            sources: Some(vec![MarkerSource {
                name: "Generated report test source".to_string(),
                url: Some("https://example.test/report-source".to_string()),
                accessed: Some("2026-08-27".to_string()),
                evidence_type: Some("Unit-test provenance".to_string()),
                conflict_of_interest: None,
                notes: None,
            }]),
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
            clinical_semantics: None,
        };

        let m2 = MarkerDefinition {
            rsid: "rs12345".to_string(),
            gene: "GENEA".to_string(),
            variant_name: None,
            effect_allele: "C".to_string(),
            impact: "Impact B".to_string(),
            evidence_tier: "B".to_string(),
            interpretation: "Interp B".to_string(),
            do_not_claim: vec![],
            confirm_with: vec![],
            effect_direction: EffectDirection::Risk,
            raw_dna_limitation: None,
            clinical_confirmation_required: None,
            sex_scope: None,
            context_tags: None,
            sources: None,
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
            clinical_semantics: None,
        };

        let m_missing = MarkerDefinition {
            rsid: "rs67890".to_string(),
            gene: "GENEB".to_string(),
            variant_name: None,
            effect_allele: "T".to_string(),
            impact: "Impact C".to_string(),
            evidence_tier: "A".to_string(),
            interpretation: "Interp C".to_string(),
            do_not_claim: vec![],
            confirm_with: vec![],
            effect_direction: EffectDirection::Risk,
            raw_dna_limitation: None,
            clinical_confirmation_required: None,
            sex_scope: None,
            context_tags: None,
            sources: None,
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
            clinical_semantics: None,
        };

        let template = ReportTemplate {
            title: "Deduplication Test".to_string(),
            description: "Test".to_string(),
            sections: vec![
                SectionDefinition {
                    name: "Section 1".to_string(),
                    markers: vec![m1],
                },
                SectionDefinition {
                    name: "Section 2".to_string(),
                    markers: vec![m2, m_missing],
                },
            ],
        };

        let mut report = generate_report(&conn, sample_id, &template).unwrap();

        // 1. Deduplication checks
        assert_eq!(report.variants.len(), 2);
        assert!(report.variants.contains_key("rs12345"));
        assert!(report.variants.contains_key("rs67890"));

        let ambiguous_call = report.user_calls.get("rs67890").unwrap();
        assert_eq!(ambiguous_call.call_status, CallStatus::AmbiguousRawCall);
        let ambiguous_link = report
            .category_links
            .get(&report.sections[1].link_ids[1])
            .unwrap();
        assert_eq!(ambiguous_link.assertion_status, AssertionStatus::BlockedRawCall);

        assert_eq!(report.user_calls.len(), 2);
        assert!(report.user_calls.contains_key("rs12345"));
        assert!(report.user_calls.contains_key("rs67890"));

        // But we should have 3 category links total (2 for rs12345, 1 for rs67890)
        assert_eq!(report.category_links.len(), 3);

        // 2. Link-ID resolution checks
        let sec1 = &report.sections[0];
        let sec2 = &report.sections[1];

        assert_eq!(sec1.link_ids.len(), 1);
        assert_eq!(sec2.link_ids.len(), 2);

        let link1 = report.category_links.get(&sec1.link_ids[0]).unwrap();
        let link2 = report.category_links.get(&sec2.link_ids[0]).unwrap();
        let link_missing = report.category_links.get(&sec2.link_ids[1]).unwrap();

        assert_eq!(link1.rsid, "rs12345");
        assert_eq!(link2.rsid, "rs12345");
        assert_eq!(link_missing.rsid, "rs67890");

        assert_eq!(link1.evidence_tier, "A");
        assert_eq!(link2.evidence_tier, "B");

        assert_eq!(report.references.len(), 1);
        assert_eq!(link1.reference_ids.len(), 1);
        assert_eq!(link1.reference_ids[0], report.references[0].id);
        assert!(link2.reference_ids.is_empty());

        // Technical Markdown must preserve the source call even when a
        // strand-normalized call is also available for interpretation.
        report
            .user_calls
            .get_mut("rs12345")
            .unwrap()
            .normalized_genotype = Some("GG".to_string());
        let markdown = render_markdown(&report);
        assert!(markdown.contains("Exact raw genotype calls"));
        assert!(markdown.contains("`CC` | `GG`"));

        // 3. Omission check: effect_count is Some(2) for rs12345, but None for rs67890
        assert_eq!(link1.effect_count, Some(2));
        assert_eq!(link_missing.effect_count, None);

        // Convert report to JSON and verify "effect_count" is NOT serialized for rs67890 (None)
        let _json_str = serde_json::to_string(&report).unwrap();

        let serialized_link_missing = serde_json::to_value(link_missing).unwrap();
        assert!(
            !serialized_link_missing
                .as_object()
                .unwrap()
                .contains_key("effect_count")
        );

        // Unknown bases must not be treated as verified calls. A mixed call is
        // ambiguous, while an all-unknown call is simply no data.
        conn.execute(
            "UPDATE genotypes SET allele1 = 'N', allele2 = 'A' WHERE sample_id = ? AND rsid = ?",
            [&sample_id.to_string(), "rs67890"],
        )
        .unwrap();
        let mixed_unknown_report = generate_report(&conn, sample_id, &template).unwrap();
        assert_eq!(
            mixed_unknown_report.user_calls.get("rs67890").unwrap().call_status,
            CallStatus::AmbiguousRawCall
        );

        conn.execute(
            "UPDATE genotypes SET allele1 = 'N', allele2 = 'N' WHERE sample_id = ? AND rsid = ?",
            [&sample_id.to_string(), "rs67890"],
        )
        .unwrap();
        let all_unknown_report = generate_report(&conn, sample_id, &template).unwrap();
        assert_eq!(
            all_unknown_report.user_calls.get("rs67890").unwrap().call_status,
            CallStatus::NoData
        );
    }
}
