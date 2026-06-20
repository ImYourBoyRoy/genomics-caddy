// ./src-tauri/src/research/evidence/pgs_match.rs
//! PGS Catalog scoring file download, cache, and personal match rate.

use super::cache::{fetch_text_cached, pgs_file_ttl};
use super::scoring::compute_personal_direction;
use crate::research::util::{normalize_rsid, unix_now};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgsMatchSummary {
    pub pgs_id: String,
    pub trait_reported: Option<String>,
    pub total_variants: u32,
    pub matched_variants: u32,
    pub missing_variants: u32,
    pub ambiguous_variants: u32,
    pub excluded_variants: u32,
    pub match_rate_pct: f32,
    pub variant_allele_match: bool,
    pub effect_allele: Option<String>,
    pub effect_weight: Option<f64>,
    pub personal_dosage: Option<i32>,
    pub personal_direction: String,
    pub scoring_file_url: Option<String>,
}

pub async fn compute_pgs_match_for_score(
    db_path: &Path,
    sample_id: i64,
    pgs_id: &str,
    rsid: &str,
    allele1: &str,
    allele2: &str,
    score_meta: &Value,
) -> Result<PgsMatchSummary, String> {
    let trait_reported = score_meta["trait_reported"]
        .as_str()
        .map(String::from);
    let file_url = score_meta["ftp_scoring_file"]
        .as_str()
        .or_else(|| {
            score_meta["harmonized_scoring_files"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|f| f["location"].as_str())
        })
        .map(String::from);

    let Some(file_url) = file_url else {
        return Ok(variant_only_summary(
            pgs_id,
            trait_reported,
            rsid,
            allele1,
            allele2,
            None,
            None,
        ));
    };

    let file_key = format!("pgs_catalog|file|{}|{}", pgs_id, super::source_records::payload_hash(&file_url));
    let file_text = fetch_text_cached(
        db_path,
        "pgs_catalog",
        "scoring_file",
        &file_key,
        &file_url,
        pgs_file_ttl(),
    )
    .await?;

    let cache_dir = db_path
        .parent()
        .map(|p| p.join("pgs_cache"))
        .unwrap_or_else(|| PathBuf::from("pgs_cache"));
    let _ = std::fs::create_dir_all(&cache_dir);
    let local_path = cache_dir.join(format!("{}.txt.gz", pgs_id));
    if !local_path.exists() {
        let _ = std::fs::write(local_path.with_extension("txt"), &file_text);
    }

    let rows = parse_scoring_file(&file_text);
    let total = rows.len() as u32;
    if total == 0 {
        return Ok(variant_only_summary(
            pgs_id,
            trait_reported,
            rsid,
            allele1,
            allele2,
            Some(file_url),
            None,
        ));
    }

    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let user_genotypes = load_user_genotypes_for_rsids(&conn, sample_id, &rows)?;

    let mut matched = 0u32;
    let mut missing = 0u32;
    let mut ambiguous = 0u32;
    let mut excluded = 0u32;
    let mut this_variant_match = false;
    let mut this_effect_allele: Option<String> = None;
    let mut this_weight: Option<f64> = None;
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());

    for row in &rows {
        let Some(row_rsid) = row.rsid.as_deref() else {
            excluded += 1;
            continue;
        };
        let Some((a1, a2)) = user_genotypes.get(&row_rsid.to_uppercase()) else {
            missing += 1;
            continue;
        };
        let (direction, _dosage, _) = compute_personal_direction(
            &format!("{}/{}", a1, a2),
            row.effect_allele.as_deref(),
            None,
            row.effect_weight,
            None,
        );
        if direction == "unknown" && row.effect_allele.is_some() {
            ambiguous += 1;
        } else if direction == "no_personal_match" {
            missing += 1;
        } else if direction != "unknown" {
            matched += 1;
        } else {
            ambiguous += 1;
        }

        if row_rsid.eq_ignore_ascii_case(&rsid_norm) {
            this_variant_match = direction != "no_personal_match" && direction != "unknown";
            this_effect_allele = row.effect_allele.clone();
            this_weight = row.effect_weight;
        }
    }

    let (personal_direction, personal_dosage, _) = compute_personal_direction(
        &format!("{}/{}", allele1, allele2),
        this_effect_allele.as_deref(),
        None,
        this_weight,
        None,
    );

    let match_rate = if total > 0 {
        (matched as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    Ok(PgsMatchSummary {
        pgs_id: pgs_id.to_string(),
        trait_reported,
        total_variants: total,
        matched_variants: matched,
        missing_variants: missing,
        ambiguous_variants: ambiguous,
        excluded_variants: excluded,
        match_rate_pct: match_rate,
        variant_allele_match: this_variant_match,
        effect_allele: this_effect_allele,
        effect_weight: this_weight,
        personal_dosage,
        personal_direction,
        scoring_file_url: Some(file_url),
    })
}

struct ScoringRow {
    rsid: Option<String>,
    effect_allele: Option<String>,
    effect_weight: Option<f64>,
}

fn parse_scoring_file(text: &str) -> Vec<ScoringRow> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 {
            continue;
        }
        // Common formats: chr pos rsID effect other weight  OR  chr pos effect other weight rsID
        let rsid = cols
            .iter()
            .find(|c| c.starts_with("rs") || c.starts_with("RS"))
            .map(|s| s.to_string());
        let effect_idx = cols.iter().position(|c| {
            c.len() == 1 && c.chars().all(|ch| ch.is_ascii_alphabetic())
        });
        let (effect_allele, weight) = if let Some(idx) = effect_idx {
            let effect = cols.get(idx).map(|s| s.to_string());
            let weight = cols.get(idx + 2).and_then(|s| s.parse::<f64>().ok());
            (effect, weight)
        } else {
            (None, None)
        };
        rows.push(ScoringRow {
            rsid,
            effect_allele,
            effect_weight: weight,
        });
    }
    rows
}

