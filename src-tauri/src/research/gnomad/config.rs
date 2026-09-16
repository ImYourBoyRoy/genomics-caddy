// ./src-tauri/src/research/gnomad/config.rs
use super::types::{
    DEFAULT_EXOME_TEMPLATE, DEFAULT_GENOME_TEMPLATE, DEFAULT_RELEASE, GnomadConfig,
    GnomadDatasetPolicy, GnomadHttpsProvider, GnomadSourceMode, LEGACY_EXOME_TEMPLATE,
    LEGACY_GENOME_TEMPLATE, LEGACY_RELEASE,
};
use super::manifest::discover_latest_gnomad_release;
use crate::db_runtime;
use rusqlite::{Connection, params};
use std::path::Path;

pub fn load_gnomad_config(conn: &Connection) -> Result<GnomadConfig, String> {
    super::schema::migrate_gnomad_schema(conn).map_err(|e| e.to_string())?;
    let mut cfg = conn.query_row(
        "SELECT enabled, source_mode, release, provider, local_vcf_dir,
                max_remote_concurrent_files, max_queries_per_second,
                graphql_enabled_for_sweep, graphql_fallback_enabled, dataset_policy,
                exome_template, genome_template, auto_discover_release
         FROM gnomad_config WHERE id = 1",
        [],
        |row| {
            Ok(GnomadConfig {
                enabled: row.get::<_, i64>(0)? != 0,
                source_mode: GnomadSourceMode::from_str_loose(&row.get::<_, String>(1)?),
                release: row.get(2)?,
                provider: GnomadHttpsProvider::from_str_loose(&row.get::<_, String>(3)?),
                local_vcf_dir: row.get(4)?,
                max_remote_concurrent_files: row.get::<_, i64>(5)? as usize,
                max_queries_per_second: row.get(6)?,
                graphql_enabled_for_sweep: row.get::<_, i64>(7)? != 0,
                graphql_fallback_enabled: row.get::<_, i64>(8)? != 0,
                dataset_policy: GnomadDatasetPolicy::from_str_loose(&row.get::<_, String>(9)?),
                exome_template: row.get(10)?,
                genome_template: row.get(11)?,
                auto_discover_release: row.get::<_, i64>(12)? != 0,
            })
        },
    )
    .map_err(|e| format!("Failed to load gnomAD config: {e}"))?;

    // The original built-in defaults pointed at gnomAD 4.1. The current
    // official variant release is 4.1.1, so migrate only an untouched remote
    // default row; custom releases and local VCF setups remain authoritative.
    let is_default_remote = cfg.source_mode == GnomadSourceMode::RemoteIndexedVcfHttps
        && cfg.local_vcf_dir.as_deref().map(str::trim).unwrap_or("").is_empty()
        && cfg.release == LEGACY_RELEASE
        && cfg.exome_template == LEGACY_EXOME_TEMPLATE
        && cfg.genome_template == LEGACY_GENOME_TEMPLATE;
    if is_default_remote {
        cfg.release = DEFAULT_RELEASE.to_string();
        cfg.exome_template = DEFAULT_EXOME_TEMPLATE.to_string();
        cfg.genome_template = DEFAULT_GENOME_TEMPLATE.to_string();
        cfg.auto_discover_release = true;
        conn.execute(
            "UPDATE reference.gnomad_config
             SET release = ?, exome_template = ?, genome_template = ?, auto_discover_release = 1
             WHERE id = 1",
            params![cfg.release, cfg.exome_template, cfg.genome_template],
        )
        .map_err(|e| format!("Failed to migrate the default gnomAD release: {e}"))?;
        let _ = super::cache::mark_stale_release(conn, &cfg.release);
    }

    // Rows created before release discovery existed get a pinned migration
    // default. Recognize only the untouched built-in remote configuration as
    // auto-managed; custom releases stay pinned until explicitly opted in.
    let is_current_builtin = cfg.source_mode == GnomadSourceMode::RemoteIndexedVcfHttps
        && cfg.local_vcf_dir.as_deref().map(str::trim).unwrap_or("").is_empty()
        && cfg.release == DEFAULT_RELEASE
        && cfg.exome_template == DEFAULT_EXOME_TEMPLATE
        && cfg.genome_template == DEFAULT_GENOME_TEMPLATE;
    if is_current_builtin && !cfg.auto_discover_release {
        cfg.auto_discover_release = true;
        conn.execute(
            "UPDATE reference.gnomad_config SET auto_discover_release = 1 WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Failed to enable gnomAD release discovery: {e}"))?;
    }

    Ok(cfg)
}

