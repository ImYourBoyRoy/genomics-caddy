// ./src-tauri/src/research/promote.rs
/*
Purpose: Auto-promote high-confidence vector-enriched GWAS hits into SQLite for trait report display.
Writes vector_promoted_findings and syncs discovered_findings for banner/report integration.
*/

use super::enrich::PreparedEnrichment;
use super::util::{best_gwas_pvalue, unix_now};
use rusqlite::{Connection, params};
use std::path::Path;

const PROMOTION_SCORE_MIN: f32 = 0.4;
const PROMOTION_PVALUE_MAX: f64 = 5e-8;

pub fn is_curated_marker_rsid(conn: &Connection, rsid: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM evidence_library WHERE LOWER(rsid) = LOWER(?) LIMIT 1",
        params![rsid],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn eligible_for_vector_promotion(payload: &serde_json::Value) -> bool {
    let has_gwas = payload["has_gwas"].as_bool().unwrap_or(false);
    if !has_gwas {
        return false;
    }
    let score = payload["significance_score"].as_f64().unwrap_or(0.0) as f32;
    if score >= PROMOTION_SCORE_MIN {
        return true;
    }
    best_gwas_pvalue(
        payload["gwas_associations"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[]),
    )
    .map(|p| p < PROMOTION_PVALUE_MAX)
    .unwrap_or(false)
}

pub fn maybe_promote_vector_finding(
    db_path: &Path,
    sample_id: i64,
    prepared: &PreparedEnrichment,
) -> Result<bool, String> {
    if !eligible_for_vector_promotion(&prepared.payload) {
        return Ok(false);
    }

    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    if is_curated_marker_rsid(&conn, &prepared.rsid) {
        return Ok(false);
    }

    let p = &prepared.payload;
    let gene = p["gene_symbol"].as_str().filter(|s| !s.is_empty());
    let genotype = p["genotype"].as_str().unwrap_or("");
    let trait_summary = p["gwas_traits"]
        .as_str()
        .or_else(|| p["gwas_trait"].as_str())
        .unwrap_or("")
        .to_string();
    let trait_categories = p
        .get("trait_categories")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let categories_json =
        serde_json::to_string(&trait_categories).unwrap_or_else(|_| "[]".to_string());
    let score = p["significance_score"].as_f64().unwrap_or(0.0) as f32;
    let best_p = best_gwas_pvalue(
        p["gwas_associations"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[]),
    );
    let enrichment_version = p["enrichment_version"].as_str().unwrap_or("4.1");
    let clinvar = p["clinvar_significance"].as_str();
    let now = unix_now();

    let notes = format!(
        "Vector research auto-promotion (v{}). Traits: {}. Score: {:.2}.",
        enrichment_version, trait_summary, score
    );

    conn.execute(
        "INSERT OR REPLACE INTO vector_promoted_findings (
            sample_id, rsid, gene, user_genotype, trait_summary, trait_categories,
            significance_score, gwas_best_pvalue, enrichment_version, promoted_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            sample_id,
            prepared.rsid,
            gene,
            genotype,
            trait_summary,
            categories_json,
            score,
            best_p,
            enrichment_version,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO discovered_findings (
            sample_id, rsid, gene, user_genotype, allele_match_status, orientation_status,
            clinvar_clinical_significance, clinvar_condition, notes, interpretation_status
         ) VALUES (?, ?, ?, ?, 'vector_research', 'verified', ?, ?, ?, 'active_research')",
        params![
            sample_id,
            prepared.rsid,
            gene,
            genotype,
            clinvar,
            trait_summary,
            notes,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(true)
}

pub(crate) fn promote_enrichment_batch(
    db_path: &Path,
    sample_id: i64,
    batch: &[PreparedEnrichment],
) -> u32 {
    let mut promoted = 0u32;
    for prepared in batch {
        if maybe_promote_vector_finding(db_path, sample_id, prepared).unwrap_or(false) {
            promoted += 1;
        }
    }
    promoted
}