fn load_user_genotypes_for_rsids(
    conn: &Connection,
    sample_id: i64,
    rows: &[ScoringRow],
) -> Result<HashMap<String, (String, String)>, String> {
    let rsids: Vec<String> = rows
        .iter()
        .filter_map(|r| r.rsid.as_ref())
        .map(|r| r.to_uppercase())
        .collect();
    if rsids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::new();
    // Batch in chunks of 400 to avoid SQLite variable limits
    for chunk in rsids.chunks(400) {
        let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT rsid, allele1, allele2 FROM genotypes WHERE sample_id = ? AND UPPER(rsid) IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(sample_id)];
        for r in chunk {
            params_vec.push(Box::new(r.clone()));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            map.insert(row.0.to_uppercase(), (row.1, row.2));
        }
    }
    Ok(map)
}

fn variant_only_summary(
    pgs_id: &str,
    trait_reported: Option<String>,
    _rsid: &str,
    _a1: &str,
    _a2: &str,
    file_url: Option<String>,
    weight: Option<f64>,
) -> PgsMatchSummary {
    PgsMatchSummary {
        pgs_id: pgs_id.to_string(),
        trait_reported,
        total_variants: 0,
        matched_variants: 0,
        missing_variants: 0,
        ambiguous_variants: 0,
        excluded_variants: 0,
        match_rate_pct: 0.0,
        variant_allele_match: false,
        effect_allele: None,
        effect_weight: weight,
        personal_dosage: None,
        personal_direction: "unknown".into(),
        scoring_file_url: file_url,
    }
}

pub fn store_pgs_match_cache(conn: &Connection, sample_id: i64, summary: &PgsMatchSummary) -> Result<(), String> {
    let now = unix_now();
    conn.execute(
        "INSERT OR REPLACE INTO pgs_match_cache (
            sample_id, pgs_id, trait_reported, total_variants, matched_variants,
            missing_variants, ambiguous_variants, match_rate_pct, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            sample_id,
            summary.pgs_id,
            summary.trait_reported,
            summary.total_variants,
            summary.matched_variants,
            summary.missing_variants,
            summary.ambiguous_variants,
            summary.match_rate_pct,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
