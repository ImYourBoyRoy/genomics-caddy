// ./src-tauri/src/offline/import_dbsnp.rs
use super::compress::open_text_auto;
use crate::research::util::normalize_rsid;
use rayon::prelude::*;
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde_json::Value;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn normalize_numeric_rsid(val: &str) -> Option<String> {
    let clean = val.trim().trim_start_matches("rs").trim_start_matches("RS");
    clean.parse::<u64>().ok().map(|n| format!("rs{}", n))
}

use tauri::Emitter;

fn deserialize_rsid<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Helper {
        String(String),
        Number(u64),
    }

    match Helper::deserialize(deserializer)? {
        Helper::String(s) => Ok(s),
        Helper::Number(n) => Ok(n.to_string()),
    }
}

#[derive(Deserialize)]
struct MergeRecord {
    #[serde(deserialize_with = "deserialize_rsid")]
    merged_rsid: String,
}

#[derive(Deserialize)]
struct SnapshotData {
    #[serde(default)]
    merged_into: Vec<String>,
}

#[derive(Deserialize)]
struct DbSnpRecord {
    #[serde(deserialize_with = "deserialize_rsid")]
    refsnp_id: String,
    dbsnp1_merges: Option<Vec<MergeRecord>>,
    merged_snapshot_data: Option<SnapshotData>,
}

struct AliasInsert {
    rsid: String,
    merged_into: Option<String>,
    withdrawn: i32,
    source: String,
    raw_refsnp_id: Option<String>,
}

