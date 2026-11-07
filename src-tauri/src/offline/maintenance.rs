//! Safe removal of downloaded reference sources and their imported indexes.
//!
//! This module deliberately knows only manifest-owned source paths and the
//! curated catalog sidecars. It never walks or removes `samples/`, profile
//! databases, exports, or app settings.

use super::manifest::{AssetKind, OfflineAssetId, all_assets, asset_def, local_path};
use rusqlite::{Connection, params};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize)]
pub struct OfflineMaintenanceResult {
    pub files_removed: u32,
    pub bytes_freed: u64,
    pub databases_removed: u32,
    pub records_removed: u64,
}

struct CatalogDatabase {
    id: &'static str,
    schema: &'static str,
    filename: &'static str,
    assets: &'static [OfflineAssetId],
    tables: &'static [&'static str],
    temp_views: &'static [&'static str],
    staging_files: &'static [&'static str],
}

const GWAS_ASSETS: &[OfflineAssetId] = &[OfflineAssetId::GwasCatalog];
const CLINVAR_ASSETS: &[OfflineAssetId] = &[OfflineAssetId::ClinvarVariantSummary];
const CLINVAR_SUBMISSION_ASSETS: &[OfflineAssetId] = &[OfflineAssetId::ClinvarSubmissionSummary];
const CLINPGX_ASSETS: &[OfflineAssetId] = &[
    OfflineAssetId::PharmgkbClinicalVariants,
    OfflineAssetId::PharmgkbGenes,
];
const CLINGEN_ASSETS: &[OfflineAssetId] = &[OfflineAssetId::ClingenGeneValidity];
const MANE_ASSETS: &[OfflineAssetId] = &[OfflineAssetId::ManeSelectSummary];
const DBSNP_ASSETS: &[OfflineAssetId] = &[
    OfflineAssetId::DbsnpMergedJson,
    OfflineAssetId::DbsnpWithdrawnJson,
];

const DATABASES: &[CatalogDatabase] = &[
    CatalogDatabase {
        id: "gwas",
        schema: "gwas",
        filename: "gwas.db",
        assets: GWAS_ASSETS,
        tables: &["gwas_reference"],
        temp_views: &["gwas_reference"],
        staging_files: &[],
    },
    CatalogDatabase {
        id: "clinvar",
        schema: "clinvar",
        filename: "clinvar.db",
        assets: CLINVAR_ASSETS,
        tables: &["clinvar_reference"],
        temp_views: &["clinvar_reference"],
        staging_files: &["clinvar_staging.db"],
    },
    CatalogDatabase {
        id: "clinvar_submissions",
        schema: "clinvar_submissions",
        filename: "clinvar_submissions.db",
        assets: CLINVAR_SUBMISSION_ASSETS,
        tables: &["clinvar_submissions"],
        temp_views: &[],
        staging_files: &[],
    },
    CatalogDatabase {
        id: "clinpgx",
        schema: "pharmgkb",
        filename: "pharmgkb.db",
        assets: CLINPGX_ASSETS,
        tables: &["pharmgkb_clinical_variants", "pharmgkb_genes"],
        temp_views: &["pharmgkb_clinical_variants", "pharmgkb_genes"],
        staging_files: &[],
    },
    CatalogDatabase {
        id: "clingen",
        schema: "clingen",
        filename: "clingen.db",
        assets: CLINGEN_ASSETS,
        tables: &["clingen_gene_validity"],
        temp_views: &["clingen_gene_validity"],
        staging_files: &[],
    },
    CatalogDatabase {
        id: "mane",
        schema: "mane",
        filename: "mane.db",
        assets: MANE_ASSETS,
        tables: &["mane_transcripts"],
        temp_views: &["mane_transcripts"],
        staging_files: &[],
    },
    CatalogDatabase {
        id: "dbsnp",
        schema: "dbsnp",
        filename: "dbsnp.db",
        assets: DBSNP_ASSETS,
        tables: &["rsid_aliases", "refsnp_meta"],
        temp_views: &["rsid_aliases"],
        staging_files: &["dbsnp_staging.db", "dbsnp_staging.checkpoint.json"],
    },
];

