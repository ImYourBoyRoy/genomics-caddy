// ./src-tauri/src/research/gnomad/vcf_remote.rs
use super::allele_norm::cache_alleles;
use super::cache::{hash_record, write_cache};
use super::config::{index_urls, tabix_reference_name, vcf_url};
use super::types::{GnomadConfig, GnomadContext, GnomadLookupStatus, GnomadSourceMode};
use super::vcf_parse::{match_user_allele, parse_site_line, record_to_context};
use crate::research::http::BLOCKING_HTTP_CLIENT;
use crate::research::http::HTTP_CLIENT;
use crate::research::tuning::gnomad_remote_timeout_secs;
use noodles_bgzf as bgzf;
use noodles_core::Region;
use noodles_csi::binning_index::BinningIndex;
use noodles_tabix as tabix;
use std::io::{BufRead, Cursor, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[allow(clippy::too_many_arguments)]
pub async fn query_remote_vcf(
    db_path: &Path,
    cfg: &GnomadConfig,
    cache_dir: &Path,
    chrom: &str,
    pos: i64,
    allele1: &str,
    allele2: &str,
    genotype: Option<&str>,
    dataset: &str,
    rsid: &str,
) -> GnomadContext {
    let template = if dataset == "exomes" {
        &cfg.exome_template
    } else {
        &cfg.genome_template
    };
    let vcf = vcf_url(cfg, template, chrom);
    let (tbi_url, csi_url) = index_urls(&vcf);

    let timeout = std::time::Duration::from_secs(gnomad_remote_timeout_secs());
    let result = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking({
            let vcf = vcf.clone();
            let tbi_url = tbi_url.clone();
            let csi_url = csi_url.clone();
            let cache_dir = cache_dir.to_path_buf();
            let chrom = chrom.to_string();
            let cfg_release = cfg.release.clone();
            let allele1 = allele1.to_string();
            let allele2 = allele2.to_string();
            let dataset = dataset.to_string();
            move || {
                query_remote_sync(
                    &cache_dir,
                    &vcf,
                    &tbi_url,
                    &csi_url,
                    &chrom,
                    pos,
                    &allele1,
                    &allele2,
                    &cfg_release,
                    &dataset,
                )
            }
        }),
    )
    .await;

    match result {
        Ok(Ok(Ok((ctx, raw_hash, ref_a, alt_a)))) => {
            if let Ok(conn) = crate::db::connect(db_path) {
                let (ref_n, alt_n) = cache_alleles(&ref_a, &alt_a);
                let _ = write_cache(
                    &conn,
                    super::cache::CacheWriteInput {
                        release: ctx.release.clone(),
                        source_mode: GnomadSourceMode::RemoteIndexedVcfHttps,
                        dataset: ctx.dataset.clone(),
                        chrom: chrom.to_string(),
                        pos,
                        ref_allele: ref_n,
                        alt: alt_n,
                        variant_id: ctx.variant_id.clone(),
                        rsids: if ctx.rsids.is_empty() {
                            vec![rsid.to_string()]
                        } else {
                            ctx.rsids.clone()
                        },
                        genotype: genotype.map(String::from),
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
                        source_file: Some(vcf.clone()),
                        source_index: Some(tbi_url),
                        raw_record_hash: raw_hash,
                        lookup_status: GnomadLookupStatus::from_str_loose(&ctx.lookup_status),
                    },
                );
            }
            ctx
        }
        Ok(Ok(Err(status))) => GnomadContext::empty(status),
        Ok(Err(_)) => GnomadContext::empty(GnomadLookupStatus::NetworkError),
        Err(_) => GnomadContext::empty(GnomadLookupStatus::NetworkError),
    }
}

