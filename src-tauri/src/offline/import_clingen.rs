// ./src-tauri/src/offline/import_clingen.rs
use super::compress::open_text_auto;
use rusqlite::{Connection, params};
use std::io::BufRead;
use std::path::Path;

fn col(headers: &[&str], names: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let u = h.trim().to_lowercase();
        names.iter().any(|n| u == n.to_lowercase())
    })
}

pub fn import_clingen_gene_validity(conn: &Connection, path: &Path) -> Result<u64, String> {
    let reader = open_text_auto(path)?;
    let mut lines = reader.lines();
    let header = lines
        .next()
        .transpose()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "ClinGen CSV empty".to_string())?;
    let headers: Vec<&str> = header.split(',').map(|s| s.trim_matches('"')).collect();

    let gene_idx = col(&headers, &["Gene Symbol", "gene_symbol", "Gene"]);
    let disease_idx = col(&headers, &["Disease Label", "disease_label", "Disease"]);
    let class_idx = col(
        &headers,
        &[
            "Classification",
            "classification",
            "Gene-Disease Validity Classification",
        ],
    );
    let moi_idx = col(&headers, &["Mode of Inheritance", "moi", "MOI"]);
    let url_idx = col(&headers, &["Report URL", "report_url", "ReportURL"]);
    let hgnc_idx = col(&headers, &["HGNC ID", "hgnc_id", "HGNC"]);

    conn.execute("DELETE FROM reference.clingen_gene_validity", [])
        .map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line in lines {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        let gene = gene_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let disease = disease_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let (Some(gene), Some(disease)) = (gene, disease) else {
            continue;
        };
        let classification = class_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let moi = moi_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let report_url = url_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());
        let hgnc = hgnc_idx
            .and_then(|i| parts.get(i))
            .map(|s| s.trim().to_string());

        tx.execute(
            "INSERT OR REPLACE INTO reference.clingen_gene_validity
             (hgnc_id, gene_symbol, disease_label, classification, moi, report_url)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![hgnc, gene, disease, classification, moi, report_url],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
