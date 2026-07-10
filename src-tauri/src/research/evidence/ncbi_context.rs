// ./src-tauri/src/research/evidence/ncbi_context.rs
//! dbSNP Variation Service + ClinVar E-utilities for enrichment provenance.

use super::cache::{clinvar_ttl, dbsnp_ttl, fetch_json_cached};
use super::source_records;
use crate::research::state::NCBI_SEMAPHORE;
use crate::research::util::{normalize_rsid, unix_now};
use serde_json::{Value, json};
use std::path::Path;

const DBSNP_BASE: &str = "https://api.ncbi.nlm.nih.gov/variation/v0";
const EUTILS_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";

#[derive(Debug, Clone, Default)]
pub struct DbsnpContext {
    pub chromosome: Option<String>,
    pub position: Option<i64>,
    pub alleles: Option<String>,
    pub narrative: Option<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone, Default)]
pub struct ClinvarLiveContext {
    pub significance: Option<String>,
    pub condition: Option<String>,
    pub variation_id: Option<String>,
    pub review_status: Option<String>,
    pub narrative: Option<String>,
    pub provenance: Value,
}

pub async fn fetch_dbsnp_context(db_path: &Path, rsid: &str) -> DbsnpContext {
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());
    let numeric = rsid_norm.trim_start_matches("RS").trim_start_matches("rs");
    let url = format!("{}/refsnp/{}", DBSNP_BASE, numeric);
    let cache_key = format!("dbsnp|refsnp|{}", rsid_norm);

    let _permit = NCBI_SEMAPHORE.acquire().await.ok();

    let body = match fetch_json_cached(
        db_path,
        "dbsnp",
        "refsnp",
        &cache_key,
        &url,
        dbsnp_ttl(),
        None,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return DbsnpContext {
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                ..Default::default()
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "dbsnp",
            "refsnp",
            Some(&rsid_norm),
            None,
            None,
            Some(&url),
            &body,
        );
    }

    let (chromosome, position, alleles) = parse_refsnp_placements(&body);
    let hit = chromosome.is_some() || alleles.is_some();
    let narrative = if hit {
        Some(format!(
            "dbSNP {}: chr{} pos {} alleles {}",
            rsid_norm,
            chromosome.as_deref().unwrap_or("?"),
            position
                .map(|p| p.to_string())
                .unwrap_or_else(|| "?".into()),
            alleles.as_deref().unwrap_or("?")
        ))
    } else {
        None
    };

    DbsnpContext {
        chromosome,
        position,
        alleles,
        narrative,
        provenance: json!({
            "queried": true,
            "hit": hit,
            "fetched_at": unix_now(),
        }),
    }
}

