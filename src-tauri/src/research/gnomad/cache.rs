// ./src-tauri/src/research/gnomad/cache.rs
use super::allele_norm::{cache_alleles, normalize_allele_token, normalize_chrom_key};
use super::types::{GnomadContext, GnomadLookupStatus, GnomadSourceMode, PARSER_VERSION};
use crate::research::sweep_metrics::record_gnomad_cache_lookup;
use crate::research::util::normalize_rsid;
use rusqlite::{Connection, Row, params};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

const USABLE_CACHE_STATUSES: &str = "'found', 'remote_vcf_hit', 'local_vcf_hit', 'graphql_hit', 'cache_hit'";

#[derive(Debug, Clone, Default)]
pub struct FrequencyCacheStatus {
    pub current_rows: u64,
    pub stale_rows: u64,
}

/// Report cache freshness without exposing any genotype or allele values.
pub fn frequency_cache_status(db_path: &std::path::Path, release: &str) -> FrequencyCacheStatus {
    let Ok(conn) = crate::db::connect(db_path) else {
        return FrequencyCacheStatus::default();
    };
    let current_rows = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM reference.gnomad_variant_cache
                 WHERE release = ? AND lookup_status IN ({USABLE_CACHE_STATUSES})"
            ),
            params![release],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    let stale_rows = conn
        .query_row(
            "SELECT COUNT(*) FROM reference.gnomad_variant_cache
                 WHERE (release != ? OR lookup_status = 'stale_release')",
            params![release],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        .max(0) as u64;
    FrequencyCacheStatus {
        current_rows,
        stale_rows,
    }
}

const CACHE_ROW_SQL: &str =
    "SELECT release, dataset, chrom, pos, ref, alt, variant_id, rsids_json, ac, an, af,
                ac_exomes, an_exomes, af_exomes, ac_genomes, an_genomes, af_genomes,
                popmax, popmax_population, faf95_popmax, faf95_popmax_population,
                homozygote_count, hemizygote_count, filters_json, flags_json, info_json,
                source_url, source_mode, fetched_at, user_allele_match_status, lookup_status
         FROM gnomad_variant_cache";

#[derive(Debug, Clone)]
struct CacheEntry {
    chrom: String,
    ref_allele: String,
    alt: String,
    dataset: String,
    ctx: GnomadContext,
}

pub fn cache_key(
    release: &str,
    dataset: &str,
    chrom: &str,
    pos: i64,
    ref_allele: &str,
    alt: &str,
) -> String {
    let (r, a) = cache_alleles(ref_allele, alt);
    let ch = normalize_chrom_key(chrom);
    format!("{release}|{dataset}|{ch}|{pos}|{r}|{a}")
}

