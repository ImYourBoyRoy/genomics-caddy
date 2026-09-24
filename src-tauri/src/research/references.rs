// ./src-tauri/src/research/references.rs
/*
Module Docstring:
Purpose: Download and load external reference catalogs into ./data/references.
Responsibilities:
- Fetch GWAS Catalog association file into the local data folder.
- Parse rsIDs into SQLite gwas_reference for discovery-scope sweeps.
- Seed lightweight fallback rsIDs from discovery_catalog when no download exists.
Key Inputs: Data directory path, SQLite connection.
Key Outputs: Row counts, sync status payloads for the UI.
Operational Notes: Large GWAS file is streamed; re-import replaces gwas_reference rows.
*/

use super::util::{normalize_gwas_reference_rsids, normalize_rsid, parse_gene_tokens};
use crate::db::get_pack_str;
use crate::offline::compress::{
    DEFAULT_COMPRESSION_THRESHOLD_BYTES, compress_if_large, open_text_auto,
};
use crate::paths::{self, gwas_catalog_file, gwas_catalog_gz_file, gwas_catalog_zip_file};
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, copy};
use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

const GWAS_DOWNLOAD_URL: &str = "https://ftp.ebi.ac.uk/pub/databases/gwas/releases/latest/gwas-catalog-associations_ontology-annotated-full.zip";
const GWAS_DOWNLOAD_URL_LEGACY: &str =
    "https://ftp.ebi.ac.uk/pub/databases/gwas/releases/latest/gwas-catalog-associations-full.zip";
const MAX_STORED_GWAS_ASSOCIATIONS_PER_RSID: usize = 12;

#[derive(Debug, Serialize, Clone)]
pub struct ReferenceStatus {
    pub data_dir: String,
    pub gwas_rsid_count: u64,
    pub gwas_file_present: bool,
    pub gwas_gz_present: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct GwasSyncResult {
    pub rsid_count: u64,
    pub downloaded: bool,
    pub file_path: String,
    pub message: String,
}

pub fn reference_status(data_dir: &Path, conn: &Connection) -> Result<ReferenceStatus, String> {
    let count: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as u64;
    Ok(ReferenceStatus {
        data_dir: data_dir.to_string_lossy().to_string(),
        gwas_rsid_count: count,
        gwas_file_present: gwas_catalog_file(data_dir).exists(),
        gwas_gz_present: gwas_catalog_gz_file(data_dir).exists()
            || gwas_catalog_zip_file(data_dir).exists(),
    })
}

pub fn normalize_gwas_reference_rsids_on_startup(conn: &Connection) -> Result<usize, String> {
    normalize_gwas_reference_rsids(conn)
}

pub fn seed_gwas_reference_fallback(
    conn: &Connection,
    data_dir: Option<&Path>,
) -> Result<usize, String> {
    let existing: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())? as u64;
    if existing > 0 {
        return Ok(0);
    }

    let catalog_str = get_pack_str(data_dir, "discovery_catalog");
    let Some(catalog_str) = catalog_str else {
        return Ok(0);
    };

