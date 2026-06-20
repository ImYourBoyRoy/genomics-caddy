// ./src-tauri/src/research/gnomad/batch.rs
use super::config::load_gnomad_config;
use super::lookup::{lookup_variant_coords, resolve_gnomad_context, VariantCoords};
use super::types::{
    GnomadBatchProgress, GnomadBatchRequest, GnomadLookupRequest,
    GnomadLookupStatus, GnomadSourceMode,
};
use rusqlite::params;
use std::collections::HashMap;
use std::path::Path;

/// Warm SQLite gnomAD cache before parallel enrichment — grouped by chromosome, sorted by position.
pub async fn prefetch_gnomad_batch(
    db_path: &Path,
    data_dir: &Path,
    sample_id: i64,
    rsids: &[String],
) -> GnomadBatchProgress {
    if rsids.is_empty() {
        return GnomadBatchProgress {
            total_candidates: 0,
            cache_hits: 0,
            remote_queries: 0,
            local_queries: 0,
            allele_matches: 0,
            allele_mismatches: 0,
            missing_coordinates: 0,
            errors: 0,
            current_chromosome: None,
            estimated_remaining: Some(0),
        };
    }

    let cfg = match crate::db::connect(db_path) {
        Ok(conn) => load_gnomad_config(&conn).ok(),
        Err(_) => None,
    };
    let Some(cfg) = cfg else {
        return empty_progress(rsids.len());
    };
    if !cfg.enabled {
        return empty_progress(rsids.len());
    }

    let mode = if cfg.graphql_enabled_for_sweep {
        GnomadSourceMode::GraphQlInteractive
    } else if cfg.source_mode == GnomadSourceMode::PythonToolboxSidecar {
        GnomadSourceMode::RemoteIndexedVcfHttps
    } else {
        cfg.source_mode
    };

    let mut by_chrom: HashMap<String, Vec<(String, VariantCoords)>> = HashMap::new();
    let mut progress = empty_progress(rsids.len());

    for rsid in rsids {
        match lookup_variant_coords(db_path, sample_id, rsid) {
            Ok(coords) => {
                by_chrom
                    .entry(coords.chrom.clone())
                    .or_default()
                    .push((rsid.clone(), coords));
            }
            Err(GnomadLookupStatus::MissingCoordinate) => progress.missing_coordinates += 1,
            Err(_) => progress.errors += 1,
        }
    }

    for (chrom, mut entries) in by_chrom {
        progress.current_chromosome = Some(chrom.clone());
        entries.sort_by_key(|(_, c)| c.pos);
        for (rsid, coords) in entries {
            crate::research::sweep_metrics::set_batch_current_rsid(&rsid);
            crate::research::sweep_metrics::bump_batch_prefetch();
            let ctx = resolve_gnomad_context(
                db_path,
                data_dir,
                &cfg,
                &cfg.release,
                mode,
                &rsid,
                sample_id,
                &coords,
                None,
                None,
                false,
            )
            .await;
            tally_progress(&mut progress, &ctx);
        }
    }

    progress.estimated_remaining = Some(0);
    progress
}

pub async fn batch_enrich_gnomad_context(
    db_path: &Path,
    data_dir: &Path,
    req: GnomadBatchRequest,
) -> (GnomadBatchProgress, Vec<(String, super::types::GnomadContext)>) {
    let limit = req.limit.unwrap_or(500);
    let candidates = collect_candidates(db_path, &req, limit);
    let progress = prefetch_gnomad_batch(db_path, data_dir, req.sample_id, &candidates).await;

    let cfg = crate::db::connect(db_path)
        .ok()
        .and_then(|conn| load_gnomad_config(&conn).ok())
        .unwrap_or_default();

    let mode = req
        .source_mode
        .as_deref()
        .map(GnomadSourceMode::from_str_loose)
        .unwrap_or(cfg.source_mode);

    let mut results = Vec::new();
    for rsid in &candidates {
        let ctx = super::lookup::get_gnomad_context(
            db_path,
            data_dir,
            GnomadLookupRequest {
                sample_id: req.sample_id,
                rsid: rsid.clone(),
                chrom_grch38: None,
                pos_grch38: None,
                reference_allele: None,
                alternate_allele: None,
                genotype: None,
                dataset: None,
                release: req.release.clone(),
                source_mode: Some(mode.as_str().to_string()),
                force_refresh: req.force_refresh,
            },
        )
        .await;
        results.push((rsid.clone(), ctx));
    }

    (progress, results)
}

