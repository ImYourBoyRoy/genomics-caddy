// ./src-tauri/src/offline/discovery_export.rs
/*
Purpose: Export marker-pack coverage vs genome-wide catalog hits for pack authoring.
Responsibilities:
- Collect hardcoded pack rsIDs/genes from marker packs on disk.
- Intersect a sample's genotypes with ClinVar / GWAS / PharmGKB via temp-table joins.
- Write two JSON files: pack coverage + full associated findings beyond packs.
- Power in-app discovery browser with cancel, progress events, and short-lived cache.
Key Inputs: sample_id, App/Data marker packs, attached catalog DBs, sample genome.db.
Key Outputs: JSON paths under App/Data/exports/; discovery:query_progress events.
*/

use crate::db;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

static DISCOVERY_CANCEL: AtomicBool = AtomicBool::new(false);

struct DiscoveryJoinCache {
    sample_id: i64,
    findings: Vec<Value>,
    pack_rsids: BTreeSet<String>,
    geno_count: i64,
    created: Instant,
}

fn discovery_cache() -> &'static Mutex<Option<DiscoveryJoinCache>> {
    static CACHE: OnceLock<Mutex<Option<DiscoveryJoinCache>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn reset_discovery_cancel() {
    DISCOVERY_CANCEL.store(false, Ordering::SeqCst);
}

pub fn cancel_discovery_query() {
    DISCOVERY_CANCEL.store(true, Ordering::SeqCst);
}

pub fn is_discovery_cancelled() -> bool {
    DISCOVERY_CANCEL.load(Ordering::SeqCst)
}

pub fn invalidate_discovery_cache() {
    if let Ok(mut guard) = discovery_cache().lock() {
        *guard = None;
    }
}

fn emit_discovery_progress(app: Option<&AppHandle>, percent: u8, message: &str) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "discovery:query_progress",
            json!({
                "percent": percent,
                "message": message,
                "cancelled": is_discovery_cancelled(),
            }),
        );
    }
}

fn check_discovery_cancel() -> Result<(), String> {
    if is_discovery_cancelled() {
        Err("Discovery query cancelled.".into())
    } else {
        Ok(())
    }
}

fn pack_rsids_and_genes(data_dir: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>, Value), String> {
    let manifest_str = db::get_manifest_str(Some(data_dir));
    let manifest: Value = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Parse marker pack manifest: {e}"))?;

    let mut rsids = BTreeSet::new();
    let mut genes = BTreeSet::new();
    let mut packs_out = serde_json::Map::new();

    if let Some(packs_array) = manifest.get("packs").and_then(|v| v.as_array()) {
        for pack_info in packs_array {
            let Some(pack_id) = pack_info.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(pack_str) = db::get_pack_str(Some(data_dir), pack_id) else {
                continue;
            };
            let Ok(pack_json) = serde_json::from_str::<Value>(&pack_str) else {
                continue;
            };

            let mut pack_rsids = BTreeSet::new();
            let mut pack_genes = BTreeSet::new();
            let mut collect_marker = |m: &Value| {
                if let Some(rsid) = m.get("rsid").and_then(|v| v.as_str()) {
                    let r = rsid.trim().to_lowercase();
                    if r.starts_with("rs") {
                        pack_rsids.insert(r.clone());
                        rsids.insert(r);
                    }
                }
                if let Some(gene) = m.get("gene").and_then(|v| v.as_str()) {
                    let g = gene.trim().to_uppercase();
                    if !g.is_empty() {
                        pack_genes.insert(g.clone());
                        genes.insert(g);
                    }
                }
            };
            if let Some(sections) = pack_json.get("sections").and_then(|v| v.as_array()) {
                for sec in sections {
                    if let Some(markers) = sec.get("markers").and_then(|v| v.as_array()) {
                        for m in markers {
                            collect_marker(m);
                        }
                    }
                }
            }
            if let Some(markers) = pack_json.get("markers").and_then(|v| v.as_array()) {
                for m in markers {
                    collect_marker(m);
                }
            }

            packs_out.insert(
                pack_id.to_string(),
                json!({
                    "title": pack_json.get("title").cloned().unwrap_or(json!(pack_id)),
                    "rsid_count": pack_rsids.len(),
                    "gene_count": pack_genes.len(),
                    "rsids": pack_rsids.into_iter().collect::<Vec<_>>(),
                    "genes": pack_genes.into_iter().collect::<Vec<_>>(),
                }),
            );
        }
    }

    Ok((
        rsids,
        genes,
        json!({
            "kind": "marker_pack_coverage",
            "generated_at": chrono_iso(),
            "packs": packs_out,
        }),
    ))
}

