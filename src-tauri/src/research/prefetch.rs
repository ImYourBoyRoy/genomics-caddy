// ./src-tauri/src/research/prefetch.rs
//! Warm SQLite API caches before parallel per-variant prepare (mirrors gnomAD batch prefetch).

use super::enrich::fetch_pubmed_abstracts;
use super::evidence::ncbi_context::{fetch_clinvar_live, fetch_dbsnp_context};
use super::sources_config::{
    enrichment_clinvar_live_enabled, enrichment_fast_index_mode, enrichment_gtex_enabled,
    enrichment_pubmed_enabled, enrichment_vep_dbsnp_enabled,
};
use super::types::ScoredMarker;
use super::sources::{
    fetch_ensembl_vep_genes, fetch_gtex_eqtls_for_rsid, fetch_gtex_gencode_id_cached,
    fetch_gwas_associations, gwas_needs_api_supplement, load_local_gwas_associations,
};
use super::sweep_metrics::{bump_batch_prefetch, set_batch_current_rsid, timed_async, SweepPhase};
use super::tuning::{
    prefetch_concurrency, prefetch_batch_timeout_secs, prepare_variant_timeout_secs,
};
use std::time::Duration;
use super::types::QdrantConfig;
use super::util::{is_placeholder_gene, lookup_variant_locus, resolve_gene_name};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub async fn prefetch_enrichment_batch(
    db_path: &Path,
    sample_id: i64,
    batch: &[&ScoredMarker],
    config: &QdrantConfig,
) {
    if enrichment_fast_index_mode() || batch.is_empty() {
        return;
    }

    let deadline = Duration::from_secs(prefetch_batch_timeout_secs(batch.len()));
    match tokio::time::timeout(
        deadline,
        prefetch_enrichment_batch_inner(db_path, sample_id, batch, config),
    )
    .await
    {
        Ok(()) => {}
        Err(_) => {
            eprintln!(
                "[prefetch] API prefetch timed out after {}s ({} variants) — continuing to prepare",
                deadline.as_secs(),
                batch.len()
            );
        }
    }
}

/// Spawn prefetch tasks for one marker. Semaphore acquire runs inside each task so markers can prefetch in parallel.
fn spawn_marker_prefetch_tasks(
    join_set: &mut JoinSet<()>,
    sem: &Arc<Semaphore>,
    db_path: &Path,
    sample_id: i64,
    marker: &ScoredMarker,
    ncbi_key: Option<&str>,
) {
    let db = db_path.to_path_buf();
    let rsid = marker.rsid.clone();
    let gene = marker.gene.clone();
    let clinvar_sig = marker.clinvar_sig.clone();
    let ncbi_key = ncbi_key.map(str::to_string);

    if gwas_needs_api_supplement(db_path, &rsid) {
        let db = db.clone();
        let rsid = rsid.clone();
        join_set.spawn(async move {
            let _ = timed_async(
                SweepPhase::Gwas,
                fetch_gwas_associations(&db, &rsid, false),
            )
            .await;
        });
    }

    let has_local_gwas = load_local_gwas_associations(&db, &rsid)
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let needs_pubmed = clinvar_sig
        .as_deref()
        .filter(|s| !s.is_empty())
        .is_some()
        || has_local_gwas;

    if enrichment_clinvar_live_enabled()
        && clinvar_sig.as_deref().filter(|s| !s.is_empty()).is_none()
    {
        let skip_live_clinvar = crate::db::connect(db_path)
            .ok()
            .map(|c| crate::offline::offline_clinvar_available(&c))
            .unwrap_or(false);
        if !skip_live_clinvar {
        let sem = sem.clone();
        let db = db.clone();
        let rsid = rsid.clone();
        let key = ncbi_key.clone();
        join_set.spawn(async move {
            let permit = match tokio::time::timeout(Duration::from_secs(60), sem.acquire_owned())
                .await
            {
                Ok(Ok(p)) => p,
                _ => return,
            };
            let _permit = permit;
            let _ = tokio::time::timeout(
                Duration::from_secs(prepare_variant_timeout_secs()),
                timed_async(
                    SweepPhase::Clinvar,
                    fetch_clinvar_live(&db, &rsid, key.as_deref()),
                ),
            )
            .await;
        });
        }
    }

    if enrichment_pubmed_enabled() && needs_pubmed {
        let sem = sem.clone();
        let db = db.clone();
        let rsid = rsid.clone();
        let key = ncbi_key.clone();
        join_set.spawn(async move {
            let permit = match tokio::time::timeout(Duration::from_secs(60), sem.acquire_owned())
                .await
            {
                Ok(Ok(p)) => p,
                _ => return,
            };
            let _permit = permit;
            let _ = tokio::time::timeout(
                Duration::from_secs(prepare_variant_timeout_secs()),
                timed_async(
                    SweepPhase::Pubmed,
                    fetch_pubmed_abstracts(&db, &rsid, key.as_deref()),
                ),
            )
            .await;
        });
    }

    if enrichment_gtex_enabled()
        && let Some(g) = gene.as_deref().filter(|g| !is_placeholder_gene(g)) {
            let sem = sem.clone();
            let db = db.clone();
            let rsid = rsid.clone();
            let gene = g.to_string();
            join_set.spawn(async move {
                let permit = match tokio::time::timeout(Duration::from_secs(60), sem.acquire_owned())
                    .await
                {
                    Ok(Ok(p)) => p,
                    _ => return,
                };
                let _permit = permit;
                let _ = tokio::time::timeout(
                    Duration::from_secs(prepare_variant_timeout_secs()),
                    timed_async(
                        SweepPhase::Gtex,
                        fetch_gtex_eqtls_for_rsid(&db, &rsid, Some(&gene)),
                    ),
                )
                .await;
            });
        }

    let (resolved_gene, _) = resolve_gene_name(&db, &rsid, gene.as_deref(), &[]);
    let needs_vep = resolved_gene.is_none()
        || gene.as_deref().map(is_placeholder_gene).unwrap_or(true);
    if enrichment_vep_dbsnp_enabled() && needs_vep {
        let sem = sem.clone();
        let db = db.clone();
        let rsid = rsid.clone();
        join_set.spawn(async move {
            let permit = match tokio::time::timeout(Duration::from_secs(60), sem.acquire_owned())
                .await
            {
                Ok(Ok(p)) => p,
                _ => return,
            };
            let _permit = permit;
            let _ = tokio::time::timeout(
                Duration::from_secs(prepare_variant_timeout_secs()),
                timed_async(SweepPhase::Vep, fetch_ensembl_vep_genes(&db, &rsid)),
            )
            .await;
        });
    }

    let locus = lookup_variant_locus(&db, sample_id, &rsid);
    if enrichment_vep_dbsnp_enabled() && locus.is_none() {
        let sem = sem.clone();
        let db = db.clone();
        let rsid = rsid.clone();
        join_set.spawn(async move {
            let permit = match tokio::time::timeout(Duration::from_secs(60), sem.acquire_owned())
                .await
            {
                Ok(Ok(p)) => p,
                _ => return,
            };
            let _permit = permit;
            let _ = tokio::time::timeout(
                Duration::from_secs(prepare_variant_timeout_secs()),
                timed_async(SweepPhase::Vep, fetch_dbsnp_context(&db, &rsid)),
            )
            .await;
        });
    }
}