pub fn remove_download(
    data_dir: &Path,
    db_path: &Path,
    asset_id: &str,
) -> Result<OfflineMaintenanceResult, String> {
    let id = OfflineAssetId::from_str_id(asset_id)
        .ok_or_else(|| format!("Unknown reference download: {asset_id}"))?;
    let def = asset_def(id).ok_or_else(|| format!("Unknown reference download: {asset_id}"))?;
    if id == OfflineAssetId::GnomadIndexManifest {
        let mut result = OfflineMaintenanceResult::default();
        remove_gnomad_indexes(data_dir, &mut result)?;
        result.records_removed = clear_gnomad_reference_cache(db_path)?;
        clear_registry_entry(db_path, id)?;
        return Ok(result);
    }
    if !matches!(def.kind, AssetKind::RemoteFile | AssetKind::GwasSync) {
        return Err(format!("{} is not a removable download.", def.label));
    }

    let custom_dir = super::sync::get_custom_download_dir_from_db(db_path);
    let mut result = OfflineMaintenanceResult::default();
    remove_asset_files(data_dir, custom_dir.as_deref(), id, &mut result)?;
    remove_import_staging(data_dir, db_path, id, &mut result)?;
    Ok(result)
}

pub fn delete_database(
    data_dir: &Path,
    db_path: &Path,
    database_id: &str,
) -> Result<OfflineMaintenanceResult, String> {
    let spec = database_spec(database_id)?;
    clear_catalog_database(data_dir, db_path, spec)
}

pub fn purge_all_downloads(
    data_dir: &Path,
    db_path: &Path,
) -> Result<OfflineMaintenanceResult, String> {
    let custom_dir = super::sync::get_custom_download_dir_from_db(db_path);
    let mut result = OfflineMaintenanceResult::default();

    // Remove only known app-managed sources and their resumable partial files.
    // User DNA, profile directories, exports, and arbitrary files in a custom
    // download folder are outside this boundary.
    for def in all_assets() {
        match def.kind {
            AssetKind::RemoteFile | AssetKind::GwasSync => {
                remove_asset_files(data_dir, custom_dir.as_deref(), def.id, &mut result)?;
            }
            AssetKind::Derived if def.id == OfflineAssetId::GnomadIndexManifest => {
                remove_asset_files(data_dir, custom_dir.as_deref(), def.id, &mut result)?;
            }
            AssetKind::Derived => {}
        }
    }

    for database in DATABASES {
        let removed = clear_catalog_database(data_dir, db_path, database)?;
        result.files_removed = result.files_removed.saturating_add(removed.files_removed);
        result.bytes_freed = result.bytes_freed.saturating_add(removed.bytes_freed);
        result.databases_removed = result
            .databases_removed
            .saturating_add(removed.databases_removed);
        result.records_removed = result
            .records_removed
            .saturating_add(removed.records_removed);
    }

    // gnomAD tabix indexes and its warmed frequency cache are also managed
    // reference data, although they are not entries in the source-download
    // manifest. Remove only known app-generated index extensions and metadata.
    remove_gnomad_indexes(data_dir, &mut result)?;
    result.records_removed = result
        .records_removed
        .saturating_add(clear_gnomad_reference_cache(db_path)?);

    let conn = crate::db::connect(db_path).map_err(|error| error.to_string())?;
    super::schema::migrate_offline_schema(&conn).map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM reference.offline_asset_registry", [])
        .map_err(|error| error.to_string())?;
    Ok(result)
}

