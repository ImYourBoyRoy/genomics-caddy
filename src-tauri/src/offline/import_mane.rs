// ./src-tauri/src/offline/import_mane.rs
/*
Purpose: Import MANE Select summary TSV into the mane catalog DB.
*/

use super::compress::open_text_auto;
use rusqlite::{Connection, params};
use std::io::BufRead;
use std::path::Path;

pub fn import_mane_summary(conn: &Connection, path: &Path) -> Result<u64, String> {
    let reader = open_text_auto(path)?;
    let mut lines = reader.lines();

    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "MANE summary empty".to_string())?;
    let headers: Vec<&str> = header.split('\t').collect();

    let gene_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("symbol") || h.contains("gene"))
        .unwrap_or(0);
    let ensembl_idx = headers.iter().position(|h| h.contains("Ensembl"));
    let refseq_idx = headers.iter().position(|h| h.contains("RefSeq"));
    let status_idx = headers.iter().position(|h| h.contains("MANE"));
    let coord_idx = headers
        .iter()
        .position(|h| h.contains("GRCh38") || h.contains("coordinates"));

    let table = if crate::offline::schema::schema_attached(conn, "mane") {
        "mane.mane_transcripts"
    } else {
        "reference.mane_transcripts"
    };
    conn.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let gene = parts
            .get(gene_idx)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let Some(gene) = gene else { continue };

        let ensembl = ensembl_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let refseq = refseq_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let status = status_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let coords = coord_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());

        tx.execute(
            &format!(
                "INSERT OR REPLACE INTO {table}
                 (gene_symbol, ensembl_transcript, refseq_transcript, mane_status, grch38_coordinates)
                 VALUES (?, ?, ?, ?, ?)"
            ),
            params![gene, ensembl, refseq, status, coords],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
