// ./src-tauri/src/research/gnomad/lookup.rs
use super::cache::read_cache_for_variant;
use super::config::load_gnomad_config;
use super::graphql::fetch_graphql_context;
use super::manifest::{get_or_build_manifest, GnomadReleaseManifest};
use super::types::{
    GnomadConfig, GnomadContext, GnomadDatasetPolicy, GnomadLookupRequest, GnomadLookupStatus,
    GnomadSourceMode,
};
use super::vcf_local::query_local_vcf;
use super::vcf_remote::query_remote_vcf;
use crate::research::tuning::skip_gnomad_in_sweep;
use crate::research::util::normalize_rsid;
use rusqlite::params;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VariantCoords {
    pub chrom: String,
    pub pos: i64,
    pub allele1: String,
    pub allele2: String,
    pub genotype: String,
}

pub fn lookup_variant_coords(
    db_path: &Path,
    sample_id: i64,
    rsid: &str,
) -> Result<VariantCoords, GnomadLookupStatus> {
    let rsid_key = normalize_rsid(rsid).ok_or(GnomadLookupStatus::MissingCoordinate)?;
    let conn = crate::db::connect(db_path).map_err(|_| GnomadLookupStatus::MissingCoordinate)?;
    conn.query_row(
        "SELECT chromosome, position_grch38, allele1, allele2 FROM genotypes
         WHERE sample_id = ? AND LOWER(rsid) = LOWER(?) LIMIT 1",
        params![sample_id, rsid_key],
        |row| {
            let chrom: String = row.get(0)?;
            let pos: Option<i64> = row.get(1)?;
            let a1: String = row.get(2)?;
            let a2: String = row.get(3)?;
            Ok((chrom, pos, a1, a2))
        },
    )
    .map_err(|_| GnomadLookupStatus::MissingCoordinate)
    .and_then(|(chrom, pos, a1, a2)| {
        let pos = pos.filter(|p| *p > 0).ok_or(GnomadLookupStatus::MissingCoordinate)?;
        Ok(VariantCoords {
            chrom,
            pos,
            allele1: a1.clone(),
            allele2: a2.clone(),
            genotype: format!("{a1}/{a2}"),
        })
    })
}

pub fn gnomad_cache_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("gnomad_indexes")
}

