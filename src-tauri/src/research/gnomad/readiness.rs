// ./src-tauri/src/research/gnomad/readiness.rs
use super::cache::{frequency_cache_status, hash_record};
use super::config::{index_urls, vcf_url};
use super::lookup::gnomad_cache_dir;
use super::manifest::{GnomadReleaseManifest, get_or_build_manifest};
use super::types::{
    GnomadConfig, GnomadHttpsProvider, GnomadIndexSyncResult, GnomadMissingItem,
    GnomadReadinessStatus, GnomadSourceMode,
};
use crate::research::http::HTTP_CLIENT;
use std::path::{Path, PathBuf};

const GNOMAD_DOWNLOADS_URL: &str = "https://gnomad.broadinstitute.org/downloads";

pub fn index_cache_path(cache_dir: &Path, vcf_url: &str) -> PathBuf {
    let key = hash_record(vcf_url);
    cache_dir.join(format!("{key}.tbi"))
}

fn manifest_fields(manifest: &GnomadReleaseManifest) -> (Vec<String>, Vec<String>, Vec<String>) {
    (
        manifest.exome_contigs.clone(),
        manifest.genome_contigs.clone(),
        manifest.unsupported_contigs.clone(),
    )
}

fn base_status(
    cfg: &GnomadConfig,
    data_dir: &Path,
    manifest: &GnomadReleaseManifest,
    db_path: Option<&Path>,
) -> GnomadReadinessStatus {
    let (exome, genome, unsupported) = manifest_fields(manifest);
    let cache = db_path
        .map(|path| frequency_cache_status(path, &cfg.release))
        .unwrap_or_default();
    GnomadReadinessStatus {
        ready: true,
        enabled: cfg.enabled,
        source_mode: cfg.source_mode.as_str().to_string(),
        release: cfg.release.clone(),
        provider: provider_label(cfg.provider),
        remote_urls_ok: true,
        indexes_expected: 0,
        indexes_cached: 0,
        frequency_cache_rows: cache.current_rows,
        stale_frequency_cache_rows: cache.stale_rows,
        frequency_cache_ready: cache.current_rows > 0,
        frequency_cache_summary: frequency_cache_summary(&cache, &cfg.release),
        cache_dir: gnomad_cache_dir(data_dir).to_string_lossy().to_string(),
        local_dir: cfg.local_vcf_dir.clone(),
        missing_items: Vec::new(),
        summary: String::new(),
        primary_action: None,
        downloads_page_url: GNOMAD_DOWNLOADS_URL.into(),
        manifest_exome_contigs: exome,
        manifest_genome_contigs: genome,
        unsupported_contigs: unsupported,
    }
}

pub async fn get_gnomad_readiness(
    cfg: &GnomadConfig,
    data_dir: &Path,
    db_path: &Path,
) -> GnomadReadinessStatus {
    let manifest = get_or_build_manifest(cfg, data_dir, false).await;

    if !cfg.enabled {
        let mut s = base_status(cfg, data_dir, &manifest, Some(db_path));
        s.ready = true;
        s.enabled = false;
        s.summary = "gnomAD enrichment is disabled.".into();
        return s;
    }

    let effective_mode = if cfg.source_mode == GnomadSourceMode::PythonToolboxSidecar {
        GnomadSourceMode::RemoteIndexedVcfHttps
    } else {
        cfg.source_mode
    };

    match effective_mode {
        GnomadSourceMode::GraphQlInteractive => {
            let mut s = base_status(cfg, data_dir, &manifest, Some(db_path));
            s.summary = "GraphQL mode — no index downloads. Best for single variants; sweeps should use Remote VCF.".into();
            s
        }
        GnomadSourceMode::LocalIndexedVcf => local_readiness(cfg, &manifest, db_path).await,
        GnomadSourceMode::RemoteIndexedVcfHttps => remote_readiness(cfg, data_dir, &manifest, Some(db_path)).await,
        GnomadSourceMode::PythonToolboxSidecar => remote_readiness(cfg, data_dir, &manifest, Some(db_path)).await,
    }
}