pub fn import_dbsnp_merged(
    conn: &Connection,
    path: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let reader: Box<dyn BufRead> =
        open_text_auto(path).map_err(|error| format!("Open dbSNP file: {error}"))?;

    // Locate active dbsnp.db file path
    let dbsnp_path_str: String = conn
        .query_row(
            "SELECT file FROM pragma_database_list() WHERE name = 'dbsnp'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Query dbsnp.db path: {e}"))?;

    let dbsnp_path = PathBuf::from(&dbsnp_path_str);
    let dbsnp_dir = dbsnp_path.parent().unwrap_or_else(|| Path::new("."));
    let staging_path = dbsnp_dir.join("dbsnp_staging.db");

    if staging_path.is_file() {
        let _ = std::fs::remove_file(&staging_path);
    }

    // Open staging database connection
    let staging_conn =
        Connection::open(&staging_path).map_err(|e| format!("Open staging DB: {e}"))?;

    // Apply ingestion write performance PRAGMAs
    staging_conn.execute("PRAGMA synchronous = OFF", []).ok();
    staging_conn
        .execute("PRAGMA journal_mode = MEMORY", [])
        .ok();
    staging_conn.execute("PRAGMA cache_size = 100000", []).ok();

    staging_conn
        .execute(
            "CREATE TABLE IF NOT EXISTS rsid_aliases (
            rsid TEXT PRIMARY KEY,
            merged_into TEXT,
            withdrawn INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'dbsnp',
            raw_refsnp_id TEXT
        )",
            [],
        )
        .map_err(|e| format!("Create staging schema: {e}"))?;

    let start_time = std::time::Instant::now();
    let total_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut bytes_processed = 0u64;

    let parse_dbsnp_line = |line: &str| -> Vec<AliasInsert> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let record: DbSnpRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let current = match normalize_numeric_rsid(&record.refsnp_id) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut inserts = Vec::new();

        if let Some(merges) = record.dbsnp1_merges {
            for m in merges {
                if let Some(old_rsid) = normalize_numeric_rsid(&m.merged_rsid) {
                    inserts.push(AliasInsert {
                        rsid: old_rsid,
                        merged_into: Some(current.clone()),
                        withdrawn: 0,
                        source: "dbsnp_merged".to_string(),
                        raw_refsnp_id: Some(current.clone()),
                    });
                }
            }
        }

        if let Some(snap) = record.merged_snapshot_data {
            for to_val in snap.merged_into {
                if let Some(to_rsid) = normalize_numeric_rsid(&to_val) {
                    inserts.push(AliasInsert {
                        rsid: current.clone(),
                        merged_into: Some(to_rsid.clone()),
                        withdrawn: 0,
                        source: "dbsnp_merged".to_string(),
                        raw_refsnp_id: Some(to_rsid),
                    });
                }
            }
        }

        inserts
    };

    let mut tx = staging_conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;
    let mut chunk = Vec::with_capacity(20_000);

    for line_res in reader.lines() {
        let line = line_res.map_err(|e| e.to_string())?;
        let line_len = line.len() as u64 + 1;
        bytes_processed += line_len;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        chunk.push(line);

        if chunk.len() >= 20_000 {
            let parsed_inserts: Vec<AliasInsert> =
                chunk.par_iter().flat_map(|l| parse_dbsnp_line(l)).collect();

            for row in parsed_inserts {
                tx.execute(
                    "INSERT OR REPLACE INTO rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                     VALUES (?, ?, ?, ?, ?)",
                    params![row.rsid, row.merged_into, row.withdrawn, row.source, row.raw_refsnp_id],
                ).ok();
                count += 1;
            }

            tx.commit().map_err(|e| e.to_string())?;
            if let Some(handle) = app {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed = if elapsed > 0.0 {
                    count as f64 / elapsed
                } else {
                    0.0
                };
                let percent = if total_bytes > 0 {
                    ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u64
                } else {
                    0
                };
                let percent_bounded = percent.min(99);
                let eta_seconds = if speed > 0.0 && total_bytes > bytes_processed {
                    let remaining_bytes = total_bytes - bytes_processed;
                    let avg_bytes_per_row = bytes_processed as f64 / count as f64;
                    let remaining_rows = remaining_bytes as f64 / avg_bytes_per_row;
                    Some((remaining_rows / speed) as u64)
                } else {
                    None
                };

                let _ = handle.emit(
                    "offline:import_progress",
                    serde_json::json!({
                        "asset_id": "dbsnp_merged_json",
                        "rows_processed": count,
                        "percent": percent_bounded,
                        "rows_per_second": speed as u64,
                        "eta_seconds": eta_seconds,
                        "message": format!(
                            "Importing dbSNP merges: {}% complete • {} rows ({:.0} rows/s)...",
                            percent_bounded, count, speed
                        )
                    }),
                );
            }
            tx = staging_conn
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            chunk.clear();
        }
    }

    // Remainder
    if !chunk.is_empty() {
        let parsed_inserts: Vec<AliasInsert> =
            chunk.par_iter().flat_map(|l| parse_dbsnp_line(l)).collect();

        for row in parsed_inserts {
            tx.execute(
                "INSERT OR REPLACE INTO rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                 VALUES (?, ?, ?, ?, ?)",
                params![row.rsid, row.merged_into, row.withdrawn, row.source, row.raw_refsnp_id],
            ).ok();
            count += 1;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    // Drop staging connection to release file lock before swap
    drop(staging_conn);

    // Detach current dbsnp database on the main connection to release lock
    let _ = conn.execute("DETACH DATABASE dbsnp", []);

    // Swap files atomically
    if dbsnp_path.is_file() {
        let _ = std::fs::remove_file(&dbsnp_path);
    }
    std::fs::rename(&staging_path, &dbsnp_path)
        .map_err(|e| format!("Staging swap rename failed: {e}"))?;

    // Re-attach new dbsnp database to connection
    let attach_dbsnp = format!(
        "ATTACH DATABASE '{}' AS dbsnp",
        dbsnp_path.to_string_lossy().replace('\\', "/")
    );
    conn.execute(&attach_dbsnp, [])
        .map_err(|e| format!("Re-attach dbsnp failed: {e}"))?;

    Ok(count)
}

pub fn import_dbsnp_withdrawn(
    conn: &Connection,
    path: &Path,
    _app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let reader: Box<dyn BufRead> =
        open_text_auto(path).map_err(|error| format!("Open dbSNP withdrawn: {error}"))?;

    // Apply ingestion write performance PRAGMAs
    conn.execute("PRAGMA synchronous = OFF", []).ok();
    conn.execute("PRAGMA journal_mode = MEMORY", []).ok();
    conn.execute("PRAGMA cache_size = 100000", []).ok();

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
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| {
                        v.as_str()
                            .map(String::from)
                            .or_else(|| v.as_u64().map(|n| n.to_string()))
                    })
                    .collect(),
                Value::Object(map) => map.keys().cloned().collect(),
                Value::String(s) => vec![s.clone()],
                Value::Number(n) => vec![n.to_string()],
                _ => Vec::new(),
            };

            for id in ids {
                if let Some(rsid) = normalize_rsid(&id) {
                    tx.execute(
                        "INSERT OR REPLACE INTO dbsnp.rsid_aliases (rsid, merged_into, withdrawn, source)
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