    let catalog: serde_json::Value = serde_json::from_str(&catalog_str)
        .map_err(|e| format!("discovery_catalog parse error: {}", e))?;
    let markers = catalog["markers"].as_array().cloned().unwrap_or_default();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut inserted = 0usize;
    let gwas_table = if crate::offline::schema::schema_attached(conn, "gwas") {
        "gwas.gwas_reference"
    } else {
        "reference.gwas_reference"
    };
    for marker in markers {
        if let Some(rsid) = marker["rsid"].as_str() {
            let gene = marker["gene"].as_str().unwrap_or("");
            let impact = marker["impact"].as_str().unwrap_or("");
            tx.execute(
                &format!(
                    "INSERT OR IGNORE INTO {gwas_table} (rsid, association_count, top_trait, primary_gene, mapped_genes, reported_genes, associations_json)
                     VALUES (?, 1, ?, ?, ?, ?, ?)"
                ),
                params![
                    normalize_rsid(rsid).unwrap_or_else(|| rsid.to_lowercase()),
                    format!("{} — {}", gene, impact),
                    gene,
                    gene,
                    gene,
                    serde_json::json!([super::util::canonical_gwas_association(
                        &format!("{} — {}", gene, impact),
                        Some(1e-10),
                        &parse_gene_tokens(gene),
                        Some(gene),
                        None,
                        "discovery_catalog_fallback",
                    )]).to_string(),
                ],
            )
            .map_err(|e| e.to_string())?;
            inserted += 1;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(inserted)
}

pub async fn sync_gwas_reference(
    data_dir: &Path,
    db_path: &Path,
) -> Result<GwasSyncResult, String> {
    paths::ensure_data_layout(data_dir).map_err(|e| e.to_string())?;
    let (source_path, downloaded) = ensure_gwas_source_file(data_dir).await?;
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let rsid_count = import_gwas_reference_tsv(&conn, &source_path, None)?;
    let final_path = compress_if_large(&source_path, DEFAULT_COMPRESSION_THRESHOLD_BYTES)?;
    Ok(GwasSyncResult {
        rsid_count,
        downloaded,
        file_path: final_path.to_string_lossy().to_string(),
        message: format!(
            "Loaded {} GWAS-linked rsIDs with gene/trait cross-refs from {}. Re-run enrichment to refresh vectors.",
            rsid_count,
            final_path.file_name().unwrap_or_default().to_string_lossy()
        ),
    })
}

/// Import GWAS catalog from files already on disk (blocking — safe inside spawn_blocking).
pub fn import_gwas_from_local_files(
    data_dir: &Path,
    db_path: &Path,
    app: Option<&AppHandle>,
) -> Result<u64, String> {
    paths::ensure_data_layout(data_dir).map_err(|e| e.to_string())?;
    let tsv_path = gwas_catalog_file(data_dir);
    let zip_path = gwas_catalog_zip_file(data_dir);
    let gz_path = gwas_catalog_gz_file(data_dir);
    if let Some(handle) = app {
        let _ = handle.emit(
            "offline:import_progress",
            serde_json::json!({
                "asset_id": "gwas_catalog",
                "rows_processed": 0,
                "percent": 0,
                "rows_per_second": 0,
                "eta_seconds": null,
                "message": "Preparing GWAS catalog import…"
            }),
        );
    }
    let source_path = if tsv_path.exists() {
        tsv_path
    } else if gz_path.exists() {
        gz_path
    } else if zip_path.exists() {
        if let Some(handle) = app {
            let _ = handle.emit(
                "offline:import_progress",
                serde_json::json!({
                    "asset_id": "gwas_catalog",
                    "rows_processed": 0,
                    "percent": 1,
                    "rows_per_second": 0,
                    "eta_seconds": null,
                    "message": "Extracting GWAS catalog zip…"
                }),
            );
        }
        extract_gwas_tsv_from_zip(&zip_path, &tsv_path)?;
        tsv_path
    } else {
        return Err("GWAS catalog file not found — download Tier 0 first.".into());
    };
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let row_count = import_gwas_reference_tsv(&conn, &source_path, app)?;
    compress_if_large(&source_path, DEFAULT_COMPRESSION_THRESHOLD_BYTES)?;
    Ok(row_count)
}

async fn ensure_gwas_source_file(data_dir: &Path) -> Result<(std::path::PathBuf, bool), String> {
    let gz_path = gwas_catalog_gz_file(data_dir);
    let zip_path = gwas_catalog_zip_file(data_dir);
    let tsv_path = gwas_catalog_file(data_dir);
    let mut downloaded = false;

    if !gz_path.exists() && !zip_path.exists() && !tsv_path.exists() {
        download_gwas_catalog(&zip_path).await?;
        downloaded = true;
    }

    let source_path = if tsv_path.exists() {
        tsv_path
    } else if gz_path.exists() {
        gz_path
    } else if zip_path.exists() {
        extract_gwas_tsv_from_zip(&zip_path, &tsv_path)?;
        tsv_path
    } else {
        return Err("GWAS catalog file missing after download attempt.".to_string());
    };
    Ok((source_path, downloaded))
}

async fn download_gwas_catalog(dest_zip: &Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_error = String::new();
    for url in [GWAS_DOWNLOAD_URL, GWAS_DOWNLOAD_URL_LEGACY] {
        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("GWAS download failed: {}", e);
                continue;
            }
        };

        if !response.status().is_success() {
            last_error = format!("GWAS download failed with HTTP {}", response.status());
            continue;
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("GWAS download read failed: {}", e))?;

        if let Some(parent) = dest_zip.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(dest_zip, &bytes)
            .map_err(|e| format!("Failed to write GWAS file: {}", e))?;
        return Ok(());
    }

