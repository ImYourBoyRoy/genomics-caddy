// ./src-tauri/src/agent.rs
/*
Module Docstring:
Purpose: Evidence-Grade Variant Validation & Safety QA Audit engine for Genomics Caddy.
Responsibilities:
- Define the VariantEvidence struct matching the Svelte types.
- Query local SQLite database for genotypes in chromosome regions.
- Fetch and cache Ensembl coordinates, dbSNP, and ClinVar records.
- Classify variant evidence, matching user genotype alleles against ClinVar pathogenicity.
- Gate unverified strand orientations for high-stakes variants (e.g., DPYD rs55886062).
- Run deterministic safety audits (diagnoses, dosages, and unverified pathogenicity checks).
- Save active discoveries incrementally to a JSON findings stack file.
Key Inputs: SQLite Connection, Sample ID, target rsIDs, or gene symbols.
Key Outputs: VariantEvidence objects, SafetyCheckResult reports.
Operational Notes: Compiles with rusqlite, serde, and reqwest.
*/

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VariantEvidence {
    pub rsid: String,
    pub gene: Option<String>,
    pub user_genotype: Option<String>,
    pub user_alleles: Vec<String>,
    pub matched_effect_allele: Option<String>,
    pub clinically_relevant_allele: Option<String>,
    pub allele_match_status: String, // "matched" | "not_matched" | "no_data" | "orientation_unverified" | "ambiguous"
    pub orientation_status: String,  // "verified" | "unverified" | "conflicting" | "not_required"
    pub genome_build: Option<String>,
    pub hgvs: Option<String>,
    pub clinvar_variation_id: Option<String>,
    pub clinvar_clinical_significance: Option<String>,
    pub clinvar_review_status: Option<String>,
    pub clinvar_condition: Option<String>,
    pub clinvar_conflict_status: Option<String>,
    pub evidence_source_ids: Vec<String>,
    pub interpretation_status: String, // "active_clinical" | "active_research" | "benign" | "modifier_only" | "not_active_for_user" | "blocked_unverified" | "conflicting" | "insufficient_evidence" | "mechanism_context_only"
    pub allowed_language: String,
    pub forbidden_language: Vec<String>,
    pub requires_clinical_confirmation: bool,
    pub safe_for_ai_context: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetyFinding {
    pub rule_id: String,
    pub severity: String, // "warning" | "fail"
    pub message: String,
    pub excerpt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetyCheckResult {
    pub passed: bool,
    pub severity: String, // "pass" | "warning" | "fail"
    pub findings: Vec<SafetyFinding>,
}

// ---------------------------------------------------------------------------
// Network Caching Utility
// ---------------------------------------------------------------------------

pub async fn fetch_api_cached(
    db_path: &Path,
    url: &str,
    api_key: Option<&str>,
    ttl_secs: u64,
) -> Result<Value, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let ttl = ttl_secs as i64;

    // Check cache
    {
        let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
        if let Ok((cached_json, fetched_at)) = conn.query_row(
            "SELECT response_json, fetched_at FROM api_cache WHERE url = ?",
            params![url],
            |row| {
                let json: String = row.get(0)?;
                let fetched: i64 = row.get(1)?;
                Ok((json, fetched))
            },
        ) && now - fetched_at < ttl
            && let Ok(parsed) = serde_json::from_str::<Value>(&cached_json)
        {
            return Ok(parsed);
        }
    }

    // Cache miss, execute query
    crate::config::validate_external_url(url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(crate::config::redirect_policy())
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let mut target_url = url.to_string();

    if url.contains("eutils.ncbi.nlm.nih.gov")
        && let Some(key) = api_key
        && !key.trim().is_empty()
    {
        let separator = if url.contains('?') { "&" } else { "?" };
        target_url = format!("{}{}{}api_key={}", url, separator, "", key.trim());
    }

    let res = client
        .get(&target_url)
        .header("User-Agent", "GenomicsCaddy/0.1")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("NCBI returned status {}", res.status()));
    }

    let json_val: Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    let json_str = serde_json::to_string(&json_val).unwrap_or_default();
    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO api_cache_db.api_cache (url, response_json, fetched_at) VALUES (?, ?, ?)",
            params![url, json_str, now],
        );
    }

    Ok(json_val)
}