pub fn save_gnomad_config(conn: &Connection, cfg: &GnomadConfig) -> Result<(), String> {
    super::schema::migrate_gnomad_schema(conn).map_err(|e| e.to_string())?;
    super::validate::validate_gnomad_config_paths(cfg)?;
    let previous_release = load_gnomad_config(conn).ok().map(|c| c.release);
    let provider = match cfg.provider {
        GnomadHttpsProvider::Aws => "aws",
        GnomadHttpsProvider::Google => "google",
    };
    let mut source_mode = cfg.source_mode;
    if source_mode == GnomadSourceMode::PythonToolboxSidecar {
        source_mode = GnomadSourceMode::RemoteIndexedVcfHttps;
    }
    conn.execute(
        "UPDATE gnomad_config SET
            enabled = ?, source_mode = ?, release = ?, provider = ?,
            local_vcf_dir = ?, max_remote_concurrent_files = ?,
            max_queries_per_second = ?, graphql_enabled_for_sweep = ?,
            graphql_fallback_enabled = ?, dataset_policy = ?,
            exome_template = ?, genome_template = ?, auto_discover_release = ?
         WHERE id = 1",
        params![
            if cfg.enabled { 1 } else { 0 },
            source_mode.as_str(),
            cfg.release,
            provider,
            cfg.local_vcf_dir,
            cfg.max_remote_concurrent_files as i64,
            cfg.max_queries_per_second,
            if cfg.graphql_enabled_for_sweep { 1 } else { 0 },
            if cfg.graphql_fallback_enabled { 1 } else { 0 },
            cfg.dataset_policy.as_str(),
            cfg.exome_template,
            cfg.genome_template,
            if cfg.auto_discover_release { 1 } else { 0 },
        ],
    )
    .map_err(|e| e.to_string())?;
    if previous_release.as_deref() != Some(cfg.release.as_str()) {
        let _ = super::cache::mark_stale_release(conn, &cfg.release);
    }
    Ok(())
}

/// Load the configuration that runtime lookups should actually use.
///
/// Remote auto-managed configurations are refreshed from the public release
/// listing before a lookup starts. The discovery result is cached by
/// `discover_latest_gnomad_release`, so this does not probe the network on
/// every variant. Local folders and explicitly pinned releases remain
/// unchanged.
pub async fn load_effective_gnomad_config(
    db_path: &Path,
    data_dir: &Path,
) -> Result<GnomadConfig, String> {
    let db_path_buf = db_path.to_path_buf();
    let cfg = db_runtime::with_connection(db_path_buf.clone(), load_gnomad_config).await?;
    let follows_latest = cfg.auto_discover_release
        && cfg.source_mode == GnomadSourceMode::RemoteIndexedVcfHttps
        && cfg.local_vcf_dir.as_deref().map(str::trim).unwrap_or("").is_empty();
    if !follows_latest {
        return Ok(cfg);
    }

    let Some(discovered) = discover_latest_gnomad_release(&cfg, data_dir, false).await else {
        return Ok(cfg);
    };
    if discovered.release == cfg.release
        && discovered.exome_template == cfg.exome_template
        && discovered.genome_template == cfg.genome_template
    {
        return Ok(cfg);
    }

    let mut updated = cfg;
    updated.release = discovered.release;
    updated.exome_template = discovered.exome_template;
    updated.genome_template = discovered.genome_template;
    let persisted = updated.clone();
    db_runtime::with_connection(db_path_buf, move |conn| save_gnomad_config(conn, &persisted))
        .await?;
    Ok(updated)
}

pub fn vcf_url(cfg: &GnomadConfig, template: &str, chrom: &str) -> String {
    let chrom_key = normalize_chrom_for_template(chrom);
    let object_key = template.replace("{chrom}", &chrom_key);
    format!("{}/{}", cfg.provider.base_url(), object_key)
}

pub fn index_urls(vcf_url: &str) -> (String, String) {
    (format!("{vcf_url}.tbi"), format!("{vcf_url}.csi"))
}

pub fn normalize_chrom_for_template(chrom: &str) -> String {
    let c = chrom.trim().trim_start_matches("chr").to_uppercase();
    if c == "X" || c == "Y" || c == "M" || c == "MT" {
        if c == "MT" { "M".to_string() } else { c }
    } else {
        c.trim_start_matches('0').to_string()
    }
}

pub fn tabix_reference_name(chrom: &str) -> String {
    let c = chrom.trim().trim_start_matches("chr");
    if c.parse::<u32>().is_ok() {
        format!("chr{c}")
    } else {
        format!("chr{}", c.to_uppercase())
    }
}