fn clear_gnomad_reference_cache(db_path: &Path) -> Result<u64, String> {
    let conn = crate::db::connect(db_path).map_err(|error| error.to_string())?;
    crate::research::gnomad::migrate_gnomad_schema(&conn).map_err(|error| error.to_string())?;
    conn.execute("DELETE FROM reference.gnomad_variant_cache", [])
        .map(|rows| rows as u64)
        .map_err(|error| format!("Clear gnomAD frequency cache: {error}"))
}

fn clear_registry_entry(db_path: &Path, id: OfflineAssetId) -> Result<(), String> {
    let conn = crate::db::connect(db_path).map_err(|error| error.to_string())?;
    super::schema::migrate_offline_schema(&conn).map_err(|error| error.to_string())?;
    conn.execute(
        "UPDATE reference.offline_asset_registry
         SET row_count = 0, synced_at = 0, update_available = 0,
             previous_row_count = NULL, previous_version_label = NULL,
             source_changed_at = NULL, local_sha256 = NULL, local_bytes = 0,
             version_label = NULL, remote_etag = NULL, remote_last_modified = NULL,
             remote_content_length = NULL, checked_etag = NULL,
             checked_last_modified = NULL, checked_content_filename = NULL,
             checked_at = NULL, downloaded_at = NULL, indexed_at = NULL
         WHERE asset_id = ?",
        params![id.as_str()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn database_spec(database_id: &str) -> Result<&'static CatalogDatabase, String> {
    DATABASES
        .iter()
        .find(|entry| entry.id == database_id)
        .ok_or_else(|| format!("Unknown reference database: {database_id}"))
}

fn clear_catalog_database(
    data_dir: &Path,
    db_path: &Path,
    spec: &CatalogDatabase,
) -> Result<OfflineMaintenanceResult, String> {
    let sidecar = data_dir.join(spec.filename);
    let sidecar_existed = sidecar.exists();
    let conn = crate::db::connect(db_path).map_err(|error| error.to_string())?;
    super::schema::migrate_offline_schema(&conn).map_err(|error| error.to_string())?;

    if sidecar.exists() && !super::schema::schema_attached(&conn, spec.schema) {
        return Err(format!(
            "Could not open the {} database safely. Close the app and retry.",
            spec.id
        ));
    }

    let mut records_removed = 0u64;
    for table in spec.tables {
        let catalog_table = format!("{}.{}", spec.schema, table);
        if super::schema::schema_attached(&conn, spec.schema)
            && table_exists(&conn, spec.schema, table)
        {
            records_removed = records_removed.saturating_add(
                conn.execute(&format!("DELETE FROM {catalog_table}"), [])
                    .map_err(|error| format!("Clear {} database: {error}", spec.id))?
                    as u64,
            );
        }
        // Older builds kept fallback copies in reference.db. Clear those too
        // so a deleted catalog cannot remain queryable through a legacy path.
        if table_exists(&conn, "reference", table) {
            records_removed = records_removed.saturating_add(
                conn.execute(&format!("DELETE FROM reference.{table}"), [])
                    .map_err(|error| format!("Clear legacy {} data: {error}", spec.id))?
                    as u64,
            );
        }
    }

    for asset in spec.assets {
        conn.execute(
            "UPDATE reference.offline_asset_registry
             SET row_count = 0, synced_at = 0, update_available = 0,
                 previous_row_count = NULL, previous_version_label = NULL,
                 indexed_at = NULL,
                 source_changed_at = NULL, local_sha256 = NULL
             WHERE asset_id = ?",
            params![asset.as_str()],
        )
        .map_err(|error| error.to_string())?;
    }

    for view in spec.temp_views {
        conn.execute(&format!("DROP VIEW IF EXISTS temp.{view}"), [])
            .map_err(|error| error.to_string())?;
    }
    if super::schema::schema_attached(&conn, spec.schema) {
        conn.execute(&format!("DETACH DATABASE {}", spec.schema), [])
            .map_err(|error| format!("Close {} database: {error}", spec.id))?;
    }
    drop(conn);

    let mut result = OfflineMaintenanceResult {
        records_removed,
        ..Default::default()
    };
    for path in sidecar_paths(&sidecar) {
        remove_file_if_present(&path, &mut result)?;
    }
    for staging_file in spec.staging_files {
        for path in sidecar_paths(&data_dir.join(staging_file)) {
            remove_file_if_present(&path, &mut result)?;
        }
    }
    if sidecar_existed && !sidecar.exists() {
        result.databases_removed = 1;
    }
    Ok(result)
}

fn remove_import_staging(
    data_dir: &Path,
    db_path: &Path,
    id: OfflineAssetId,
    result: &mut OfflineMaintenanceResult,
) -> Result<(), String> {
    let staging_files: &[&str] = match id {
        OfflineAssetId::ClinvarVariantSummary => &["clinvar_staging.db"],
        OfflineAssetId::DbsnpMergedJson | OfflineAssetId::DbsnpWithdrawnJson => {
            &["dbsnp_staging.db", "dbsnp_staging.checkpoint.json"]
        }
        _ => &[],
    };
    if id == OfflineAssetId::ClinvarSubmissionSummary {
        let conn = crate::db::connect(db_path).map_err(|error| error.to_string())?;
        if super::schema::schema_attached(&conn, "clinvar_submissions") {
            conn.execute(
                "DROP TABLE IF EXISTS clinvar_submissions.clinvar_submissions_stage",
                [],
            )
            .map_err(|error| format!("Clear ClinVar submission import staging: {error}"))?;
        }
    }
    for filename in staging_files {
        let path = data_dir.join(filename);
        let related_paths = if filename.ends_with(".db") {
            sidecar_paths(&path).to_vec()
        } else {
            vec![path]
        };
        for related in related_paths {
            remove_file_if_present(&related, result)?;
        }
    }
    Ok(())
}

fn remove_gnomad_indexes(
    data_dir: &Path,
    result: &mut OfflineMaintenanceResult,
) -> Result<(), String> {
    let dir = data_dir.join("gnomad_indexes");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Inspect gnomAD index downloads: {error}")),
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("Inspect gnomAD index download: {error}"))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let known_metadata = matches!(
            name.as_ref(),
            "release_manifest.json" | "release_discovery.json"
        );
        let known_index = matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("tbi" | "csi")
        );
        if known_metadata || known_index {
            remove_file_if_present(&path, result)?;
        }
    }
    Ok(())
}