// Helper: parse alt allele from ClinVar title
fn parse_clinically_relevant_allele(title: &str) -> Option<String> {
    if let Some(pos) = title.find("c.") {
        let sub = &title[pos..];
        if let Some(arrow_pos) = sub.find('>')
            && arrow_pos + 1 < sub.len()
        {
            return Some(sub[arrow_pos + 1..arrow_pos + 2].to_string());
        }
    }
    if let Some(pos) = title.find("g.") {
        let sub = &title[pos..];
        if let Some(arrow_pos) = sub.find('>')
            && arrow_pos + 1 < sub.len()
        {
            return Some(sub[arrow_pos + 1..arrow_pos + 2].to_string());
        }
    }
    if title.contains("dup")
        && let Some(pos) = title.find("dup")
        && pos + 3 < title.len()
    {
        return Some(title[pos + 3..pos + 4].to_string());
    }
    if title.contains("del")
        && let Some(pos) = title.find("del")
        && pos + 3 < title.len()
    {
        return Some(title[pos + 3..pos + 4].to_string());
    }
    None
}

// ---------------------------------------------------------------------------
// Core Variant Evidence Builder
// ---------------------------------------------------------------------------

fn complement_allele(a: &str) -> String {
    match a {
        "A" => "T".to_string(),
        "T" => "A".to_string(),
        "C" => "G".to_string(),
        "G" => "C".to_string(),
        other => other.to_string(),
    }
}

pub fn resolve_strand_orientation(
    user_alleles: &[String],
    expected_plus_alleles: &[String],
) -> (String, Vec<String>, String) {
    if user_alleles.is_empty() || expected_plus_alleles.is_empty() {
        return (
            "no_data".to_string(),
            user_alleles.to_vec(),
            "not_required".to_string(),
        );
    }

    // Convert everything to uppercase
    let user: Vec<String> = user_alleles.iter().map(|s| s.to_uppercase()).collect();
    let expected: Vec<String> = expected_plus_alleles
        .iter()
        .map(|s| s.to_uppercase())
        .collect();

    // Check if user alleles are perfectly subset of expected plus alleles
    let mut direct_match = true;
    for a in &user {
        if !expected.contains(a) {
            direct_match = false;
        }
    }

    if direct_match {
        // Direct match!
        // Is it a palindrome?
        let is_palindrome = (expected.contains(&"A".to_string())
            && expected.contains(&"T".to_string()))
            || (expected.contains(&"C".to_string()) && expected.contains(&"G".to_string()));

        if is_palindrome {
            return ("ambiguous".to_string(), user, "ambiguous".to_string());
        } else {
            return ("matched".to_string(), user, "verified".to_string());
        }
    }

    // Check for reverse complement match
    let complemented: Vec<String> = user.iter().map(|s| complement_allele(s)).collect();
    let mut complement_match = true;
    for a in &complemented {
        if !expected.contains(a) {
            complement_match = false;
        }
    }

    if complement_match {
        // Flipped strand verified!
        return ("matched".to_string(), complemented, "flipped".to_string());
    }

    // Conflicting alleles
    (
        "orientation_unverified".to_string(),
        user,
        "conflicting".to_string(),
    )
}

// ---------------------------------------------------------------------------
// Index-first: reuse vector enrichment when available
// ---------------------------------------------------------------------------

async fn try_index_backed_evidence(
    db_path: &Path,
    sample_id: i64,
    rsid: &str,
    user_genotype: Option<String>,
    user_alleles: Vec<String>,
) -> Result<Option<VariantEvidence>, String> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let fact_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM association_facts WHERE sample_id = ? AND rsid = ?",
            params![sample_id, rsid],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if fact_count == 0 {
        return Ok(None);
    }

    let cfg = match crate::config::load_qdrant_config(&conn) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let Some((payload, _point_id)) = crate::research::find_point_payload_by_rsid(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
        sample_id,
        rsid,
    )
    .await?
    else {
        return Ok(None);
    };

    let dq = payload["data_quality_score"].as_f64().unwrap_or(0.0) as f32;
    if dq < 0.25 {
        return Ok(None);
    }

    let gene = payload["gene_symbol"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let clinvar_sig = payload["clinvar_significance"]
        .as_str()
        .map(|s| s.to_string());
    let trait_summary = payload["gwas_traits"]
        .as_str()
        .or_else(|| payload["gwas_trait"].as_str())
        .unwrap_or("")
        .to_string();

    let mut notes = vec![format!(
        "Loaded from local vector index ({} association facts, data quality {:.2}). Live NCBI fetch skipped.",
        fact_count, dq
    )];
    if !trait_summary.is_empty() {
        notes.push(format!("Indexed traits: {}", trait_summary));
    }

    let has_clinvar = clinvar_sig.is_some();
    let interpretation = if has_clinvar {
        "active_research"
    } else {
        "mechanism_context_only"
    };

    let alleles_empty = user_alleles.is_empty();
    Ok(Some(VariantEvidence {
        rsid: rsid.to_string(),
        gene,
        user_genotype,
        user_alleles,
        matched_effect_allele: None,
        clinically_relevant_allele: None,
        allele_match_status: if alleles_empty {
            "no_data".into()
        } else {
            "orientation_unverified".into()
        },
        orientation_status: "not_required".into(),
        genome_build: Some("GRCh38".into()),
        hgvs: None,
        clinvar_variation_id: None,
        clinvar_clinical_significance: clinvar_sig,
        clinvar_review_status: None,
        clinvar_condition: if trait_summary.is_empty() {
            None
        } else {
            Some(trait_summary.clone())
        },
        clinvar_conflict_status: None,
        evidence_source_ids: vec!["vector_index".into(), "association_facts".into()],
        interpretation_status: interpretation.into(),
        allowed_language: "Discuss as research context; cite rsID and indexed sources.".into(),
        forbidden_language: vec![
            "diagnosis".into(),
            "prescribe".into(),
            "you have the disease".into(),
        ],
        requires_clinical_confirmation: has_clinvar,
        safe_for_ai_context: true,
        notes,
    }))
}

