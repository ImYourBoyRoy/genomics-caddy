// ./src-tauri/src/offline/import_clingen.rs
/*
Purpose: Import ClinGen gene–disease validity CSV into the clingen catalog DB.
Responsibilities:
- Skip ClinGen preamble rows until the real header line.
- Parse quoted CSV with the `csv` crate (fields contain commas).
- Upsert into clingen.clingen_gene_validity (or reference.* during migration).
Key Inputs: gene-validity.csv from ClinGen download.
Key Outputs: Row count inserted.
*/

use csv::ReaderBuilder;
use rusqlite::{Connection, params};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn normalize_header(h: &str) -> String {
    h.trim()
        .trim_matches('"')
        .to_uppercase()
        .replace(['_', '-'], " ")
}

fn find_col(headers: &[String], aliases: &[&str]) -> Option<usize> {
    let wanted: Vec<String> = aliases.iter().map(|a| normalize_header(a)).collect();
    headers.iter().position(|h| wanted.iter().any(|w| h == w))
}

/// ClinGen files start with title/metadata rows; the real header contains GENE SYMBOL.
fn skip_to_header_line(path: &Path) -> Result<(usize, String), String> {
    let file = File::open(path).map_err(|e| format!("Open ClinGen CSV: {e}"))?;
    let reader = BufReader::new(file);
    for (idx, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| e.to_string())?;
        let upper = line.to_uppercase();
        if upper.contains("GENE SYMBOL") && upper.contains("DISEASE") {
            return Ok((idx, line));
        }
    }
    Err("ClinGen CSV: could not find header row containing GENE SYMBOL".into())
}

pub fn import_clingen_gene_validity(conn: &Connection, path: &Path) -> Result<u64, String> {
    let (_header_line_idx, header_line) = skip_to_header_line(path)?;

    // Re-open and skip until after the header line we detected.
    let file = File::open(path).map_err(|e| format!("Open ClinGen CSV: {e}"))?;
    let mut buf = BufReader::new(file);
    loop {
        let mut line = String::new();
        let n = buf.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("ClinGen CSV ended before header".into());
        }
        if line.trim() == header_line.trim() {
            break;
        }
    }

    let mut rdr = ReaderBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_reader(buf);

    let headers: Vec<String> = header_line
        .split(',')
        .map(|s| normalize_header(s.trim_matches('"')))
        .collect();

    let gene_idx = find_col(&headers, &["GENE SYMBOL", "GENE", "SYMBOL"])
        .ok_or_else(|| format!("ClinGen CSV missing GENE SYMBOL column; headers={headers:?}"))?;
    let disease_idx = find_col(&headers, &["DISEASE LABEL", "DISEASE"])
        .ok_or_else(|| format!("ClinGen CSV missing DISEASE LABEL column; headers={headers:?}"))?;
    let class_idx = find_col(&headers, &["CLASSIFICATION"]);
    let moi_idx = find_col(&headers, &["MOI", "MODE OF INHERITANCE"]);
    let url_idx = find_col(&headers, &["ONLINE REPORT", "REPORT URL", "REPORTURL"]);
    let hgnc_idx = find_col(&headers, &["GENE ID (HGNC)", "HGNC ID", "HGNC"]);

    // Prefer dedicated clingen schema; fall back to reference during migration.
    let table = if crate::offline::schema::schema_attached(conn, "clingen") {
        "clingen.clingen_gene_validity"
    } else {
        "reference.clingen_gene_validity"
    };

    conn.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for result in rdr.records() {
        let record = result.map_err(|e| format!("ClinGen CSV parse: {e}"))?;
        let get = |idx: Option<usize>| -> Option<String> {
            idx.and_then(|i| record.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let gene = get(Some(gene_idx));
        let disease = get(Some(disease_idx));
        let (Some(gene), Some(disease)) = (gene, disease) else {
            continue;
        };
        // Skip decorative separator rows
        if gene.starts_with('+') || disease.starts_with('+') {
            continue;
        }

        tx.execute(
            &format!(
                "INSERT OR REPLACE INTO {table}
                 (hgnc_id, gene_symbol, disease_label, classification, moi, report_url)
                 VALUES (?, ?, ?, ?, ?, ?)"
            ),
            params![
                get(hgnc_idx),
                gene,
                disease,
                get(class_idx),
                get(moi_idx),
                get(url_idx)
            ],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