    Err(last_error)
}

fn extract_gwas_tsv_from_zip(zip_path: &Path, tsv_path: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Open GWAS zip failed: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.to_string();
        if !name.ends_with(".tsv") {
            continue;
        }

        if let Some(parent) = tsv_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = File::create(tsv_path).map_err(|e| e.to_string())?;
        copy(&mut entry, &mut out).map_err(|e| format!("Extract GWAS TSV failed: {}", e))?;
        return Ok(());
    }

    Err("GWAS zip did not contain a .tsv file".to_string())
}

fn find_column(cols: &[&str], candidates: &[&str]) -> Option<usize> {
    cols.iter().position(|col| {
        let normalized = col.trim().to_uppercase();
        candidates.iter().any(|candidate| {
            normalized == candidate.to_uppercase()
                || col.eq_ignore_ascii_case(candidate)
                || normalized.replace(' ', "") == candidate.to_uppercase().replace(' ', "")
        })
    })
}

#[derive(Default)]
struct GwasRsidAggregate {
    count: u32,
    top_trait: String,
    best_pvalue: Option<f64>,
    mapped_genes: HashSet<String>,
    reported_genes: HashSet<String>,
    associations: Vec<serde_json::Value>,
}

impl GwasRsidAggregate {
    fn record_association(
        &mut self,
        trait_name: &str,
        pvalue: Option<f64>,
        reported: &[String],
        mapped: Option<&str>,
        study_accession: Option<&str>,
    ) {
        self.count += 1;
        if self.top_trait.is_empty() && !trait_name.is_empty() {
            self.top_trait = trait_name.to_string();
        }
        if let Some(p) = pvalue {
            if self.best_pvalue.is_none_or(|current| p < current) {
                self.best_pvalue = Some(p);
                if !trait_name.is_empty() {
                    self.top_trait = trait_name.to_string();
                }
            }
        }
        for gene in reported {
            self.reported_genes.insert(gene.clone());
        }
        if let Some(gene) = mapped.filter(|g| !g.is_empty()) {
            self.mapped_genes.insert(gene.to_string());
        }
        let association = super::util::canonical_gwas_association(
            trait_name,
            pvalue,
            reported,
            mapped,
            study_accession,
            "gwas_catalog_local",
        );
        if self.associations.len() < MAX_STORED_GWAS_ASSOCIATIONS_PER_RSID {
            self.associations.push(association);
        } else {
            let weakest_index = self
                .associations
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    left["pvalue"]
                        .as_f64()
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&right["pvalue"].as_f64().unwrap_or(f64::INFINITY))
                })
                .map(|(index, _)| index);
            if let Some(index) = weakest_index {
                let weakest_pvalue = self.associations[index]["pvalue"]
                    .as_f64()
                    .unwrap_or(f64::INFINITY);
                if pvalue.unwrap_or(f64::INFINITY) < weakest_pvalue {
                    self.associations[index] = association;
                }
            }
        }
    }

    fn primary_gene(&self) -> String {
        self.mapped_genes
            .iter()
            .next()
            .cloned()
            .or_else(|| self.reported_genes.iter().next().cloned())
            .unwrap_or_default()
    }
}

