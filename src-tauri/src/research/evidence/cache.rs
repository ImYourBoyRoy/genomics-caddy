// ./src-tauri/src/research/evidence/cache.rs
//! Layered API cache using `api_cache_entries` (cross-sample, keyed by source + entity).

use crate::research::http::{ensure_research_outbound_url, HTTP_CLIENT};
use crate::research::sweep_metrics::{phase_for_api_source, record_phase_cache};
use crate::research::util::{string_to_u64, unix_now};
use rusqlite::params;
use serde_json::Value;
use std::path::Path;

const GWAS_TTL_SECS: i64 = 30 * 24 * 3600;
const ENSEMBL_TTL_SECS: i64 = 90 * 24 * 3600;
const CLINVAR_TTL_SECS: i64 = 21 * 24 * 3600;
const PGS_TTL_SECS: i64 = 60 * 24 * 3600;
const PGS_FILE_TTL_SECS: i64 = 90 * 24 * 3600;
const REACTOME_TTL_SECS: i64 = 90 * 24 * 3600;
const OPENTARGETS_TTL_SECS: i64 = 60 * 24 * 3600;
const PHARMGKB_TTL_SECS: i64 = 30 * 24 * 3600;
const OLS4_TTL_SECS: i64 = 90 * 24 * 3600;
const DBSNP_TTL_SECS: i64 = 90 * 24 * 3600;
const GTEX_TTL_SECS: i64 = 30 * 24 * 3600;
const PUBMED_TTL_SECS: i64 = 14 * 24 * 3600;

pub fn gwas_cache_key(rsid: &str) -> String {
    format!("gwas_catalog|{}", rsid.to_uppercase())
}

pub fn ensembl_vep_cache_key(rsid: &str) -> String {
    format!("ensembl|vep|{}", rsid.to_uppercase())
}

pub fn clinvar_cache_key(rsid: &str) -> String {
    format!("clinvar|{}", rsid.to_uppercase())
}

pub async fn fetch_json_post_cached(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    query: &str,
    variables: &Value,
    ttl_secs: i64,
) -> Result<Value, String> {
    let body_hash = super::source_records::payload_hash(&format!("{}|{}", query, variables));
    let full_key = format!("{}|{}", cache_key, &body_hash[..8.min(body_hash.len())]);

    if let Some(cached) = read_fresh_cache(db_path, &full_key, ttl_secs)? {
        return Ok(cached);
    }

    ensure_research_outbound_url(url)?;

    let payload = serde_json::json!({ "query": query, "variables": variables });
    let mut last_err = String::new();
    let mut body: Option<Value> = None;
    let mut status_code = 0i32;
    for attempt in 0..3u32 {
        let res = HTTP_CLIENT
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("{} POST failed: {}", source_name, e))?;
        status_code = res.status().as_u16() as i32;
        if res.status().is_success() {
            body = Some(
                res.json()
                    .await
                    .map_err(|e| format!("{} parse error: {}", source_name, e))?,
            );
            break;
        }
        last_err = format!("HTTP {}", status_code);
        if status_code == 429 || status_code >= 500 {
            tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt))).await;
            continue;
        }
        break;
    }

    let Some(body) = body else {
        write_error_cache(db_path, source_name, endpoint_family, &full_key, url, status_code)?;
        return Err(format!("{} returned {}", source_name, last_err));
    };

    write_fresh_cache(
        db_path,
        source_name,
        endpoint_family,
        &full_key,
        url,
        status_code,
        &body,
    )?;
    Ok(body)
}