pub async fn fetch_clinvar_live(
    db_path: &Path,
    rsid: &str,
    ncbi_api_key: Option<&str>,
) -> ClinvarLiveContext {
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());

    if let Ok(conn) = crate::db::connect(db_path)
        && let Some(local) = crate::offline::lookup_clinvar_local(&conn, &rsid_norm)
    {
        return crate::offline::clinvar_local_to_live_context(&local);
    }

    let mut search_url = format!(
        "{}esearch.fcgi?db=clinvar&term={}&retmode=json",
        EUTILS_BASE, rsid_norm
    );
    if let Some(key) = ncbi_api_key.filter(|k| !k.trim().is_empty()) {
        search_url.push_str(&format!("&api_key={}", key.trim()));
    }

    let _permit = NCBI_SEMAPHORE.acquire().await.ok();

    let search_key = format!("clinvar|esearch|{}", rsid_norm);
    let search_val = match fetch_json_cached(
        db_path,
        "clinvar",
        "esearch",
        &search_key,
        &search_url,
        clinvar_ttl(),
        ncbi_api_key,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return ClinvarLiveContext {
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                ..Default::default()
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "clinvar",
            "esearch",
            Some(&rsid_norm),
            None,
            None,
            Some(&search_url),
            &search_val,
        );
    }

    let id_str = search_val["esearchresult"]["idlist"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if id_str.is_empty() {
        return ClinvarLiveContext {
            provenance: json!({ "queried": true, "hit": false, "ids": 0 }),
            ..Default::default()
        };
    }

    let mut summary_url = format!(
        "{}esummary.fcgi?db=clinvar&id={}&retmode=json",
        EUTILS_BASE, id_str
    );
    if let Some(key) = ncbi_api_key.filter(|k| !k.trim().is_empty()) {
        summary_url.push_str(&format!("&api_key={}", key.trim()));
    }

    let summary_key = format!("clinvar|esummary|{}", id_str);
    let summary_val = match fetch_json_cached(
        db_path,
        "clinvar",
        "esummary",
        &summary_key,
        &summary_url,
        clinvar_ttl(),
        ncbi_api_key,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return ClinvarLiveContext {
                variation_id: Some(id_str),
                provenance: json!({ "queried": true, "hit": false, "error": e }),
                ..Default::default()
            };
        }
    };

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "clinvar",
            "esummary",
            Some(&rsid_norm),
            None,
            None,
            Some(&summary_url),
            &summary_val,
        );
    }

    let summary = summary_val["result"][&id_str].clone();
    let significance = summary["clinical_significance"]
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);
    let review_status = summary["clinical_significance"]
        .get("review_status")
        .and_then(|v| v.as_str())
        .map(String::from);
    let condition = summary["trait_set"]
        .as_array()
        .map(|traits| {
            traits
                .iter()
                .filter_map(|t| t["trait_name"].as_str().map(String::from))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty());

    let narrative = significance.as_ref().map(|sig| {
        format!(
            "ClinVar (E-utilities): {} — {}{}",
            sig,
            condition.as_deref().unwrap_or("condition unknown"),
            review_status
                .as_ref()
                .map(|r| format!(" ({})", r))
                .unwrap_or_default()
        )
    });

    ClinvarLiveContext {
        significance,
        condition,
        variation_id: Some(id_str),
        review_status,
        narrative,
        provenance: json!({
            "queried": true,
            "hit": true,
            "endpoint": "eutils",
            "fetched_at": unix_now(),
        }),
    }
}

fn parse_refsnp_placements(body: &Value) -> (Option<String>, Option<i64>, Option<String>) {
    let placements = body
        .get("primary_snapshot_data")
        .and_then(|v| v.get("placements_with_allele"))
        .and_then(|v| v.as_array());

    let Some(placements) = placements else {
        return (None, None, None);
    };

    for placement in placements {
        let seq_id = placement["seq_id"].as_str().unwrap_or("");
        if !seq_id.contains("NC_") && !seq_id.starts_with("chr") {
            continue;
        }
        let chromosome = seq_id_to_chr(seq_id);
        let mut alleles: Vec<String> = Vec::new();
        let mut position: Option<i64> = None;

        if let Some(arr) = placement["alleles"].as_array() {
            for allele in arr {
                if let Some(spdi) = allele.get("allele").and_then(|a| a.get("spdi")) {
                    if position.is_none() {
                        position = spdi["position"].as_i64();
                    }
                    if let Some(seq) = spdi["deleted_sequence"].as_str()
                        && !seq.is_empty()
                        && !alleles.contains(&seq.to_string())
                    {
                        alleles.push(seq.to_string());
                    }
                    if let Some(seq) = spdi["inserted_sequence"].as_str()
                        && !seq.is_empty()
                        && !alleles.contains(&seq.to_string())
                    {
                        alleles.push(seq.to_string());
                    }
                }
                if let Some(spdi) = allele.get("spdi")
                    && position.is_none()
                {
                    position = spdi["position"].as_i64();
                }
            }
        }

        if placement["placement_annot"].is_object() && position.is_none() {
            position = placement["placement_annot"]["seq_id_traits_assembly"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|t| t["seq_id_trait"].as_array())
                .and_then(|a| a.first())
                .and_then(|t| t["base"].as_i64());
        }

        let allele_str = if alleles.is_empty() {
            placement["allele_string"].as_str().map(String::from)
        } else {
            Some(alleles.join("/"))
        };

        if chromosome.is_some() || position.is_some() {
            return (chromosome, position, allele_str);
        }
    }

    (None, None, None)
}

fn seq_id_to_chr(seq_id: &str) -> Option<String> {
    if seq_id.starts_with("chr") {
        return Some(seq_id.trim_start_matches("chr").to_string());
    }
    if let Some(rest) = seq_id.strip_prefix("NC_") {
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if num == "000023" {
            return Some("X".into());
        }
        if num == "000024" {
            return Some("Y".into());
        }
        if num == "000012" {
            return Some("MT".into());
        }
        let trimmed = num.trim_start_matches('0');
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}
