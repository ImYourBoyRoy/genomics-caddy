// ./src-tauri/src/lib.rs
/*
Module Docstring:
Purpose: Tauri library entry point and command handler declarations.
Responsibilities:
- Declare all internal Rust modules.
- Expose Tauri command APIs for Svelte frontend to query samples, import genomes, run liftovers, and evaluate reports.
- Emits real-time import progress events to the frontend.
Key Inputs: Tauri app states and command parameters.
Key Outputs: JSON responses to the frontend.
Operational Notes: Manages SQLite database connection pools and executes requests.
*/

// Clippy configuration:
// Disable optional, stylistic pedantic warnings that do not represent defects.
// This defines the project style guidelines for the binary target.
#![allow(
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::must_use_candidate,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::map_unwrap_or,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::manual_let_else,
    clippy::format_push_string,
    clippy::duration_suboptimal_units,
    clippy::match_same_arms,
    clippy::bool_to_int_with_if,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::items_after_statements,
    clippy::unnecessary_wraps,
    clippy::assigning_clones,
    clippy::wildcard_imports,
    clippy::manual_is_variant_and,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::single_match_else,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::collapsible_if,
    clippy::ref_option,
    clippy::redundant_else,
    clippy::implicit_hasher,
    clippy::manual_string_new,
    clippy::single_char_pattern,
    clippy::implicit_clone,
    clippy::unnested_or_patterns,
    clippy::trivially_copy_pass_by_ref,
    clippy::explicit_iter_loop,
    clippy::fn_params_excessive_bools,
    clippy::unchecked_time_subtraction,
    clippy::match_wildcard_for_single_variants,
    clippy::ignored_unit_patterns,
    clippy::large_stack_arrays,
    clippy::unnecessary_debug_formatting,
    clippy::unused_async
)]

pub mod agent;
mod agent_commands;
mod agent_ui;
mod agent_ui_endpoint;
pub mod app_log;
pub mod config;
pub mod db;
pub mod db_crypto;
mod db_runtime;
pub mod file_utils;
pub mod inference_host;
pub mod library;
mod portable_update;
pub mod liftover;
pub mod mcp;
pub mod offline;
pub mod parser;
pub mod paths;
pub mod report;
pub mod research;
mod service_ops;
mod stream_control;

use db::{DbSnpRecord, SampleInfo};
use report::GeneratedReport;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use zip::{ZipWriter, write::SimpleFileOptions};

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    percentage: u32,
    status: String,
}

pub(crate) fn get_data_dir(_app: &AppHandle) -> PathBuf {
    paths::resolve_data_dir()
}

pub(crate) fn get_db_path(app: &AppHandle) -> PathBuf {
    paths::db_path(&get_data_dir(app))
}

fn get_chain_path(app: &AppHandle) -> PathBuf {
    offline::liftover_chain_path(&get_data_dir(app), &get_db_path(app))
}

#[derive(Debug, serde::Serialize, Clone)]
struct GenomeImportPreview {
    source_file_name: String,
    diagnostics: parser::ParseDiagnostics,
    liftover_available: bool,
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("Failed to hash import file: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to hash import file: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn source_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown source file")
        .to_string()
}

#[tauri::command]
async fn select_file() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("Genomic Data", &["txt", "csv", "tsv", "zip"])
        .pick_file();
    Ok(file.map(|p| {
        let path_str = p.to_string_lossy().to_string();
        config::register_import_path(&path_str);
        path_str
    }))
}

#[tauri::command]
async fn select_directory() -> Result<Option<String>, String> {
    let folder = rfd::FileDialog::new().pick_folder();
    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
async fn inspect_genome(app: AppHandle, file_path: String) -> Result<GenomeImportPreview, String> {
    config::validate_import_path(&file_path)?;
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let parsed = parser::parse_dna_file_with_metadata(&file_path, |_| {})?;
        Ok(GenomeImportPreview {
            source_file_name: source_file_name(Path::new(&file_path)),
            diagnostics: parsed.diagnostics,
            liftover_available: offline::liftover_chain_path(&data_dir, &db_path).is_file(),
        })
    })
    .await
    .map_err(|e| format!("Genome preview worker failed: {e}"))?
}

#[tauri::command]
async fn save_report_json(content: String, default_filename: String) -> Result<bool, String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .add_filter("JSON Report", &["json"])
        .save_file();

    if let Some(path) = file {
        let path_str = path.to_string_lossy().to_string();
        config::register_export_path(&path_str);
        config::validate_export_path(&path_str)?;
        file_utils::atomic_write(&path, content.as_bytes())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(serde::Deserialize)]
struct ExportBundleFile {
    filename: String,
    content: String,
}

#[tauri::command]
async fn save_report_bundle(
    files: Vec<ExportBundleFile>,
    default_filename: String,
) -> Result<bool, String> {
    if files.is_empty() {
        return Err("Cannot export an empty report bundle".into());
    }

    let file = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .add_filter("Genomics Caddy bundle", &["zip"])
        .save_file();

    let Some(path) = file else {
        return Ok(false);
    };

    let path_str = path.to_string_lossy().to_string();
    config::register_export_path(&path_str);
    config::validate_registered_export_path(&path_str, "zip")?;

    let temp_name = format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|name| name.to_str()).unwrap_or("genomics_bundle"),
        std::process::id(),
    );
    let temp_path = path.with_file_name(temp_name);

    let write_result = (|| -> Result<(), String> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| format!("Failed to create bundle: {e}"))?;
        let mut archive = ZipWriter::new(file);
        let mut names = HashSet::new();
        for entry in files {
            let name = entry.filename.trim();
            if name.is_empty()
                || name.len() > 128
                || name.contains('/')
                || name.contains('\\')
                || name == "."
                || name == ".."
                || name.chars().any(char::is_control)
                || !names.insert(name.to_string())
            {
                return Err("Bundle contains an invalid file name".into());
            }
            archive
                .start_file(name, SimpleFileOptions::default())
                .map_err(|e| format!("Failed to create bundle entry: {e}"))?;
            archive
                .write_all(entry.content.as_bytes())
                .map_err(|e| format!("Failed to write bundle entry: {e}"))?;
        }
        let file = archive
            .finish()
            .map_err(|e| format!("Failed to finalize bundle: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Failed to sync bundle: {e}"))?;
        std::fs::rename(&temp_path, &path)
            .map_err(|e| format!("Failed to finalize bundle path: {e}"))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result.map(|()| true)
}

#[tauri::command]
async fn get_app_bootstrap(app: AppHandle) -> Result<db::AppBootstrapStatus, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn =
            db::open_user_db_with_progress(&db_path, Some(&app)).map_err(|e| e.to_string())?;
        db::get_bootstrap_status(&conn, &data_dir)
    })
    .await
    .map_err(|e| format!("Bootstrap worker failed: {}", e))?
}