pub fn hash_record(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

fn parse_json_vec(raw: Option<String>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
}

fn map_cache_row(row: &Row) -> rusqlite::Result<CacheEntry> {
    let filters: Vec<String> = row
        .get::<_, Option<String>>(23)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let flags: Vec<String> = row
        .get::<_, Option<String>>(24)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let pop_freq: serde_json::Value = row
        .get::<_, Option<String>>(25)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    let ctx = GnomadContext {
        lookup_status: row.get::<_, String>(30)?,
        release: row.get(0)?,
        dataset: row.get(1)?,
        variant_id: row.get(6)?,
        rsids: parse_json_vec(row.get(7)?),
        ac: row.get(8)?,
        an: row.get(9)?,
        af: row.get(10)?,
        ac_exomes: row.get(11)?,
        an_exomes: row.get(12)?,
        af_exomes: row.get(13)?,
        ac_genomes: row.get(14)?,
        an_genomes: row.get(15)?,
        af_genomes: row.get(16)?,
        popmax: row.get(17)?,
        popmax_population: row.get(18)?,
        faf95_popmax: row.get(19)?,
        faf95_popmax_population: row.get(20)?,
        homozygote_count: row.get(21)?,
        hemizygote_count: row.get(22)?,
        filters,
        flags,
        population_frequencies: pop_freq,
        source_url: row.get(26)?,
        source_mode: row.get(27)?,
        fetched_at: row.get(28)?,
        warnings: Vec::new(),
        user_allele_match_status: row.get(29)?,
    };
    Ok(CacheEntry {
        chrom: row.get(2)?,
        ref_allele: row.get(4)?,
        alt: row.get(5)?,
        dataset: row.get(1)?,
        ctx,
    })
}

fn rsid_in_entry(entry: &CacheEntry, rsid: Option<&str>) -> bool {
    let Some(rsid_key) = rsid.and_then(normalize_rsid) else {
        return true;
    };
    entry.ctx.rsids.iter().any(|r| {
        normalize_rsid(r)
            .map(|k| k.eq_ignore_ascii_case(&rsid_key))
            .unwrap_or(false)
    })
}

fn user_alleles_match_row(user_a1: &str, user_a2: &str, _ref_a: &str, alt: &str) -> bool {
    let u1 = normalize_allele_token(user_a1);
    let u2 = normalize_allele_token(user_a2);
    if u1.is_empty() || u2.is_empty() || u1 == "0" || u2 == "0" || u1 == "N" || u2 == "N" {
        return false;
    }
    let a = normalize_allele_token(alt);
    u1 == a || u2 == a
}

fn merge_cached_entries(entries: &[CacheEntry]) -> GnomadContext {
    let mut merged = entries[0].ctx.clone();
    for entry in entries.iter().skip(1) {
        if entry.dataset == "genomes" || entry.ctx.af_genomes.is_some() {
            merged.ac_genomes = entry.ctx.ac_genomes.or(merged.ac_genomes);
            merged.an_genomes = entry.ctx.an_genomes.or(merged.an_genomes);
            merged.af_genomes = entry.ctx.af_genomes.or(merged.af_genomes);
        }
        if entry.dataset == "exomes" || entry.ctx.af_exomes.is_some() {
            merged.ac_exomes = entry.ctx.ac_exomes.or(merged.ac_exomes);
            merged.an_exomes = entry.ctx.an_exomes.or(merged.an_exomes);
            merged.af_exomes = entry.ctx.af_exomes.or(merged.af_exomes);
        }
        if merged.ac.is_none() {
            merged.ac = entry.ctx.ac;
        }
        if merged.an.is_none() {
            merged.an = entry.ctx.an;
        }
        if merged.af.is_none() {
            merged.af = entry.ctx.af;
        }
        if merged.variant_id.is_none() {
            merged.variant_id = entry.ctx.variant_id.clone();
        }
        if merged.rsids.is_empty() {
            merged.rsids = entry.ctx.rsids.clone();
        }
    }
    if entries.len() > 1 {
        merged.dataset = "combined".into();
        merged.af = merged.best_af();
    }
    merged.lookup_status = GnomadLookupStatus::CacheHit.as_str().to_string();
    merged
}

fn fetch_cache_rows_at_locus(
    conn: &Connection,
    release: &str,
    pos: i64,
) -> Option<Vec<CacheEntry>> {
    let sql = format!("{CACHE_ROW_SQL} WHERE release = ? AND pos = ? ORDER BY fetched_at DESC");
    let mut stmt = conn.prepare(&sql).ok()?;
    let rows = stmt.query_map(params![release, pos], map_cache_row).ok()?;
    let entries: Vec<CacheEntry> = rows.filter_map(|r| r.ok()).collect();
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

/// Read cached gnomAD context without requiring ref/alt up front (matches prefetch writes).
#[allow(clippy::too_many_arguments)]
pub fn read_cache_for_variant(
    conn: &Connection,
    release: &str,
    chrom: &str,
    pos: i64,
    rsid: Option<&str>,
    user_allele1: Option<&str>,
    user_allele2: Option<&str>,
    ref_allele: Option<&str>,
    alt_allele: Option<&str>,
) -> Option<GnomadContext> {
    if let (Some(r), Some(a)) = (ref_allele, alt_allele) {
        for dataset in ["combined", "exomes", "genomes"] {
            if let Some(mut cached) = read_cache(conn, release, dataset, chrom, pos, r, a) {
                cached.lookup_status = GnomadLookupStatus::CacheHit.as_str().to_string();
                record_gnomad_cache_lookup(true);
                return Some(cached);
            }
        }
    }

    let entries = fetch_cache_rows_at_locus(conn, release, pos)?;
    let target_chrom = normalize_chrom_key(chrom);
    let mut candidates: Vec<CacheEntry> = entries
        .into_iter()
        .filter(|e| normalize_chrom_key(&e.chrom) == target_chrom)
        .filter(|e| rsid_in_entry(e, rsid))
        .collect();

    if candidates.is_empty() {
        record_gnomad_cache_lookup(false);
        return None;
    }

    if let (Some(a1), Some(a2)) = (user_allele1, user_allele2)
        && let Some(matched) = candidates
            .iter()
            .find(|e| user_alleles_match_row(a1, a2, &e.ref_allele, &e.alt))
            .map(|e| normalize_allele_token(&e.alt))
    {
        let same_alt: Vec<CacheEntry> = candidates
            .into_iter()
            .filter(|e| normalize_allele_token(&e.alt) == matched)
            .collect();
        record_gnomad_cache_lookup(true);
        return Some(merge_cached_entries(&same_alt));
    }

    candidates.sort_by(|a, b| {
        b.ctx
            .best_af()
            .partial_cmp(&a.ctx.best_af())
            .unwrap_or(Ordering::Equal)
    });
    let best_alt = normalize_allele_token(&candidates[0].alt);
    let same_alt: Vec<CacheEntry> = candidates
        .into_iter()
        .filter(|e| normalize_allele_token(&e.alt) == best_alt)
        .collect();
    record_gnomad_cache_lookup(true);
    Some(merge_cached_entries(&same_alt))
}

pub fn read_cache(
    conn: &Connection,
    release: &str,
    dataset: &str,
    chrom: &str,
    pos: i64,
    ref_allele: &str,
    alt: &str,
) -> Option<GnomadContext> {
    let key = cache_key(release, dataset, chrom, pos, ref_allele, alt);
    conn.query_row(
        &format!("{CACHE_ROW_SQL} WHERE cache_id = ?"),
        params![key],
        map_cache_row,
    )
    .ok()
    .map(|entry| entry.ctx)
}

pub struct CacheWriteInput {
    pub release: String,
    pub source_mode: GnomadSourceMode,
    pub dataset: String,
    pub chrom: String,
    pub pos: i64,
    pub ref_allele: String,
    pub alt: String,
    pub variant_id: Option<String>,
    pub rsids: Vec<String>,
    pub genotype: Option<String>,
    pub user_allele_match_status: Option<String>,
    pub ac: Option<i64>,
    pub an: Option<i64>,
    pub af: Option<f64>,
    pub ac_exomes: Option<i64>,
    pub an_exomes: Option<i64>,
    pub af_exomes: Option<f64>,
    pub ac_genomes: Option<i64>,
    pub an_genomes: Option<i64>,
    pub af_genomes: Option<f64>,
    pub popmax: Option<f64>,
    pub popmax_population: Option<String>,
    pub faf95_popmax: Option<f64>,
    pub faf95_popmax_population: Option<String>,
    pub homozygote_count: Option<i64>,
    pub hemizygote_count: Option<i64>,
    pub filters: Vec<String>,
    pub flags: Vec<String>,
    pub info_json: serde_json::Value,
    pub source_url: Option<String>,
    pub source_file: Option<String>,
    pub source_index: Option<String>,
    pub raw_record_hash: String,
    pub lookup_status: GnomadLookupStatus,
}

pub fn write_cache(conn: &Connection, input: CacheWriteInput) -> Result<(), String> {
    let key = cache_key(
        &input.release,
        &input.dataset,
        &input.chrom,
        input.pos,
        &input.ref_allele,
        &input.alt,
    );
    let now = unix_now();
    conn.execute(
        "INSERT OR REPLACE INTO reference.gnomad_variant_cache (
            cache_id, release, source_mode, dataset, chrom, pos, ref, alt,
            variant_id, rsids_json, genotype, user_allele_match_status,
            ac, an, af, ac_exomes, an_exomes, af_exomes, ac_genomes, an_genomes, af_genomes,
            popmax, popmax_population, faf95_popmax, faf95_popmax_population,
            homozygote_count, hemizygote_count, filters_json, flags_json, info_json,
            source_url, source_file, source_index, fetched_at, raw_record_hash,
            parser_version, lookup_status
        ) VALUES (
            ?, ?, ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?
        )",
        params![
            key,
            input.release,
            input.source_mode.as_str(),
            input.dataset,
            input.chrom,
            input.pos,
            input.ref_allele,
            input.alt,
            input.variant_id,
            serde_json::to_string(&input.rsids).unwrap_or_else(|_| "[]".into()),
            input.genotype,
            input.user_allele_match_status,
            input.ac,
            input.an,
            input.af,
            input.ac_exomes,
            input.an_exomes,
            input.af_exomes,
            input.ac_genomes,
            input.an_genomes,
            input.af_genomes,
            input.popmax,
            input.popmax_population,
            input.faf95_popmax,
            input.faf95_popmax_population,
            input.homozygote_count,
            input.hemizygote_count,
            serde_json::to_string(&input.filters).ok(),
            serde_json::to_string(&input.flags).ok(),
            serde_json::to_string(&input.info_json).ok(),
            input.source_url,
            input.source_file,
            input.source_index,
            now,
            input.raw_record_hash,
            PARSER_VERSION,
            input.lookup_status.as_str(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_gnomad_cache(conn: &Connection) -> Result<usize, String> {
    let n = conn
        .execute("DELETE FROM reference.gnomad_variant_cache", [])
        .map_err(|e| e.to_string())?;
    Ok(n)
}

pub fn mark_stale_release(conn: &Connection, old_release: &str) -> Result<usize, String> {
    let n = conn
        .execute(
            "UPDATE reference.gnomad_variant_cache SET lookup_status = ? WHERE release != ?",
            params![GnomadLookupStatus::StaleRelease.as_str(), old_release],
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub fn context_to_cache_write(
    ctx: &GnomadContext,
    source_mode: GnomadSourceMode,
    chrom: &str,
    pos: i64,
    ref_allele: &str,
    alt: &str,
    genotype: Option<String>,
    raw_hash: String,
) -> CacheWriteInput {
    CacheWriteInput {
        release: ctx.release.clone(),
        source_mode,
        dataset: ctx.dataset.clone(),
        chrom: chrom.to_string(),
        pos,
        ref_allele: ref_allele.to_string(),
        alt: alt.to_string(),
        variant_id: ctx.variant_id.clone(),
        rsids: ctx.rsids.clone(),
        genotype,
        user_allele_match_status: ctx.user_allele_match_status.clone(),
        ac: ctx.ac,
        an: ctx.an,
        af: ctx.af,
        ac_exomes: ctx.ac_exomes,
        an_exomes: ctx.an_exomes,
        af_exomes: ctx.af_exomes,
        ac_genomes: ctx.ac_genomes,
        an_genomes: ctx.an_genomes,
        af_genomes: ctx.af_genomes,
        popmax: ctx.popmax,
        popmax_population: ctx.popmax_population.clone(),
        faf95_popmax: ctx.faf95_popmax,
        faf95_popmax_population: ctx.faf95_popmax_population.clone(),
        homozygote_count: ctx.homozygote_count,
        hemizygote_count: ctx.hemizygote_count,
        filters: ctx.filters.clone(),
        flags: ctx.flags.clone(),
        info_json: ctx.population_frequencies.clone(),
        source_url: ctx.source_url.clone(),
        source_file: None,
        source_index: None,
        raw_record_hash: raw_hash,
        lookup_status: GnomadLookupStatus::from_str_loose(&ctx.lookup_status),
    }
}

#[cfg(test)]
mod tests {
    use super::super::schema::migrate_gnomad_schema;
    use super::super::types::{GnomadLookupStatus, GnomadSourceMode};
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("ATTACH DATABASE ':memory:' AS reference", [])
            .unwrap();
        migrate_gnomad_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn locus_cache_lookup_matches_prefetch_dataset() {
        let conn = test_conn();
        write_cache(
            &conn,
            CacheWriteInput {
                release: "4.1".into(),
                source_mode: GnomadSourceMode::RemoteIndexedVcfHttps,
                dataset: "exomes".into(),
                chrom: "22".into(),
                pos: 12345,
                ref_allele: "A".into(),
                alt: "G".into(),
                variant_id: Some("22-12345-A-G".into()),
                rsids: vec!["rs999".into()],
                genotype: None,
                user_allele_match_status: None,
                ac: Some(10),
                an: Some(1000),
                af: Some(0.01),
                ac_exomes: Some(10),
                an_exomes: Some(1000),
                af_exomes: Some(0.01),
                ac_genomes: None,
                an_genomes: None,
                af_genomes: None,
                popmax: None,
                popmax_population: None,
                faf95_popmax: None,
                faf95_popmax_population: None,
                homozygote_count: None,
                hemizygote_count: None,
                filters: vec![],
                flags: vec![],
                info_json: serde_json::json!({}),
                source_url: None,
                source_file: None,
                source_index: None,
                raw_record_hash: "abc".into(),
                lookup_status: GnomadLookupStatus::RemoteVcfHit,
            },
        )
        .unwrap();

        let hit = read_cache_for_variant(
            &conn,
            "4.1",
            "chr22",
            12345,
            Some("rs999"),
            Some("A"),
            Some("G"),
            None,
            None,
        );
        assert!(hit.is_some());
        assert_eq!(
            hit.unwrap().lookup_status,
            GnomadLookupStatus::CacheHit.as_str()
        );
    }
}
