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
use std::collections::HashMap;

use crate::offline::schema::schema_attached;

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

/// Local reference DB data fetched per-rsID during report generation.
#[derive(Debug, Default)]
struct LocalEnrichment {
    pub clinvar_significance: Option<String>,
    pub clinvar_conditions: Option<String>,
    pub clinvar_review_status: Option<String>,
    pub gwas_top_trait: Option<String>,
    pub gwas_best_pvalue: Option<f64>,
    pub gwas_association_count: Option<i64>,
    pub population_af: Option<f64>,
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
    pub sources: Option<Vec<MarkerSource>>,
    pub variant_type: Option<String>,
    pub expected_plus_alleles: Option<Vec<String>>,
    pub strand: Option<String>,
    pub source_build: Option<String>,
    pub hgvs: Option<String>,
    pub allele_orientation_verified: Option<bool>,
    pub orientation_source: Option<String>,
    pub interpretation_blocked_if_unverified: Option<bool>,
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
    pub rsid: String,
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
    pub interpretation_blocked_if_unverified: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<MarkerSource>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClinVarAnnotation {
    pub clinical_significance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_status: Option<String>,
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
}

#[derive(Debug, Serialize, Clone)]
pub struct PharmGkbAnnotation {
    pub drug: String,
    pub phenotype: String,
    pub evidence_level: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClinGenAnnotation {
    pub gene_symbol: String,
    pub disease_label: String,
    pub classification: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ManeAnnotation {
    pub gene_symbol: String,
    pub ensembl_transcript: String,
    pub refseq_transcript: String,
    pub mane_status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VariantEnrichment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinvar: Option<ClinVarAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gwas_hits: Vec<GwasHit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dbsnp: Option<DbsnpAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population: Option<PopulationAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pharmgkb: Option<PharmGkbAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clingen: Option<ClinGenAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mane: Option<ManeAnnotation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub db_enriched_sources: Vec<EnrichedSource>,
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
    /// Explicit catalog readiness notes (never silent when ClinVar/dbSNP expected but missing).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub catalog_warnings: Vec<String>,
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

    // Query ClinVar (fail loud via catalog_warnings when schema missing — do not silently skip forever)
    let sql_clinvar = format!(
        "SELECT rsid, clinical_significance, conditions, review_status 
         FROM clinvar.clinvar_reference WHERE rsid IN ({})",
        placeholders
    );
    let mut clinvar_data = HashMap::new();
    match conn.prepare(&sql_clinvar) {
        Ok(mut stmt) => {
            if let Ok(rows) = stmt.query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            }) {
                for r in rows.flatten() {
                    clinvar_data.insert(r.0.to_lowercase(), r);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "ClinVar enrichment query failed (catalog missing or not indexed?): {e}"
            );
        }
    }

    // Query GWAS Catalog
    let sql_gwas = format!(
        "SELECT rsid, top_trait, best_pvalue, association_count 
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

        if let Some(c_row) = clinvar_data.get(&curr) {
            entry.clinvar_significance = c_row.1.clone();
            entry.clinvar_conditions = c_row.2.clone();
            entry.clinvar_review_status = c_row.3.clone();
            has_data = true;
        }

        if let Some(g_row) = gwas_data.get(&curr) {
            entry.gwas_top_trait = g_row.1.clone();
            entry.gwas_best_pvalue = g_row.2;
            entry.gwas_association_count = g_row.3;
            has_data = true;
        }

        if has_data {
            map.insert(rsid_lower, entry);
        }
    }

    map
}

/// Build enriched sources from a LocalEnrichment record for a specific rsID.
fn build_enriched_sources(rsid: &str, enr: &LocalEnrichment) -> Vec<EnrichedSource> {
    let mut sources = Vec::new();

    // ClinVar
    if let Some(ref sig) = enr.clinvar_significance {
        let review = enr
            .clinvar_review_status
            .as_deref()
            .unwrap_or("unknown review status");
        let conditions = enr.clinvar_conditions.as_deref().unwrap_or("");
        sources.push(EnrichedSource {
            source_type: "ClinVar".to_string(),
            citation: format!("ClinVar: {} ({})", sig, review),
            details: if conditions.is_empty() {
                None
            } else {
                Some(conditions.to_string())
            },
            url: Some(format!(
                "https://www.ncbi.nlm.nih.gov/clinvar/?term={}%5BVariant+ID%5D",
                rsid
            )),
        });
    }

    // GWAS
    if let Some(ref trait_name) = enr.gwas_top_trait
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
        sources.push(EnrichedSource {
            source_type: "gnomAD".to_string(),
            citation: format!("gnomAD AF: {:.4} ({})", af, classify_population_rarity(af)),
            details: None,
            url: Some(gnomad_url(rsid)),
        });
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
}

fn fetch_gnomad_dbsnp_metadata(
    conn: &Connection,
    rsids: &[String],
) -> HashMap<String, RawDbsnpGnomad> {
    if rsids.is_empty() {
        return HashMap::new();
    }
    let mut map = HashMap::new();
    // Offline gnomAD VCF hits use `remote_vcf_hit`; older API paths used `found`.
    let sql = "SELECT chrom, pos, ref, alt, af, release, rsids_json 
               FROM reference.gnomad_variant_cache 
               WHERE lookup_status IN ('found', 'remote_vcf_hit')";
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<f64>>(4).ok().flatten(),
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        }) {
            for row_res in rows.flatten() {
                if let (chrom, pos, ref_allele, alt_allele, af, release, Some(rsids_json)) = row_res
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
                },
            );
        }
    }
    filtered_map
}

