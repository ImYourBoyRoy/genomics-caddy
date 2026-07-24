// ./src-tauri/src/offline/tier2.rs
//! Build variant locus index from imported genotypes (per-sample or all samples).
//!
//! Genotypes live in per-sample SQLite files under `App/Data/samples/<id>/`.
//! Never query `genotypes` on the registry (`user_genome.db`) connection.

use crate::research::util::normalize_rsid;
use rusqlite::{Connection, params};
use std::path::Path;

fn variant_key(assembly: &str, chrom: &str, pos: i64, a1: &str, a2: &str) -> String {
    format!("{assembly}|{chrom}|{pos}|{a1}|{a2}")
}

/// Build/rebuild `variant_locus` inside an already-opened **sample** database.
pub fn build_variant_locus_for_sample(conn: &Connection, sample_id: i64) -> Result<u64, String> {
    // Guard: registry DB has no genotypes table after the per-sample split.
    let has_genotypes: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'genotypes' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !has_genotypes {
        return Err(format!(
            "Sample {sample_id}: genotypes table missing (open the sample DB, not the registry)"
        ));
    }

    conn.execute(
        "DELETE FROM variant_locus WHERE sample_id = ?",
        params![sample_id],
    )
    .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT rsid, chromosome, position_grch38, allele1, allele2
             FROM genotypes
             WHERE sample_id = ? AND position_grch38 IS NOT NULL AND position_grch38 > 0",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![sample_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for row in rows {
        let (rsid_raw, chrom, pos_opt, a1, a2) = row.map_err(|e| e.to_string())?;
        let Some(pos) = pos_opt.filter(|p| *p > 0) else {
            continue;
        };
        let rsid = normalize_rsid(&rsid_raw).unwrap_or(rsid_raw);
        let key = variant_key("GRCh38", &chrom, pos, &a1, &a2);
        tx.execute(
            "INSERT OR REPLACE INTO variant_locus
             (variant_key, assembly, chrom, pos, ref_allele, alt_allele, rsid, sample_id, source)
             VALUES (?, 'GRCh38', ?, ?, ?, ?, ?, ?, 'genotype')",
            params![key, chrom, pos, a1, a2, rsid, sample_id],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Open the sample DB under `data_dir` and rebuild its variant locus index.
pub fn build_variant_locus_for_sample_in_data_dir(
    data_dir: &Path,
    sample_id: i64,
) -> Result<u64, String> {
    let conn = crate::db::connect_sample(data_dir, sample_id).map_err(|e| e.to_string())?;
    build_variant_locus_for_sample(&conn, sample_id)
}

/// Rebuild locus indexes for every registered sample (each in its own DB).
pub fn build_variant_locus_all_samples(
    data_dir: &Path,
    registry: &Connection,
) -> Result<u64, String> {
    let samples = crate::db::get_samples(registry).map_err(|e| e.to_string())?;
    if samples.is_empty() {
        return Ok(0);
    }

    let mut total = 0u64;
    let mut errors: Vec<String> = Vec::new();
    for sample in samples {
        match build_variant_locus_for_sample_in_data_dir(data_dir, sample.id) {
            Ok(n) => total += n,
            Err(e) => errors.push(format!("sample {}: {e}", sample.id)),
        }
    }
    if total == 0 && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(total)
}

pub fn ensure_tier2_meta(data_dir: &Path) -> Result<(), String> {
    let dir = data_dir.join("tier2");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let meta = dir.join("variant_locus.meta");
    if !meta.exists() {
        std::fs::write(&meta, "tier2 variant locus index\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Sum `variant_locus` rows across all sample databases (registry has no genotypes).
pub fn count_variant_locus_rows(data_dir: &Path, registry: &Connection) -> u64 {
    let Ok(samples) = crate::db::get_samples(registry) else {
        return 0;
    };
    samples
        .iter()
        .filter_map(|sample| crate::db::connect_sample(data_dir, sample.id).ok())
        .map(|conn| {
            conn.query_row("SELECT COUNT(*) FROM variant_locus", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as u64
        })
        .sum()
}