pub async fn fetch_text_cached(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    ttl_secs: i64,
) -> Result<String, String> {
    if let Some(cached) = read_fresh_text_cache(db_path, cache_key, ttl_secs)? {
        if let Some(phase) = phase_for_api_source(source_name) {
            record_phase_cache(phase, true);
        }
        return Ok(cached);
    }
    if let Some(phase) = phase_for_api_source(source_name) {
        record_phase_cache(phase, false);
    }

    ensure_research_outbound_url(url)?;

    let mut last_err = String::new();
    let mut text: Option<String> = None;
    let mut status_code = 0i32;
    for attempt in 0..3u32 {
        let res = HTTP_CLIENT
            .get(url)
            .send()
            .await
            .map_err(|e| format!("{} request failed: {}", source_name, e))?;
        status_code = res.status().as_u16() as i32;
        if res.status().is_success() {
            text = Some(
                res.text()
                    .await
                    .map_err(|e| format!("{} text error: {}", source_name, e))?,
            );
            break;
        }
        last_err = format!("HTTP {}", status_code);
        if status_code == 429 || status_code >= 500 {
            tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt))).await;
            continue;
        }
        break;
    }

    let Some(text) = text else {
        write_error_cache(db_path, source_name, endpoint_family, cache_key, url, status_code)?;
        return Err(format!("{} returned {}", source_name, last_err));
    };

    write_fresh_text_cache(
        db_path,
        source_name,
        endpoint_family,
        cache_key,
        url,
        status_code,
        &text,
        ttl_secs,
    )?;
    Ok(text)
}

