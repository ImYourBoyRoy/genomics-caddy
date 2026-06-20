// ./src-tauri/src/research/gnomad/vcf_local.rs
use super::cache::{hash_record, write_cache, CacheWriteInput};
use super::allele_norm::cache_alleles;
use super::config::{tabix_reference_name};
use super::types::{GnomadConfig, GnomadContext, GnomadLookupStatus, GnomadSourceMode};
use super::vcf_parse::{match_user_allele, parse_site_line, record_to_context};
use crate::research::tuning::gnomad_remote_timeout_secs;
use noodles_core::Region;
use noodles_tabix as tabix;
use std::path::Path;
use std::str::FromStr;

pub async fn query_local_vcf(
    db_path: &Path,
    cfg: &GnomadConfig,
    chrom: &str,
    pos: i64,
    allele1: &str,
    allele2: &str,
    genotype: Option<&str>,
    dataset: &str,
    rsid: &str,
) -> GnomadContext {
    let Some(dir) = cfg.local_vcf_dir.as_ref() else {
        return GnomadContext::empty(GnomadLookupStatus::SourceUnavailable);
    };
    let template = if dataset == "exomes" {
        &cfg.exome_template
    } else {
        &cfg.genome_template
    };
    let vcf_path = match super::validate::resolve_local_vcf_path(dir, template, chrom) {
        Ok(path) => path,
        Err(_err) => {
            return GnomadContext::empty(GnomadLookupStatus::SourceUnavailable);
        }
    };

    let timeout = std::time::Duration::from_secs(gnomad_remote_timeout_secs());
    let result = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking({
        let vcf_path = vcf_path.clone();
        let release = cfg.release.clone();
        let allele1 = allele1.to_string();
        let allele2 = allele2.to_string();
        let dataset = dataset.to_string();
        let chrom = chrom.to_string();
        move || query_local_sync(&vcf_path, &chrom, pos, &allele1, &allele2, &release, &dataset)
    }),
    )
    .await;

    match result {
        Ok(Ok(Ok((ctx, raw_hash, ref_a, alt_a, source_file)))) => {
            if let Ok(conn) = crate::db::connect(db_path) {
                let (ref_n, alt_n) = cache_alleles(&ref_a, &alt_a);
                let _ = write_cache(
                    &conn,
                    CacheWriteInput {
                        release: ctx.release.clone(),
                        source_mode: GnomadSourceMode::LocalIndexedVcf,
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
                        source_url: None,
                        source_file: Some(source_file),
                        source_index: None,
                        raw_record_hash: raw_hash,
                        lookup_status: GnomadLookupStatus::from_str_loose(&ctx.lookup_status),
                    },
                );
            }
            ctx
        }
        Ok(Ok(Err(status))) => GnomadContext::empty(status),
        Ok(Err(_)) | Err(_) => GnomadContext::empty(GnomadLookupStatus::NetworkError),
    }
}

fn query_local_sync(
    vcf_path: &Path,
    chrom: &str,
    pos: i64,
    allele1: &str,
    allele2: &str,
    release: &str,
    dataset: &str,
) -> Result<(GnomadContext, String, String, String, String), GnomadLookupStatus> {
    if !vcf_path.exists() {
        return Err(GnomadLookupStatus::SourceUnavailable);
    }

    let mut reader = tabix::io::indexed_reader::Builder::default()
        .build_from_path(vcf_path)
        .map_err(|_| GnomadLookupStatus::SourceUnavailable)?;

    let ref_name = tabix_reference_name(chrom);
    let region = Region::from_str(&format!("{ref_name}:{pos}-{pos}"))
        .map_err(|_| GnomadLookupStatus::ParserError)?;

    let query = reader
        .query(&region)
        .map_err(|_| GnomadLookupStatus::ParserError)?;

    let source = vcf_path.display().to_string();
    let mut fallback: Option<(super::vcf_parse::ParsedSiteRecord, String)> = None;

    for record in query {
        let record = record.map_err(|_| GnomadLookupStatus::ParserError)?;
        let line = record.as_ref().to_string();
        let Some(rec) = parse_site_line(&line) else {
            continue;
        };
        if rec.pos != pos {
            continue;
        }
        if let Some((ref_a, alt_a)) =
            match_user_allele(&rec.ref_allele, &rec.alt_alleles, allele1, allele2)
        {
            let ctx = record_to_context(
                &rec,
                &alt_a,
                release,
                dataset,
                GnomadSourceMode::LocalIndexedVcf,
                &source,
                GnomadLookupStatus::LocalVcfHit,
                Some("allele_match"),
            );
            let raw_hash = hash_record(&line);
            return Ok((ctx, raw_hash, ref_a, alt_a, source));
        }
        if fallback.is_none() {
            fallback = Some((rec, line));
        }
    }

    if let Some((rec, line)) = fallback {
        let alt_a = rec.alt_alleles.first().cloned().unwrap_or_default();
        let ctx = record_to_context(
            &rec,
            &alt_a,
            release,
            dataset,
            GnomadSourceMode::LocalIndexedVcf,
            &source,
            GnomadLookupStatus::AlleleMismatch,
            Some("allele_mismatch"),
        );
        let raw_hash = hash_record(&line);
        return Ok((ctx, raw_hash, rec.ref_allele.clone(), alt_a, source));
    }

    Err(GnomadLookupStatus::NoRecordAtPosition)
}
