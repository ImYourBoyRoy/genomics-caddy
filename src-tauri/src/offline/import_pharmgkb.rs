// ./src-tauri/src/offline/import_pharmgkb.rs
use crate::research::util::normalize_rsid;
use rusqlite::{params, Connection};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

fn extract_tsv_from_zip(zip_path: &Path, needle: &str) -> Result<Vec<u8>, String> {
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.to_lowercase();
        if !name.ends_with(".tsv") && !name.ends_with(".txt") {
            continue;
        }
        if !name.contains(needle) && needle != "any" {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        return Ok(buf);
    }
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.to_lowercase();
        if name.ends_with(".tsv") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            return Ok(buf);
        }
    }
    Err(format!("No TSV found in {}", zip_path.display()))
}

fn col_index(headers: &[&str], name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(name) || h.replace(' ', "") == name.replace(' ', ""))
}

pub fn import_pharmgkb_clinical_variants(conn: &Connection, zip_path: &Path) -> Result<u64, String> {
    let bytes = extract_tsv_from_zip(zip_path, "clinical")?;
    conn.execute("DELETE FROM reference.pharmgkb_clinical_variants", [])
        .map_err(|e| e.to_string())?;

    let reader = BufReader::new(bytes.as_slice());
    let mut lines = reader.lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "PharmGKB clinical TSV empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();

    let rs_idx = col_index(&headers, "Variant/Haplotypes")
        .or_else(|| col_index(&headers, "RSID"))
        .or_else(|| col_index(&headers, "Variant"));
    let gene_idx = col_index(&headers, "Gene");
    let drug_idx = col_index(&headers, "Drug(s)")
        .or_else(|| col_index(&headers, "Chemical"));
    let pheno_idx = col_index(&headers, "Phenotype Category")
        .or_else(|| col_index(&headers, "Phenotype"));
    let level_idx = col_index(&headers, "Level of Evidence")
        .or_else(|| col_index(&headers, "Evidence Level"));

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let rsid = rs_idx
            .and_then(|i| parts.get(i))
            .and_then(|s| normalize_rsid(s.trim()))
            .or_else(|| {
                rs_idx
                    .and_then(|i| parts.get(i))
                    .and_then(|s| s.split(';').find_map(|t| normalize_rsid(t.trim())))
            });
        let Some(rsid) = rsid else {
            continue;
        };
        let gene = gene_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let drug = drug_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let phenotype = pheno_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let evidence = level_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        tx.execute(
            "INSERT INTO reference.pharmgkb_clinical_variants (rsid, gene, drug, phenotype, evidence_level, raw_json)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![rsid, gene, drug, phenotype, evidence, line],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

pub fn import_pharmgkb_genes(conn: &Connection, zip_path: &Path) -> Result<u64, String> {
    let bytes = extract_tsv_from_zip(zip_path, "genes")?;
    conn.execute("DELETE FROM reference.pharmgkb_genes", [])
        .map_err(|e| e.to_string())?;

    let reader = BufReader::new(bytes.as_slice());
    let mut lines = reader.lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "PharmGKB genes TSV empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();
    let id_idx = col_index(&headers, "PharmGKB Accession Id")
        .or_else(|| col_index(&headers, "Accession Id"));
    let sym_idx = col_index(&headers, "Symbol").or_else(|| col_index(&headers, "Gene Symbol"));
    let name_idx = col_index(&headers, "Name");

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;
    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let id = id_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let symbol = sym_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let Some(symbol) = symbol else { continue };
        let name = name_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let pgx_id = id.unwrap_or_else(|| symbol.clone());
        tx.execute(
            "INSERT OR REPLACE INTO reference.pharmgkb_genes (pharmgkb_id, symbol, name, raw_json) VALUES (?, ?, ?, ?)",
            params![pgx_id, symbol, name, line],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