pub fn is_priority_variant(db_path: &Path, sample_id: i64, rsid: &str) -> bool {
    let Some(rsid_key) = normalize_rsid(rsid) else {
        return false;
    };
    let Ok(conn) = crate::db::connect(db_path) else {
        return false;
    };
    let in_assoc: bool = conn
        .query_row(
            "SELECT 1 FROM association_facts WHERE sample_id = ? AND LOWER(rsid) = LOWER(?) LIMIT 1",
            params![sample_id, rsid_key],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if in_assoc {
        return true;
    }
    conn.query_row(
        "SELECT 1 FROM discovered_findings WHERE sample_id = ? AND LOWER(rsid) = LOWER(?) LIMIT 1",
        params![sample_id, rsid_key],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

pub async fn get_gnomad_context(
    db_path: &Path,
    data_dir: &Path,
    req: GnomadLookupRequest,
) -> GnomadContext {
    let (cfg, release, source_mode, force) = {
        let conn = match crate::db::connect(db_path) {
            Ok(c) => c,
            Err(_) => return GnomadContext::empty(GnomadLookupStatus::ParserError),
        };
        let cfg = match load_gnomad_config(&conn) {
            Ok(c) => c,
            Err(e) => {
                let mut ctx = GnomadContext::empty(GnomadLookupStatus::ParserError);
                ctx.warnings.push(e);
                return ctx;
            }
        };
        if !cfg.enabled {
            return GnomadContext::empty(GnomadLookupStatus::NotQueried);
        }
        let release = req.release.clone().unwrap_or(cfg.release.clone());
        let source_mode = req
            .source_mode
            .as_deref()
            .map(GnomadSourceMode::from_str_loose)
            .unwrap_or(cfg.source_mode);
        let force = req.force_refresh.unwrap_or(false);
        (cfg, release, source_mode, force)
    };

    let coords = if let (Some(ch), Some(pos)) = (req.chrom_grch38.clone(), req.pos_grch38) {
        let a1 = req
            .genotype
            .as_deref()
            .and_then(|g| g.split('/').next())
            .unwrap_or("N")
            .to_string();
        let a2 = req
            .genotype
            .as_deref()
            .and_then(|g| g.split('/').nth(1))
            .unwrap_or("N")
            .to_string();
        VariantCoords {
            chrom: ch,
            pos,
            allele1: a1,
            allele2: a2,
            genotype: req.genotype.unwrap_or_else(|| "N/N".into()),
        }
    } else {
        match lookup_variant_coords(db_path, req.sample_id, &req.rsid) {
            Ok(c) => c,
            Err(status) => return GnomadContext::empty(status),
        }
    };

    let ref_allele = req.reference_allele.clone();
    let alt_allele = req.alternate_allele.clone();

    if !force
        && let Ok(conn) = crate::db::connect(db_path)
            && let Some(mut cached) = read_cache_for_variant(
                &conn,
                &release,
                &coords.chrom,
                coords.pos,
                Some(&req.rsid),
                Some(&coords.allele1),
                Some(&coords.allele2),
                ref_allele.as_deref(),
                alt_allele.as_deref(),
            ) {
                cached.lookup_status = GnomadLookupStatus::CacheHit.as_str().to_string();
                return cached;
            }

    resolve_gnomad_context(
        db_path,
        data_dir,
        &cfg,
        &release,
        source_mode,
        &req.rsid,
        req.sample_id,
        &coords,
        ref_allele.as_deref(),
        alt_allele.as_deref(),
        force,
    )
    .await
}

fn datasets_for_policy(policy: GnomadDatasetPolicy) -> Vec<&'static str> {
    match policy {
        GnomadDatasetPolicy::Combined => vec!["exomes", "genomes"],
        GnomadDatasetPolicy::ExomesOnly => vec!["exomes"],
        GnomadDatasetPolicy::GenomesOnly => vec!["genomes"],
        GnomadDatasetPolicy::Auto => vec!["exomes", "genomes"],
    }
}

fn vcf_hit(ctx: &GnomadContext) -> bool {
    matches!(
        GnomadLookupStatus::from_str_loose(&ctx.lookup_status),
        GnomadLookupStatus::RemoteVcfHit
            | GnomadLookupStatus::LocalVcfHit
            | GnomadLookupStatus::CacheHit
    ) && ctx.best_af().is_some()
}

fn should_try_second_dataset(policy: GnomadDatasetPolicy, first: &GnomadContext) -> bool {
    policy == GnomadDatasetPolicy::Auto
        && matches!(
            GnomadLookupStatus::from_str_loose(&first.lookup_status),
            GnomadLookupStatus::NoRecordAtPosition | GnomadLookupStatus::AlleleMismatch
        )
}

fn should_graphql_fallback(cfg: &GnomadConfig, ctx: &GnomadContext, priority: bool) -> bool {
    if !cfg.graphql_fallback_enabled || !priority {
        return false;
    }
    matches!(
        GnomadLookupStatus::from_str_loose(&ctx.lookup_status),
        GnomadLookupStatus::NoRecordAtPosition
            | GnomadLookupStatus::AlleleMismatch
            | GnomadLookupStatus::NetworkError
            | GnomadLookupStatus::IndexMissing
    )
}

pub async fn resolve_gnomad_context(
    db_path: &Path,
    data_dir: &Path,
    cfg: &GnomadConfig,
    release: &str,
    source_mode: GnomadSourceMode,
    rsid: &str,
    sample_id: i64,
    coords: &VariantCoords,
    ref_allele: Option<&str>,
    alt_allele: Option<&str>,
    force: bool,
) -> GnomadContext {
    if !force
        && let Ok(conn) = crate::db::connect(db_path)
            && let Some(mut cached) = read_cache_for_variant(
                &conn,
                release,
                &coords.chrom,
                coords.pos,
                Some(rsid),
                Some(&coords.allele1),
                Some(&coords.allele2),
                ref_allele,
                alt_allele,
            ) {
                cached.lookup_status = GnomadLookupStatus::CacheHit.as_str().to_string();
                return cached;
            }

    let manifest = get_or_build_manifest(cfg, data_dir, false).await;
    if !manifest.is_supported_chrom(&coords.chrom) {
        return GnomadContext::empty(GnomadLookupStatus::UnsupportedContig);
    }

    let _permit = crate::research::tuning::acquire_gnomad_permit().await;

    let effective_mode = if source_mode == GnomadSourceMode::PythonToolboxSidecar {
        GnomadSourceMode::RemoteIndexedVcfHttps
    } else if cfg.graphql_enabled_for_sweep {
        GnomadSourceMode::GraphQlInteractive
    } else {
        source_mode
    };

    let mut ctx = match effective_mode {
        GnomadSourceMode::GraphQlInteractive => {
            fetch_graphql_context(
                db_path,
                rsid,
                release,
                Some(&coords.genotype),
                Some(&coords.chrom),
                Some(coords.pos),
                ref_allele,
                alt_allele,
            )
            .await
        }
        GnomadSourceMode::LocalIndexedVcf => {
            query_datasets_local(db_path, cfg, coords, rsid, release, &manifest).await
        }
        GnomadSourceMode::RemoteIndexedVcfHttps => {
            query_datasets_remote(db_path, data_dir, cfg, coords, rsid, release, &manifest).await
        }
        GnomadSourceMode::PythonToolboxSidecar => {
            GnomadContext::empty(GnomadLookupStatus::SourceUnavailable)
        }
    };

    let priority = is_priority_variant(db_path, sample_id, rsid);
    if should_graphql_fallback(cfg, &ctx, priority) {
        let gql = fetch_graphql_context(
            db_path,
            rsid,
            release,
            Some(&coords.genotype),
            Some(&coords.chrom),
            Some(coords.pos),
            ref_allele,
            alt_allele,
        )
        .await;
        if gql.best_af().is_some() {
            return gql;
        }
    }

    ctx.release = release.to_string();
    ctx
}

async fn query_datasets_remote(
    db_path: &Path,
    data_dir: &Path,
    cfg: &GnomadConfig,
    coords: &VariantCoords,
    rsid: &str,
    release: &str,
    manifest: &GnomadReleaseManifest,
) -> GnomadContext {
    let cache_dir = gnomad_cache_dir(data_dir);
    let datasets = datasets_for_policy(cfg.dataset_policy);
    let mut merged = GnomadContext::empty(GnomadLookupStatus::NoRecordAtPosition);

    for (i, dataset) in datasets.iter().enumerate() {
        if !manifest.contig_available(&coords.chrom, dataset) {
            continue;
        }
        let ctx = query_remote_vcf(
            db_path,
            cfg,
            &cache_dir,
            &coords.chrom,
            coords.pos,
            &coords.allele1,
            &coords.allele2,
            Some(&coords.genotype),
            dataset,
            rsid,
        )
        .await;
        merged = if i == 0 {
            ctx
        } else {
            merge_exome_genome(merged, ctx, release)
        };
        if cfg.dataset_policy == GnomadDatasetPolicy::Auto && vcf_hit(&merged) {
            break;
        }
        if i == 0 && !should_try_second_dataset(cfg.dataset_policy, &merged) {
            break;
        }
    }
    merged
}

async fn query_datasets_local(
    db_path: &Path,
    cfg: &GnomadConfig,
    coords: &VariantCoords,
    rsid: &str,
    release: &str,
    manifest: &GnomadReleaseManifest,
) -> GnomadContext {
    let datasets = datasets_for_policy(cfg.dataset_policy);
    let mut merged = GnomadContext::empty(GnomadLookupStatus::NoRecordAtPosition);

    for (i, dataset) in datasets.iter().enumerate() {
        if !manifest.contig_available(&coords.chrom, dataset) {
            continue;
        }
        let ctx = query_local_vcf(
            db_path,
            cfg,
            &coords.chrom,
            coords.pos,
            &coords.allele1,
            &coords.allele2,
            Some(&coords.genotype),
            dataset,
            rsid,
        )
        .await;
        merged = if i == 0 {
            ctx
        } else {
            merge_exome_genome(merged, ctx, release)
        };
        if cfg.dataset_policy == GnomadDatasetPolicy::Auto && vcf_hit(&merged) {
            break;
        }
        if i == 0 && !should_try_second_dataset(cfg.dataset_policy, &merged) {
            break;
        }
    }
    merged
}

fn merge_exome_genome(mut exome: GnomadContext, genome: GnomadContext, release: &str) -> GnomadContext {
    if exome.lookup_status == GnomadLookupStatus::NoRecordAtPosition.as_str()
        && genome.lookup_status != GnomadLookupStatus::NoRecordAtPosition.as_str()
    {
        exome = genome;
    } else if exome.lookup_status != GnomadLookupStatus::NoRecordAtPosition.as_str()
        && genome.lookup_status != GnomadLookupStatus::NoRecordAtPosition.as_str()
    {
        exome.ac_genomes = genome.ac_genomes;
        exome.an_genomes = genome.an_genomes;
        exome.af_genomes = genome.af_genomes;
        exome.dataset = "combined".into();
        exome.af = exome.best_af();
    }
    exome.release = release.to_string();
    exome
}

pub async fn fetch_gnomad_for_enrichment(
    db_path: &Path,
    data_dir: &Path,
    sample_id: i64,
    rsid: &str,
    allele1: &str,
    allele2: &str,
) -> (Option<f64>, GnomadContext) {
    if skip_gnomad_in_sweep() {
        return (None, GnomadContext::empty(GnomadLookupStatus::NotQueried));
    }

    let cfg = {
        let conn = match crate::db::connect(db_path) {
            Ok(c) => c,
            Err(_) => return (None, GnomadContext::empty(GnomadLookupStatus::ParserError)),
        };
        match load_gnomad_config(&conn) {
            Ok(c) => c,
            Err(_) => return (None, GnomadContext::empty(GnomadLookupStatus::ParserError)),
        }
    };

    if !cfg.enabled {
        return (None, GnomadContext::empty(GnomadLookupStatus::NotQueried));
    }

    let mode = if cfg.graphql_enabled_for_sweep {
        GnomadSourceMode::GraphQlInteractive
    } else {
        cfg.source_mode
    };

    let coords = match lookup_variant_coords(db_path, sample_id, rsid) {
        Ok(mut c) => {
            if c.allele1 == "0" || c.allele1.is_empty() {
                c.allele1 = allele1.to_string();
            }
            if c.allele2 == "0" || c.allele2.is_empty() {
                c.allele2 = allele2.to_string();
            }
            c.genotype = format!("{}/{}", c.allele1, c.allele2);
            c
        }
        Err(status) => return (None, GnomadContext::empty(status)),
    };

    let ctx = resolve_gnomad_context(
        db_path,
        data_dir,
        &cfg,
        &cfg.release,
        mode,
        rsid,
        sample_id,
        &coords,
        None,
        None,
        false,
    )
    .await;

    (ctx.best_af(), ctx)
}

pub fn apply_gnomad_to_payload(payload: &mut serde_json::Map<String, serde_json::Value>, ctx: &GnomadContext) {
    payload.insert("has_gnomad".into(), serde_json::json!(ctx.best_af().is_some()));
    payload.insert("gnomad_release".into(), serde_json::json!(ctx.release));
    payload.insert("gnomad_dataset".into(), serde_json::json!(ctx.dataset));
    payload.insert("gnomad_lookup_status".into(), serde_json::json!(ctx.lookup_status));
    payload.insert("gnomad_source_mode".into(), serde_json::json!(ctx.source_mode));
    payload.insert("gnomad_last_checked".into(), serde_json::json!(ctx.fetched_at));
    if let Some(af) = ctx.af.or(ctx.best_af()) {
        payload.insert("gnomad_af".into(), serde_json::json!(af));
    }
    if let Some(ac) = ctx.ac {
        payload.insert("gnomad_ac".into(), serde_json::json!(ac));
    }
    if let Some(an) = ctx.an {
        payload.insert("gnomad_an".into(), serde_json::json!(an));
    }
    if let Some(v) = ctx.popmax {
        payload.insert("gnomad_popmax".into(), serde_json::json!(v));
    }
    if let Some(v) = &ctx.popmax_population {
        payload.insert("gnomad_popmax_population".into(), serde_json::json!(v));
    }
    if let Some(v) = ctx.faf95_popmax {
        payload.insert("gnomad_faf95_popmax".into(), serde_json::json!(v));
    }
    if let Some(v) = ctx.homozygote_count {
        payload.insert("gnomad_homozygote_count".into(), serde_json::json!(v));
    }
    if let Some(v) = ctx.hemizygote_count {
        payload.insert("gnomad_hemizygote_count".into(), serde_json::json!(v));
    }
    if !ctx.filters.is_empty() {
        payload.insert("gnomad_filters".into(), serde_json::json!(ctx.filters));
    }
}
