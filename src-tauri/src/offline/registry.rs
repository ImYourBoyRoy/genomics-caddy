// ./src-tauri/src/offline/registry.rs
use super::download::RemoteHead;
use super::manifest::OfflineAssetId;
use rusqlite::{Connection, params};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct RegistryRow {
    pub remote_etag: Option<String>,
    pub remote_last_modified: Option<String>,
    pub remote_content_length: Option<i64>,
    pub local_bytes: i64,
    pub version_label: Option<String>,
    pub synced_at: i64,
    pub update_available: bool,
    pub local_sha256: Option<String>,
    pub checked_etag: Option<String>,
    pub checked_last_modified: Option<String>,
    pub checked_content_filename: Option<String>,
    pub checked_at: Option<i64>,
    pub previous_row_count: Option<i64>,
    pub previous_version_label: Option<String>,
    pub source_changed_at: Option<i64>,
    pub downloaded_at: Option<i64>,
    pub indexed_at: Option<i64>,
}

pub fn read_registry(conn: &Connection, asset_id: &str) -> Option<RegistryRow> {
    conn.query_row(
        "SELECT remote_etag, remote_last_modified, remote_content_length, local_bytes,
                version_label, synced_at, update_available, local_sha256,
                checked_etag, checked_last_modified, checked_content_filename, checked_at,
                previous_row_count, previous_version_label, source_changed_at,
                downloaded_at, indexed_at
         FROM offline_asset_registry WHERE asset_id = ?",
        params![asset_id],
        |row| {
            Ok(RegistryRow {
                remote_etag: row.get(0)?,
                remote_last_modified: row.get(1)?,
                remote_content_length: row.get(2)?,
                local_bytes: row.get(3)?,
                version_label: row.get(4)?,
                synced_at: row.get(5)?,
                update_available: row.get::<_, i64>(6)? != 0,
                local_sha256: row.get(7)?,
                checked_etag: row.get(8)?,
                checked_last_modified: row.get(9)?,
                checked_content_filename: row.get(10)?,
                checked_at: row.get(11)?,
                previous_row_count: row.get(12)?,
                previous_version_label: row.get(13)?,
                source_changed_at: row.get(14)?,
                downloaded_at: row.get(15)?,
                indexed_at: row.get(16)?,
            })
        },
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_registry(
    conn: &Connection,
    asset_id: OfflineAssetId,
    tier: u8,
    local_path: &Path,
    source_url: &str,
    head: Option<&RemoteHead>,
    local_bytes: u64,
    local_sha256: Option<&str>,
    row_count: u64,
    version_label: Option<&str>,
    update_available: bool,
) -> Result<(), String> {
    let now = crate::research::util::unix_now();
    let previous = read_registry(conn, asset_id.as_str());
    let source_changed = previous.as_ref().is_some_and(|old| {
        let hash_changed = local_sha256
            .zip(old.local_sha256.as_deref())
            .is_some_and(|(new, old_hash)| new != old_hash);
        let hash_missing = local_sha256.is_none() || old.local_sha256.is_none();
        let label_changed = version_label
            .zip(old.version_label.as_deref())
            .is_some_and(|(new, old_label)| new != old_label);
        hash_changed || (hash_missing && label_changed)
    });
    if source_changed {
        conn.execute(
            "UPDATE reference.offline_asset_registry
             SET previous_row_count = row_count,
                 previous_version_label = version_label,
                 source_changed_at = ?
             WHERE asset_id = ?",
            params![now, asset_id.as_str()],
        )
        .map_err(|e| e.to_string())?;
    }
    let etag = head.and_then(|h| h.etag.as_deref());
    let lm = head.and_then(|h| h.last_modified.as_deref());
    let clen = head.and_then(|h| h.content_length.map(|n| n as i64));
    // When head is absent (import-only re-sync), keep any previously probed remote size.
    let existing_clen = if clen.is_none() {
        read_registry(conn, asset_id.as_str()).and_then(|r| r.remote_content_length)
    } else {
        None
    };
    let clen = clen.or(existing_clen);
    let downloaded_at = head.map(|_| now);
    let indexed_at = (head.is_none() && row_count > 0 && is_catalog_index(asset_id)).then_some(now);
    conn.execute(
        "INSERT INTO reference.offline_asset_registry (
            asset_id, tier, local_path, source_url, remote_etag, remote_last_modified,
            remote_content_length, local_sha256, local_bytes, row_count, version_label,
            synced_at, update_available, downloaded_at, indexed_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(asset_id) DO UPDATE SET
            tier = excluded.tier,
            local_path = excluded.local_path,
            source_url = excluded.source_url,
            remote_etag = COALESCE(excluded.remote_etag, reference.offline_asset_registry.remote_etag),
            remote_last_modified = COALESCE(excluded.remote_last_modified, reference.offline_asset_registry.remote_last_modified),
            remote_content_length = COALESCE(excluded.remote_content_length, reference.offline_asset_registry.remote_content_length),
            local_sha256 = excluded.local_sha256,
            local_bytes = excluded.local_bytes,
            row_count = excluded.row_count,
            version_label = COALESCE(excluded.version_label, reference.offline_asset_registry.version_label),
            synced_at = excluded.synced_at,
            update_available = excluded.update_available,
            downloaded_at = COALESCE(excluded.downloaded_at, reference.offline_asset_registry.downloaded_at),
            indexed_at = COALESCE(excluded.indexed_at, reference.offline_asset_registry.indexed_at)",
        params![
            asset_id.as_str(),
            tier as i64,
            local_path.to_string_lossy().to_string(),
            source_url,
            etag,
            lm,
            clen,
            local_sha256,
            local_bytes as i64,
            row_count as i64,
            version_label,
            now,
            if update_available { 1 } else { 0 },
            downloaded_at,
            indexed_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn is_catalog_index(asset_id: OfflineAssetId) -> bool {
    matches!(
        asset_id,
        OfflineAssetId::GwasCatalog
            | OfflineAssetId::ClinvarVariantSummary
            | OfflineAssetId::ClinvarSubmissionSummary
            | OfflineAssetId::PharmgkbClinicalVariants
            | OfflineAssetId::PharmgkbGenes
            | OfflineAssetId::ClingenGeneValidity
            | OfflineAssetId::ManeSelectSummary
            | OfflineAssetId::DbsnpMergedJson
            | OfflineAssetId::DbsnpWithdrawnJson
            | OfflineAssetId::Tier2VariantLocus
    )
}

pub fn record_remote_probe(
    conn: &Connection,
    asset_id: &str,
    available: bool,
    head: &RemoteHead,
) -> Result<(), String> {
    conn.execute(
        "UPDATE reference.offline_asset_registry
         SET update_available = ?, checked_etag = ?, checked_last_modified = ?,
             checked_content_filename = ?, checked_at = ?
         WHERE asset_id = ?",
        params![
            if available { 1 } else { 0 },
            head.etag.as_deref(),
            head.last_modified.as_deref(),
            head.content_filename.as_deref(),
            crate::research::util::unix_now(),
            asset_id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Clear all transient update flags before a fresh authoritative remote probe.
pub fn clear_update_flags(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE reference.offline_asset_registry
         SET update_available = 0
         WHERE update_available != 0",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist a remote Content-Length probe without marking the asset synced.
pub fn store_remote_content_length(
    conn: &Connection,
    asset_id: OfflineAssetId,
    tier: u8,
    source_url: &str,
    content_length: u64,
) -> Result<(), String> {
    let now = crate::research::util::unix_now();
    conn.execute(
        "INSERT INTO reference.offline_asset_registry (
            asset_id, tier, local_path, source_url, remote_content_length,
            local_bytes, row_count, synced_at, update_available
         ) VALUES (?, ?, '', ?, ?, 0, 0, ?, 0)
         ON CONFLICT(asset_id) DO UPDATE SET
            remote_content_length = excluded.remote_content_length,
            source_url = excluded.source_url,
            tier = excluded.tier",
        params![
            asset_id.as_str(),
            tier as i64,
            source_url,
            content_length as i64,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn row_count_for_asset(conn: &Connection, asset_id: OfflineAssetId) -> u64 {
    fn catalog_table_count(conn: &Connection, schema: &str, table: &str) -> u64 {
        let qualified = if super::schema::schema_attached(conn, schema) {
            format!("{schema}.{table}")
        } else {
            format!("reference.{table}")
        };
        super::schema::table_count(conn, &qualified)
    }

    match asset_id {
        OfflineAssetId::GwasCatalog => catalog_table_count(conn, "gwas", "gwas_reference"),
        OfflineAssetId::ClinvarVariantSummary => {
            catalog_table_count(conn, "clinvar", "clinvar_reference")
        }
        OfflineAssetId::ClinvarSubmissionSummary => {
            catalog_table_count(conn, "clinvar_submissions", "clinvar_submissions")
        }
        OfflineAssetId::PharmgkbClinicalVariants => {
            catalog_table_count(conn, "pharmgkb", "pharmgkb_clinical_variants")
        }
        OfflineAssetId::PharmgkbGenes => catalog_table_count(conn, "pharmgkb", "pharmgkb_genes"),
        OfflineAssetId::ClingenGeneValidity => {
            catalog_table_count(conn, "clingen", "clingen_gene_validity")
        }
        OfflineAssetId::ManeSelectSummary => catalog_table_count(conn, "mane", "mane_transcripts"),
        OfflineAssetId::DbsnpMergedJson | OfflineAssetId::DbsnpWithdrawnJson => {
            catalog_table_count(conn, "dbsnp", "rsid_aliases")
        }
        OfflineAssetId::Tier2VariantLocus => {
            // Locus rows live in per-sample DBs; prefer last synced registry count.
            conn.query_row(
                "SELECT COALESCE(row_count, 0) FROM offline_asset_registry WHERE asset_id = ?",
                params![asset_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .or_else(|_| {
                conn.query_row(
                    "SELECT COALESCE(row_count, 0) FROM reference.offline_asset_registry WHERE asset_id = ?",
                    params![asset_id.as_str()],
                    |row| row.get::<_, i64>(0),
                )
            })
            .unwrap_or(0) as u64
        }
        OfflineAssetId::LiftoverChain => {
            if read_registry(conn, asset_id.as_str())
                .map(|r| r.local_bytes > 0)
                .unwrap_or(false)
            {
                1
            } else {
                0
            }
        }
        OfflineAssetId::GnomadIndexManifest => {
            if read_registry(conn, asset_id.as_str()).is_some() {
                1
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn successful_download_metadata_clears_previous_update_flag() {
        let conn = Connection::open_in_memory().expect("open registry test connection");
        conn.execute("ATTACH DATABASE ':memory:' AS reference", [])
            .expect("attach reference test database");
        crate::offline::schema::migrate_offline_schema(&conn)
            .expect("create offline registry schema");
        let old_head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"old\"".into()),
            last_modified: Some("Sat, 11 Jul 2026 00:00:00 GMT".into()),
            content_filename: Some("catalog-old.tsv".into()),
        };
        let new_head = RemoteHead {
            content_length: Some(120),
            etag: Some("\"new\"".into()),
            last_modified: Some("Sun, 12 Jul 2026 00:00:00 GMT".into()),
            content_filename: Some("catalog-new.tsv".into()),
        };

        upsert_registry(
            &conn,
            OfflineAssetId::GwasCatalog,
            0,
            Path::new("catalog.tsv"),
            "https://example.invalid/catalog.tsv",
            Some(&old_head),
            100,
            None,
            10,
            Some("catalog-old.tsv"),
            true,
        )
        .expect("seed an update flag");
        upsert_registry(
            &conn,
            OfflineAssetId::GwasCatalog,
            0,
            Path::new("catalog.tsv"),
            "https://example.invalid/catalog.tsv",
            Some(&new_head),
            120,
            None,
            20,
            Some("catalog-new.tsv"),
            false,
        )
        .expect("record the successful download");

        let registry = read_registry(&conn, "gwas_catalog").expect("read updated registry");
        assert!(!registry.update_available);
        assert_eq!(registry.remote_etag.as_deref(), Some("\"new\""));
        assert_eq!(registry.local_bytes, 120);
    }

    #[test]
    fn download_and_index_timestamps_are_tracked_separately() {
        let conn = Connection::open_in_memory().expect("open registry test connection");
        conn.execute("ATTACH DATABASE ':memory:' AS reference", [])
            .expect("attach reference test database");
        crate::offline::schema::migrate_offline_schema(&conn)
            .expect("create offline registry schema");
        let head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"fixture\"".into()),
            last_modified: Some("Mon, 02 Nov 2026 00:00:00 GMT".into()),
            content_filename: Some("clinicalVariants.zip".into()),
        };

        upsert_registry(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants,
            1,
            Path::new("clinicalVariants.zip"),
            "https://example.invalid/clinicalVariants.zip",
            Some(&head),
            100,
            Some("fixture-hash"),
            0,
            Some("clinicalVariants.zip"),
            false,
        )
        .expect("record completed source download");
        let downloaded = read_registry(&conn, "pharmgkb_clinical_variants")
            .expect("read downloaded source status");
        let downloaded_at = downloaded.downloaded_at;
        assert!(downloaded_at.is_some());
        assert_eq!(downloaded.indexed_at, None);

        upsert_registry(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants,
            1,
            Path::new("clinicalVariants.zip"),
            "https://example.invalid/clinicalVariants.zip",
            None,
            100,
            Some("fixture-hash"),
            5,
            None,
            false,
        )
        .expect("record completed index import");
        let indexed =
            read_registry(&conn, "pharmgkb_clinical_variants").expect("read indexed source status");
        assert_eq!(indexed.downloaded_at, downloaded_at);
        assert!(indexed.indexed_at.is_some());
    }

    #[test]
    fn changed_source_remembers_previous_row_total_and_revision_label() {
        let conn = Connection::open_in_memory().expect("open registry test connection");
        conn.execute("ATTACH DATABASE ':memory:' AS reference", [])
            .expect("attach reference test database");
        crate::offline::schema::migrate_offline_schema(&conn)
            .expect("create offline registry schema");
        let old_head = RemoteHead {
            content_length: Some(100),
            etag: Some("\"old\"".into()),
            last_modified: None,
            content_filename: Some("clinicalVariants-old.zip".into()),
        };
        let new_head = RemoteHead {
            content_length: Some(120),
            etag: Some("\"new\"".into()),
            last_modified: None,
            content_filename: Some("clinicalVariants-new.zip".into()),
        };
        upsert_registry(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants,
            1,
            Path::new("clinicalVariants.zip"),
            "https://example.invalid/clinicalVariants.zip",
            Some(&old_head),
            100,
            Some("old-hash"),
            12,
            Some("clinicalVariants-old.zip"),
            false,
        )
        .expect("seed old source metadata");
        upsert_registry(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants,
            1,
            Path::new("clinicalVariants.zip"),
            "https://example.invalid/clinicalVariants.zip",
            Some(&new_head),
            120,
            Some("new-hash"),
            0,
            Some("clinicalVariants-new.zip"),
            false,
        )
        .expect("record changed source metadata");

        let row =
            read_registry(&conn, "pharmgkb_clinical_variants").expect("read changed source status");
        assert_eq!(row.previous_row_count, Some(12));
        assert_eq!(
            row.previous_version_label.as_deref(),
            Some("clinicalVariants-old.zip")
        );
        assert!(row.source_changed_at.is_some());
    }

    #[test]
    fn counts_attached_clingen_rows_without_relying_on_a_temp_view() {
        let conn = Connection::open_in_memory().expect("open test connection");
        conn.execute("ATTACH DATABASE ':memory:' AS clingen", [])
            .expect("attach ClinGen test database");
        conn.execute(
            "CREATE TABLE clingen.clingen_gene_validity (gene_symbol TEXT, disease_label TEXT)",
            [],
        )
        .expect("create ClinGen test table");
        conn.execute(
            "INSERT INTO clingen.clingen_gene_validity (gene_symbol, disease_label) VALUES ('TEST', 'Test condition')",
            [],
        )
        .expect("insert ClinGen test row");

        assert_eq!(
            row_count_for_asset(&conn, OfflineAssetId::ClingenGeneValidity),
            1
        );
    }

    #[test]
    fn counts_attached_gwas_rows_without_relying_on_a_temp_view() {
        let conn = Connection::open_in_memory().expect("open test connection");
        conn.execute("ATTACH DATABASE ':memory:' AS gwas", [])
            .expect("attach GWAS test database");
        conn.execute(
            "CREATE TABLE gwas.gwas_reference (rsid TEXT PRIMARY KEY)",
            [],
        )
        .expect("create GWAS test table");
        conn.execute(
            "INSERT INTO gwas.gwas_reference (rsid) VALUES ('rs-test')",
            [],
        )
        .expect("insert GWAS test row");

        assert_eq!(row_count_for_asset(&conn, OfflineAssetId::GwasCatalog), 1);
    }
}
