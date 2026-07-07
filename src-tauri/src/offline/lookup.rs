// ./src-tauri/src/offline/lookup.rs
//! Local-first lookups used by enrichment before live APIs.

use crate::research::evidence::ncbi_context::ClinvarLiveContext;
use crate::research::util::normalize_rsid;
use rusqlite::{params, Connection};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct ClinvarLocalRecord {
    pub rsid: String,
    pub gene: Option<String>,
    pub significance: String,
    pub conditions: String,
    pub relevant_allele: String,
    pub review_status: String,
    pub variation_id: Option<String>,
}

pub fn lookup_clinvar_local(conn: &Connection, rsid: &str) -> Option<ClinvarLocalRecord> {
    let rsid_norm = normalize_rsid(rsid)?;
    conn.query_row(
        "SELECT rsid, gene, clinical_significance, conditions, relevant_allele,
                review_status, variation_id
         FROM clinvar_reference WHERE UPPER(rsid) = UPPER(?)",
        params![rsid_norm],
        |row| {
            Ok(ClinvarLocalRecord {
                rsid: row.get(0)?,
                gene: row.get(1)?,
                significance: row.get(2)?,
                conditions: row.get(3)?,
                relevant_allele: row.get(4)?,
                review_status: row.get(5)?,
                variation_id: row.get(6)?,
            })
        },
    )
    .ok()
    .filter(|r| !r.significance.is_empty() || !r.conditions.is_empty())
}

pub fn clinvar_local_to_live_context(rec: &ClinvarLocalRecord) -> ClinvarLiveContext {
    let sig = if rec.significance.is_empty() {
        None
    } else {
        Some(rec.significance.clone())
    };
    let narrative = match (&sig, &rec.conditions) {
        (Some(s), c) if !c.is_empty() => Some(format!(
            "ClinVar (local): {} — {} [{}]",
            rec.rsid, s, c
        )),
        (Some(s), _) => Some(format!("ClinVar (local): {} — {}", rec.rsid, s)),
        _ => None,
    };
    ClinvarLiveContext {
        significance: sig,
        condition: if rec.conditions.is_empty() {
            None
        } else {
            Some(rec.conditions.clone())
        },
        variation_id: rec.variation_id.clone(),
        review_status: if rec.review_status.is_empty() {
            None
        } else {
            Some(rec.review_status.clone())
        },
        narrative,
        provenance: json!({
            "queried": true,
            "hit": true,
            "source": "clinvar_reference_local",
            "local": true,
        }),
    }
}

pub fn lookup_pharmgkb_local(conn: &Connection, rsid: &str) -> (Vec<String>, u32, bool) {
    let rsid_norm = normalize_rsid(rsid).unwrap_or_else(|| rsid.to_uppercase());
    let mut stmt = match conn.prepare(
        "SELECT drug, phenotype, evidence_level FROM pharmgkb_clinical_variants WHERE UPPER(rsid) = UPPER(?) LIMIT 20",
    ) {
        Ok(s) => s,
        Err(_) => return (vec![], 0, false),
    };
    let rows = match stmt.query_map(params![rsid_norm], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return (vec![], 0, false),
    };

    let mut drugs = Vec::new();
    let mut count = 0u32;
    for row in rows.flatten() {
        count += 1;
        if let Some(d) = row.0
            && !drugs.contains(&d) {
                drugs.push(d);
            }
    }
    (drugs, count, count > 0)
}

pub fn lookup_mane_transcript(conn: &Connection, gene: &str) -> Option<String> {
    conn.query_row(
        "SELECT ensembl_transcript FROM mane_transcripts WHERE UPPER(gene_symbol) = UPPER(?) LIMIT 1",
        params![gene],
        |row| row.get(0),
    )
    .ok()
    .flatten()
}

pub fn lookup_clingen_validity(conn: &Connection, gene: &str) -> Vec<String> {
    let mut stmt = match conn.prepare(
        "SELECT disease_label, classification FROM clingen_gene_validity WHERE UPPER(gene_symbol) = UPPER(?) LIMIT 5",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = match stmt.query_map(params![gene], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    }) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    rows.flatten()
        .map(|(disease, class)| {
            if let Some(c) = class {
                format!("{disease} ({c})")
            } else {
                disease
            }
        })
        .collect()
}

pub fn resolve_rsid_alias(conn: &Connection, rsid: &str) -> Option<String> {
    let rsid_norm = normalize_rsid(rsid)?;
    conn.query_row(
        "SELECT merged_into, withdrawn FROM rsid_aliases WHERE UPPER(rsid) = UPPER(?)",
        params![rsid_norm],
        |row| {
            let withdrawn: i64 = row.get(1)?;
            if withdrawn != 0 {
                return Ok(None);
            }
            row.get::<_, Option<String>>(0)
        },
    )
    .ok()
    .flatten()
    .and_then(|s| normalize_rsid(&s))
}

pub fn lookup_variant_locus_local(
    conn: &Connection,
    rsid: &str,
) -> Option<(String, i64, String, String)> {
    let rsid_norm = normalize_rsid(rsid)?;
    conn.query_row(
        "SELECT chrom, pos, ref_allele, alt_allele FROM variant_locus WHERE UPPER(rsid) = UPPER(?) LIMIT 1",
        params![rsid_norm],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    )
    .ok()
}

pub fn offline_clinvar_available(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM clinvar_reference",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

pub fn offline_pharmgkb_available(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM pharmgkb_clinical_variants",
        [],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}
