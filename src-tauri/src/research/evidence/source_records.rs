// ./src-tauri/src/research/evidence/source_records.rs
//! Persist raw source API responses into `source_records` for provenance and packet export.

use crate::research::util::{string_to_u64, unix_now};
use rusqlite::{Connection, params};
use serde_json::Value;

pub const PARSER_VERSION: &str = "1.0.0";

pub fn payload_hash(body: &str) -> String {
    format!("{:016x}", string_to_u64(body))
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_source_record(
    conn: &Connection,
    source_name: &str,
    endpoint_family: &str,
    rsid: Option<&str>,
    gene_symbol: Option<&str>,
    trait_name: Option<&str>,
    study_accession: Option<&str>,
    pubmed_id: Option<&str>,
    source_url: Option<&str>,
    raw_body: &str,
    source_entity_id: Option<&str>,
) -> Result<String, String> {
    let hash = payload_hash(raw_body);
    let now = unix_now();
    let record_id = format!(
        "sr_{}_{}",
        source_name,
        string_to_u64(&format!(
            "{}|{}|{}",
            rsid.unwrap_or(""),
            endpoint_family,
            hash
        ))
    );

    let existing: Option<String> = conn
        .query_row(
            "SELECT source_record_id FROM source_records
             WHERE source_name = ? AND endpoint_family = ? AND raw_payload_hash = ?",
            params![source_name, endpoint_family, hash],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        return Ok(id);
    }

    conn.execute(
        "INSERT OR REPLACE INTO api_cache_db.source_records (
            source_record_id, source_name, endpoint_family, source_entity_type,
            source_entity_id, rsid, gene_symbol, trait_name, study_accession,
            pubmed_id, source_url, fetched_at, raw_payload_hash, parser_version,
            schema_version_seen, record_quality_flags_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            record_id,
            source_name,
            endpoint_family,
            "variant",
            source_entity_id,
            rsid,
            gene_symbol,
            trait_name,
            study_accession,
            pubmed_id,
            source_url,
            now,
            hash,
            PARSER_VERSION,
            super::types::EVIDENCE_SCHEMA_VERSION,
            "[]",
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(record_id)
}

#[allow(clippy::too_many_arguments)]
pub fn record_generic_json(
    conn: &Connection,
    source_name: &str,
    endpoint_family: &str,
    rsid: Option<&str>,
    gene_symbol: Option<&str>,
    trait_name: Option<&str>,
    source_url: Option<&str>,
    body: &Value,
) -> Result<String, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    upsert_source_record(
        conn,
        source_name,
        endpoint_family,
        rsid,
        gene_symbol,
        trait_name,
        None,
        None,
        source_url,
        &body_str,
        rsid,
    )
}

pub fn record_gwas_api_response(
    conn: &Connection,
    rsid: &str,
    url: &str,
    body: &Value,
) -> Result<Vec<String>, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let mut ids = Vec::new();

    if let Some(assocs) = body["_embedded"]["associations"].as_array() {
        for assoc in assocs.iter().take(12) {
            let trait_name = assoc["trait"]["trait"].as_str().or_else(|| {
                assoc["efoTraits"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|t| t["trait"].as_str())
            });
            let study = assoc["studyAccession"].as_str();
            let assoc_str = serde_json::to_string(assoc).unwrap_or_default();
            let id = upsert_source_record(
                conn,
                "gwas_catalog",
                "associations",
                Some(rsid),
                None,
                trait_name,
                study,
                None,
                Some(url),
                &assoc_str,
                study,
            )?;
            ids.push(id);
        }
    }

    if ids.is_empty() {
        let id = upsert_source_record(
            conn,
            "gwas_catalog",
            "associations",
            Some(rsid),
            None,
            None,
            None,
            None,
            Some(url),
            &body_str,
            None,
        )?;
        ids.push(id);
    }

    Ok(ids)
}

pub fn record_clinvar_local(
    conn: &Connection,
    rsid: &str,
    significance: &str,
    gene: Option<&str>,
) -> Result<String, String> {
    let body = serde_json::json!({
        "rsid": rsid,
        "clinical_significance": significance,
        "gene": gene,
        "source": "clinvar_reference_local",
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    upsert_source_record(
        conn,
        "clinvar",
        "local_reference",
        Some(rsid),
        gene,
        None,
        None,
        None,
        None,
        &body_str,
        Some(rsid),
    )
}

pub fn record_ensembl_vep(
    conn: &Connection,
    rsid: &str,
    url: &str,
    body: &Value,
) -> Result<String, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    upsert_source_record(
        conn,
        "ensembl",
        "vep",
        Some(rsid),
        body.as_array()
            .and_then(|a| a.first())
            .and_then(|e| e["transcript_consequences"].as_array())
            .and_then(|tc| tc.first())
            .and_then(|t| t["gene_symbol"].as_str()),
        None,
        None,
        None,
        Some(url),
        &body_str,
        Some(rsid),
    )
}

pub fn record_pubmed_search(
    conn: &Connection,
    rsid: &str,
    url: &str,
    body: &Value,
) -> Result<String, String> {
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    upsert_source_record(
        conn,
        "pubmed",
        "esearch",
        Some(rsid),
        None,
        None,
        None,
        None,
        Some(url),
        &body_str,
        Some(rsid),
    )
}

pub fn list_source_records_for_rsid(
    conn: &Connection,
    rsid: &str,
    limit: u32,
) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT source_record_id, source_name, endpoint_family, rsid, gene_symbol,
                    trait_name, study_accession, pubmed_id, source_url, fetched_at,
                    raw_payload_hash, parser_version
             FROM source_records WHERE UPPER(rsid) = UPPER(?) ORDER BY fetched_at DESC LIMIT ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![rsid, limit], |row| {
            Ok(serde_json::json!({
                "source_record_id": row.get::<_, String>(0)?,
                "source_name": row.get::<_, String>(1)?,
                "endpoint_family": row.get::<_, String>(2)?,
                "rsid": row.get::<_, Option<String>>(3)?,
                "gene_symbol": row.get::<_, Option<String>>(4)?,
                "trait_name": row.get::<_, Option<String>>(5)?,
                "study_accession": row.get::<_, Option<String>>(6)?,
                "pubmed_id": row.get::<_, Option<String>>(7)?,
                "source_url": row.get::<_, Option<String>>(8)?,
                "fetched_at": row.get::<_, i64>(9)?,
                "raw_payload_hash": row.get::<_, String>(10)?,
                "parser_version": row.get::<_, Option<String>>(11)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn source_record_count(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM source_records", [], |row| row.get(0))
        .map_err(|e| e.to_string())
}