#[tauri::command]
fn get_app_paths(_app: AppHandle) -> Result<serde_json::Value, String> {
    let info = paths::resolve_data_dir_info();
    let project_root = paths::resolve_project_root(None);
    let data_dir = info.path;
    let db_path = paths::db_path(&data_dir);
    let chain_path = offline::liftover_chain_path(&data_dir, &db_path);
    let env_path = config::recommended_env_path();
    Ok(serde_json::json!({
        "project_root": project_root.to_string_lossy().to_string(),
        "app_layout_dir": paths::app_layout_dir(&project_root).to_string_lossy().to_string(),
        "data_dir": data_dir.to_string_lossy().to_string(),
        "data_dir_mode": info.mode,
        "db_path": db_path.to_string_lossy().to_string(),
        "chain_path": chain_path.to_string_lossy().to_string(),
        "references_dir": paths::references_dir(&data_dir).to_string_lossy().to_string(),
        "raw_downloads_dir": paths::raw_downloads_dir(&data_dir).to_string_lossy().to_string(),
        "env_path": env_path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
async fn get_all_marker_packs(app: AppHandle) -> Result<serde_json::Value, String> {
    let data_dir = get_data_dir(&app);
    let manifest_str = db::get_manifest_str(Some(&data_dir));
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let mut packs_map = serde_json::Map::new();
    if let Some(packs_array) = manifest.get("packs").and_then(|v| v.as_array()) {
        for pack_info in packs_array {
            if let Some(pack_id) = pack_info.get("id").and_then(|v| v.as_str())
                && let Some(pack_str) = db::get_pack_str(Some(&data_dir), pack_id)
                && let Ok(pack_json) = serde_json::from_str::<serde_json::Value>(&pack_str)
            {
                packs_map.insert(pack_id.to_string(), pack_json);
            }
        }
    }

    Ok(serde_json::json!({
        "manifest": manifest,
        "packs": packs_map,
    }))
}

#[tauri::command]
async fn get_marker_pack_warnings() -> Result<std::collections::HashMap<String, String>, String> {
    Ok(db::get_warnings())
}

#[tauri::command]
async fn reload_marker_packs(
    app: AppHandle,
) -> Result<std::collections::HashMap<String, String>, String> {
    let data_dir = get_data_dir(&app);
    db::clear_marker_packs_registry();
    db::sync_marker_packs_registry(Some(&data_dir));
    Ok(db::get_warnings())
}

/// Split a keyword query into tokens (max 8). Multi-token queries use AND across
/// tokens; each token may match any searchable column via LIKE. This is substring /
/// keyword search — not edit-distance fuzzy matching.
fn keyword_like_patterns(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .take(8)
        .map(|t| format!("%{}%", t.to_lowercase()))
        .collect()
}

fn anded_or_where(token_count: usize, column_exprs: &[&str]) -> String {
    let or_inner = column_exprs
        .iter()
        .map(|c| format!("{c} LIKE ?"))
        .collect::<Vec<_>>()
        .join(" OR ");
    (0..token_count)
        .map(|_| format!("({or_inner})"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn expand_like_params(patterns: &[String], columns_per_token: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(patterns.len() * columns_per_token);
    for pattern in patterns {
        for _ in 0..columns_per_token {
            out.push(pattern.clone());
        }
    }
    out
}

fn like_params_with_page(
    patterns: &[String],
    columns_per_token: usize,
    limit: u32,
    offset: u32,
) -> Vec<rusqlite::types::Value> {
    let mut out: Vec<rusqlite::types::Value> =
        Vec::with_capacity(patterns.len() * columns_per_token + 2);
    for pattern in expand_like_params(patterns, columns_per_token) {
        out.push(rusqlite::types::Value::Text(pattern));
    }
    out.push(rusqlite::types::Value::Integer(i64::from(limit)));
    out.push(rusqlite::types::Value::Integer(i64::from(offset)));
    out
}

fn like_params_only(
    patterns: &[String],
    columns_per_token: usize,
) -> Vec<rusqlite::types::Value> {
    expand_like_params(patterns, columns_per_token)
        .into_iter()
        .map(rusqlite::types::Value::Text)
        .collect()
}

const CLINVAR_PHENOTYPE_EXPR: &str =
    "COALESCE(NULLIF(TRIM(phenotype_list), ''), NULLIF(TRIM(conditions), ''), '')";

#[tauri::command]
async fn query_local_reference_db(
    app: AppHandle,
    table: String,
    search_query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let q_limit = limit.unwrap_or(20).clamp(1, 100);
    let q_offset = offset.unwrap_or(0);
    let patterns = keyword_like_patterns(search_query.as_deref().unwrap_or(""));
    let has_search = !patterns.is_empty();

    db_runtime::with_connection(db_path, move |conn| {
        match table.as_str() {
            "clinvar" => {
                let select_cols = format!(
                    "rsid, variation_id, clinical_significance, review_status, {CLINVAR_PHENOTYPE_EXPR}, last_evaluated"
                );
                let map_row = |r: &rusqlite::Row<'_>| {
                    Ok(serde_json::json!({
                        "rsid": r.get::<_, String>(0)?,
                        "variation_id": r.get::<_, String>(1)?,
                        "clinical_significance": r.get::<_, String>(2)?,
                        "review_status": r.get::<_, String>(3)?,
                        "phenotype_names": r.get::<_, String>(4)?,
                        "last_evaluated": r.get::<_, Option<String>>(5)?,
                    }))
                };
                let (rows, total) = if has_search {
                    let cols = [
                        "LOWER(rsid)",
                        "LOWER(COALESCE(NULLIF(TRIM(phenotype_list), ''), conditions, ''))",
                        "LOWER(clinical_significance)",
                        "LOWER(COALESCE(gene_symbol, gene, ''))",
                    ];
                    let where_sql = anded_or_where(patterns.len(), &cols);
                    let count_bind = like_params_only(&patterns, cols.len());
                    let count_sql =
                        format!("SELECT COUNT(*) FROM clinvar_reference WHERE {where_sql}");
                    let total: i64 = conn
                        .query_row(
                            &count_sql,
                            rusqlite::params_from_iter(count_bind.iter()),
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let sql = format!(
                        "SELECT {select_cols} FROM clinvar_reference WHERE {where_sql} LIMIT ? OFFSET ?"
                    );
                    let page_bind = like_params_with_page(&patterns, cols.len(), q_limit, q_offset);
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(page_bind.iter()), map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                } else {
                    let total: i64 = conn
                        .query_row("SELECT COUNT(*) FROM clinvar_reference", [], |r| r.get(0))
                        .map_err(|e| e.to_string())?;
                    let sql = format!("SELECT {select_cols} FROM clinvar_reference LIMIT ? OFFSET ?");
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params![q_limit, q_offset], map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                };
                Ok(serde_json::json!({ "rows": rows, "total": total }))
            }
            "pharmgkb" => {
                let map_row = |r: &rusqlite::Row<'_>| {
                    Ok(serde_json::json!({
                        "rsid": r.get::<_, String>(0)?,
                        "gene": r.get::<_, Option<String>>(1)?,
                        "drug": r.get::<_, Option<String>>(2)?,
                        "phenotype": r.get::<_, Option<String>>(3)?,
                        "evidence_level": r.get::<_, Option<String>>(4)?,
                    }))
                };
                let (rows, total) = if has_search {
                    let cols = [
                        "LOWER(rsid)",
                        "LOWER(COALESCE(gene, ''))",
                        "LOWER(COALESCE(drug, ''))",
                        "LOWER(COALESCE(phenotype, ''))",
                    ];
                    let where_sql = anded_or_where(patterns.len(), &cols);
                    let count_bind = like_params_only(&patterns, cols.len());
                    let total: i64 = conn
                        .query_row(
                            &format!(
                                "SELECT COUNT(*) FROM pharmgkb_clinical_variants WHERE {where_sql}"
                            ),
                            rusqlite::params_from_iter(count_bind.iter()),
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let sql = format!(
                        "SELECT rsid, gene, drug, phenotype, evidence_level \
                         FROM pharmgkb_clinical_variants WHERE {where_sql} LIMIT ? OFFSET ?"
                    );
                    let page_bind =
                        like_params_with_page(&patterns, cols.len(), q_limit, q_offset);
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(page_bind.iter()), map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                } else {
                    let total: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM pharmgkb_clinical_variants",
                            [],
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let mut stmt = conn
                        .prepare(
                            "SELECT rsid, gene, drug, phenotype, evidence_level \
                             FROM pharmgkb_clinical_variants LIMIT ? OFFSET ?",
                        )
                        .map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params![q_limit, q_offset], map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                };
                Ok(serde_json::json!({ "rows": rows, "total": total }))
            }
            "clingen" => {
                let map_row = |r: &rusqlite::Row<'_>| {
                    Ok(serde_json::json!({
                        "gene_symbol": r.get::<_, String>(0)?,
                        "disease_label": r.get::<_, String>(1)?,
                        "classification": r.get::<_, Option<String>>(2)?,
                        "moi": r.get::<_, Option<String>>(3)?,
                        "report_url": r.get::<_, Option<String>>(4)?,
                    }))
                };
                let (rows, total) = if has_search {
                    let cols = [
                        "LOWER(gene_symbol)",
                        "LOWER(disease_label)",
                        "LOWER(COALESCE(classification, ''))",
                    ];
                    let where_sql = anded_or_where(patterns.len(), &cols);
                    let count_bind = like_params_only(&patterns, cols.len());
                    let total: i64 = conn
                        .query_row(
                            &format!(
                                "SELECT COUNT(*) FROM clingen_gene_validity WHERE {where_sql}"
                            ),
                            rusqlite::params_from_iter(count_bind.iter()),
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let sql = format!(
                        "SELECT gene_symbol, disease_label, classification, moi, report_url \
                         FROM clingen_gene_validity WHERE {where_sql} LIMIT ? OFFSET ?"
                    );
                    let page_bind =
                        like_params_with_page(&patterns, cols.len(), q_limit, q_offset);
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(page_bind.iter()), map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                } else {
                    let total: i64 = conn
                        .query_row("SELECT COUNT(*) FROM clingen_gene_validity", [], |r| {
                            r.get(0)
                        })
                        .map_err(|e| e.to_string())?;
                    let mut stmt = conn
                        .prepare(
                            "SELECT gene_symbol, disease_label, classification, moi, report_url \
                             FROM clingen_gene_validity LIMIT ? OFFSET ?",
                        )
                        .map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params![q_limit, q_offset], map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                };
                Ok(serde_json::json!({ "rows": rows, "total": total }))
            }
            "gwas" => {
                let map_row = |r: &rusqlite::Row<'_>| {
                    Ok(serde_json::json!({
                        "rsid": r.get::<_, String>(0)?,
                        "trait_name": r.get::<_, String>(1)?,
                        "p_value": r.get::<_, f64>(2)?,
                        "or_or_beta": r.get::<_, Option<String>>(3)?,
                        "pubmed_id": r.get::<_, Option<String>>(4)?,
                        "study_title": r.get::<_, Option<String>>(5)?,
                        "journal": r.get::<_, Option<String>>(6)?,
                    }))
                };
                let (rows, total) = if has_search {
                    let cols = [
                        "LOWER(rsid)",
                        "LOWER(trait_name)",
                        "LOWER(COALESCE(study_title, ''))",
                    ];
                    let where_sql = anded_or_where(patterns.len(), &cols);
                    let count_bind = like_params_only(&patterns, cols.len());
                    let total: i64 = conn
                        .query_row(
                            &format!("SELECT COUNT(*) FROM gwas_reference WHERE {where_sql}"),
                            rusqlite::params_from_iter(count_bind.iter()),
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let sql = format!(
                        "SELECT rsid, trait_name, p_value, or_or_beta, pubmed_id, study_title, journal \
                         FROM gwas_reference WHERE {where_sql} LIMIT ? OFFSET ?"
                    );
                    let page_bind =
                        like_params_with_page(&patterns, cols.len(), q_limit, q_offset);
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(page_bind.iter()), map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                } else {
                    let total: i64 = conn
                        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |r| r.get(0))
                        .map_err(|e| e.to_string())?;
                    let mut stmt = conn
                        .prepare(
                            "SELECT rsid, trait_name, p_value, or_or_beta, pubmed_id, study_title, journal \
                             FROM gwas_reference LIMIT ? OFFSET ?",
                        )
                        .map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params![q_limit, q_offset], map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                };
                Ok(serde_json::json!({ "rows": rows, "total": total }))
            }
            "mane" => {
                let map_row = |r: &rusqlite::Row<'_>| {
                    Ok(serde_json::json!({
                        "gene_symbol": r.get::<_, String>(0)?,
                        "ensembl_transcript": r.get::<_, Option<String>>(1)?,
                        "refseq_transcript": r.get::<_, Option<String>>(2)?,
                        "mane_status": r.get::<_, Option<String>>(3)?,
                        "grch38_coordinates": r.get::<_, Option<String>>(4)?,
                    }))
                };
                let (rows, total) = if has_search {
                    let cols = [
                        "LOWER(gene_symbol)",
                        "LOWER(COALESCE(ensembl_transcript, ''))",
                        "LOWER(COALESCE(refseq_transcript, ''))",
                    ];
                    let where_sql = anded_or_where(patterns.len(), &cols);
                    let count_bind = like_params_only(&patterns, cols.len());
                    let total: i64 = conn
                        .query_row(
                            &format!("SELECT COUNT(*) FROM mane_transcripts WHERE {where_sql}"),
                            rusqlite::params_from_iter(count_bind.iter()),
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    let sql = format!(
                        "SELECT gene_symbol, ensembl_transcript, refseq_transcript, mane_status, grch38_coordinates \
                         FROM mane_transcripts WHERE {where_sql} LIMIT ? OFFSET ?"
                    );
                    let page_bind =
                        like_params_with_page(&patterns, cols.len(), q_limit, q_offset);
                    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params_from_iter(page_bind.iter()), map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                } else {
                    let total: i64 = conn
                        .query_row("SELECT COUNT(*) FROM mane_transcripts", [], |r| r.get(0))
                        .map_err(|e| e.to_string())?;
                    let mut stmt = conn
                        .prepare(
                            "SELECT gene_symbol, ensembl_transcript, refseq_transcript, mane_status, grch38_coordinates \
                             FROM mane_transcripts LIMIT ? OFFSET ?",
                        )
                        .map_err(|e| e.to_string())?;
                    let mapped = stmt
                        .query_map(rusqlite::params![q_limit, q_offset], map_row)
                        .map_err(|e| e.to_string())?;
                    let mut vec = Vec::new();
                    for row in mapped {
                        vec.push(row.map_err(|e| e.to_string())?);
                    }
                    (vec, total)
                };
                Ok(serde_json::json!({ "rows": rows, "total": total }))
            }
            _ => Err(format!("Unknown database table: {}", table)),
        }
    })
    .await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn import_genome(
    app: AppHandle,
    file_path: String,
    sample_name: String,
    replace_existing_sample_id: Option<i64>,
) -> Result<i64, String> {
    config::validate_import_path(&file_path)?;
    let app_handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let source_path = Path::new(&file_path);
        let source_file_sha256 = sha256_file(source_path)?;
        app_handle
            .emit(
                "import-progress",
                ProgressPayload {
                    percentage: 5,
                    status: "Loading file...".to_string(),
                },
            )
            .ok();

        let app_clone = app_handle.clone();
        let parsed = parser::parse_dna_file_with_metadata(&file_path, move |status| {
            app_clone
                .emit(
                    "import-progress",
                    ProgressPayload {
                        percentage: 15,
                        status: status.to_string(),
                    },
                )
                .ok();
        })?;

        let post_parse_sha256 = sha256_file(source_path)?;
        if source_file_sha256 != post_parse_sha256 {
            return Err("The DNA export changed while it was being read; import was cancelled.".to_string());
        }
        let import_id = format!(
            "import-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("Failed to create import ID: {e}"))?
                .as_millis(),
            &source_file_sha256[..16]
        );
        let provenance = parser::ImportProvenance::new(
            import_id,
            source_file_name(source_path),
            source_file_sha256,
            parsed.diagnostics.clone(),
        );

        app_handle
            .emit(
                "import-progress",
                ProgressPayload {
                    percentage: 45,
                    status: "Initializing liftover database...".to_string(),
                },
            )
            .ok();

        let data_dir = get_data_dir(&app_handle);
        let db_path = paths::db_path(&data_dir);
        let chain_path = offline::liftover_chain_path(&data_dir, &db_path);
        let liftover_engine = if chain_path.exists() {
            match liftover::LiftoverEngine::new(&chain_path) {
                Ok(engine) => Some(engine),
                Err(e) => {
                    eprintln!("Failed to initialize liftover engine: {}", e);
                    None
                }
            }
        } else {
            println!(
                "No liftover chain found at {:?}. Importing without GRCh38 coordinates.",
                chain_path
            );
            None
        };

        let mut conn = db::open_user_db(&db_path).map_err(|e| e.to_string())?;

        let app_clone = app_handle.clone();
        let sample_id = db::import_raw_genome_with_provenance(
            &mut conn,
            &data_dir,
            &sample_name,
            &parsed.records,
            liftover_engine.as_ref(),
            replace_existing_sample_id,
            Some(provenance),
            move |pct, status| {
                app_clone
                    .emit(
                        "import-progress",
                        ProgressPayload {
                            percentage: pct,
                            status: status.to_string(),
                        },
                    )
                    .ok();
            },
        )?;

        app_handle
            .emit(
                "import-progress",
                ProgressPayload {
                    percentage: 96,
                    status: "Building variant placement index...".to_string(),
                },
            )
            .ok();

        let sample_conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        let index_status = match crate::offline::tier2::build_variant_locus_for_sample(&sample_conn, sample_id) {
            Ok(placements) => format!("Variant placement index ready ({placements} placements)."),
            Err(e) => {
                eprintln!(
                    "Warning: failed to build variant locus index for sample {}: {}",
                    sample_id, e
                );
                "Variant placement index unavailable; keeping the imported genotype data.".to_string()
            }
        };
        app_handle
            .emit(
                "import-progress",
                ProgressPayload {
                    percentage: 98,
                    status: index_status,
                },
            )
            .ok();
        app_handle
            .emit(
                "import-progress",
                ProgressPayload {
                    percentage: 100,
                    status: "DNA profile import complete.".to_string(),
                },
            )
            .ok();

        Ok(sample_id)
    })
    .await
    .map_err(|e| format!("Import worker failed: {}", e))?
}

#[tauri::command]
async fn get_samples(app: AppHandle) -> Result<Vec<SampleInfo>, String> {
    let db_path = get_db_path(&app);
    db_runtime::with_connection(db_path, |conn| {
        db::get_samples(conn).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn query_rsids(
    app: AppHandle,
    sample_id: i64,
    rsids: Vec<String>,
) -> Result<Vec<DbSnpRecord>, String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        db::query_by_rsids(&conn, sample_id, &rsids).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Sample database worker failed: {e}"))?
}

#[tauri::command]
async fn query_region(
    app: AppHandle,
    sample_id: i64,
    chromosome: String,
    start: u64,
    end: u64,
) -> Result<Vec<DbSnpRecord>, String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        db::query_region(&conn, sample_id, &chromosome, start, end).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Sample database worker failed: {e}"))?
}

#[tauri::command]
fn log_js_error(
    message: String,
    source: Option<String>,
    line: Option<u32>,
    col: Option<u32>,
    stack: Option<String>,
) {
    let details = format!(
        "JS ERROR: {} | Source: {} | Line: {:?} | Col: {:?} | Stack: {:?}",
        message,
        source.unwrap_or_else(|| "unknown".to_string()),
        line,
        col,
        stack
    );
    app_log::log("WEBVIEW_ERROR", &details);
}

#[tauri::command]
async fn generate_report(
    app: AppHandle,
    sample_id: i64,
    template_json: String,
) -> Result<GeneratedReport, String> {
    app_log::log(
        "CMD_GENERATE_REPORT",
        &format!("Invoked for sample_id = {}", sample_id),
    );
    let res = config::validate_template_json(&template_json);
    if let Err(e) = &res {
        app_log::log("CMD_GENERATE_REPORT", &format!("Validation failed: {}", e));
        return Err(e.clone());
    }
    let data_dir = get_data_dir(&app);
    // Resolve an auto-managed gnomAD release before the synchronous report
    // worker reads its cache, so report metadata cannot lag behind the
    // release selected by the runtime lookup paths. Failure is best-effort:
    // the local report remains available when the public listing is offline.
    let db_path = get_db_path(&app);
    let _ = research::gnomad::load_effective_gnomad_config(&db_path, &data_dir).await;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        let template: report::ReportTemplate = serde_json::from_str(&template_json)
            .map_err(|e| format!("Failed to parse report template JSON: {}", e))?;
        report::generate_report(&conn, sample_id, &template)
    })
    .await
    .map_err(|e| format!("Report worker failed: {e}"))?;
    match &result {
        Ok(_) => app_log::log("CMD_GENERATE_REPORT", "Report generated successfully."),
        Err(e) => app_log::log("CMD_GENERATE_REPORT", &format!("ERROR: {}", e)),
    }
    result
}

#[tauri::command]
async fn delete_sample(app: AppHandle, sample_id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let data_dir = get_data_dir(&app);
    db_runtime::with_connection_mut(db_path, move |conn| {
        db::delete_sample(conn, &data_dir, sample_id).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
async fn get_chromosome_counts(
    app: AppHandle,
    sample_id: i64,
) -> Result<std::collections::HashMap<String, i64>, String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        db::get_chromosome_counts(&conn, sample_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Sample database worker failed: {e}"))?
}

#[tauri::command]
async fn check_chain_status(app: AppHandle) -> Result<bool, String> {
    let chain_path = get_chain_path(&app);
    Ok(chain_path.exists())
}

#[tauri::command]
async fn download_chain_file(app: AppHandle) -> Result<(), String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    let result = offline::sync_single_asset(
        &data_dir,
        &db_path,
        "liftover_chain",
        true,
        None,
        Some(&app),
    )
    .await?;
    if result.errors.is_empty() {
        Ok(())
    } else {
        Err(result.errors.join("; "))
    }
}

// ── Evidence Library & RAG Commands ───────────────────────────────────

#[derive(Debug, serde::Serialize)]
struct EvidenceRecord {
    rsid: String,
    gene: String,
    evidence_text: String,
    source_citation: String,
    has_embedding: bool,
    similarity: Option<f32>,
}

#[tauri::command]
async fn list_evidence_sources(app: AppHandle) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    db_runtime::with_connection(db_path, |conn| {
        let mut stmt = conn
            .prepare("SELECT DISTINCT source_citation FROM evidence_library ORDER BY source_citation ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut sources = Vec::new();
        for s in rows.flatten() {
            sources.push(s);
        }
        Ok(sources)
    })
    .await
}

#[tauri::command]
async fn get_evidence_for_marker(
    app: AppHandle,
    rsid: String,
) -> Result<Vec<EvidenceRecord>, String> {
    let db_path = get_db_path(&app);
    db_runtime::with_connection(db_path, move |conn| {
        let mut stmt = conn
            .prepare("SELECT rsid, gene, evidence_text, source_citation, embedding FROM evidence_library WHERE rsid = ?")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![rsid], |row| {
                let embedding: Option<String> = row.get(4)?;
                Ok(EvidenceRecord {
                    rsid: row.get(0)?,
                    gene: row.get(1)?,
                    evidence_text: row.get(2)?,
                    source_citation: row.get(3)?,
                    has_embedding: embedding.is_some() && !embedding.as_ref().unwrap().trim().is_empty(),
                    similarity: None,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| e.to_string())?);
        }
        Ok(results)
    })
    .await
}

#[tauri::command]
async fn search_evidence(
    app: AppHandle,
    query: String,
    ollama_url: Option<String>,
    ollama_token: Option<String>,
) -> Result<Vec<EvidenceRecord>, String> {
    let db_path = get_db_path(&app);
    let query_clean = query.trim().to_string();
    if query_clean.is_empty() {
        return Ok(Vec::new());
    }

    // 1. Perform keyword search on a background worker thread
    let query_for_db = query_clean.clone();
    let keyword_hits = db_runtime::with_connection(db_path.clone(), move |conn| {
        let search_pattern = config::sql_like_contains_pattern(&query_for_db);
        let mut stmt = conn
            .prepare(
                "SELECT rsid, gene, evidence_text, source_citation, embedding 
             FROM evidence_library 
             WHERE rsid LIKE ? OR gene LIKE ? OR LOWER(evidence_text) LIKE ?",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(
                rusqlite::params![search_pattern, search_pattern, search_pattern],
                |row| {
                    let embedding: Option<String> = row.get(4)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        embedding,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        let mut hits = Vec::new();
        for r in rows {
            hits.push(r.map_err(|e| e.to_string())?);
        }
        Ok(hits)
    })
    .await?;

    // 2. If Ollama URL is provided, try Semantic Vector Search
    if let Some(ref url) = ollama_url
        && !url.trim().is_empty()
    {
        let clean_url = config::validate_service_url(url)?;
        let token_ref = ollama_token.as_deref();

        // Try to find an embedding model on the server
        if let Some(embed_model) = get_embedding_model(&clean_url, token_ref).await {
            // Fetch query embedding
            if let Ok(query_embedding) =
                fetch_embedding(&clean_url, token_ref, &embed_model, &query_clean).await
            {
                // Generate embeddings on-demand for the top keyword hits (up to 10) to fill the vector cache
                let mut new_embeddings = Vec::new();
                for (rsid, _gene, text, citation, embedding_opt) in keyword_hits.iter().take(10) {
                    if (embedding_opt.is_none()
                        || embedding_opt.as_ref().unwrap().trim().is_empty())
                        && let Ok(emb) =
                            fetch_embedding(&clean_url, token_ref, &embed_model, text).await
                    {
                        new_embeddings.push((rsid.clone(), citation.clone(), emb));
                    }
                }

                // Save new embeddings back to database
                if !new_embeddings.is_empty() {
                    let db_path_save = db_path.clone();
                    let _ = db_runtime::with_connection_mut(db_path_save, move |conn| {
                            for (rsid, citation, emb) in new_embeddings {
                                if let Ok(emb_json) = serde_json::to_string(&emb) {
                                    let _ = conn.execute(
                                        "UPDATE evidence_library SET embedding = ? WHERE rsid = ? AND source_citation = ?",
                                        rusqlite::params![emb_json, rsid, citation],
                                    );
                                }
                            }
                            Ok(())
                        })
                        .await;
                }

                // Reload all rows with cached embeddings and calculate similarity
                let vector_results = db_runtime::with_connection(db_path.clone(), move |conn| {
                        let mut stmt_all = conn.prepare(
                            "SELECT rsid, gene, evidence_text, source_citation, embedding FROM evidence_library WHERE embedding IS NOT NULL AND embedding != ''"
                        ).map_err(|e| e.to_string())?;

                        let rows_all = stmt_all.query_map([], |row| {
                            let embedding_str: String = row.get(4)?;
                            Ok(EvidenceRecord {
                                rsid: row.get(0)?,
                                gene: row.get(1)?,
                                evidence_text: row.get(2)?,
                                source_citation: row.get(3)?,
                                has_embedding: true,
                                similarity: serde_json::from_str::<Vec<f32>>(&embedding_str)
                                    .ok()
                                    .map(|v| cosine_similarity(&query_embedding, &v)),
                            })
                        }).map_err(|e| e.to_string())?;

                        let mut results = Vec::new();
                        for r in rows_all {
                            if let Ok(rec) = r
                                && rec.similarity.is_some() {
                                    results.push(rec);
                                }
                        }
                        Ok(results)
                    })
                    .await
                    .unwrap_or_default();

                let mut sorted_results = vector_results;
                // Sort by similarity descending
                sorted_results.sort_by(|a, b| {
                    b.similarity
                        .unwrap_or(0.0)
                        .partial_cmp(&a.similarity.unwrap_or(0.0))
                        .unwrap()
                });

                // Only return results with similarity > 0.35
                sorted_results.retain(|r| r.similarity.unwrap_or(0.0) > 0.35);

                if !sorted_results.is_empty() {
                    return Ok(sorted_results);
                }
            }
        }
    }

    // Fallback: convert keyword hits to EvidenceRecords
    let fallback_results = keyword_hits
        .into_iter()
        .map(|(rsid, gene, text, citation, emb_opt)| EvidenceRecord {
            rsid,
            gene,
            evidence_text: text,
            source_citation: citation,
            has_embedding: emb_opt.is_some() && !emb_opt.unwrap().trim().is_empty(),
            similarity: None,
        })
        .collect();

    Ok(fallback_results)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

async fn get_embedding_model(url: &str, token: Option<&str>) -> Option<String> {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/tags", url));
    if let Some(t) = token
        && !t.trim().is_empty()
    {
        req = req.header(
            "Authorization",
            if t.to_lowercase().starts_with("bearer ") {
                t.to_string()
            } else {
                format!("Bearer {}", t)
            },
        );
    }
    let res = req.send().await.ok()?;
    #[derive(serde::Deserialize)]
    struct OllamaModel {
        name: String,
    }
    #[derive(serde::Deserialize)]
    struct OllamaTags {
        models: Vec<OllamaModel>,
    }
    let tags = res.json::<OllamaTags>().await.ok()?;
    for m in &tags.models {
        if m.name.contains("embed") {
            return Some(m.name.clone());
        }
    }
    tags.models.first().map(|m| m.name.clone())
}

async fn fetch_embedding(
    url: &str,
    token: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/embeddings", url));
    if let Some(t) = token
        && !t.trim().is_empty()
    {
        req = req.header(
            "Authorization",
            if t.to_lowercase().starts_with("bearer ") {
                t.to_string()
            } else {
                format!("Bearer {}", t)
            },
        );
    }
    let payload = serde_json::json!({
        "model": model,
        "prompt": prompt,
    });
    let res = req.json(&payload).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }
    #[derive(serde::Deserialize)]
    struct EmbeddingResponse {
        embedding: Vec<f32>,
    }
    let resp = res
        .json::<EmbeddingResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.embedding)
}

#[tauri::command]
fn cancel_ollama_stream() {
    stream_control::request_ollama_stream_cancel();
}

#[tauri::command]
async fn get_active_ollama_models(
    url: String,
    token: Option<String>,
) -> Result<serde_json::Value, String> {
    let clean_url = config::validate_service_url(&url)?;
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/ps", clean_url));

    if let Some(t) = token
        && !t.trim().is_empty()
    {
        let t_val = t.trim();
        req = req.header(
            "Authorization",
            if t_val.to_lowercase().starts_with("bearer ") {
                t_val.to_string()
            } else {
                format!("Bearer {}", t_val)
            },
        );
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let info: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(info)
}

#[tauri::command]
async fn scan_ollama_models(url: String, token: Option<String>) -> Result<Vec<String>, String> {
    let report = inference_host::discover_ollama_models(&url, token.as_deref()).await?;
    if let Some(err) = report.error {
        return Err(err);
    }
    Ok(report.models.into_iter().map(|m| m.name).collect())
}

#[tauri::command]
async fn probe_inference_host(ollama_url: Option<String>) -> Result<inference_host::InferenceHostProfile, String> {
    Ok(inference_host::probe_local_host(ollama_url.as_deref()))
}

#[tauri::command]
async fn discover_ollama_models(
    url: String,
    token: Option<String>,
) -> Result<inference_host::OllamaDiscoveryReport, String> {
    inference_host::discover_ollama_models(&url, token.as_deref()).await
}

#[tauri::command]
async fn probe_localhost_services() -> Result<service_ops::LocalhostServiceStatus, String> {
    Ok(service_ops::probe_localhost_services().await)
}

#[tauri::command]
async fn get_ollama_version(url: String, token: Option<String>) -> Result<serde_json::Value, String> {
    service_ops::get_ollama_version(&url, token.as_deref()).await
}

#[tauri::command]
async fn get_qdrant_version(url: String, api_key: Option<String>) -> Result<serde_json::Value, String> {
    service_ops::get_qdrant_version(&url, api_key.as_deref()).await
}

#[tauri::command]
async fn check_ollama_update(url: String, token: Option<String>) -> Result<service_ops::ServiceUpdateCheck, String> {
    service_ops::check_ollama_update(&url, token.as_deref()).await
}

#[tauri::command]
async fn check_qdrant_update(url: String, api_key: Option<String>) -> Result<service_ops::ServiceUpdateCheck, String> {
    service_ops::check_qdrant_update(&url, api_key.as_deref()).await
}

#[tauri::command]
async fn pull_ollama_model(
    app: AppHandle,
    url: String,
    token: Option<String>,
    name: String,
) -> Result<serde_json::Value, String> {
    service_ops::pull_ollama_model(&app, &url, token.as_deref(), &name).await
}

#[tauri::command]
async fn delete_ollama_model(
    url: String,
    token: Option<String>,
    name: String,
) -> Result<(), String> {
    service_ops::delete_ollama_model(&url, token.as_deref(), &name).await
}

#[tauri::command]
async fn probe_vector_provider(
    provider: String,
    url: String,
    api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    service_ops::probe_vector_provider(&provider, &url, api_key.as_deref()).await
}

#[tauri::command]
async fn show_ollama_model(
    url: String,
    token: Option<String>,
    name: String,
) -> Result<serde_json::Value, String> {
    let clean_url = config::validate_service_url(&url)?;
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/show", clean_url));

    if let Some(t) = token
        && !t.trim().is_empty()
    {
        let t_val = t.trim();
        req = req.header(
            "Authorization",
            if t_val.to_lowercase().starts_with("bearer ") {
                t_val.to_string()
            } else {
                format!("Bearer {}", t_val)
            },
        );
    }

    let payload = serde_json::json!({
        "name": name
    });

    let res = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let details: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(details)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn stream_ollama_chat(
    app: AppHandle,
    stream_id: String,
    url: String,
    token: Option<String>,
    model: String,
    messages: Vec<serde_json::Value>,
    temperature: Option<f64>,
    num_predict: Option<u32>,
) -> Result<(), String> {
    stream_control::validate_stream_id(&stream_id)?;
    let clean_url = config::validate_service_url(&url)?;
    stream_control::reset_ollama_stream(&stream_id);
    let chunk_event = stream_control::ollama_chunk_event(&stream_id);
    let done_event = stream_control::ollama_done_event(&stream_id);

    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/chat", clean_url));

    if let Some(t) = token
        && !t.trim().is_empty()
    {
        let t_val = t.trim();
        req = req.header(
            "Authorization",
            if t_val.to_lowercase().starts_with("bearer ") {
                t_val.to_string()
            } else {
                format!("Bearer {}", t_val)
            },
        );
    }

    let payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "options": {
            "temperature": temperature.unwrap_or(0.0),
            "num_predict": num_predict.unwrap_or(2048)
        }
    });

    let mut res = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let mut prompt_eval_count = None;
    let mut eval_count = None;
    let mut in_thinking = false;

    let mut buffer = String::new();
    while let Some(chunk) = res
        .chunk()
        .await
        .map_err(|e| format!("Stream error: {}", e))?
    {
        if stream_control::is_ollama_stream_cancelled() {
            app.emit(&done_event, serde_json::json!({ "cancelled": true }))
                .ok();
            return Ok(());
        }
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                let message = val.get("message");

                if let Some(reasoning) = message
                    .and_then(|m| m.get("reasoning"))
                    .and_then(|r| r.as_str())
                    && !reasoning.is_empty()
                {
                    if !in_thinking {
                        app.emit(&chunk_event, "<think>").ok();
                        in_thinking = true;
                    }
                    app.emit(&chunk_event, reasoning).ok();
                }

                if let Some(content) = message
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    && !content.is_empty()
                {
                    if in_thinking {
                        app.emit(&chunk_event, "</think>").ok();
                        in_thinking = false;
                    }
                    app.emit(&chunk_event, content).ok();
                }

                if let Some(pec) = val.get("prompt_eval_count").and_then(|v| v.as_u64()) {
                    prompt_eval_count = Some(pec);
                }
                if let Some(ec) = val.get("eval_count").and_then(|v| v.as_u64()) {
                    eval_count = Some(ec);
                }
            }
        }
    }

    if !buffer.trim().is_empty()
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(buffer.trim())
    {
        let message = val.get("message");

        if let Some(reasoning) = message
            .and_then(|m| m.get("reasoning"))
            .and_then(|r| r.as_str())
            && !reasoning.is_empty()
        {
            if !in_thinking {
                app.emit(&chunk_event, "<think>").ok();
                in_thinking = true;
            }
            app.emit(&chunk_event, reasoning).ok();
        }

        if let Some(content) = message
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            && !content.is_empty()
        {
            if in_thinking {
                app.emit(&chunk_event, "</think>").ok();
                in_thinking = false;
            }
            app.emit(&chunk_event, content).ok();
        }

        if let Some(pec) = val.get("prompt_eval_count").and_then(|v| v.as_u64()) {
            prompt_eval_count = Some(pec);
        }
        if let Some(ec) = val.get("eval_count").and_then(|v| v.as_u64()) {
            eval_count = Some(ec);
        }
    }

    if in_thinking {
        app.emit(&chunk_event, "</think>").ok();
    }

    #[derive(serde::Serialize, Clone)]
    struct DonePayload {
        prompt_eval_count: Option<u64>,
        eval_count: Option<u64>,
    }

    app.emit(
        &done_event,
        DonePayload {
            prompt_eval_count,
            eval_count,
        },
    )
    .ok();
    Ok(())
}

#[tauri::command]
async fn get_chat_sessions(
    app: AppHandle,
    sample_id: Option<i64>,
) -> Result<Vec<db::DbChatSession>, String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let sample_ids = if let Some(id) = sample_id {
            vec![id]
        } else {
            let registry =
                db::open_user_db(paths::db_path(&data_dir)).map_err(|e| e.to_string())?;
            db::get_samples(&registry)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|sample| sample.id)
                .collect()
        };
        let mut sessions = Vec::new();
        for id in sample_ids {
            let conn = db::connect_sample(&data_dir, id).map_err(|e| e.to_string())?;
            sessions.extend(db::get_chat_sessions(&conn, Some(id)).map_err(|e| e.to_string())?);
        }
        sessions.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(sessions)
    })
    .await
    .map_err(|e| format!("Chat database worker failed: {e}"))?
}

#[tauri::command]
async fn save_chat_session(app: AppHandle, session: db::DbChatSession) -> Result<(), String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let sample_id = session
            .sample_id
            .ok_or("A chat session must belong to a sample")?;
        let mut conn = db::connect_sample(&data_dir, sample_id).map_err(|e| e.to_string())?;
        db::save_chat_session(&mut conn, &session).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Chat database worker failed: {e}"))?
}

