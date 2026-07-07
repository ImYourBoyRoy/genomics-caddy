// ./src-tauri/src/offline/import_dbsnp.rs
use crate::research::util::normalize_rsid;
use bzip2::read::BzDecoder;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn normalize_numeric_rsid(val: &str) -> Option<String> {
    let clean = val.trim().trim_start_matches("rs").trim_start_matches("RS");
    clean.parse::<u64>().ok().map(|n| format!("rs{}", n))
}

pub fn import_dbsnp_merged(conn: &Connection, path: &Path) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("Open dbSNP file: {e}"))?;
    let is_bz2 = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("bz2"));

    let reader: Box<dyn BufRead> = if is_bz2 {
        Box::new(BufReader::new(BzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };

    let mut tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse line as JSON Value
        let obj: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed or truncated lines
        };

        let refsnp_id_raw = obj.get("refsnp_id")
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_u64().map(|n| n.to_string())));
        
        let current_rsid = refsnp_id_raw.as_deref().and_then(normalize_numeric_rsid);

        if let Some(ref current) = current_rsid {
            // 1. Process dbsnp1_merges
            if let Some(merges) = obj.get("dbsnp1_merges").and_then(|m| m.as_array()) {
                for m in merges {
                    let old_rsid_raw = m.get("merged_rsid")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_u64().map(|n| n.to_string())));
                    if let Some(old_rsid) = old_rsid_raw.as_deref().and_then(normalize_numeric_rsid) {
                        tx.execute(
                            "INSERT OR REPLACE INTO reference.rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                             VALUES (?, ?, 0, 'dbsnp_merged', ?)",
                            params![old_rsid, current, current],
                        ).ok();
                        count += 1;
                    }
                }
            }

            // 2. Process merged_snapshot_data.merged_into
            if let Some(snap) = obj.get("merged_snapshot_data") {
                if let Some(merged_into_arr) = snap.get("merged_into").and_then(|a| a.as_array()) {
                    for to_val in merged_into_arr {
                        let to_rsid_raw = to_val.as_str().map(|s| s.to_string()).or_else(|| to_val.as_u64().map(|n| n.to_string()));
                        if let Some(to_rsid) = to_rsid_raw.as_deref().and_then(normalize_numeric_rsid) {
                            tx.execute(
                                "INSERT OR REPLACE INTO reference.rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                                 VALUES (?, ?, 0, 'dbsnp_merged', ?)",
                                params![current, to_rsid, to_rsid],
                            ).ok();
                            count += 1;
                        }
                    }
                }
            }
        }

        if count > 0 && count.is_multiple_of(50_000) {
            tx.commit().map_err(|e| e.to_string())?;
            tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

pub fn import_dbsnp_withdrawn(conn: &Connection, path: &Path) -> Result<u64, String> {
    // For withdrawn, we can do a simplified streaming or compatibility parsing.
    // If it's a small file we can parse it as a JSON Array.
    let file = File::open(path).map_err(|e| format!("Open dbSNP withdrawn: {e}"))?;
    let is_bz2 = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("bz2"));
    
    let reader: Box<dyn BufRead> = if is_bz2 {
        Box::new(BufReader::new(BzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try parsing line as array or single value
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            let ids = match &val {
                Value::Array(arr) => arr.iter().filter_map(|v| {
                    v.as_str().map(String::from).or_else(|| v.as_u64().map(|n| n.to_string()))
                }).collect(),
                Value::Object(map) => map.keys().cloned().collect(),
                Value::String(s) => vec![s.clone()],
                Value::Number(n) => vec![n.to_string()],
                _ => Vec::new(),
            };

            for id in ids {
                if let Some(rsid) = normalize_rsid(&id) {
                    tx.execute(
                        "INSERT OR REPLACE INTO reference.rsid_aliases (rsid, merged_into, withdrawn, source)
                         VALUES (?, NULL, 1, 'dbsnp_withdrawn')",
                        params![rsid],
                    ).ok();
                    count += 1;
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