fn build_clinvar_annotation(enr: &LocalEnrichment) -> Option<ClinVarAnnotation> {
    enr.clinvar_significance
        .as_ref()
        .map(|sig| ClinVarAnnotation {
            clinical_significance: sig.clone(),
            conditions: enr.clinvar_conditions.clone(),
            review_status: enr.clinvar_review_status.clone(),
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
) -> HashMap<String, PharmGkbAnnotation> {
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
        "SELECT LOWER(rsid), drug, phenotype, evidence_level FROM pharmgkb_clinical_variants WHERE rsid IN ({})",
        placeholders
    );
    let params: Vec<&dyn rusqlite::types::ToSql> = current_rsids
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let mut query_map = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let rsid: String = row.get(0).unwrap_or_default();
                let drug: String = row.get(1).unwrap_or_default();
                let phenotype: String = row.get(2).unwrap_or_default();
                let evidence_level: String = row.get(3).unwrap_or_default();
                query_map.insert(
                    rsid.to_lowercase(),
                    PharmGkbAnnotation {
                        drug,
                        phenotype,
                        evidence_level,
                    },
                );
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
) -> HashMap<String, ClinGenAnnotation> {
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
        "SELECT LOWER(gene_symbol), disease_label, classification FROM clingen_gene_validity WHERE gene_symbol IN ({})",
        placeholders
    );
    let params: Vec<&dyn rusqlite::types::ToSql> = genes
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let mut map = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let gene_symbol: String = row.get(0).unwrap_or_default();
                let disease_label: String = row.get(1).unwrap_or_default();
                let classification: String = row.get(2).unwrap_or_default();
                map.insert(
                    gene_symbol,
                    ClinGenAnnotation {
                        gene_symbol: row.get::<_, String>(0).unwrap_or_default(),
                        disease_label,
                        classification,
                    },
                );
            }
        }
    }
    map
}

fn fetch_mane_enrichment(conn: &Connection, genes: &[String]) -> HashMap<String, ManeAnnotation> {
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
        "SELECT LOWER(gene_symbol), ensembl_transcript, refseq_transcript, mane_status FROM mane_transcripts WHERE gene_symbol IN ({})",
        placeholders
    );
    let params: Vec<&dyn rusqlite::types::ToSql> = genes
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let mut map = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(&sql) {
        if let Ok(mut rows) = stmt.query(params.as_slice()) {
            while let Ok(Some(row)) = rows.next() {
                let gene_symbol: String = row.get(0).unwrap_or_default();
                let ensembl_transcript: String = row.get(1).unwrap_or_default();
                let refseq_transcript: String = row.get(2).unwrap_or_default();
                let mane_status: String = row.get(3).unwrap_or_default();
                map.insert(
                    gene_symbol,
                    ManeAnnotation {
                        gene_symbol: row.get::<_, String>(0).unwrap_or_default(),
                        ensembl_transcript,
                        refseq_transcript,
                        mane_status,
                    },
                );
            }
        }
    }
    map
}