fn table_exists(conn: &Connection, schema: &str, table: &str) -> bool {
    conn.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?)"
        ),
        params![table],
        |row| row.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn sidecar_paths(db: &Path) -> [PathBuf; 4] {
    [
        db.to_path_buf(),
        PathBuf::from(format!("{}-wal", db.display())),
        PathBuf::from(format!("{}-shm", db.display())),
        PathBuf::from(format!("{}-journal", db.display())),
    ]
}

fn asset_paths(data_dir: &Path, custom_dir: Option<&Path>, id: OfflineAssetId) -> Vec<PathBuf> {
    let Some(def) = asset_def(id) else {
        return Vec::new();
    };
    if id == OfflineAssetId::GnomadIndexManifest {
        let mut paths = HashSet::new();
        add_download_variants(&mut paths, &local_path(data_dir, None, def));
        return paths.into_iter().collect();
    }
    let mut bases = vec![data_dir.to_path_buf()];
    if let Some(custom) = custom_dir
        && custom != data_dir
    {
        bases.push(custom.to_path_buf());
    }

    let mut paths = HashSet::new();
    for base in bases {
        let mut canonical = local_path(data_dir, Some(&base), def);
        if id == OfflineAssetId::GwasCatalog {
            let references = base.join("references");
            for name in [
                "gwas-catalog-associations_ontology-annotated-full.zip",
                "gwas-catalog-associations_ontology-annotated.tsv.gz",
                "gwas-catalog-associations_ontology-annotated.tsv",
            ] {
                canonical = references.join(name);
                add_download_variants(&mut paths, &canonical);
            }
            continue;
        }
        add_download_variants(&mut paths, &canonical);
    }
    paths.into_iter().collect()
}

