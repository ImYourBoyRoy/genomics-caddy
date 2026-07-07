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

use crate::db::get_pack_str;
use crate::paths::{self, gwas_catalog_file, gwas_catalog_gz_file, gwas_catalog_zip_file};
use super::util::{normalize_gwas_reference_rsids, normalize_rsid, parse_gene_tokens};
use flate2::read::GzDecoder;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{copy, BufRead, BufReader};
use std::path::Path;
use zip::ZipArchive;

const GWAS_DOWNLOAD_URL: &str =
    "https://ftp.ebi.ac.uk/pub/databases/gwas/releases/latest/gwas-catalog-associations_ontology-annotated-full.zip";
const GWAS_DOWNLOAD_URL_LEGACY: &str =
    "https://ftp.ebi.ac.uk/pub/databases/gwas/releases/latest/gwas-catalog-associations-full.zip";

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
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| row.get::<_, i64>(0))
        .unwrap_or(0) as u64;
    Ok(ReferenceStatus {
        data_dir: data_dir.to_string_lossy().to_string(),
        gwas_rsid_count: count,
        gwas_file_present: gwas_catalog_file(data_dir).exists(),
        gwas_gz_present: gwas_catalog_gz_file(data_dir).exists() || gwas_catalog_zip_file(data_dir).exists(),
    })
}

pub fn normalize_gwas_reference_rsids_on_startup(conn: &Connection) -> Result<usize, String> {
    normalize_gwas_reference_rsids(conn)
}

pub fn seed_gwas_reference_fallback(conn: &Connection, data_dir: Option<&Path>) -> Result<usize, String> {
    let existing: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())? as u64;
    if existing > 0 {
        return Ok(0);
    }

    let catalog_str = get_pack_str(data_dir, "discovery_catalog");
    let Some(catalog_str) = catalog_str else {
        return Ok(0);
    };

    let catalog: serde_json::Value =
        serde_json::from_str(&catalog_str).map_err(|e| format!("discovery_catalog parse error: {}", e))?;
    let markers = catalog["markers"].as_array().cloned().unwrap_or_default();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut inserted = 0usize;
    for marker in markers {
        if let Some(rsid) = marker["rsid"].as_str() {
            let gene = marker["gene"].as_str().unwrap_or("");
            let impact = marker["impact"].as_str().unwrap_or("");
            tx.execute(
                "INSERT OR IGNORE INTO reference.gwas_reference (rsid, association_count, top_trait, primary_gene, mapped_genes, reported_genes, associations_json)
                 VALUES (?, 1, ?, ?, ?, ?, ?)",
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

pub async fn sync_gwas_reference(data_dir: &Path, db_path: &Path) -> Result<GwasSyncResult, String> {
    paths::ensure_data_layout(data_dir).map_err(|e| e.to_string())?;
    let (source_path, downloaded) = ensure_gwas_source_file(data_dir).await?;
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    let rsid_count = import_gwas_reference_tsv(&conn, &source_path)?;
    Ok(GwasSyncResult {
        rsid_count,
        downloaded,
        file_path: source_path.to_string_lossy().to_string(),
        message: format!(
            "Loaded {} GWAS-linked rsIDs with gene/trait cross-refs from {}. Re-run enrichment to refresh vectors.",
            rsid_count,
            source_path.file_name().unwrap_or_default().to_string_lossy()
        ),
    })
}

/// Import GWAS catalog from files already on disk (blocking — safe inside spawn_blocking).
pub fn import_gwas_from_local_files(data_dir: &Path, db_path: &Path) -> Result<u64, String> {
    paths::ensure_data_layout(data_dir).map_err(|e| e.to_string())?;
    let tsv_path = gwas_catalog_file(data_dir);
    let zip_path = gwas_catalog_zip_file(data_dir);
    let gz_path = gwas_catalog_gz_file(data_dir);
    let source_path = if tsv_path.exists() {
        tsv_path
    } else if zip_path.exists() {
        extract_gwas_tsv_from_zip(&zip_path, &tsv_path)?;
        tsv_path
    } else if gz_path.exists() {
        decompress_gz_to_tsv(&gz_path, &tsv_path)?;
        tsv_path
    } else {
        return Err("GWAS catalog file not found — download Tier 0 first.".into());
    };
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    import_gwas_reference_tsv(&conn, &source_path)
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
    } else if zip_path.exists() {
        extract_gwas_tsv_from_zip(&zip_path, &tsv_path)?;
        tsv_path
    } else if gz_path.exists() {
        decompress_gz_to_tsv(&gz_path, &tsv_path)?;
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
        std::fs::write(dest_zip, &bytes).map_err(|e| format!("Failed to write GWAS file: {}", e))?;
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

fn decompress_gz_to_tsv(gz_path: &Path, tsv_path: &Path) -> Result<(), String> {
    let gz = File::open(gz_path).map_err(|e| e.to_string())?;
    let decoder = GzDecoder::new(gz);
    let mut reader = BufReader::new(decoder);
    let mut out = File::create(tsv_path).map_err(|e| e.to_string())?;
    copy(&mut reader, &mut out).map_err(|e| e.to_string())?;
    Ok(())
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
            self.best_pvalue = Some(match self.best_pvalue {
                Some(current) => current.min(p),
                None => p,
            });
        }
        for gene in reported {
            self.reported_genes.insert(gene.clone());
        }
        if let Some(gene) = mapped.filter(|g| !g.is_empty()) {
            self.mapped_genes.insert(gene.to_string());
        }
        if self.associations.len() < 12 {
            self.associations.push(super::util::canonical_gwas_association(
                trait_name,
                pvalue,
                reported,
                mapped,
                study_accession,
                "gwas_catalog_local",
            ));
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

fn import_gwas_reference_tsv(conn: &Connection, path: &Path) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("Open GWAS TSV failed: {}", e))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "GWAS TSV is empty".to_string())?;

    let cols: Vec<&str> = header.split('\t').collect();
    let snps_idx = find_column(&cols, &["SNPS"])
        .ok_or_else(|| "GWAS TSV missing SNPS column".to_string())?;
    let trait_idx = find_column(
        &cols,
        &["DISEASE/TRAIT", "MAPPED_TRAIT", "DISEASE TRAIT"],
    );
    let reported_idx = find_column(
        &cols,
        &["REPORTED GENE(S)", "REPORTED_GENE(S)", "REPORTED GENES"],
    );
    let mapped_idx = find_column(&cols, &["MAPPED_GENE", "MAPPED GENE"]);
    let pvalue_idx = find_column(&cols, &["P-VALUE", "PVALUE", "P VALUE"]);
    let study_idx = find_column(
        &cols,
        &["STUDY ACCESSION", "GWAS CATALOG STUDY ACCESSION", "STUDY_ACCESSION"],
    );

    conn.execute("DELETE FROM reference.gwas_reference", [])
        .map_err(|e| e.to_string())?;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut seen: HashMap<String, GwasRsidAggregate> = HashMap::new();

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
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
    }

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
        let associations_json =
            serde_json::to_string(&agg.associations).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            "INSERT INTO reference.gwas_reference (rsid, association_count, top_trait, primary_gene, mapped_genes, reported_genes, best_pvalue, associations_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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
    }

    tx.commit().map_err(|e| e.to_string())?;
    let _ = normalize_gwas_reference_rsids(conn);
    let total: u64 = conn
        .query_row("SELECT COUNT(*) FROM gwas_reference", [], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())? as u64;
    Ok(total)
}