async fn remote_readiness(
    cfg: &GnomadConfig,
    data_dir: &Path,
    manifest: &GnomadReleaseManifest,
    db_path: Option<&Path>,
) -> GnomadReadinessStatus {
    let cache_dir = gnomad_cache_dir(data_dir);
    let smoke = super::validate::test_gnomad_source_urls(cfg).await;
    let mut missing = Vec::new();

    if !smoke.smoke_test_ok {
        missing.push(GnomadMissingItem {
            id: "remote_urls".into(),
            kind: "url_unreachable".into(),
            chrom: Some("22".into()),
            dataset: None,
            label: "Public gnomAD URLs are not reachable with current release/provider.".into(),
            url: Some(smoke.exome_vcf_url.clone()),
            fix_action: "switch_provider".into(),
        });
        for err in smoke.errors {
            missing.push(GnomadMissingItem {
                id: format!("url_{}", hash_record(&err)),
                kind: "url_unreachable".into(),
                chrom: None,
                dataset: None,
                label: err,
                url: None,
                fix_action: "test_urls".into(),
            });
        }
    }

    let expected = manifest.expected_index_count();
    let mut cached = 0u32;

    if expected == 0 {
        missing.push(GnomadMissingItem {
            id: "manifest_empty".into(),
            kind: "manifest_unavailable".into(),
            chrom: None,
            dataset: None,
            label: "No usable gnomAD contigs were discovered for this release/provider.".into(),
            url: Some(GNOMAD_DOWNLOADS_URL.into()),
            fix_action: "refresh_manifest".into(),
        });
    }

    for chrom in &manifest.exome_contigs {
        let vcf = vcf_url(cfg, &cfg.exome_template, chrom);
        let path = index_cache_path(&cache_dir, &vcf);
        if path.exists() {
            cached += 1;
            continue;
        }
        let (tbi_url, _) = index_urls(&vcf);
        missing.push(GnomadMissingItem {
            id: format!("idx_exomes_{chrom}"),
            kind: "tabix_index".into(),
            chrom: Some(chrom.clone()),
            dataset: Some("exomes".into()),
            label: format!("Tabix index missing for exomes chr{chrom}"),
            url: Some(tbi_url),
            fix_action: "download_index".into(),
        });
    }

    for chrom in &manifest.genome_contigs {
        let vcf = vcf_url(cfg, &cfg.genome_template, chrom);
        let path = index_cache_path(&cache_dir, &vcf);
        if path.exists() {
            cached += 1;
            continue;
        }
        let (tbi_url, _) = index_urls(&vcf);
        missing.push(GnomadMissingItem {
            id: format!("idx_genomes_{chrom}"),
            kind: "tabix_index".into(),
            chrom: Some(chrom.clone()),
            dataset: Some("genomes".into()),
            label: format!("Tabix index missing for genomes chr{chrom}"),
            url: Some(tbi_url),
            fix_action: "download_index".into(),
        });
    }

    let index_missing = cached < expected;
    let ready = smoke.smoke_test_ok && expected > 0 && !index_missing;
    let missing_count = expected.saturating_sub(cached);
    let unsupported_note = if manifest.unsupported_contigs.is_empty() {
        String::new()
    } else {
        format!(
            " Unsupported contigs (no public site VCF): {}.",
            manifest.unsupported_contigs.join(", ")
        )
    };

    let cache = db_path
        .map(|path| frequency_cache_status(path, &cfg.release))
        .unwrap_or_default();
    let cache_summary = frequency_cache_summary(&cache, &cfg.release);
    let summary = if !smoke.smoke_test_ok {
        "Fix URL/provider settings before enrichment.".into()
    } else if expected == 0 {
        "No usable gnomAD contigs were discovered. Refresh the release manifest before enrichment.".into()
    } else if index_missing {
        format!(
            "{cached}/{expected} tabix indexes cached (manifest-driven). Download {missing_count} small index files.{unsupported_note}"
        )
    } else {
        format!(
            "Ready — {cached} tabix indexes match release manifest. {cache_summary}{unsupported_note}"
        )
    };

    let (exome, genome, unsupported) = manifest_fields(manifest);

    GnomadReadinessStatus {
        ready,
        enabled: true,
        source_mode: cfg.source_mode.as_str().to_string(),
        release: cfg.release.clone(),
        provider: provider_label(cfg.provider),
        remote_urls_ok: smoke.smoke_test_ok,
        indexes_expected: expected,
        indexes_cached: cached,
        frequency_cache_rows: cache.current_rows,
        stale_frequency_cache_rows: cache.stale_rows,
        frequency_cache_ready: cache.current_rows > 0,
        frequency_cache_summary: cache_summary,
        cache_dir: cache_dir.to_string_lossy().to_string(),
        local_dir: cfg.local_vcf_dir.clone(),
        missing_items: missing,
        summary,
        primary_action: if expected == 0 && smoke.smoke_test_ok {
            Some("Refresh gnomAD manifest".into())
        } else if index_missing && smoke.smoke_test_ok {
            Some(format!("Download {missing_count} tabix indexes"))
        } else if !smoke.smoke_test_ok {
            Some("Test source URLs".into())
        } else {
            None
        },
        downloads_page_url: GNOMAD_DOWNLOADS_URL.into(),
        manifest_exome_contigs: exome,
        manifest_genome_contigs: genome,
        unsupported_contigs: unsupported,
    }
}