fn add_download_variants(paths: &mut HashSet<PathBuf>, canonical: &Path) {
    for path in [
        canonical.to_path_buf(),
        PathBuf::from(format!("{}.part", canonical.display())),
        PathBuf::from(format!("{}.part.meta", canonical.display())),
        PathBuf::from(format!("{}.md5", canonical.display())),
        PathBuf::from(format!("{}.sha256", canonical.display())),
    ] {
        paths.insert(path);
    }
}

fn remove_asset_files(
    data_dir: &Path,
    custom_dir: Option<&Path>,
    id: OfflineAssetId,
    result: &mut OfflineMaintenanceResult,
) -> Result<(), String> {
    for path in asset_paths(data_dir, custom_dir, id) {
        remove_file_if_present(&path, result)?;
    }
    Ok(())
}

fn remove_file_if_present(
    path: &Path,
    result: &mut OfflineMaintenanceResult,
) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Inspect app-managed file: {error}")),
    };
    if !metadata.file_type().is_file() && !metadata.file_type().is_symlink() {
        return Err("Refusing to remove a non-file from a reference download location.".into());
    }
    fs::remove_file(path).map_err(|error| format!("Remove app-managed download: {error}"))?;
    result.files_removed = result.files_removed.saturating_add(1);
    result.bytes_freed = result.bytes_freed.saturating_add(metadata.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn test_dir(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "genomics_caddy_maintenance_{label}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }

    #[test]
    fn removing_a_download_keeps_its_imported_database() {
        let data_dir = test_dir("download");
        fs::create_dir_all(&data_dir).expect("create temporary data directory");
        let db_path = data_dir.join("user_genome.db");
        let conn = crate::db::connect(&db_path).expect("open fixture registry");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "pharmgkb")
            .expect("attach fixture ClinPGx database");
        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_genes (pharmgkb_id, symbol, name)
             VALUES ('PGX1', 'GENE1', 'Fixture gene')",
            [],
        )
        .expect("seed non-personal reference row");
        let download = data_dir.join("raw_downloads/pharmgkb/genes.zip");
        fs::create_dir_all(download.parent().expect("download parent"))
            .expect("create download directory");
        fs::write(&download, b"fixture catalog").expect("write source file");
        drop(conn);

        remove_download(&data_dir, &db_path, "pharmgkb_genes").expect("remove source file only");
        assert!(!download.exists());

        let conn = crate::db::connect(&db_path).expect("reopen fixture registry");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM pharmgkb.pharmgkb_genes", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count retained index"),
            1
        );
        drop(conn);
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn deleting_clinpgx_database_removes_both_tables_but_keeps_profile_data() {
        let data_dir = test_dir("database");
        fs::create_dir_all(data_dir.join("samples/1")).expect("create profile fixture dir");
        let db_path = data_dir.join("user_genome.db");
        fs::write(
            data_dir.join("samples/1/genome.db"),
            b"private fixture placeholder",
        )
        .expect("write profile placeholder");
        let conn = crate::db::connect(&db_path).expect("open fixture registry");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "pharmgkb")
            .expect("attach fixture ClinPGx database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reference.pharmgkb_genes (
                 pharmgkb_id TEXT PRIMARY KEY, symbol TEXT, name TEXT
             )",
            [],
        )
        .expect("create legacy ClinPGx fallback table");
        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_genes (pharmgkb_id, symbol) VALUES ('PGX1', 'GENE1')",
            [],
        )
        .expect("seed fixture ClinPGx gene");
        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_clinical_variants (rsid, gene)
             VALUES ('rs9001', 'GENE1')",
            [],
        )
        .expect("seed fixture ClinPGx annotation");
        conn.execute(
            "INSERT INTO reference.pharmgkb_genes (pharmgkb_id, symbol, name)
             VALUES ('LEGACY1', 'LEGACY', 'Legacy fixture gene')",
            [],
        )
        .expect("seed legacy fallback copy");
        drop(conn);

        let source = data_dir.join("raw_downloads/pharmgkb/genes.zip");
        fs::create_dir_all(source.parent().expect("ClinPGx source parent"))
            .expect("create ClinPGx source directory");
        fs::write(&source, b"fixture source").expect("write source download");

        let result = delete_database(&data_dir, &db_path, "clinpgx")
            .expect("delete ClinPGx reference database");
        assert_eq!(result.databases_removed, 1);
        assert_eq!(result.records_removed, 3);
        assert!(!data_dir.join("pharmgkb.db").exists());
        assert!(
            source.exists(),
            "database deletion must keep source downloads"
        );
        assert!(data_dir.join("samples/1/genome.db").exists());
        let conn = crate::db::connect(&db_path).expect("reopen fixture registry");
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM reference.pharmgkb_genes", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count legacy fallback copy"),
            0
        );
        drop(conn);
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn purging_reference_downloads_preserves_sample_databases_and_unknown_custom_files() {
        let data_dir = test_dir("purge");
        let custom_dir = data_dir.join("custom-downloads");
        fs::create_dir_all(custom_dir.join("raw_downloads/clinvar"))
            .expect("create custom download directory");
        fs::create_dir_all(data_dir.join("gnomad_indexes"))
            .expect("create gnomAD index fixture directory");
        fs::create_dir_all(data_dir.join("samples/1")).expect("create profile fixture dir");
        let db_path = data_dir.join("user_genome.db");
        fs::write(
            custom_dir.join("raw_downloads/clinvar/variant_summary.txt.gz"),
            b"fixture download",
        )
        .expect("write source file");
        fs::write(custom_dir.join("keep-me.txt"), b"user file")
            .expect("write unrelated custom file");
        let gnomad_index = data_dir.join("gnomad_indexes/fixture.tbi");
        fs::write(&gnomad_index, b"fixture index").expect("write gnomAD tabix index");
        fs::write(data_dir.join("gnomad_indexes/keep-me.txt"), b"user file")
            .expect("write unrelated gnomAD-directory file");
        fs::write(
            data_dir.join("samples/1/genome.db"),
            b"private fixture placeholder",
        )
        .expect("write profile placeholder");
        let conn = crate::db::connect(&db_path).expect("open fixture registry");
        conn.execute(
            "CREATE TABLE app_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .expect("create fixture app metadata");
        conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value)
             VALUES ('custom_download_dir', ?)",
            params![custom_dir.to_string_lossy().to_string()],
        )
        .expect("set custom downloads root");
        crate::research::gnomad::migrate_gnomad_schema(&conn)
            .expect("create gnomAD reference cache schema");
        conn.execute(
            "INSERT INTO reference.gnomad_variant_cache (
                 cache_id, release, source_mode, dataset, chrom, pos, ref, alt,
                 fetched_at, parser_version, lookup_status
             ) VALUES ('fixture', 'test', 'test', 'test', '1', 1, 'A', 'C', 1, 'test', 'test')",
            [],
        )
        .expect("seed gnomAD cache fixture");
        drop(conn);

        let result = purge_all_downloads(&data_dir, &db_path).expect("purge reference downloads");
        assert_eq!(result.records_removed, 1);
        assert!(
            !custom_dir
                .join("raw_downloads/clinvar/variant_summary.txt.gz")
                .exists()
        );
        assert!(custom_dir.join("keep-me.txt").exists());
        assert!(!gnomad_index.exists());
        assert!(data_dir.join("gnomad_indexes/keep-me.txt").exists());
        assert!(data_dir.join("samples/1/genome.db").exists());
        let _ = fs::remove_dir_all(data_dir);
    }
}