fn import_gwas_reference_tsv(
    conn: &Connection,
    path: &Path,
    app: Option<&AppHandle>,
) -> Result<u64, String> {
    let total_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let reader = open_text_auto(path).map_err(|error| format!("Open GWAS TSV failed: {error}"))?;
    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "GWAS TSV is empty".to_string())?;

    let cols: Vec<&str> = header.split('\t').collect();
    let snps_idx =
        find_column(&cols, &["SNPS"]).ok_or_else(|| "GWAS TSV missing SNPS column".to_string())?;
    let trait_idx = find_column(&cols, &["DISEASE/TRAIT", "MAPPED_TRAIT", "DISEASE TRAIT"]);
    let reported_idx = find_column(
        &cols,
        &["REPORTED GENE(S)", "REPORTED_GENE(S)", "REPORTED GENES"],
    );
    let mapped_idx = find_column(&cols, &["MAPPED_GENE", "MAPPED GENE"]);
    let pvalue_idx = find_column(&cols, &["P-VALUE", "PVALUE", "P VALUE"]);
    let study_idx = find_column(
        &cols,
        &[
            "STUDY ACCESSION",
            "GWAS CATALOG STUDY ACCESSION",
            "STUDY_ACCESSION",
        ],
    );

    let gwas_table = if crate::offline::schema::schema_attached(conn, "gwas") {
        "gwas.gwas_reference"
    } else {
        "reference.gwas_reference"
    };
    conn.execute(&format!("DELETE FROM {gwas_table}"), [])
        .map_err(|e| e.to_string())?;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut seen: HashMap<String, GwasRsidAggregate> = HashMap::new();
    let start = Instant::now();
    let mut lines_read = 0u64;
    let mut bytes_approx = header.len() as u64 + 1;
    let mut last_emit = Instant::now();

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        bytes_approx = bytes_approx.saturating_add(line.len() as u64 + 1);
        lines_read += 1;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() <= snps_idx {
            continue;
        }
        let snps_field = parts[snps_idx];
        let trait_name = trait_idx
            .and_then(|idx| parts.get(idx))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let reported = reported_idx
            .and_then(|idx| parts.get(idx))
            .map(|s| parse_gene_tokens(s))
            .unwrap_or_default();
        let mapped = mapped_idx
            .and_then(|idx| parts.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|g| !g.is_empty());
        let pvalue = pvalue_idx
            .and_then(|idx| parts.get(idx))
            .and_then(|s| s.trim().parse::<f64>().ok());
        let study_accession = study_idx
            .and_then(|idx| parts.get(idx))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        for token in snps_field.split([';', ',', '|', ' ']) {
            if let Some(rsid) = normalize_rsid(token) {
                let entry = seen.entry(rsid).or_default();
                entry.record_association(
                    &trait_name,
                    pvalue,
                    &reported,
                    mapped.as_deref(),
                    study_accession.as_deref(),
                );
            }
        }

        if let Some(handle) = app
            && last_emit.elapsed().as_millis() >= 400
        {
            last_emit = Instant::now();
            let elapsed = start.elapsed().as_secs_f64().max(0.1);
            let lines_per_sec = lines_read as f64 / elapsed;
            let percent = if total_bytes > 0 {
                ((bytes_approx as f64 / total_bytes as f64) * 70.0).min(70.0) as u64
            } else {
                0
            };
            let eta_seconds = if lines_per_sec > 0.0 && total_bytes > bytes_approx {
                let remaining_bytes = (total_bytes - bytes_approx) as f64;
                let avg_bytes_per_line = bytes_approx as f64 / lines_read.max(1) as f64;
                Some(((remaining_bytes / avg_bytes_per_line) / lines_per_sec) as u64)
            } else {
                None
            };
            let _ = handle.emit(
                "offline:import_progress",
                serde_json::json!({
                    "asset_id": "gwas_catalog",
                    "rows_processed": lines_read,
                    "percent": percent,
                    "rows_per_second": lines_per_sec as u64,
                    "eta_seconds": eta_seconds,
                    "message": format!(
                        "Parsing GWAS associations: {} lines ({:.0}/s) · {} unique rsIDs…",
                        lines_read, lines_per_sec, seen.len()
                    )
                }),
            );
        }
    }

    if let Some(handle) = app {
        let _ = handle.emit(
            "offline:import_progress",
            serde_json::json!({
                "asset_id": "gwas_catalog",
                "rows_processed": seen.len() as u64,
                "percent": 75,
                "rows_per_second": 0,
                "eta_seconds": null,
                "message": format!("Writing {} unique rsIDs to SQLite…", seen.len())
            }),
        );
    }

    let unique_count = seen.len() as u64;
    let mut written = 0u64;
    for (rsid, agg) in seen {
        let mapped_genes = agg
            .mapped_genes
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let reported_genes = agg
            .reported_genes
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        let mut stored_associations = agg.associations.clone();
        stored_associations.sort_by(|left, right| {
            left["pvalue"]
                .as_f64()
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right["pvalue"].as_f64().unwrap_or(f64::INFINITY))
        });
        let associations_json =
            serde_json::to_string(&stored_associations).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            &format!(
                "INSERT INTO {gwas_table} (rsid, association_count, top_trait, primary_gene, mapped_genes, reported_genes, best_pvalue, associations_json)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            ),
            params![
                rsid,
                agg.count as i64,
                agg.top_trait,
                agg.primary_gene(),
                mapped_genes,
                reported_genes,
                agg.best_pvalue,
                associations_json,
            ],
        )
        .map_err(|e| e.to_string())?;
        written += 1;
        if let Some(handle) = app
            && written.is_multiple_of(25_000)
        {
            let percent = 75 + ((written as f64 / unique_count.max(1) as f64) * 20.0) as u64;
            let _ = handle.emit(
                "offline:import_progress",
                serde_json::json!({
                    "asset_id": "gwas_catalog",
                    "rows_processed": written,
                    "percent": percent.min(95),
                    "rows_per_second": 0,
                    "eta_seconds": null,
                    "message": format!("Writing rsIDs to SQLite: {written}/{unique_count}…")
                }),
            );
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    let _ = normalize_gwas_reference_rsids(conn);
    let total: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())? as u64;
    if let Some(handle) = app {
        let _ = handle.emit(
            "offline:import_progress",
            serde_json::json!({
                "asset_id": "gwas_catalog",
                "rows_processed": total,
                "percent": 100,
                "rows_per_second": 0,
                "eta_seconds": 0,
                "message": format!("GWAS catalog ready — {total} rsIDs indexed")
            }),
        );
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_strongest_gwas_records_within_the_per_rsid_limit() {
        let mut aggregate = GwasRsidAggregate::default();
        for index in 0..20 {
            let trait_name = format!("Synthetic trait {index}");
            let study_accession = format!("GCST{index:06}");
            let pvalue = 10f64.powi(-(index + 1));
            aggregate.record_association(
                &trait_name,
                Some(pvalue),
                &[],
                None,
                Some(&study_accession),
            );
        }

        assert_eq!(aggregate.count, 20);
        assert_eq!(aggregate.best_pvalue, Some(1e-20));
        assert_eq!(aggregate.top_trait, "Synthetic trait 19");
        assert_eq!(aggregate.associations.len(), MAX_STORED_GWAS_ASSOCIATIONS_PER_RSID);
        assert!(aggregate
            .associations
            .iter()
            .any(|record| record["trait_name"] == "Synthetic trait 19"));
        assert!(!aggregate
            .associations
            .iter()
            .any(|record| record["trait_name"] == "Synthetic trait 0"));
    }
}