fn read_fresh_text_cache(db_path: &Path, cache_key: &str, ttl_secs: i64) -> Result<Option<String>, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let row: Option<(String, i64, String)> = conn
        .query_row(
            "SELECT response_body, fetched_at, cache_status FROM api_cache_entries
             WHERE normalized_cache_key = ?",
            params![cache_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();
    let Some((body, fetched_at, status)) = row else {
        return Ok(None);
    };
    if status.as_str() != "fresh" || unix_now() - fetched_at > ttl_secs {
        return Ok(None);
    }
    Ok(Some(body))
}

fn write_fresh_text_cache(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    status: i32,
    text: &str,
    ttl_secs: i64,
) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let now = unix_now();
    let cache_id = format!("cache_{}", string_to_u64(cache_key));
    conn.execute(
        "INSERT OR REPLACE INTO api_cache_entries (
            cache_id, source_name, endpoint_family, request_method, request_url,
            normalized_cache_key, response_status, response_body, response_content_type,
            fetched_at, expires_at, cache_status
         ) VALUES (?, ?, ?, 'GET', ?, ?, ?, ?, 'text/plain', ?, ?, 'fresh')",
        params![
            cache_id,
            source_name,
            endpoint_family,
            url,
            cache_key,
            status,
            text,
            now,
            now + ttl_secs,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn fetch_json_cached(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    ttl_secs: i64,
    api_key: Option<&str>,
) -> Result<Value, String> {
    if let Some(cached) = read_fresh_cache(db_path, cache_key, ttl_secs)? {
        if let Some(phase) = phase_for_api_source(source_name) {
            record_phase_cache(phase, true);
        }
        return Ok(cached);
    }
    if let Some(phase) = phase_for_api_source(source_name) {
        record_phase_cache(phase, false);
    }

    let effective_url = if let Some(key) = api_key {
        if url.contains("eutils.ncbi.nlm.nih.gov") && !url.contains("api_key=") {
            let sep = if url.contains('?') { '&' } else { '?' };
            format!("{}{}api_key={}", url, sep, key)
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };

    ensure_research_outbound_url(&effective_url)?;

    let mut last_err = String::new();
    let mut body: Option<Value> = None;
    let mut status_code = 0i32;
    for attempt in 0..3u32 {
        let res = HTTP_CLIENT
            .get(&effective_url)
            .send()
            .await
            .map_err(|e| format!("{} request failed: {}", source_name, e))?;
        status_code = res.status().as_u16() as i32;
        if res.status().is_success() {
            body = Some(
                res.json()
                    .await
                    .map_err(|e| format!("{} parse error: {}", source_name, e))?,
            );
            break;
        }
        last_err = format!("HTTP {}", status_code);
        if status_code == 429 || status_code >= 500 {
            let delay_ms = 500u64 * (1 << attempt);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            continue;
        }
        break;
    }

    let Some(body) = body else {
        write_error_cache(db_path, source_name, endpoint_family, cache_key, url, status_code)?;
        return Err(format!("{} returned {}", source_name, last_err));
    };

    write_fresh_cache(
        db_path,
        source_name,
        endpoint_family,
        cache_key,
        url,
        status_code,
        &body,
    )?;
    Ok(body)
}

fn read_fresh_cache(db_path: &Path, cache_key: &str, ttl_secs: i64) -> Result<Option<Value>, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let row: Option<(String, i64, String)> = conn
        .query_row(
            "SELECT response_body, fetched_at, cache_status FROM api_cache_entries
             WHERE normalized_cache_key = ?",
            params![cache_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    let Some((body, fetched_at, status)) = row else {
        return Ok(None);
    };
    if status.as_str() != "fresh" {
        return Ok(None);
    }
    let now = unix_now();
    if now - fetched_at > ttl_secs {
        let _ = conn.execute(
            "UPDATE api_cache_entries SET cache_status = 'stale' WHERE normalized_cache_key = ?",
            params![cache_key],
        );
        return Ok(None);
    }
    serde_json::from_str(&body).map(Some).map_err(|e| e.to_string())
}

fn write_fresh_cache(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    status: i32,
    body: &Value,
) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let now = unix_now();
    let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let cache_id = format!("cache_{}", string_to_u64(cache_key));
    conn.execute(
        "INSERT OR REPLACE INTO api_cache_entries (
            cache_id, source_name, endpoint_family, request_method, request_url,
            normalized_cache_key, response_status, response_body, fetched_at, expires_at,
            cache_status
         ) VALUES (?, ?, ?, 'GET', ?, ?, ?, ?, ?, ?, 'fresh')",
        params![
            cache_id,
            source_name,
            endpoint_family,
            url,
            cache_key,
            status,
            body_str,
            now,
            now + ttl_secs_for(source_name),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_error_cache(
    db_path: &Path,
    source_name: &str,
    endpoint_family: &str,
    cache_key: &str,
    url: &str,
    status: i32,
) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let now = unix_now();
    let cache_id = format!("cache_{}", string_to_u64(cache_key));
    let _ = conn.execute(
        "INSERT OR REPLACE INTO api_cache_entries (
            cache_id, source_name, endpoint_family, request_method, request_url,
            normalized_cache_key, response_status, fetched_at, cache_status, error_count, last_error
         ) VALUES (?, ?, ?, 'GET', ?, ?, ?, ?, 'error', 1, ?)",
        params![
            cache_id,
            source_name,
            endpoint_family,
            url,
            cache_key,
            status,
            now,
            format!("HTTP {}", status),
        ],
    );
    Ok(())
}

fn ttl_secs_for(source_name: &str) -> i64 {
    match source_name {
        "ensembl" => ENSEMBL_TTL_SECS,
        "clinvar" => CLINVAR_TTL_SECS,
        "pgs_catalog" => PGS_TTL_SECS,
        "reactome" => REACTOME_TTL_SECS,
        "open_targets" => OPENTARGETS_TTL_SECS,
        "pharmgkb" | "clinpgx" => PHARMGKB_TTL_SECS,
        "ols4" => OLS4_TTL_SECS,
        "gtex" => GTEX_TTL_SECS,
        "pubmed" => PUBMED_TTL_SECS,
        "dbsnp" => DBSNP_TTL_SECS,
        _ => GWAS_TTL_SECS,
    }
}

pub fn gwas_ttl() -> i64 {
    GWAS_TTL_SECS
}
pub fn ensembl_ttl() -> i64 {
    ENSEMBL_TTL_SECS
}
pub fn clinvar_ttl() -> i64 {
    CLINVAR_TTL_SECS
}
pub fn pgs_ttl() -> i64 {
    PGS_TTL_SECS
}
pub fn pgs_file_ttl() -> i64 {
    PGS_FILE_TTL_SECS
}
pub fn reactome_ttl() -> i64 {
    REACTOME_TTL_SECS
}
pub fn opentargets_ttl() -> i64 {
    OPENTARGETS_TTL_SECS
}
pub fn pharmgkb_ttl() -> i64 {
    PHARMGKB_TTL_SECS
}
pub fn ols4_ttl() -> i64 {
    OLS4_TTL_SECS
}
pub fn dbsnp_ttl() -> i64 {
    DBSNP_TTL_SECS
}
pub fn gtex_ttl() -> i64 {
    GTEX_TTL_SECS
}
pub fn pubmed_ttl() -> i64 {
    PUBMED_TTL_SECS
}
