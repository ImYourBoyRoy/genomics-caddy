// ./src-tauri/src/offline/import_dbsnp.rs
/*
Purpose: Import NCBI dbSNP merged/withdrawn RefSNP JSON into dbsnp.db.
Responsibilities:
- Build rsid_aliases (old → current merge map + withdrawn flags).
- Capture per-RefSNP metadata (dates, build, citations, MANE ids) for future use.
- Stream large NDJSON/bz2 sources with progress events.
Key Inputs: refsnp-merged.json(.bz2), refsnp-withdrawn.json(.bz2); attached dbsnp schema.
Key Outputs: dbsnp.rsid_aliases + dbsnp.refsnp_meta rows; offline:import_progress events.
Operational Notes:
- Merged JSON is a merge-history catalog, not allele/AF data — DB size << raw JSON is expected.
- Prefer compressed .bz2 when present to avoid multi-GB uncompressed scans.
*/

use super::compress::open_text_auto;
use crate::research::util::normalize_rsid;
use rayon::prelude::*;
use rusqlite::{Connection, params};
use serde::Deserialize;
use serde_json::Value;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use tauri::Emitter;

fn normalize_numeric_rsid(val: &str) -> Option<String> {
    let clean = val.trim().trim_start_matches("rs").trim_start_matches("RS");
    clean.parse::<u64>().ok().map(|n| format!("rs{}", n))
}

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
    #[serde(default)]
    create_date: Option<String>,
    #[serde(default)]
    last_update_date: Option<String>,
    #[serde(default)]
    last_update_build_id: Option<String>,
    #[serde(default)]
    dbsnp1_merges: Option<Vec<MergeRecord>>,
    #[serde(default)]
    merged_snapshot_data: Option<SnapshotData>,
    #[serde(default)]
    citations: Option<Vec<Value>>,
    #[serde(default)]
    mane_select_ids: Option<Vec<Value>>,
}

struct AliasInsert {
    rsid: String,
    merged_into: Option<String>,
    withdrawn: i32,
    source: String,
    raw_refsnp_id: Option<String>,
}

struct MetaInsert {
    refsnp_id: String,
    create_date: Option<String>,
    last_update_date: Option<String>,
    last_update_build_id: Option<String>,
    citation_count: i64,
    citations_json: String,
    mane_select_ids_json: String,
}

fn create_dbsnp_staging_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS rsid_aliases (
            rsid TEXT PRIMARY KEY,
            merged_into TEXT,
            withdrawn INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'dbsnp',
            raw_refsnp_id TEXT
        );
        CREATE TABLE IF NOT EXISTS refsnp_meta (
            refsnp_id TEXT PRIMARY KEY,
            create_date TEXT,
            last_update_date TEXT,
            last_update_build_id TEXT,
            citation_count INTEGER NOT NULL DEFAULT 0,
            citations_json TEXT NOT NULL DEFAULT '[]',
            mane_select_ids_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_rsid_aliases_merged_into ON rsid_aliases(merged_into);
        CREATE INDEX IF NOT EXISTS idx_rsid_aliases_withdrawn ON rsid_aliases(withdrawn);
        CREATE INDEX IF NOT EXISTS idx_rsid_aliases_source ON rsid_aliases(source);
        ",
    )
    .map_err(|e| format!("Create staging schema: {e}"))?;
    Ok(())
}

fn emit_import_progress(
    app: Option<&tauri::AppHandle>,
    asset_id: &str,
    count: u64,
    percent: u64,
    speed: f64,
    eta_seconds: Option<u64>,
    message: String,
) {
    let Some(handle) = app else { return };
    let _ = handle.emit(
        "offline:import_progress",
        serde_json::json!({
            "asset_id": asset_id,
            "rows_processed": count,
            "percent": percent,
            "rows_per_second": speed as u64,
            "eta_seconds": eta_seconds,
            "message": message
        }),
    );
}

fn progress_percent(bytes_processed: u64, total_bytes: u64) -> u64 {
    if total_bytes == 0 {
        return 0;
    }
    ((bytes_processed as f64 / total_bytes as f64) * 100.0)
        .floor()
        .clamp(0.0, 99.0) as u64
}

/// Prefer compressed source when both uncompressed JSON and .bz2 exist.
pub fn prefer_compressed_dbsnp_path(path: &Path) -> PathBuf {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("bz2") || e.eq_ignore_ascii_case("gz"))
    {
        return path.to_path_buf();
    }
    let bz2 = PathBuf::from(format!("{}.bz2", path.display()));
    if bz2.is_file() {
        return bz2;
    }
    path.to_path_buf()
}