#[tauri::command]
async fn delete_chat_session(app: AppHandle, session_id: String) -> Result<(), String> {
    let data_dir = get_data_dir(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let registry = db::open_user_db(paths::db_path(&data_dir)).map_err(|e| e.to_string())?;
        for sample in db::get_samples(&registry).map_err(|e| e.to_string())? {
            let conn = db::connect_sample(&data_dir, sample.id).map_err(|e| e.to_string())?;
            if db::delete_chat_session(&conn, &session_id).map_err(|e| e.to_string())? > 0 {
                break;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Chat database worker failed: {e}"))?
}

#[tauri::command]
async fn fetch_external_api(
    app: AppHandle,
    url: String,
    api_key: Option<String>,
    ttl_secs: Option<u64>,
) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let url_for_cache = url.clone();
    let ttl = ttl_secs.unwrap_or(86400 * 7) as i64;

    let cache_hit = db_runtime::with_connection(db_path.clone(), move |conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_cache_db.api_cache (
                url TEXT PRIMARY KEY,
                response_json TEXT NOT NULL,
                fetched_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to initialize api_cache table: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if let Ok((cached_json, fetched_at)) = conn.query_row(
            "SELECT response_json, fetched_at FROM api_cache WHERE url = ?",
            rusqlite::params![url_for_cache],
            |row| {
                let json: String = row.get(0)?;
                let fetched: i64 = row.get(1)?;
                Ok((json, fetched))
            },
        ) && now - fetched_at < ttl
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&cached_json)
        {
            return Ok(Some(parsed));
        }

        Ok(None)
    })
    .await?;

    if let Some(parsed) = cache_hit {
        return Ok(parsed);
    }

    // Cache miss — validate URL before network egress
    config::validate_external_url(&url)?;

    let effective_api_key = if url.contains("eutils.ncbi.nlm.nih.gov") {
        if api_key.as_ref().is_some_and(|k| !k.trim().is_empty()) {
            api_key
        } else {
            db_runtime::with_connection(db_path.clone(), |conn| {
                Ok(config::load_qdrant_config(conn)
                    .ok()
                    .and_then(|c| c.ncbi_api_key))
            })
            .await
            .ok()
            .flatten()
        }
    } else {
        api_key
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(config::redirect_policy())
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let mut target_url = url.clone();

    if url.contains("eutils.ncbi.nlm.nih.gov")
        && let Some(ref key) = effective_api_key
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
        return Err(format!("Server returned HTTP error: {}", res.status()));
    }

    let json_val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    let json_str = serde_json::to_string(&json_val).unwrap_or_default();
    let url_write = url.clone();
    let _ = db_runtime::with_connection(db_path, move |conn| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "INSERT OR REPLACE INTO api_cache_db.api_cache (url, response_json, fetched_at) VALUES (?, ?, ?)",
            rusqlite::params![url_write, json_str, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await;

    Ok(json_val)
}

#[tauri::command]
fn get_current_exe() -> Result<String, String> {
    let p = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().to_string().replace('\\', "/"))
}

#[tauri::command]
fn get_mcp_tools() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!([
        {
            "name": "list_samples",
            "description": "Lists all imported DNA samples in the local SQLite database.",
            "params": []
        },
        {
            "name": "get_variants_by_rsid",
            "description": "Queries specific genotypes for a list of rsIDs in a given sample.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "rsids", "type": "array of strings", "required": true, "description": "List of rsIDs to query, e.g. ['rs1801133']" }
            ]
        },
        {
            "name": "get_variants_in_region",
            "description": "Queries all standard variants in a given chromosome region (GRCh38 coordinates).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "chromosome", "type": "string", "required": true, "description": "Chromosome number or label (e.g. '1', 'X')" },
                { "name": "start", "type": "integer", "required": true, "description": "Start base-pair position" },
                { "name": "end", "type": "integer", "required": true, "description": "End base-pair position" }
            ]
        },
        {
            "name": "generate_report",
            "description": "Generates a full direction-aware trait report for a sample using the built-in marker database. Returns evaluated markers with severity classes, section summaries, and risk-direction-only signal scores.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "template_json", "type": "string", "required": true, "description": "JSON string of the report template with sections and markers" }
            ]
        },
        {
            "name": "reload_report",
            "description": "Regenerates a report from current local resources and returns status=ready only after generation completes. Use after resource sync to confirm report reload completion.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "template_json", "type": "string", "required": false, "description": "Optional JSON string of the report template" }
            ]
        },
        {
            "name": "list_packs",
            "description": "Lists all available predefined genomic marker packs/bundles and their descriptions.",
            "params": []
        },
        {
            "name": "get_report_for_packs",
            "description": "Generates an evaluated genomic report for specific pack IDs (or all if omitted). Can optionally filter to only return active variant findings (effect_count > 0).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" },
                { "name": "pack_ids", "type": "array of strings", "required": false, "description": "Predefined pack IDs to evaluate, e.g. ['core', 'pgx', 'nutrients']. If omitted, evaluates all." },
                { "name": "only_active_findings", "type": "boolean", "required": false, "description": "If true, only returns evaluated markers with effect alleles detected (effect_count > 0). Defaults to false." }
            ]
        },
        {
            "name": "list_evidence_sources",
            "description": "Lists all unique source citations in the local RAG evidence library.",
            "params": []
        },
        {
            "name": "get_evidence_for_marker",
            "description": "Queries the local evidence library for references and interpretation notes associated with a specific rsID.",
            "params": [
                { "name": "rsid", "type": "string", "required": true, "description": "The target rsID, e.g. 'rs4680'" }
            ]
        },
        {
            "name": "search_evidence",
            "description": "Performs keyword and semantic vector search in the local evidence library.",
            "params": [
                { "name": "query", "type": "string", "required": true, "description": "The search term or query" },
                { "name": "ollama_url", "type": "string", "required": false, "description": "Ollama server URL for semantic search embeddings" },
                { "name": "ollama_token", "type": "string", "required": false, "description": "Authentication token for remote Ollama server" }
            ]
        },
        {
            "name": "get_chat_sessions",
            "description": "Lists saved consultation chat sessions from the local SQLite database.",
            "params": [
                { "name": "sample_id", "type": "integer", "required": false, "description": "Filter sessions by sample ID (optional)" }
            ]
        },
        {
            "name": "delete_chat_session",
            "description": "Deletes a specific consultation chat session.",
            "params": [
                { "name": "session_id", "type": "string", "required": true, "description": "The session ID to delete" }
            ]
        },
        {
            "name": "export_chat_history",
            "description": "Exports a saved chat session history in a clean, human-readable Markdown format.",
            "params": [
                { "name": "session_id", "type": "string", "required": true, "description": "The session ID to export" }
            ]
        },
        {
            "name": "get_app_paths",
            "description": "Retrieves the local application directory paths (database and marker packs folders).",
            "params": []
        },
        {
            "name": "get_offline_update_status",
            "description": "Returns local/offline reference inventory, update availability, and indexed row counts without genotype values.",
            "params": []
        },
        {
            "name": "sync_offline_asset",
            "description": "Downloads, validates, and imports one offline reference asset, then returns an authoritative final_status snapshot. Requires --mcp-write.",
            "params": [
                { "name": "asset_id", "type": "string", "required": true, "description": "Manifest asset ID" },
                { "name": "force", "type": "boolean", "required": false, "description": "Re-download an existing asset" },
                { "name": "sample_id", "type": "integer", "required": false, "description": "Sample ID for sample-derived Tier 2 assets" }
            ]
        },
        {
            "name": "sync_offline_data",
            "description": "Syncs one offline data tier or all tiers. Requires --mcp-write; a tier request keeps its sync fields and adds final_status, while an all-tier request returns {results, final_status}; no genotype values.",
            "params": [
                { "name": "tier", "type": "integer", "required": false, "description": "Tier 0, 1, or 2; omit for all tiers" },
                { "name": "force", "type": "boolean", "required": false, "description": "Re-download existing assets" },
                { "name": "sample_id", "type": "integer", "required": false, "description": "Sample ID for sample-derived Tier 2 assets" }
            ]
        },
        {
            "name": "check_chain_status",
            "description": "Checks if the GRCh37-to-GRCh38 liftover chain alignment file is locally present.",
            "params": []
        },
        {
            "name": "get_current_exe",
            "description": "Returns the absolute path of the running Genomics Caddy executable.",
            "params": []
        },
        {
            "name": "scan_ollama_models",
            "description": "Queries a local or remote Ollama server to list all available LLM models.",
            "params": [
                { "name": "url", "type": "string", "required": true, "description": "Ollama server URL" },
                { "name": "token", "type": "string", "required": false, "description": "Authentication token" }
            ]
        },
        {
            "name": "show_ollama_model",
            "description": "Retrieves detailed configuration and parameters for a specific Ollama model.",
            "params": [
                { "name": "url", "type": "string", "required": true, "description": "Ollama server URL" },
                { "name": "token", "type": "string", "required": false, "description": "Authentication token" },
                { "name": "name", "type": "string", "required": true, "description": "The model tag name" }
            ]
        },
        {
            "name": "get_app_bootstrap",
            "description": "Returns startup database stats after migrations: sample counts, genotype totals, GWAS/evidence counts, and paths.",
            "params": []
        },
        {
            "name": "get_research_job_status",
            "description": "Returns the persisted vector research enrichment job for a sample (idle/running/paused/complete/error).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" }
            ]
        },
        {
            "name": "get_discovered_findings_summary",
            "description": "Lists persisted Research Agent discoveries for a sample (rsID, gene, genotype, interpretation status).",
            "params": [
                { "name": "sample_id", "type": "integer", "required": true, "description": "The target sample ID" }
            ]
        },
        {
            "name": "get_active_ollama_models",
            "description": "Queries a local or remote Ollama server to list currently loaded models and VRAM usage.",
            "params": [
                { "name": "url", "type": "string", "required": true, "description": "Ollama server URL" },
                { "name": "token", "type": "string", "required": false, "description": "Authentication token" }
            ]
        }
    ]))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    config::init_env();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = paths::initialize_data_dir(app.handle());
            app_log::init(data_dir.clone());
            db::dump_default_marker_packs_if_missing(&data_dir);
            let _ = research::pack_draft::ensure_research_found_pack(&data_dir);
            agent_ui::start_http_bridge(app.handle().clone());
            // Apply window icon + ensure Linux/GNOME can match our .desktop (app id = identifier).
            if let Some(window) = app.get_webview_window("main") {
                let mut applied = false;
                if let Some(icon) = app.default_window_icon() {
                    applied = window.set_icon(icon.clone()).is_ok();
                }
                if !applied {
                    let candidates = [
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/128x128.png"),
                        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png"),
                        app.path()
                            .resource_dir()
                            .ok()
                            .map(|p| p.join("icons/128x128.png"))
                            .unwrap_or_default(),
                    ];
                    for path in candidates {
                        if path.as_os_str().is_empty() || !path.is_file() {
                            continue;
                        }
                        if let Ok(icon) = tauri::image::Image::from_path(&path) {
                            if window.set_icon(icon).is_ok() {
                                break;
                            }
                        }
                    }
                }
                #[cfg(all(debug_assertions, target_os = "linux"))]
                {
                    // Dev webviews keep a WebKit HTTP cache that can replay stale Vite
                    // `?svelte&type=style` module URLs after CSS pipeline changes.
                    let data_home = std::env::var_os("XDG_DATA_HOME")
                        .map(PathBuf::from)
                        .or_else(|| {
                            std::env::var_os("HOME")
                                .map(|h| PathBuf::from(h).join(".local/share"))
                        });
                    if let Some(data) = data_home {
                        let webkit = data.join("com.dna.explorer");
                        for leaf in ["WebKitCache", "CacheStorage"] {
                            let p = webkit.join(leaf);
                            if p.is_dir() {
                                let _ = std::fs::remove_dir_all(&p);
                            }
                        }
                    }
                    app_log::log(
                        "SYSTEM",
                        &format!(
                            "Linux window icon applied={applied}; GTK app_id=com.dna.explorer. Expect desktop file ~/.local/share/applications/com.dna.explorer.desktop"
                        ),
                    );
                }
                window.on_window_event(|event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if crate::portable_update::update_in_progress() {
                            api.prevent_close();
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_file,
            select_directory,
            save_report_json,
            save_report_bundle,
            get_app_bootstrap,
            get_app_paths,
            library::get_library_status,
            library::open_library_dir,
            library::set_library_dir,
            library::reset_library_dir,
            library::erase_library_data,
            portable_update::get_update_channel,
            portable_update::set_update_in_progress,
            portable_update::apply_portable_update,
            get_all_marker_packs,
            get_marker_pack_warnings,
            reload_marker_packs,
            query_local_reference_db,
            import_genome,
            inspect_genome,
            get_samples,
            query_rsids,
            query_region,
            generate_report,
            agent_ui::agent_ui_invoke,
            log_js_error,
            delete_sample,
            check_chain_status,
            download_chain_file,
            scan_ollama_models,
            probe_inference_host,
            discover_ollama_models,
            probe_localhost_services,
            get_ollama_version,
            get_qdrant_version,
            check_ollama_update,
            check_qdrant_update,
            pull_ollama_model,
            delete_ollama_model,
            probe_vector_provider,
            stream_ollama_chat,
            cancel_ollama_stream,
            show_ollama_model,
            get_active_ollama_models,
            get_current_exe,
            get_mcp_tools,
            list_evidence_sources,
            search_evidence,
            get_evidence_for_marker,
            get_chat_sessions,
            save_chat_session,
            delete_chat_session,
            fetch_external_api,
            get_chromosome_counts,
            research::commands::get_research_scope,
            research::commands::save_research_scope,
            research::commands::preview_research_scope,
            research::commands::preview_pipeline_tuning,
            research::commands::get_reference_status,
            research::commands::sync_gwas_reference,
            offline::commands::check_offline_data_updates,
            offline::commands::sync_offline_data_tier,
            offline::commands::sync_single_offline_asset,
            offline::commands::sync_all_offline_missing,
            offline::commands::sync_all_offline_data,
            offline::commands::build_offline_tier2,
            offline::commands::cancel_offline_import,
            offline::commands::export_discovery_findings,
            offline::commands::query_discovery_findings,
            offline::commands::cancel_discovery_query,
            offline::commands::get_custom_download_dir,
            offline::commands::set_custom_download_dir,
            offline::commands::get_offline_reference_status,
            research::commands::test_qdrant_connection,
            research::commands::get_qdrant_config,
            research::commands::save_qdrant_config,
            research::commands::create_qdrant_collection,
            research::commands::purge_qdrant_collection,
            research::commands::get_research_job_status,
            research::commands::start_research_job,
            research::commands::pause_research_job,
            research::commands::cancel_research_job,
            research::commands::resume_research_job,
            research::commands::get_research_debug_log,
            research::commands::set_research_debug_log,
            research::commands::search_qdrant_evidence,
            research::commands::search_qdrant_trait_discovery,
            research::commands::get_vector_promoted_findings,
            research::commands::get_vector_research_diagnostics,
            research::commands::get_recent_finding_previews,
            research::commands::browse_vector_store,
            research::commands::export_pack_draft_from_vectors,
            research::commands::list_pack_draft_exports,
            research::commands::list_runtime_marker_pack_ids,
            research::commands::merge_pack_draft_into_pack,
            research::commands::get_research_found_pack,
            research::commands::fill_research_found_alleles,
            research::commands::set_research_found_enabled,
            research::commands::update_research_found_marker,
            research::commands::delete_research_found_markers,
            research::commands::get_ollama_token,
            research::commands::get_ollama_service_config,
            research::commands::save_ollama_token,
            research::commands::save_ollama_url,
            research::commands::purge_database_cache,
            research::commands::select_save_path,
            research::evidence::commands::search_associations_hybrid,
            research::evidence::commands::get_similar_associations,
            research::evidence::commands::explain_vector_match_cmd,
            research::evidence::commands::get_quality_dashboard,
            research::evidence::commands::build_trait_clusters_cmd,
            research::evidence::commands::export_evidence_packet_cmd,
            research::evidence::commands::list_candidate_markers,
            research::evidence::commands::update_candidate_marker_status,
            research::evidence::commands::ensure_qdrant_payload_indexes,
            research::evidence::commands::get_variant_evidence_card,
            research::evidence::commands::backfill_evidence_payloads_cmd,
            research::evidence::commands::reembed_stale_vectors_cmd,
            research::evidence::commands::get_actionability_matrix,
            research::evidence::commands::get_chromosome_trait_overlay,
            research::evidence::commands::get_pathway_flow_rows,
            research::evidence::commands::build_vector_atlas_cmd,
            research::evidence::commands::get_vector_atlas_cached,
            research::evidence::commands::enable_named_vectors_collection,
            research::evidence::commands::get_evidence_corpus_summary,
            research::evidence::commands::browse_associations_cmd,
            research::gnomad::commands::get_gnomad_context_cmd,
            research::gnomad::commands::batch_enrich_gnomad_context_cmd,
            research::gnomad::commands::get_gnomad_config_cmd,
            research::gnomad::commands::save_gnomad_config_cmd,
            research::gnomad::commands::test_gnomad_source_urls_cmd,
            research::gnomad::commands::get_gnomad_readiness_cmd,
            research::gnomad::commands::download_gnomad_indexes_cmd,
            research::gnomad::commands::refresh_gnomad_frequency_cache_cmd,
            research::gnomad::commands::select_gnomad_local_dir_cmd,
            research::gnomad::commands::clear_gnomad_cache_cmd,
            agent_commands::get_variant_evidence,
            agent_commands::get_discovered_findings_summary,
            agent_commands::get_db_discovered_findings,
            agent_commands::run_safety_audit,
            agent_commands::export_discovered_findings,
            agent_commands::chat_ollama,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                agent_ui::cleanup_endpoint_file();
                let db_path = paths::db_path(&paths::resolve_data_dir());
                if let Err(e) = db::seal(&db_path) {
                    eprintln!("Failed to seal genome database at rest: {e}");
                }
            }
        });
}