fn chrono_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn load_findings_via_join(
    conn: &Connection,
    sample_id: i64,
    app: Option<&AppHandle>,
) -> Result<Vec<Value>, String> {
    check_discovery_cancel()?;
    emit_discovery_progress(app, 5, "Loading genotype rsIDs…");

    // Temp table of this sample's rsIDs for set-based joins (avoids 700k×chunk IN lists).
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS temp.export_rsids;
        CREATE TEMP TABLE export_rsids (
            rsid TEXT PRIMARY KEY,
            allele1 TEXT,
            allele2 TEXT
        );
        ",
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR IGNORE INTO temp.export_rsids (rsid, allele1, allele2)
         SELECT LOWER(rsid), allele1, allele2 FROM genotypes
         WHERE sample_id = ? AND rsid IS NOT NULL AND TRIM(rsid) != ''",
        params![sample_id],
    )
    .map_err(|e| e.to_string())?;

    let geno_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM temp.export_rsids", [], |r| r.get(0))
        .unwrap_or(0);
    if geno_count == 0 {
        return Err(
            "Sample has no genotypes — import a genome file before exporting catalog findings."
                .into(),
        );
    }

    let mut by_rsid: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();

    let ensure = |map: &mut std::collections::BTreeMap<String, Value>,
                  rsid: &str,
                  a1: &str,
                  a2: &str| {
        map.entry(rsid.to_string()).or_insert_with(|| {
            json!({
                "rsid": rsid,
                "genotype": format!("{a1}/{a2}"),
                "clinvar": null,
                "clinvar_annotations": [],
                "gwas": null,
                "pharmgkb": null,
                "pharmgkb_annotations": [],
            })
        });
    };

    check_discovery_cancel()?;
    emit_discovery_progress(app, 25, "Joining ClinVar…");

    if crate::offline::schema::schema_attached(conn, "clinvar") {
        let mut stmt = conn
            .prepare(
                "SELECT e.rsid, e.allele1, e.allele2,
                        c.clinical_significance, COALESCE(c.gene_symbol, ''),
                        COALESCE(c.phenotype_list, c.conditions, ''),
                        COALESCE(c.review_status, ''), COALESCE(c.variation_id, '')
                 FROM temp.export_rsids e
                 JOIN clinvar.clinvar_reference c ON LOWER(c.rsid) = e.rsid
                 WHERE c.clinical_significance IS NOT NULL
                   AND TRIM(c.clinical_significance) != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            ensure(&mut by_rsid, &row.0, &row.1, &row.2);
            if let Some(entry) = by_rsid.get_mut(&row.0) {
                let annotation = json!({
                    "clinical_significance": row.3,
                    "gene": row.4,
                    "phenotypes": row.5,
                    "review_status": row.6,
                    "variation_id": row.7,
                    "source": "clinvar",
                });
                if entry.get("clinvar").is_some_and(Value::is_null) {
                    entry["clinvar"] = annotation.clone();
                }
                entry["clinvar_annotations"]
                    .as_array_mut()
                    .expect("discovery ClinVar annotations array")
                    .push(annotation);
            }
        }
    }

    check_discovery_cancel()?;
    emit_discovery_progress(app, 55, "Joining GWAS…");

    {
        let mut stmt = conn
            .prepare(
                "SELECT e.rsid, e.allele1, e.allele2,
                        g.top_trait, g.best_pvalue, g.association_count, g.primary_gene
                 FROM temp.export_rsids e
                 JOIN gwas_reference g ON LOWER(g.rsid) = e.rsid",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<f64>>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            ensure(&mut by_rsid, &row.0, &row.1, &row.2);
            if let Some(entry) = by_rsid.get_mut(&row.0) {
                entry["gwas"] = json!({
                    "top_trait": row.3,
                    "best_pvalue": row.4,
                    "association_count": row.5,
                    "primary_gene": row.6,
                    "source": "gwas",
                });
            }
        }
    }

    check_discovery_cancel()?;
    emit_discovery_progress(app, 80, "Joining PharmGKB…");

    {
        let mut stmt = conn
            .prepare(
                "SELECT e.rsid, e.allele1, e.allele2,
                        p.gene, p.drug, p.phenotype, p.evidence_level
                 FROM temp.export_rsids e
                 JOIN pharmgkb_clinical_variants p ON LOWER(p.rsid) = e.rsid",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            ensure(&mut by_rsid, &row.0, &row.1, &row.2);
            if let Some(entry) = by_rsid.get_mut(&row.0) {
                let annotation = json!({
                    "gene": row.3,
                    "drug": row.4,
                    "phenotype": row.5,
                    "evidence_level": row.6,
                    "source": "clinpgx",
                });
                if entry.get("pharmgkb").is_some_and(Value::is_null) {
                    entry["pharmgkb"] = annotation.clone();
                }
                entry["pharmgkb_annotations"]
                    .as_array_mut()
                    .expect("discovery ClinPGx annotations array")
                    .push(annotation);
            }
        }
    }

    check_discovery_cancel()?;
    emit_discovery_progress(app, 92, "Ranking associations…");
    Ok(by_rsid.into_values().collect())
}