async fn local_readiness(
    cfg: &GnomadConfig,
    manifest: &GnomadReleaseManifest,
    db_path: &Path,
) -> GnomadReadinessStatus {
    let Some(dir) = cfg.local_vcf_dir.as_ref().filter(|d| !d.trim().is_empty()) else {
        let (exome, genome, unsupported) = manifest_fields(manifest);
        let cache = frequency_cache_status(db_path, &cfg.release);
        return GnomadReadinessStatus {
            ready: false,
            enabled: true,
            source_mode: cfg.source_mode.as_str().to_string(),
            release: cfg.release.clone(),
            provider: provider_label(cfg.provider),
            remote_urls_ok: true,
            indexes_expected: 0,
            indexes_cached: 0,
            frequency_cache_rows: cache.current_rows,
            stale_frequency_cache_rows: cache.stale_rows,
            frequency_cache_ready: cache.current_rows > 0,
            frequency_cache_summary: frequency_cache_summary(&cache, &cfg.release),
            cache_dir: String::new(),
            local_dir: None,
            missing_items: vec![GnomadMissingItem {
                id: "local_dir".into(),
                kind: "local_dir".into(),
                chrom: None,
                dataset: None,
                label: "Choose a folder with gnomAD .vcf.bgz and .tbi files.".into(),
                url: None,
                fix_action: "pick_folder".into(),
            }],
            summary: "Local mode needs a folder path.".into(),
            primary_action: Some("Choose local folder".into()),
            downloads_page_url: GNOMAD_DOWNLOADS_URL.into(),
            manifest_exome_contigs: exome,
            manifest_genome_contigs: genome,
            unsupported_contigs: unsupported,
        };
    };

    let base = PathBuf::from(dir);
    let mut missing = Vec::new();
    let mut present = 0u32;
    let expected = manifest.expected_index_count();

    for chrom in &manifest.exome_contigs {
        let file_name = cfg.exome_template.replace("{chrom}", chrom);
        let vcf_path = base.join(&file_name);
        let tbi_path = PathBuf::from(format!("{}.tbi", vcf_path.display()));
        if vcf_path.exists() && tbi_path.exists() {
            present += 1;
        } else {
            missing.push(GnomadMissingItem {
                id: format!("local_exomes_{chrom}"),
                kind: "local_file".into(),
                chrom: Some(chrom.clone()),
                dataset: Some("exomes".into()),
                label: format!("Missing local exomes chr{chrom} files"),
                url: Some(GNOMAD_DOWNLOADS_URL.into()),
                fix_action: "open_downloads".into(),
            });
        }
    }

    for chrom in &manifest.genome_contigs {
        let file_name = cfg.genome_template.replace("{chrom}", chrom);
        let vcf_path = base.join(&file_name);
        let tbi_path = PathBuf::from(format!("{}.tbi", vcf_path.display()));
        if vcf_path.exists() && tbi_path.exists() {
            present += 1;
        } else {
            missing.push(GnomadMissingItem {
                id: format!("local_genomes_{chrom}"),
                kind: "local_file".into(),
                chrom: Some(chrom.clone()),
                dataset: Some("genomes".into()),
                label: format!("Missing local genomes chr{chrom} files"),
                url: Some(GNOMAD_DOWNLOADS_URL.into()),
                fix_action: "open_downloads".into(),
            });
        }
    }

    let ready = missing.is_empty();
    let (exome, genome, unsupported) = manifest_fields(manifest);
    let cache = frequency_cache_status(db_path, &cfg.release);
    let cache_summary = frequency_cache_summary(&cache, &cfg.release);
    let summary = if ready {
        format!("Ready — {present} local files match release manifest. {cache_summary}")
    } else {
        format!("{present}/{expected} local files present per manifest.")
    };

    GnomadReadinessStatus {
        ready,
        enabled: true,
        source_mode: cfg.source_mode.as_str().to_string(),
        release: cfg.release.clone(),
        provider: provider_label(cfg.provider),
        remote_urls_ok: true,
        indexes_expected: expected,
        indexes_cached: present,
        frequency_cache_rows: cache.current_rows,
        stale_frequency_cache_rows: cache.stale_rows,
        frequency_cache_ready: cache.current_rows > 0,
        frequency_cache_summary: cache_summary,
        cache_dir: String::new(),
        local_dir: Some(dir.clone()),
        missing_items: missing,
        summary,
        primary_action: if ready {
            None
        } else {
            Some("Open gnomAD downloads".into())
        },
        downloads_page_url: GNOMAD_DOWNLOADS_URL.into(),
        manifest_exome_contigs: exome,
        manifest_genome_contigs: genome,
        unsupported_contigs: unsupported,
    }
}

