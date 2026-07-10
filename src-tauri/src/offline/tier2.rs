// ./src-tauri/src/offline/tier2.rs
//! Build variant locus index from imported genotypes (per-sample or all samples).

use crate::research::util::normalize_rsid;
use rusqlite::{Connection, params};
use std::path::Path;

fn variant_key(assembly: &str, chrom: &str, pos: i64, a1: &str, a2: &str) -> String {
    format!("{assembly}|{chrom}|{pos}|{a1}|{a2}")
}

pub fn build_variant_locus_for_sample(conn: &Connection, sample_id: i64) -> Result<u64, String> {
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

pub fn build_variant_locus_all_samples(conn: &Connection) -> Result<u64, String> {
    conn.execute("DELETE FROM variant_locus", [])
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT sample_id, rsid, chromosome, position_grch38, allele1, allele2
             FROM genotypes
             WHERE position_grch38 IS NOT NULL AND position_grch38 > 0",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut count = 0u64;

    for row in rows {
        let (sample_id, rsid_raw, chrom, pos_opt, a1, a2) = row.map_err(|e| e.to_string())?;
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

pub fn ensure_tier2_meta(data_dir: &Path) -> Result<(), String> {
    let dir = data_dir.join("tier2");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let meta = dir.join("variant_locus.meta");
    if !meta.exists() {
        std::fs::write(&meta, "tier2 variant locus index\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}