// ---------------------------------------------------------------------------
// Core Variant Evidence Builder
// ---------------------------------------------------------------------------

pub async fn get_variant_evidence_core(
    db_path: &Path,
    sample_id: i64,
    rsid: &str,
    api_key: Option<&str>,
    app_data_dir: &Path,
) -> Result<VariantEvidence, String> {
    let clean_rsid = rsid.trim().to_lowercase();
    if !clean_rsid.starts_with("rs")
        || clean_rsid.len() < 3
        || !clean_rsid[2..].chars().all(|c| c.is_ascii_digit())
    {
        return Err(format!("Invalid rsID format: {}", rsid));
    }
    let rsid_upper = clean_rsid.to_uppercase();

    // 1. Query SQLite for user genotype
    let (mut user_genotype, mut user_alleles) = {
        let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
            .map_err(|e| e.to_string())?;
        let user_geno = conn
            .query_row(
                "SELECT allele1, allele2 FROM genotypes WHERE sample_id = ? AND rsid = ?",
                params![sample_id, rsid_upper],
                |row| {
                    let a1: String = row.get(0)?;
                    let a2: String = row.get(1)?;
                    Ok((a1, a2))
                },
            )
            .ok();

        match user_geno {
            Some((a1, a2)) => {
                let geno = format!("{}/{}", a1, a2);
                let alleles = vec![a1, a2];
                (Some(geno), alleles)
            }
            None => (None, vec![]),
        }
    };

    let is_missing = user_genotype.is_none()
        || user_genotype.as_ref().unwrap() == "--"
        || user_genotype.as_ref().unwrap().contains('-')
        || user_genotype.as_ref().unwrap().contains('?')
        || user_genotype.as_ref().unwrap().contains('0');

    if let Ok(Some(index_evidence)) = try_index_backed_evidence(
        db_path,
        sample_id,
        &rsid_upper,
        user_genotype.clone(),
        user_alleles.clone(),
    )
    .await
    {
        return Ok(index_evidence);
    }

    // Load reference configurations
    let mut clinvar_sig: Option<String> = None;
    let mut clinvar_condition: Option<String> = None;
    let mut clinvar_relevant_allele: Option<String> = None;
    let mut resolved_gene: Option<String> = None;
    let mut clinvar_var_id: Option<String> = None;
    let mut clinvar_review: Option<String> = None;

    // Check local clinvar_reference table first
    let cached_ref: Option<(Option<String>, String, String, String)> = {
        let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT gene, clinical_significance, conditions, relevant_allele FROM clinvar_reference WHERE rsid = ?",
            params![rsid_upper],
            |row| {
                let g: Option<String> = row.get(0)?;
                let sig: String = row.get(1)?;
                let cond: String = row.get(2)?;
                let rel: String = row.get(3)?;
                Ok((g, sig, cond, rel))
            },
        ).ok()
    };

    if let Some((g, sig, cond, rel)) = cached_ref {
        resolved_gene = g;
        clinvar_sig = Some(sig);
        clinvar_condition = Some(cond);
        clinvar_relevant_allele = Some(rel);
    } else {
        // Fallback to discovery catalog JSON config
        let catalog_str = crate::db::get_pack_str(Some(app_data_dir), "discovery_catalog");
        let mut catalog_risk_allele: Option<String> = None;
        let mut catalog_gene: Option<String> = None;

        if let Some(ref cat_json) = catalog_str
            && let Ok(parsed_cat) = serde_json::from_str::<Value>(cat_json)
            && let Some(arr) = parsed_cat["markers"].as_array()
        {
            for m in arr {
                if m["rsid"].as_str().unwrap_or("").to_lowercase() == clean_rsid {
                    catalog_risk_allele = m["risk_allele"].as_str().map(|s| s.to_string());
                    catalog_gene = m["gene"].as_str().map(|s| s.to_string());
                    break;
                }
            }
        }

        // Live NCBI fetch as final fallback
        let search_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=clinvar&term={}&retmode=json",
            clean_rsid
        );
        let search_res = fetch_api_cached(db_path, &search_url, api_key, 86400 * 3)
            .await
            .ok();

        if let Some(search_val) = search_res
            && let Some(ids) = search_val["esearchresult"]["idlist"].as_array()
            && !ids.is_empty()
        {
            let id_str = ids[0].as_str().unwrap_or("").to_string();
            clinvar_var_id = Some(id_str.clone());
            let summary_url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=clinvar&id={}&retmode=json",
                id_str
            );
            if let Ok(summary_res) =
                fetch_api_cached(db_path, &summary_url, api_key, 86400 * 3).await
                && let Some(summary) = summary_res["result"][&id_str].as_object()
            {
                clinvar_sig = summary
                    .get("clinical_significance")
                    .and_then(|v| v.get("description"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                clinvar_review = summary
                    .get("clinical_significance")
                    .and_then(|v| v.get("review_status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                if let Some(traits) = summary.get("trait_set").and_then(|v| v.as_array()) {
                    let trait_names: Vec<String> = traits
                        .iter()
                        .filter_map(|t| {
                            t.get("trait_name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect();
                    clinvar_condition = Some(trait_names.join(", "));
                }

                if let Some(title) = summary.get("title").and_then(|v| v.as_str()) {
                    clinvar_relevant_allele = parse_clinically_relevant_allele(title);
                }
            }
        }

        // Merge with catalog overrides
        clinvar_relevant_allele = clinvar_relevant_allele
            .or(catalog_risk_allele)
            .map(|s| s.to_uppercase());
        resolved_gene = resolved_gene.or(catalog_gene);
    }

    let clinically_relevant = clinvar_relevant_allele.map(|s| s.to_uppercase());

    // 3. Dynamic Strand Resolution via Ensembl Variation REST API
    let mut expected_plus_alleles = Vec::new();
    let mut orientation_status = "not_required".to_string();
    let mut allele_match_status = "no_data".to_string();
    let mut matched_effect_allele: Option<String> = None;
    let hgvs = None;
    let mut notes = Vec::new();

    let ensembl_url = format!(
        "https://rest.ensembl.org/variation/homo_sapiens/{}?content-type=application/json",
        clean_rsid
    );
    if let Ok(ensembl_res) = fetch_api_cached(db_path, &ensembl_url, None, 86400 * 30).await
        && let Some(mappings) = ensembl_res["mappings"].as_array()
        && !mappings.is_empty()
        && let Some(allele_str) = mappings[0]["allele_string"].as_str()
    {
        expected_plus_alleles = allele_str
            .split('/')
            .map(|s| s.to_string().to_uppercase())
            .collect();
    }

    if !is_missing && !expected_plus_alleles.is_empty() {
        let (resolved_match, resolved_user_alleles, resolved_orientation) =
            resolve_strand_orientation(&user_alleles, &expected_plus_alleles);

        user_alleles = resolved_user_alleles;
        user_genotype = Some(user_alleles.join("/"));
        orientation_status = resolved_orientation;
        allele_match_status = resolved_match;

        if orientation_status == "flipped" {
            notes.push("Aligned from minus strand to plus strand complement.".to_string());
        } else if orientation_status == "ambiguous" {
            notes.push("Palindromic variant orientation is ambiguous.".to_string());
        }
    }

    // Evaluate pathogenicity stakes
    let mut is_high_stakes = false;
    let lower_gene = resolved_gene
        .as_ref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if lower_gene == "dpyd"
        || lower_gene == "brca1"
        || lower_gene == "brca2"
        || lower_gene == "cyp2c19"
        || lower_gene == "cyp2d6"
        || lower_gene == "cyp2c9"
        || lower_gene == "slco1b1"
    {
        is_high_stakes = true;
    }
    if let Some(ref sig) = clinvar_sig {
        let sig_lower = sig.to_lowercase();
        if sig_lower.contains("pathogenic") || sig_lower.contains("risk factor") {
            is_high_stakes = true;
        }
    }

    // 4. Match genotype against ClinVar risk allele
    let mut interpretation_status = "insufficient_evidence".to_string();
    let mut safe_for_ai_context = false;

    if !is_missing {
        if let Some(ref rel_allele) = clinically_relevant {
            let mut matches_rel = false;
            for c in &user_alleles {
                if c.to_uppercase() == rel_allele.to_uppercase() {
                    matches_rel = true;
                    matched_effect_allele = Some(c.clone());
                }
            }

            if matches_rel {
                allele_match_status = "matched".to_string();
            } else {
                allele_match_status = "not_matched".to_string();
            }
        }

        // Apply high-stakes verification gating
        if is_high_stakes
            && (orientation_status == "ambiguous" || orientation_status == "conflicting")
        {
            allele_match_status = "orientation_unverified".to_string();
        }

        // Apply Interpretation Status
        if allele_match_status == "orientation_unverified" || allele_match_status == "ambiguous" {
            interpretation_status = "blocked_unverified".to_string();
            safe_for_ai_context = false;
        } else if allele_match_status == "not_matched" {
            interpretation_status = "not_active_for_user".to_string();
            safe_for_ai_context = true;
        } else if allele_match_status == "matched" {
            if let Some(ref sig) = clinvar_sig {
                let sig_lower = sig.to_lowercase();
                if sig_lower.contains("conflicting") {
                    interpretation_status = "conflicting".to_string();
                    safe_for_ai_context = false;
                } else if sig_lower.contains("pathogenic") || sig_lower.contains("risk factor") {
                    interpretation_status = "active_clinical".to_string();
                    safe_for_ai_context = true;
                } else if sig_lower.contains("modifier") {
                    interpretation_status = "modifier_only".to_string();
                    safe_for_ai_context = true;
                } else if sig_lower.contains("benign") {
                    interpretation_status = "benign".to_string();
                    safe_for_ai_context = true;
                } else {
                    interpretation_status = "active_research".to_string();
                    safe_for_ai_context = true;
                }
            } else {
                interpretation_status = "active_research".to_string();
                safe_for_ai_context = true;
            }
        }
    } else {
        allele_match_status = "no_data".to_string();
        interpretation_status = "insufficient_evidence".to_string();
        safe_for_ai_context = false;
    }

    // Language guidelines
    let mut allowed_language = "This variant was resolved and validated.".to_string();
    let mut forbidden_language = vec![
        "diagnose".to_string(),
        "prescribe".to_string(),
        "dosage".to_string(),
        "cure".to_string(),
    ];

    if allele_match_status == "not_matched" {
        allowed_language = "ClinVar reports a clinically relevant risk allele at this rsID, but this genetic sample does not carry that allele.".to_string();
        forbidden_language.push("risk factor".to_string());
        forbidden_language.push("increased risk".to_string());
    } else if interpretation_status == "blocked_unverified" {
        allowed_language = "Clinical interpretation is blocked because the allele orientation (forward/reverse strand) has not been verified.".to_string();
        forbidden_language.push("pathogenic".to_string());
        forbidden_language.push("carrier".to_string());
    } else if interpretation_status == "active_clinical" {
        allowed_language = format!(
            "ClinVar reports pathogenic significance for allele {}. The sample carries genotype {}, which matches this risk factor. Clinical confirmation is required.",
            clinically_relevant.clone().unwrap_or_default(),
            user_genotype.clone().unwrap_or_default()
        );
    } else if interpretation_status == "active_research" {
        allowed_language = "This variant has research associations in PubMed but does not carry an active ClinVar clinical pathogenicity ranking.".to_string();
    } else if interpretation_status == "modifier_only" {
        allowed_language = "This variant acts as a mild phenotype modifier only and is not associated with direct disease pathogenicity.".to_string();
    }

    if is_high_stakes {
        notes.push("Gated high-stakes marker validation rules applied.".to_string());
    }

    Ok(VariantEvidence {
        rsid: rsid_upper,
        gene: resolved_gene,
        user_genotype,
        user_alleles,
        matched_effect_allele,
        clinically_relevant_allele: clinically_relevant,
        allele_match_status,
        orientation_status,
        genome_build: Some("GRCh38".to_string()),
        hgvs,
        clinvar_variation_id: clinvar_var_id,
        clinvar_clinical_significance: clinvar_sig,
        clinvar_review_status: clinvar_review,
        clinvar_condition,
        clinvar_conflict_status: None,
        evidence_source_ids: vec![],
        interpretation_status,
        allowed_language,
        forbidden_language,
        requires_clinical_confirmation: is_high_stakes,
        safe_for_ai_context,
        notes,
    })
}

// ---------------------------------------------------------------------------
// Incremental Discoveries SQLite Storage
// ---------------------------------------------------------------------------

pub fn should_persist_discovery(interpretation_status: &str) -> bool {
    matches!(
        interpretation_status,
        "active_clinical"
            | "active_research"
            | "mechanism_context_only"
            | "modifier_only"
            | "conflicting"
    )
}

pub fn append_discovered_findings_to_db(
    db_path: &Path,
    sample_id: i64,
    new_findings: Vec<VariantEvidence>,
) -> Result<usize, String> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let mut added_count = 0;

    for f in new_findings {
        let notes_str = serde_json::to_string(&f.notes).unwrap_or_default();
        let affected = conn
            .execute(
                "INSERT OR REPLACE INTO discovered_findings (
                sample_id, rsid, gene, user_genotype, allele_match_status, 
                orientation_status, clinvar_clinical_significance, clinvar_condition, 
                notes, interpretation_status
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    sample_id,
                    f.rsid.to_uppercase(),
                    f.gene,
                    f.user_genotype,
                    f.allele_match_status,
                    f.orientation_status,
                    f.clinvar_clinical_significance,
                    f.clinvar_condition,
                    notes_str,
                    f.interpretation_status
                ],
            )
            .map_err(|e| e.to_string())?;

        if affected > 0 {
            added_count += 1;
        }
    }

    Ok(added_count)
}

pub fn append_discovered_findings_to_path(
    path: &Path,
    new_findings: Vec<VariantEvidence>,
) -> Result<(), String> {
    let mut list = if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read custom export file: {}", e))?;
        serde_json::from_str::<Vec<VariantEvidence>>(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    for f in new_findings {
        if !list
            .iter()
            .any(|existing| existing.rsid.to_uppercase() == f.rsid.to_uppercase())
        {
            list.push(f);
        }
    }

    let serialized = serde_json::to_string_pretty(&list)
        .map_err(|e| format!("Failed to serialize findings: {}", e))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, serialized)
        .map_err(|e| format!("Failed to write findings to path: {}", e))?;

    Ok(())
}

pub fn get_db_discovered_findings(
    db_path: &Path,
    sample_id: i64,
) -> Result<Vec<VariantEvidence>, String> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT rsid, gene, user_genotype, allele_match_status, orientation_status, 
                clinvar_clinical_significance, clinvar_condition, notes, interpretation_status 
         FROM discovered_findings WHERE sample_id = ?",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![sample_id], |row| {
            let rsid: String = row.get(0)?;
            let gene: Option<String> = row.get(1)?;
            let user_genotype: Option<String> = row.get(2)?;
            let allele_match_status: String = row.get(3)?;
            let orientation_status: String = row.get(4)?;
            let clinvar_clinical_significance: Option<String> = row.get(5)?;
            let clinvar_condition: Option<String> = row.get(6)?;
            let notes_json: Option<String> = row.get(7)?;
            let interpretation_status: String = row.get(8)?;

            let notes: Vec<String> = notes_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            let user_alleles = user_genotype
                .as_ref()
                .map(|g| g.split('/').map(|s| s.to_string()).collect())
                .unwrap_or_default();

            Ok(VariantEvidence {
                rsid,
                gene,
                user_genotype,
                user_alleles,
                matched_effect_allele: None,
                clinically_relevant_allele: None,
                allele_match_status,
                orientation_status,
                genome_build: Some("GRCh38".to_string()),
                hgvs: None,
                clinvar_variation_id: None,
                clinvar_clinical_significance,
                clinvar_review_status: None,
                clinvar_condition,
                clinvar_conflict_status: None,
                evidence_source_ids: vec![],
                interpretation_status,
                allowed_language: "".to_string(),
                forbidden_language: vec![],
                requires_clinical_confirmation: false,
                safe_for_ai_context: true,
                notes,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }

    Ok(list)
}

// ---------------------------------------------------------------------------
// Stage 7: Deterministic Safety QA checks
// ---------------------------------------------------------------------------

fn detect_dosing_phrase(text: &str) -> Option<(String, String)> {
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower
        .split(|c: char| !c.is_alphanumeric() && c != '.')
        .collect();
    let units = vec![
        "mg",
        "mcg",
        "ug",
        "iu",
        "g",
        "milligrams",
        "micrograms",
        "grams",
        "units",
    ];

    for (idx, &word) in words.iter().enumerate() {
        if word.is_empty() {
            continue;
        }

        // Case 1: Word starts with digits and ends with a unit (e.g. "500mg")
        for &unit in &units {
            if word.ends_with(unit) && word.len() > unit.len() {
                let num_part = &word[..word.len() - unit.len()];
                if num_part.chars().any(|c| c.is_ascii_digit())
                    && num_part.chars().all(|c| c.is_ascii_digit() || c == '.')
                {
                    return Some((
                        word.to_string(),
                        format!("Dosing unit '{}' detected directly with quantity", unit),
                    ));
                }
            }
        }

        // Case 2: Word is a number and the next word is a unit (e.g. "500 mg")
        if word.chars().any(|c| c.is_ascii_digit())
            && word.chars().all(|c| c.is_ascii_digit() || c == '.')
            && idx + 1 < words.len()
        {
            let next_word = words[idx + 1];
            if units.contains(&next_word) {
                return Some((
                    format!("{} {}", word, next_word),
                    format!("Dosing unit '{}' detected after quantity", next_word),
                ));
            }
        }
    }
    None
}

fn has_safety_caveat(text: &str) -> bool {
    let t = text.to_lowercase();
    let has_professional = t.contains("medical professional")
        || t.contains("healthcare provider")
        || t.contains("clinician")
        || t.contains("doctor")
        || t.contains("physician")
        || t.contains("pharmacist");

    let has_warning_word = t.contains("harm")
        || t.contains("death")
        || t.contains("serious")
        || t.contains("dangerous")
        || t.contains("safety");

    has_professional && has_warning_word
}

fn has_citations(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("pmid")
        || t.contains("clinvar")
        || t.contains("chembl")
        || t.contains("clinicaltrials")
        || t.contains("doi:")
        || t.contains("source:")
        || t.contains("reference:")
        || t.contains("http://")
        || t.contains("https://")
        || (t.contains("[") && t.contains("]"))
}

pub fn run_deterministic_safety_checks(
    report_text: &str,
    variant_evidence_list: &[VariantEvidence],
) -> SafetyCheckResult {
    let mut findings = Vec::new();
    let text_lower = report_text.to_lowercase();

    let safety_caveat_present = has_safety_caveat(report_text);
    let citations_present = has_citations(report_text);

    // 1. Check for forbidden diagnosis claims
    let diagnosis_phrases = vec![
        (
            "you have",
            "Diagnosis Claim: Avoid absolute 'you have' statements without citations and safety disclaimers.",
        ),
        (
            "you suffer from",
            "Diagnosis Claim: Avoid declaring 'you suffer from' a disease without citations and safety disclaimers.",
        ),
        (
            "you are diagnosed with",
            "Diagnosis Claim: Avoid declaring 'you are diagnosed with' a disease without citations and safety disclaimers.",
        ),
        (
            "diagnose you",
            "Diagnosis Claim: Decline to diagnose the user without citations and safety disclaimers.",
        ),
        (
            "diagnose the",
            "Diagnosis Claim: Avoid making diagnostic assertions without citations and safety disclaimers.",
        ),
        (
            "your diagnosis",
            "Diagnosis Claim: Avoid referencing the user's diagnosis without citations and safety disclaimers.",
        ),
    ];

    for (phrase, msg) in diagnosis_phrases {
        if text_lower.contains(phrase) {
            let mut severity = "pass";
            let mut issue_message = String::new();
            if !citations_present && !safety_caveat_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Missing both source citations and medical validation caveat.",
                    msg
                );
            } else if !citations_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Must provide explicit source citations (PMID, ClinVar, etc.) to validate this diagnosis.",
                    msg
                );
            } else if !safety_caveat_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Must include the warning caveat that following a model without validating with a medical professional could cause serious harm or death.",
                    msg
                );
            }

            if severity == "fail" {
                let excerpt = if let Some(pos) = text_lower.find(phrase) {
                    let start = pos.saturating_sub(40);
                    let end = (pos + phrase.len() + 40).min(report_text.len());
                    Some(report_text[start..end].to_string())
                } else {
                    None
                };
                findings.push(SafetyFinding {
                    rule_id: "RULE_DIAGNOSIS_CLAIM".to_string(),
                    severity: "fail".to_string(),
                    message: issue_message,
                    excerpt,
                });
            }
        }
    }

    // 2. Check for treatment/dosage recommendations
    let treatment_phrases = vec![
        (
            "you should take",
            "Treatment Recommendation: Avoid recommending compounds without citations and safety disclaimers.",
        ),
        (
            "start taking",
            "Treatment Recommendation: Avoid telling the user to start taking compounds without citations and safety disclaimers.",
        ),
        (
            "stop taking",
            "Treatment Recommendation: Avoid telling the user to stop taking compounds without citations and safety disclaimers.",
        ),
        (
            "increase dose",
            "Dosage Recommendation: Avoid dosing adjustments without citations and safety disclaimers.",
        ),
        (
            "decrease dose",
            "Dosage Recommendation: Avoid dosing adjustments without citations and safety disclaimers.",
        ),
        (
            "prescribed",
            "Treatment Recommendation: Avoid referring to prescription recommendations without citations and safety disclaimers.",
        ),
        (
            "daily dose",
            "Dosage Recommendation: Avoid specifying daily dosage thresholds without citations and safety disclaimers.",
        ),
        (
            "recommended dose",
            "Dosage Recommendation: Avoid specifying recommended dose values without citations and safety disclaimers.",
        ),
    ];

    for (phrase, msg) in treatment_phrases {
        if text_lower.contains(phrase) {
            let mut severity = "pass";
            let mut issue_message = String::new();
            if !citations_present && !safety_caveat_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Missing both source citations and medical validation caveat.",
                    msg
                );
            } else if !citations_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Must provide explicit source citations (PMID, ClinVar, etc.) to validate this recommendation.",
                    msg
                );
            } else if !safety_caveat_present {
                severity = "fail";
                issue_message = format!(
                    "{}. Must include the warning caveat that following a model without validating with a medical professional could cause serious harm or death.",
                    msg
                );
            }

            if severity == "fail" {
                let excerpt = if let Some(pos) = text_lower.find(phrase) {
                    let start = pos.saturating_sub(40);
                    let end = (pos + phrase.len() + 40).min(report_text.len());
                    Some(report_text[start..end].to_string())
                } else {
                    None
                };
                findings.push(SafetyFinding {
                    rule_id: "RULE_TREATMENT_RECOMMENDATION".to_string(),
                    severity: "fail".to_string(),
                    message: issue_message,
                    excerpt,
                });
            }
        }
    }

    // Custom dosage quantity + unit detector
    if let Some((phrase_match, _detail)) = detect_dosing_phrase(report_text) {
        let mut severity = "pass";
        let mut issue_message = String::new();
        if !citations_present && !safety_caveat_present {
            severity = "fail";
            issue_message = format!(
                "Dosing Unit Detected ('{}'): Missing both source citations and medical validation caveat.",
                phrase_match
            );
        } else if !citations_present {
            severity = "fail";
            issue_message = format!(
                "Dosing Unit Detected ('{}'): Must provide explicit source citations (PMID, ClinVar, etc.) to validate this dosing.",
                phrase_match
            );
        } else if !safety_caveat_present {
            severity = "fail";
            issue_message = format!(
                "Dosing Unit Detected ('{}'): Must include the warning caveat that following a model without validating with a medical professional could cause serious harm or death.",
                phrase_match
            );
        }

        if severity == "fail" {
            let excerpt = if let Some(pos) = text_lower.find(&phrase_match.to_lowercase()) {
                let start = pos.saturating_sub(40);
                let end = (pos + phrase_match.len() + 40).min(report_text.len());
                Some(report_text[start..end].to_string())
            } else {
                None
            };
            findings.push(SafetyFinding {
                rule_id: "RULE_DOSING_MEASUREMENT".to_string(),
                severity: "fail".to_string(),
                message: issue_message,
                excerpt,
            });
        }
    }

    // 3. Absolute assertions
    let absolute_assertions = vec![
        (
            "guarantee",
            "Assertion Warning: Avoid absolute genetic guarantees.",
        ),
        (
            "prevent",
            "Assertion Warning: Avoid absolute prevention claims.",
        ),
        ("cure", "Assertion Warning: Avoid declaring genetic cures."),
        (
            "will develop",
            "Assertion Warning: Avoid absolute predictions like 'will develop'.",
        ),
    ];
    for (phrase, msg) in absolute_assertions {
        if text_lower.contains(phrase) {
            let excerpt = if let Some(pos) = text_lower.find(phrase) {
                let start = pos.saturating_sub(40);
                let end = (pos + phrase.len() + 40).min(report_text.len());
                Some(report_text[start..end].to_string())
            } else {
                None
            };
            findings.push(SafetyFinding {
                rule_id: "RULE_ABSOLUTE_ASSERTION".to_string(),
                severity: "warning".to_string(),
                message: msg.to_string(),
                excerpt,
            });
        }
    }

    // 4. Check if high-stakes variants with safe_for_ai_context = false are mentioned in report
    for v in variant_evidence_list {
        if !v.safe_for_ai_context && text_lower.contains(&v.rsid.to_lowercase()) {
            findings.push(SafetyFinding {
                    rule_id: "RULE_UNSAFE_VARIANT_MENTION".to_string(),
                    severity: "fail".to_string(),
                    message: format!(
                        "Forbidden Variant Reference: Variant {} is blocked/unverified, but is mentioned in the report context.",
                        v.rsid
                    ),
                    excerpt: None,
                });
        }
    }

    // 5. Require confirmation disclaimer if active clinical risk is mentioned
    let has_active_clinical = variant_evidence_list
        .iter()
        .any(|v| v.interpretation_status == "active_clinical");
    if has_active_clinical {
        let has_disclaimer = text_lower.contains("disclaimer")
            || text_lower.contains("medical advice")
            || text_lower.contains("educational purposes");
        if !has_disclaimer {
            findings.push(SafetyFinding {
                rule_id: "RULE_MISSING_DISCLAIMER".to_string(),
                severity: "fail".to_string(),
                message: "Missing Clinical Disclaimer: The report includes active clinical findings but lacks a safety/educational disclaimer.".to_string(),
                excerpt: None,
            });
        }
    }

    let mut passed = true;
    let mut max_severity = "pass".to_string();
    for f in &findings {
        if f.severity == "fail" {
            passed = false;
            max_severity = "fail".to_string();
        } else if f.severity == "warning" && max_severity != "fail" {
            max_severity = "warning".to_string();
        }
    }

    SafetyCheckResult {
        passed,
        severity: max_severity,
        findings,
    }
}