pub async fn download_missing_gnomad_indexes(
    cfg: &GnomadConfig,
    data_dir: &Path,
) -> GnomadIndexSyncResult {
    if cfg.source_mode != GnomadSourceMode::RemoteIndexedVcfHttps
        && cfg.source_mode != GnomadSourceMode::PythonToolboxSidecar
    {
        return GnomadIndexSyncResult {
            downloaded: 0,
            skipped: 0,
            failed: 0,
            message: "Index download applies to Remote indexed VCF mode only.".into(),
            errors: Vec::new(),
        };
    }

    let manifest = get_or_build_manifest(cfg, data_dir, true).await;
    let cache_dir = gnomad_cache_dir(data_dir);
    std::fs::create_dir_all(&cache_dir).ok();

    let readiness = remote_readiness(cfg, data_dir, &manifest, None).await;
    if !readiness.remote_urls_ok {
        return GnomadIndexSyncResult {
            downloaded: 0,
            skipped: 0,
            failed: readiness.missing_items.len() as u32,
            message: "Cannot download indexes until gnomAD URLs are reachable.".into(),
            errors: readiness
                .missing_items
                .iter()
                .filter(|m| m.kind == "url_unreachable")
                .map(|m| m.label.clone())
                .collect(),
        };
    }

    let mut downloaded = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();
    let concurrency = cfg.max_remote_concurrent_files.max(1);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

    let mut handles = Vec::new();

    for chrom in &manifest.exome_contigs {
        let vcf = vcf_url(cfg, &cfg.exome_template, chrom);
        let path = index_cache_path(&cache_dir, &vcf);
        if path.exists() {
            skipped += 1;
            continue;
        }
        let (tbi_url, _) = index_urls(&vcf);
        queue_download(
            &mut handles,
            semaphore.clone(),
            tbi_url,
            path,
            format!("exomes chr{chrom}"),
        );
    }

    for chrom in &manifest.genome_contigs {
        let vcf = vcf_url(cfg, &cfg.genome_template, chrom);
        let path = index_cache_path(&cache_dir, &vcf);
        if path.exists() {
            skipped += 1;
            continue;
        }
        let (tbi_url, _) = index_urls(&vcf);
        queue_download(
            &mut handles,
            semaphore.clone(),
            tbi_url,
            path,
            format!("genomes chr{chrom}"),
        );
    }

    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => downloaded += 1,
            Ok(Err(e)) => {
                failed += 1;
                errors.push(e);
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("task failed: {e}"));
            }
        }
    }

    let message = if failed == 0 {
        format!("Downloaded {downloaded} tabix indexes ({skipped} already cached).")
    } else {
        format!("Downloaded {downloaded}, failed {failed}, skipped {skipped} cached.")
    };

    GnomadIndexSyncResult {
        downloaded,
        skipped,
        failed,
        message,
        errors,
    }
}

fn queue_download(
    handles: &mut Vec<tokio::task::JoinHandle<Result<(), String>>>,
    semaphore: std::sync::Arc<tokio::sync::Semaphore>,
    tbi_url: String,
    path: PathBuf,
    label: String,
) {
    handles.push(tokio::spawn(async move {
        let _permit = semaphore.acquire().await.ok();
        download_index_file(&tbi_url, &path)
            .await
            .map_err(|e| format!("{label}: {e}"))
    }));
}

async fn download_index_file(url: &str, dest: &Path) -> Result<(), String> {
    let bytes = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn provider_label(provider: GnomadHttpsProvider) -> String {
    match provider {
        GnomadHttpsProvider::Aws => "aws".into(),
        GnomadHttpsProvider::Google => "google".into(),
    }
}

fn frequency_cache_summary(
    cache: &super::cache::FrequencyCacheStatus,
    release: &str,
) -> String {
    if cache.current_rows > 0 && cache.stale_rows > 0 {
        format!(
            "Frequency cache current for {release}: {} rows; {} older rows retained.",
            cache.current_rows, cache.stale_rows
        )
    } else if cache.current_rows > 0 {
        format!(
            "Frequency cache current for {release}: {} rows.",
            cache.current_rows
        )
    } else if cache.stale_rows > 0 {
        format!(
            "Frequency cache needs refresh for {release}; {} older rows retained and excluded.",
            cache.stale_rows
        )
    } else {
        format!("Frequency cache not warmed for {release} yet.")
    }
}