#[allow(clippy::too_many_arguments)]
fn query_remote_sync(
    cache_dir: &Path,
    vcf_url: &str,
    tbi_url: &str,
    csi_url: &str,
    chrom: &str,
    pos: i64,
    allele1: &str,
    allele2: &str,
    release: &str,
    dataset: &str,
) -> Result<(GnomadContext, String, String, String), GnomadLookupStatus> {
    std::fs::create_dir_all(cache_dir).map_err(|_| GnomadLookupStatus::SourceUnavailable)?;
    let index_path = ensure_index_cached(cache_dir, vcf_url, tbi_url, csi_url)?;
    let index = tabix::fs::read(&index_path).map_err(|_| GnomadLookupStatus::IndexMissing)?;

    let ref_name = tabix_reference_name(chrom);
    let region = Region::from_str(&format!("{ref_name}:{pos}-{pos}"))
        .map_err(|_| GnomadLookupStatus::ParserError)?;
    let header = index.header().ok_or(GnomadLookupStatus::ParserError)?;
    let ref_id = header
        .reference_sequence_names()
        .get_index_of(region.name())
        .ok_or(GnomadLookupStatus::ParserError)?;
    let chunks = index
        .query(ref_id, region.interval())
        .map_err(|_| GnomadLookupStatus::ParserError)?;

    if chunks.is_empty() {
        return Err(GnomadLookupStatus::NoRecordAtPosition);
    }

    let start = chunks.first().map(|c| c.start().compressed()).unwrap_or(0);
    let end = chunks.last().map(|c| c.end().compressed()).unwrap_or(start);
    let fetch_end = end.saturating_add(65535);

    let bytes = http_range_bytes(vcf_url, start, fetch_end)
        .map_err(|_| GnomadLookupStatus::NetworkError)?;

    let mut reader = bgzf::io::Reader::new(Cursor::new(bytes));
    let mut line = String::new();
    let mut matched: Option<(ParsedMatch, String)> = None;

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|_| GnomadLookupStatus::ParserError)?;
        if n == 0 {
            break;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some(rec) = parse_site_line(&line) else {
            continue;
        };
        if rec.pos != pos {
            continue;
        }
        if let Some((ref_a, alt_a)) =
            match_user_allele(&rec.ref_allele, &rec.alt_alleles, allele1, allele2)
        {
            matched = Some((
                ParsedMatch {
                    rec,
                    ref_a: ref_a.clone(),
                    alt_a: alt_a.clone(),
                },
                line.clone(),
            ));
            break;
        }
        if matched.is_none() {
            matched = Some((
                ParsedMatch {
                    rec: rec.clone(),
                    ref_a: rec.ref_allele.clone(),
                    alt_a: rec.alt_alleles.first().cloned().unwrap_or_default(),
                },
                line.clone(),
            ));
        }
    }

    let Some((parsed, line)) = matched else {
        return Err(GnomadLookupStatus::NoRecordAtPosition);
    };

    let status = if match_user_allele(
        &parsed.rec.ref_allele,
        &parsed.rec.alt_alleles,
        allele1,
        allele2,
    )
    .is_some()
    {
        GnomadLookupStatus::RemoteVcfHit
    } else {
        GnomadLookupStatus::AlleleMismatch
    };

    let user_match = if status == GnomadLookupStatus::RemoteVcfHit {
        Some("allele_match")
    } else {
        Some("allele_mismatch")
    };

    let (_ref_n, alt_n) = cache_alleles(&parsed.ref_a, &parsed.alt_a);
    let ctx = record_to_context(
        &parsed.rec,
        &alt_n,
        release,
        dataset,
        GnomadSourceMode::RemoteIndexedVcfHttps,
        vcf_url,
        status,
        user_match,
    );
    let raw_hash = hash_record(&line);
    Ok((ctx, raw_hash, parsed.ref_a, parsed.alt_a))
}

struct ParsedMatch {
    rec: super::vcf_parse::ParsedSiteRecord,
    ref_a: String,
    alt_a: String,
}

fn ensure_index_cached(
    cache_dir: &Path,
    vcf_url: &str,
    tbi_url: &str,
    csi_url: &str,
) -> Result<PathBuf, GnomadLookupStatus> {
    let tbi_path = super::readiness::index_cache_path(cache_dir, vcf_url);
    if tbi_path.exists() {
        return Ok(tbi_path);
    }
    if let Ok(bytes) = http_get_bytes(tbi_url)
        && write_bytes(&tbi_path, &bytes).is_ok()
    {
        return Ok(tbi_path);
    }
    let csi_path = cache_dir.join(format!("{}.csi", hash_record(vcf_url)));
    if let Ok(bytes) = http_get_bytes(csi_url)
        && write_bytes(&csi_path, &bytes).is_ok()
    {
        return Err(GnomadLookupStatus::IndexMissing);
    }
    Err(GnomadLookupStatus::IndexMissing)
}

fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    BLOCKING_HTTP_CLIENT
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

fn http_range_bytes(url: &str, start: u64, end: u64) -> Result<Vec<u8>, String> {
    BLOCKING_HTTP_CLIENT
        .get(url)
        .header("Range", format!("bytes={start}-{end}"))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    Ok(())
}

pub async fn head_url_ok(url: &str) -> bool {
    if crate::research::http::validate_research_outbound_url(url).is_err() {
        return false;
    }
    HTTP_CLIENT
        .head(url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
