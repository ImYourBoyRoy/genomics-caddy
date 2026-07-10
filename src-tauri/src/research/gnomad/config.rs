// ./src-tauri/src/research/gnomad/config.rs
use super::types::{GnomadConfig, GnomadDatasetPolicy, GnomadHttpsProvider, GnomadSourceMode};
use rusqlite::{Connection, params};

pub fn load_gnomad_config(conn: &Connection) -> Result<GnomadConfig, String> {
    super::schema::migrate_gnomad_schema(conn).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT enabled, source_mode, release, provider, local_vcf_dir,
                max_remote_concurrent_files, max_queries_per_second,
                graphql_enabled_for_sweep, graphql_fallback_enabled, dataset_policy,
                exome_template, genome_template
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
            })
        },
    )
    .map_err(|e| format!("Failed to load gnomAD config: {e}"))
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
            exome_template = ?, genome_template = ?
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
        ],
    )
    .map_err(|e| e.to_string())?;
    if previous_release.as_deref() != Some(cfg.release.as_str()) {
        let _ = super::cache::mark_stale_release(conn, &cfg.release);
    }
    Ok(())
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