/// Export pack coverage JSON + full genome×catalog findings JSON for pack authoring.
pub fn export_discovery_jsons(
    data_dir: &Path,
    db_path: &Path,
    sample_id: i64,
) -> Result<Value, String> {
    let exports_dir = data_dir.join("exports");
    fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;

    let (pack_rsids, pack_genes, mut pack_doc) = pack_rsids_and_genes(data_dir)?;
    if let Some(obj) = pack_doc.as_object_mut() {
        obj.insert("unique_rsid_count".into(), json!(pack_rsids.len()));
        obj.insert("unique_gene_count".into(), json!(pack_genes.len()));
        obj.insert(
            "all_pack_rsids".into(),
            json!(pack_rsids.iter().cloned().collect::<Vec<_>>()),
        );
        obj.insert(
            "all_pack_genes".into(),
            json!(pack_genes.iter().cloned().collect::<Vec<_>>()),
        );
        obj.insert("sample_id".into(), json!(sample_id));
    }

    let pack_path = exports_dir.join(format!("marker_pack_coverage_sample_{sample_id}.json"));
    let pack_json = serde_json::to_string_pretty(&pack_doc).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&pack_path, pack_json.as_bytes())?;

    // Sample DB already attaches public catalogs — run joins there.
    let sample_conn = db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| format!("Open sample DB: {e}"))?;

    let geno_count: i64 = sample_conn
        .query_row(
            "SELECT COUNT(*) FROM genotypes WHERE sample_id = ?",
            params![sample_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let findings = load_findings_via_join(&sample_conn, sample_id, None)?;

    let mut in_pack = Vec::new();
    let mut beyond_pack = Vec::new();
    for mut entry in findings {
        let rsid = entry
            .get("rsid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let in_packs = pack_rsids.contains(&rsid);
        entry
            .as_object_mut()
            .map(|o| o.insert("in_marker_packs".into(), json!(in_packs)));
        if in_packs {
            in_pack.push(entry);
        } else {
            beyond_pack.push(entry);
        }
    }

    beyond_pack.sort_by_key(|b| std::cmp::Reverse(finding_priority(b)));

    let full_doc = json!({
        "kind": "genome_catalog_findings",
        "generated_at": chrono_iso(),
        "sample_id": sample_id,
        "genotype_rsid_count": geno_count,
        "pack_rsid_count": pack_rsids.len(),
        "findings_in_packs": in_pack.len(),
        "findings_beyond_packs": beyond_pack.len(),
        "note": "beyond_packs lists genotype×catalog hits not covered by hardcoded marker packs — candidates for new pack entries.",
        "in_packs": in_pack,
        "beyond_packs": beyond_pack,
    });

    let full_path = exports_dir.join(format!("genome_catalog_findings_sample_{sample_id}.json"));
    let full_json = serde_json::to_string_pretty(&full_doc).map_err(|e| e.to_string())?;
    crate::file_utils::atomic_write(&full_path, full_json.as_bytes())?;

    Ok(json!({
        "pack_coverage_path": pack_path.to_string_lossy(),
        "full_findings_path": full_path.to_string_lossy(),
        "findings_in_packs": in_pack.len(),
        "findings_beyond_packs": beyond_pack.len(),
        "genotype_rsid_count": geno_count,
        "pack_rsid_count": pack_rsids.len(),
    }))
}

fn finding_priority(v: &Value) -> i32 {
    let mut s = 0;
    let clinvar_records = v
        .get("clinvar_annotations")
        .and_then(Value::as_array)
        .filter(|records| !records.is_empty());
    if let Some(records) = clinvar_records {
        for record in records {
            let sig = record
                .get("clinical_significance")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            if sig.contains("pathogenic") {
                s += 100;
            } else if sig.contains("risk") || sig.contains("association") {
                s += 40;
            } else if !sig.is_empty() {
                s += 10;
            }
        }
    } else if let Some(c) = v.get("clinvar").and_then(Value::as_object) {
        let sig = c
            .get("clinical_significance")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        if sig.contains("pathogenic") {
            s += 100;
        } else if sig.contains("risk") || sig.contains("association") {
            s += 40;
        } else if !sig.is_empty() {
            s += 10;
        }
    }
    if v.get("pharmgkb").and_then(|x| x.as_object()).is_some()
        || v
            .get("pharmgkb_annotations")
            .and_then(Value::as_array)
            .is_some_and(|records| !records.is_empty())
    {
        s += 30;
    }
    if v.get("gwas").and_then(|x| x.as_object()).is_some() {
        s += 5;
    }
    s
}

fn finding_matches_source(entry: &Value, source: &str) -> bool {
    match source {
        "clinvar" => entry.get("clinvar").and_then(|v| v.as_object()).is_some()
            || entry
                .get("clinvar_annotations")
                .and_then(Value::as_array)
                .is_some_and(|records| !records.is_empty()),
        "gwas" => entry.get("gwas").and_then(|v| v.as_object()).is_some(),
        "pharmgkb" => entry.get("pharmgkb").and_then(|v| v.as_object()).is_some()
            || entry
                .get("pharmgkb_annotations")
                .and_then(Value::as_array)
                .is_some_and(|records| !records.is_empty()),
        _ => true,
    }
}

fn finding_matches_query(entry: &Value, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let hay = serde_json::to_string(entry).unwrap_or_default().to_lowercase();
    hay.contains(q)
}

const DISCOVERY_CACHE_TTL: Duration = Duration::from_secs(600);

/// In-app discovery browser: ranked beyond-pack (or all) catalog hits with filter/pagination.
#[allow(clippy::too_many_arguments)]
pub fn query_discovery_findings(
    data_dir: &Path,
    db_path: &Path,
    sample_id: i64,
    beyond_packs_only: bool,
    source_filter: Option<&str>,
    query: Option<&str>,
    limit: usize,
    offset: usize,
    app: Option<&AppHandle>,
) -> Result<Value, String> {
    reset_discovery_cancel();
    emit_discovery_progress(app, 1, "Starting discovery query…");

    let cache_hit = {
        let guard = discovery_cache()
            .lock()
            .map_err(|_| "Discovery cache lock poisoned".to_string())?;
        guard.as_ref().and_then(|c| {
            if c.sample_id == sample_id && c.created.elapsed() < DISCOVERY_CACHE_TTL {
                Some((c.findings.clone(), c.pack_rsids.clone(), c.geno_count))
            } else {
                None
            }
        })
    };
    let from_cache = cache_hit.is_some();

    let (findings, pack_rsids, geno_count) = if let Some(hit) = cache_hit {
        emit_discovery_progress(app, 90, "Using cached catalog joins…");
        hit
    } else {
        let (pack_rsids, _pack_genes, _) = pack_rsids_and_genes(data_dir)?;
        let sample_conn = db::connect_sample_from_registry_path(db_path, sample_id)
            .map_err(|e| format!("Open sample DB: {e}"))?;

        let geno_count: i64 = sample_conn
            .query_row(
                "SELECT COUNT(*) FROM genotypes WHERE sample_id = ?",
                params![sample_id],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let findings = load_findings_via_join(&sample_conn, sample_id, app)?;
        if let Ok(mut guard) = discovery_cache().lock() {
            *guard = Some(DiscoveryJoinCache {
                sample_id,
                findings: findings.clone(),
                pack_rsids: pack_rsids.clone(),
                geno_count,
                created: Instant::now(),
            });
        }
        (findings, pack_rsids, geno_count)
    };

    check_discovery_cancel()?;

    let source = source_filter
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s != "all");
    let q = query
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    let mut beyond = Vec::new();
    let mut in_pack = 0usize;
    for mut entry in findings {
        let rsid = entry
            .get("rsid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let in_packs = pack_rsids.contains(&rsid);
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("in_marker_packs".into(), json!(in_packs));
        }
        if in_packs {
            in_pack += 1;
            if beyond_packs_only {
                continue;
            }
        }
        if let Some(ref src) = source {
            if !finding_matches_source(&entry, src) {
                continue;
            }
        }
        if !finding_matches_query(&entry, &q) {
            continue;
        }
        beyond.push(entry);
    }

    beyond.sort_by_key(|b| std::cmp::Reverse(finding_priority(b)));
    let total_matched = beyond.len();
    let limit = limit.clamp(1, 500);
    let page: Vec<Value> = beyond.into_iter().skip(offset).take(limit).collect();

    emit_discovery_progress(app, 100, "Discovery query complete");

    Ok(json!({
        "sample_id": sample_id,
        "genotype_rsid_count": geno_count,
        "pack_rsid_count": pack_rsids.len(),
        "findings_in_packs": in_pack,
        "total_matched": total_matched,
        "beyond_packs_only": beyond_packs_only,
        "source_filter": source,
        "query": if q.is_empty() { Value::Null } else { json!(q) },
        "limit": limit,
        "offset": offset,
        "cached": from_cache,
        "items": page,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ranks_pathogenic_above_gwas() {
        let pathogenic = json!({
            "clinvar": { "clinical_significance": "Pathogenic" }
        });
        let gwas_only = json!({
            "gwas": { "top_trait": "height" }
        });
        assert!(finding_priority(&pathogenic) > finding_priority(&gwas_only));
    }

    #[test]
    fn source_and_query_filters() {
        let entry = json!({
            "rsid": "rs123",
            "clinvar": { "clinical_significance": "risk factor", "gene": "MTHFR" },
            "gwas": null,
            "pharmgkb": null
        });
        assert!(finding_matches_source(&entry, "clinvar"));
        assert!(!finding_matches_source(&entry, "pharmgkb"));
        assert!(finding_matches_query(&entry, "mthfr"));
        assert!(!finding_matches_query(&entry, "cyp2c19"));
    }

    #[test]
    fn plural_reference_records_drive_priority_and_filters() {
        let entry = json!({
            "rsid": "rs456",
            "clinvar_annotations": [
                { "clinical_significance": "Pathogenic", "variation_id": "200" },
                { "clinical_significance": "Benign", "variation_id": "201" }
            ],
            "pharmgkb_annotations": [
                { "drug": "clopidogrel", "phenotype": "reduced response" },
                { "drug": "omeprazole", "phenotype": "increased exposure" }
            ]
        });
        assert!(finding_priority(&entry) >= 130);
        assert!(finding_matches_source(&entry, "clinvar"));
        assert!(finding_matches_source(&entry, "pharmgkb"));
        assert!(finding_matches_query(&entry, "clopidogrel"));
        assert!(finding_matches_query(&entry, "201"));
    }
}