fn empty_progress(total: usize) -> GnomadBatchProgress {
    GnomadBatchProgress {
        total_candidates: total,
        cache_hits: 0,
        remote_queries: 0,
        local_queries: 0,
        allele_matches: 0,
        allele_mismatches: 0,
        missing_coordinates: 0,
        errors: 0,
        current_chromosome: None,
        estimated_remaining: Some(total),
    }
}

fn tally_progress(progress: &mut GnomadBatchProgress, ctx: &super::types::GnomadContext) {
    match GnomadLookupStatus::from_str_loose(&ctx.lookup_status) {
        GnomadLookupStatus::CacheHit => progress.cache_hits += 1,
        GnomadLookupStatus::RemoteVcfHit => progress.remote_queries += 1,
        GnomadLookupStatus::LocalVcfHit => progress.local_queries += 1,
        GnomadLookupStatus::GraphQlHit => progress.remote_queries += 1,
        GnomadLookupStatus::AlleleMismatch => progress.allele_mismatches += 1,
        GnomadLookupStatus::MissingCoordinate => progress.missing_coordinates += 1,
        GnomadLookupStatus::NetworkError
        | GnomadLookupStatus::ParserError
        | GnomadLookupStatus::SourceUnavailable
        | GnomadLookupStatus::IndexMissing => progress.errors += 1,
        _ => {}
    }
    if ctx.user_allele_match_status.as_deref() == Some("allele_match") {
        progress.allele_matches += 1;
    }
}

fn collect_candidates(db_path: &Path, req: &GnomadBatchRequest, limit: usize) -> Vec<String> {
    if let Some(rsids) = &req.rsids {
        return rsids.iter().take(limit).cloned().collect();
    }

    let conn = crate::db::connect(db_path).ok();
    let Some(conn) = conn else {
        return Vec::new();
    };

    let priority = req.priority_mode.as_deref().unwrap_or("high_value");
    if priority == "all" {
        let mut stmt = conn
            .prepare(
                "SELECT rsid FROM genotypes WHERE sample_id = ? ORDER BY chromosome, position_grch38 LIMIT ?",
            )
            .ok();
        if let Some(ref mut s) = stmt
            && let Ok(rows) = s.query_map(params![req.sample_id, limit as i64], |row| row.get(0)) {
                return rows.filter_map(|r| r.ok()).collect();
            }
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let queries = [
        "SELECT DISTINCT rsid FROM association_facts WHERE sample_id = ? AND rsid IS NOT NULL LIMIT ?",
        "SELECT DISTINCT rsid FROM discovered_findings WHERE sample_id = ? LIMIT ?",
        "SELECT rsid FROM gwas_reference LIMIT ?",
    ];

    for sql in queries {
        if out.len() >= limit {
            break;
        }
        let remaining = (limit - out.len()) as i64;
        if let Ok(mut stmt) = conn.prepare(sql) {
            let rows: Result<Vec<String>, _> = if sql.contains("gwas_reference") {
                stmt.query_map(params![remaining], |row| row.get(0))
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
            } else {
                stmt.query_map(params![req.sample_id, remaining], |row| row.get(0))
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
            };
            if let Ok(list) = rows {
                for rsid in list {
                    if seen.insert(rsid.clone()) {
                        out.push(rsid);
                    }
                }
            }
        }
    }

    out.truncate(limit);
    out
}