/// Evaluates a template against a user's database records.
pub fn generate_report(
    conn: &Connection,
    sample_id: i64,
    template: &ReportTemplate,
) -> Result<GeneratedReport, String> {
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
    let dbsnp_data = fetch_gnomad_dbsnp_metadata(conn, &rsids);
    let pharmgkb_map = fetch_pharmgkb_enrichment(conn, &rsids);
    let clingen_map = fetch_clingen_enrichment(conn, &genes);
    let mane_map = fetch_mane_enrichment(conn, &genes);

    // Initialize mappings
    let mut variants_map: HashMap<String, CanonicalVariant> = HashMap::new();
    let mut user_calls_map: HashMap<String, UserCall> = HashMap::new();
    let mut category_links_map: HashMap<String, VariantCategoryLink> = HashMap::new();
    let mut enrichment_map_out: HashMap<String, VariantEnrichment> = HashMap::new();

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
                || raw_call.contains('?');
            let call_status = if is_missing {
                if genotype_map.contains_key(&m.rsid) {
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
                    source_build: m.source_build.clone(),
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

            let assertion_status;
            let mut interpretation_allowed = true;
            let mut effect_count: Option<u8> = None;
            let mut interpretation = m.interpretation.clone();
            let mut impact = m.impact.clone();
            let severity_class;

            if is_missing {
                assertion_status = if call_status == CallStatus::NotInRawFile {
                    AssertionStatus::NotInRawFile
                } else {
                    AssertionStatus::NoData
                };
                interpretation_allowed = false;
                severity_class = "no_data".to_string();
                no_data_count += 1;
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
                            let orientation_confirmed = dbsnp_data.contains_key(&m.rsid);
                            if orientation_confirmed {
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
                let has_metadata =
                    expected_plus_alleles.is_some() || dbsnp_data.contains_key(&m.rsid);
                let is_unverified = !has_metadata || orientation_warning;

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
                if !is_missing {
                    confirmation_required_count += 1;
                }
            } else {
                all_require_confirmation = false;
            }

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
                })
            });

            let pharmgkb = pharmgkb_map.get(&rsid_lower).cloned();
            let clingen = clingen_map.get(&gene_lower).cloned();
            let mane = mane_map.get(&gene_lower).cloned();

            let enr_with_af = enrichment_map.get(&rsid_lower).map(|e| LocalEnrichment {
                clinvar_significance: e.clinvar_significance.clone(),
                clinvar_conditions: e.clinvar_conditions.clone(),
                clinvar_review_status: e.clinvar_review_status.clone(),
                gwas_top_trait: e.gwas_top_trait.clone(),
                gwas_best_pvalue: e.gwas_best_pvalue,
                gwas_association_count: e.gwas_association_count,
                population_af: dbsnp_data.get(&m.rsid).and_then(|d| d.af),
            });

            let mut db_enriched_sources = if let Some(ref e) = enr_with_af {
                build_enriched_sources(&m.rsid, e)
            } else if let Some(ref pop) = population {
                vec![EnrichedSource {
                    source_type: "gnomAD".to_string(),
                    citation: format!(
                        "gnomAD AF: {:.4} ({})",
                        pop.allele_frequency, pop.rarity_bucket
                    ),
                    details: None,
                    url: Some(gnomad_url(&m.rsid)),
                }]
            } else {
                Vec::new()
            };

            if let Some(ref pkb) = pharmgkb {
                db_enriched_sources.push(EnrichedSource {
                    source_type: "PharmGKB".to_string(),
                    citation: format!(
                        "PharmGKB PGx Level {} (Drug: {})",
                        pkb.evidence_level, pkb.drug
                    ),
                    details: Some(pkb.phenotype.clone()),
                    url: Some(format!("https://www.pharmgkb.org/rsid/{}", m.rsid)),
                });
            }

            if let Some(ref cg) = clingen {
                db_enriched_sources.push(EnrichedSource {
                    source_type: "ClinGen".to_string(),
                    citation: format!(
                        "ClinGen Gene Validity: {} (Gene: {})",
                        cg.classification, cg.gene_symbol
                    ),
                    details: Some(cg.disease_label.clone()),
                    url: Some(format!(
                        "https://search.clinicalgenome.org/kb/genes/{}",
                        cg.gene_symbol
                    )),
                });
            }

            if let Some(ref mn) = mane {
                db_enriched_sources.push(EnrichedSource {
                    source_type: "MANE".to_string(),
                    citation: format!(
                        "MANE Transcript: {} / {}",
                        mn.refseq_transcript, mn.ensembl_transcript
                    ),
                    details: Some(format!("Status: {}", mn.mane_status)),
                    url: None,
                });
            }

            if clinvar.is_some()
                || !gwas_hits.is_empty()
                || dbsnp.is_some()
                || population.is_some()
                || pharmgkb.is_some()
                || clingen.is_some()
                || mane.is_some()
                || !db_enriched_sources.is_empty()
            {
                enrichment_map_out.insert(
                    m.rsid.clone(),
                    VariantEnrichment {
                        clinvar,
                        gwas_hits,
                        dbsnp,
                        population,
                        pharmgkb,
                        clingen,
                        mane,
                        db_enriched_sources,
                    },
                );
            }

            // Construct link object
            let assertion_text = format!("{}{}", impact, interpretation);
            let link_id = format!(
                "{}:{}:{}:{}",
                pack_id,
                category_id,
                m.rsid,
                stable_hash(&assertion_text)
            );

            let sources = m.sources.clone().unwrap_or_default();

            let link = VariantCategoryLink {
                link_id: link_id.clone(),
                rsid: m.rsid.clone(),
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
                requires_orientation_verification,
                interpretation_allowed,
                do_not_claim: m.do_not_claim.clone(),
                confirm_with: m.confirm_with.clone(),
                raw_dna_limitation: m.raw_dna_limitation.clone(),
                clinical_confirmation_required: m.clinical_confirmation_required,
                sex_scope: m.sex_scope.clone(),
                interpretation_blocked_if_unverified: m.interpretation_blocked_if_unverified,
                sources,
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
        catalog_warnings,
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
            "Summary: {} markers | {} association alleles/{} possible | {} protective | {} trait | {} context-dependent | {} no-data | {} confirmation-required\n\n",
            s.total_markers, s.risk_effect_count, s.risk_possible,
            s.protective_effect_count, s.trait_count,
            s.context_dependent_count, s.no_data_count, s.confirmation_required_count
        ));

        md.push_str("| Marker | Gene | Genotype | Effect Allele | Severity | Direction | Tier | Interpretation |\n");
        md.push_str("|---|---|---|---|---|---|---|---|\n");

        for link_id in &sec.link_ids {
            if let Some(link) = report.category_links.get(link_id) {
                let user_genotype = report
                    .user_calls
                    .get(&link.rsid)
                    .map(|uc| {
                        uc.normalized_genotype
                            .as_ref()
                            .unwrap_or(&uc.user_genotype)
                            .as_str()
                    })
                    .unwrap_or("--");

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
                    "| **{}** | **{}** | `{}` | `{}` | {} | {} | `{}` | *{}* |\n",
                    link.rsid,
                    gene,
                    user_genotype,
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
            sources: None,
            variant_type: Some("snp".to_string()),
            expected_plus_alleles: Some(vec!["A".to_string(), "C".to_string()]),
            strand: Some("minus".to_string()),
            source_build: Some("GRCh38".to_string()),
            hgvs: Some("c.1679T>G".to_string()),
            allele_orientation_verified: Some(true),
            orientation_source: Some("dbSNP".to_string()),
            interpretation_blocked_if_unverified: Some(true),
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
        assert!(evaluated2.interpretation.contains("blocked"));
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
            sources: None,
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
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
            sources: None,
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
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
            sources: None,
            variant_type: None,
            expected_plus_alleles: None,
            strand: None,
            source_build: None,
            hgvs: None,
            allele_orientation_verified: None,
            orientation_source: None,
            interpretation_blocked_if_unverified: None,
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

        let report = generate_report(&conn, sample_id, &template).unwrap();

        // 1. Deduplication checks
        assert_eq!(report.variants.len(), 2);
        assert!(report.variants.contains_key("rs12345"));
        assert!(report.variants.contains_key("rs67890"));

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
    }
}