async fn prefetch_enrichment_batch_inner(
    db_path: &Path,
    sample_id: i64,
    batch: &[&ScoredMarker],
    config: &QdrantConfig,
) {
    let ncbi_key = config.ncbi_api_key.as_deref();
    let sem = Arc::new(Semaphore::new(prefetch_concurrency()));
    let db_path = db_path.to_path_buf();

    // Prefetch all markers in the batch concurrently (bounded by prefetch_concurrency per API task).
    let mut join_set = JoinSet::new();
    for marker in batch {
        let sem = sem.clone();
        let db_path = db_path.clone();
        let marker = (*marker).clone();
        let ncbi_key = ncbi_key.map(str::to_string);
        join_set.spawn(async move {
            let mut inner = JoinSet::new();
            spawn_marker_prefetch_tasks(
                &mut inner,
                &sem,
                &db_path,
                sample_id,
                &marker,
                ncbi_key.as_deref(),
            );
            while inner.join_next().await.is_some() {}
            set_batch_current_rsid(&marker.rsid);
            bump_batch_prefetch();
            super::debug_log::log("prefetch", format!("marker done {}", marker.rsid));
        });
    }
    while join_set.join_next().await.is_some() {}

    let mut gtex_genes: HashSet<String> = HashSet::new();
    for marker in batch {
        if marker
            .gene
            .as_deref()
            .filter(|g| !is_placeholder_gene(g))
            .is_some()
        {
            continue;
        }
        let (resolved, _) =
            resolve_gene_name(&db_path, &marker.rsid, marker.gene.as_deref(), &[]);
        if let Some(g) = resolved.filter(|g| !is_placeholder_gene(g)) {
            gtex_genes.insert(g);
        }
    }

    if enrichment_gtex_enabled() && !gtex_genes.is_empty() {
        let mut join_set = JoinSet::new();
        for gene in gtex_genes {
            let sem = sem.clone();
            let db = db_path.clone();
            join_set.spawn(async move {
                let permit = match tokio::time::timeout(
                    Duration::from_secs(60),
                    sem.acquire_owned(),
                )
                .await
                {
                    Ok(Ok(p)) => p,
                    _ => return,
                };
                let _permit = permit;
                let _ = tokio::time::timeout(
                    Duration::from_secs(prepare_variant_timeout_secs()),
                    timed_async(
                        SweepPhase::Gtex,
                        fetch_gtex_gencode_id_cached(&db, &gene),
                    ),
                )
                .await;
            });
        }
        while join_set.join_next().await.is_some() {}
    }
}