pub fn import_dbsnp_merged(
    conn: &Connection,
    path: &Path,
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let path = prefer_compressed_dbsnp_path(path);
    let reader: Box<dyn BufRead> =
        open_text_auto(&path).map_err(|error| format!("Open dbSNP file: {error}"))?;

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

    let staging_conn =
        Connection::open(&staging_path).map_err(|e| format!("Open staging DB: {e}"))?;

    staging_conn.execute("PRAGMA synchronous = OFF", []).ok();
    staging_conn
        .execute("PRAGMA journal_mode = MEMORY", [])
        .ok();
    staging_conn.execute("PRAGMA cache_size = 100000", []).ok();
    create_dbsnp_staging_schema(&staging_conn)?;

    let start_time = std::time::Instant::now();
    let total_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut bytes_processed = 0u64;

    let parse_dbsnp_line = |line: &str| -> (Vec<AliasInsert>, Option<MetaInsert>) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return (Vec::new(), None);
        }
        let record: DbSnpRecord = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(_) => return (Vec::new(), None),
        };
        let current = match normalize_numeric_rsid(&record.refsnp_id) {
            Some(c) => c,
            None => return (Vec::new(), None),
        };

        let mut inserts = Vec::new();

        if let Some(merges) = &record.dbsnp1_merges {
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

        if let Some(snap) = &record.merged_snapshot_data {
            for to_val in &snap.merged_into {
                if let Some(to_rsid) = normalize_numeric_rsid(to_val) {
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

        let citations = record.citations.clone().unwrap_or_default();
        let mane = record.mane_select_ids.clone().unwrap_or_default();
        let meta = MetaInsert {
            refsnp_id: current,
            create_date: record.create_date,
            last_update_date: record.last_update_date,
            last_update_build_id: record.last_update_build_id,
            citation_count: citations.len() as i64,
            citations_json: serde_json::to_string(&citations).unwrap_or_else(|_| "[]".into()),
            mane_select_ids_json: serde_json::to_string(&mane).unwrap_or_else(|_| "[]".into()),
        };

        (inserts, Some(meta))
    };

    let mut tx = staging_conn
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut count = 0u64;
    let mut meta_count = 0u64;
    let mut chunk = Vec::with_capacity(20_000);

    for line_res in reader.lines() {
        if crate::offline::sync::is_offline_import_cancelled() {
            return Err("dbSNP merge import cancelled by user.".into());
        }
        let line = line_res.map_err(|e| e.to_string())?;
        let line_len = line.len() as u64 + 1;
        bytes_processed += line_len;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        chunk.push(line);

        if chunk.len() >= 20_000 {
            let parsed: Vec<(Vec<AliasInsert>, Option<MetaInsert>)> =
                chunk.par_iter().map(|l| parse_dbsnp_line(l)).collect();

            for (aliases, meta) in parsed {
                for row in aliases {
                    tx.execute(
                        "INSERT OR REPLACE INTO rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                         VALUES (?, ?, ?, ?, ?)",
                        params![
                            row.rsid,
                            row.merged_into,
                            row.withdrawn,
                            row.source,
                            row.raw_refsnp_id
                        ],
                    )
                    .ok();
                    count += 1;
                }
                if let Some(m) = meta {
                    tx.execute(
                        "INSERT OR REPLACE INTO refsnp_meta (
                            refsnp_id, create_date, last_update_date, last_update_build_id,
                            citation_count, citations_json, mane_select_ids_json
                         ) VALUES (?, ?, ?, ?, ?, ?, ?)",
                        params![
                            m.refsnp_id,
                            m.create_date,
                            m.last_update_date,
                            m.last_update_build_id,
                            m.citation_count,
                            m.citations_json,
                            m.mane_select_ids_json
                        ],
                    )
                    .ok();
                    meta_count += 1;
                }
            }

            tx.commit().map_err(|e| e.to_string())?;
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                count as f64 / elapsed
            } else {
                0.0
            };
            let percent = progress_percent(bytes_processed, total_bytes);
            let eta_seconds = if speed > 0.0 && total_bytes > bytes_processed {
                let remaining_bytes = total_bytes - bytes_processed;
                let avg_bytes_per_row = bytes_processed as f64 / count.max(1) as f64;
                let remaining_rows = remaining_bytes as f64 / avg_bytes_per_row;
                Some((remaining_rows / speed) as u64)
            } else {
                None
            };
            emit_import_progress(
                app,
                "dbsnp_merged_json",
                count,
                percent,
                speed,
                eta_seconds,
                format!(
                    "Importing dbSNP merge map: {percent}% · {count} aliases · {meta_count} RefSNP meta ({:.0} rows/s)",
                    speed
                ),
            );
            tx = staging_conn
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            chunk.clear();
        }
    }

    if !chunk.is_empty() {
        let parsed: Vec<(Vec<AliasInsert>, Option<MetaInsert>)> =
            chunk.par_iter().map(|l| parse_dbsnp_line(l)).collect();

        for (aliases, meta) in parsed {
            for row in aliases {
                tx.execute(
                    "INSERT OR REPLACE INTO rsid_aliases (rsid, merged_into, withdrawn, source, raw_refsnp_id)
                     VALUES (?, ?, ?, ?, ?)",
                    params![
                        row.rsid,
                        row.merged_into,
                        row.withdrawn,
                        row.source,
                        row.raw_refsnp_id
                    ],
                )
                .ok();
                count += 1;
            }
            if let Some(m) = meta {
                tx.execute(
                    "INSERT OR REPLACE INTO refsnp_meta (
                        refsnp_id, create_date, last_update_date, last_update_build_id,
                        citation_count, citations_json, mane_select_ids_json
                     ) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![
                        m.refsnp_id,
                        m.create_date,
                        m.last_update_date,
                        m.last_update_build_id,
                        m.citation_count,
                        m.citations_json,
                        m.mane_select_ids_json
                    ],
                )
                .ok();
                meta_count += 1;
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    emit_import_progress(
        app,
        "dbsnp_merged_json",
        count,
        100,
        0.0,
        None,
        format!(
            "dbSNP merge import complete · {count} aliases · {meta_count} RefSNP meta records"
        ),
    );

    drop(staging_conn);
    let _ = conn.execute("DETACH DATABASE dbsnp", []);

    if dbsnp_path.is_file() {
        let _ = std::fs::remove_file(&dbsnp_path);
    }
    std::fs::rename(&staging_path, &dbsnp_path)
        .map_err(|e| format!("Staging swap rename failed: {e}"))?;

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
    app: Option<&tauri::AppHandle>,
) -> Result<u64, String> {
    let path = prefer_compressed_dbsnp_path(path);
    let reader: Box<dyn BufRead> =
        open_text_auto(&path).map_err(|error| format!("Open dbSNP withdrawn: {error}"))?;

    // Ensure schema exists on the live attached DB (merged import may have created it).
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS dbsnp.rsid_aliases (
            rsid TEXT PRIMARY KEY,
            merged_into TEXT,
            withdrawn INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'dbsnp',
            raw_refsnp_id TEXT
        );
        CREATE TABLE IF NOT EXISTS dbsnp.refsnp_meta (
            refsnp_id TEXT PRIMARY KEY,
            create_date TEXT,
            last_update_date TEXT,
            last_update_build_id TEXT,
            citation_count INTEGER NOT NULL DEFAULT 0,
            citations_json TEXT NOT NULL DEFAULT '[]',
            mane_select_ids_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS dbsnp.idx_rsid_aliases_merged_into ON rsid_aliases(merged_into);
        CREATE INDEX IF NOT EXISTS dbsnp.idx_rsid_aliases_withdrawn ON rsid_aliases(withdrawn);
        ",
    )
    .map_err(|e| e.to_string())?;

    conn.execute("PRAGMA synchronous = OFF", []).ok();
    conn.execute("PRAGMA journal_mode = MEMORY", []).ok();
    conn.execute("PRAGMA cache_size = 100000", []).ok();

    let start_time = std::time::Instant::now();
    let total_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut bytes_processed = 0u64;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;
    let mut last_emit = std::time::Instant::now();

    for line_res in reader.lines() {
        if crate::offline::sync::is_offline_import_cancelled() {
            return Err("dbSNP withdrawn import cancelled by user.".into());
        }
        let line = line_res.map_err(|e| e.to_string())?;
        bytes_processed += line.len() as u64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };

        // NCBI withdrawn NDJSON: one object per line with refsnp_id (not a key map).
        let ids: Vec<String> = match &val {
            Value::Object(map) => map
                .get("refsnp_id")
                .and_then(|v| {
                    v.as_str()
                        .map(String::from)
                        .or_else(|| v.as_u64().map(|n| n.to_string()))
                })
                .into_iter()
                .collect(),
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(String::from)
                        .or_else(|| v.as_u64().map(|n| n.to_string()))
                        .or_else(|| {
                            v.get("refsnp_id").and_then(|x| {
                                x.as_str()
                                    .map(String::from)
                                    .or_else(|| x.as_u64().map(|n| n.to_string()))
                            })
                        })
                })
                .collect(),
            Value::String(s) => vec![s.clone()],
            Value::Number(n) => vec![n.to_string()],
            _ => Vec::new(),
        };

        for id in ids {
            if let Some(rsid) = normalize_rsid(&id).or_else(|| normalize_numeric_rsid(&id)) {
                tx.execute(
                    "INSERT INTO dbsnp.rsid_aliases (rsid, merged_into, withdrawn, source)
                     VALUES (?, NULL, 1, 'dbsnp_withdrawn')
                     ON CONFLICT(rsid) DO UPDATE SET
                        withdrawn = 1,
                        source = 'dbsnp_withdrawn'",
                    params![rsid],
                )
                .ok();
                count += 1;
            }
        }

        if last_emit.elapsed().as_millis() >= 400 {
            last_emit = std::time::Instant::now();
            let elapsed = start_time.elapsed().as_secs_f64().max(0.1);
            let speed = count as f64 / elapsed;
            let percent = progress_percent(bytes_processed, total_bytes);
            emit_import_progress(
                app,
                "dbsnp_merged_json",
                count,
                percent,
                speed,
                None,
                format!("Importing dbSNP withdrawn: {percent}% · {count} rows ({speed:.0}/s)"),
            );
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    emit_import_progress(
        app,
        "dbsnp_merged_json",
        count,
        100,
        0.0,
        None,
        format!("dbSNP withdrawn import complete · {count} rows marked withdrawn"),
    );
    Ok(count)
}
