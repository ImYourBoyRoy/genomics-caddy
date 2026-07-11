// ./src-tauri/src/offline/commands.rs
use super::sync::{
    OfflineSyncResult, OfflineUpdateCheck, build_tier2_for_sample, check_offline_updates,
    sync_all_missing, sync_offline_tier, sync_single_asset,
};
use crate::{get_data_dir, get_db_path};
use tauri::AppHandle;

#[tauri::command]
pub async fn check_offline_data_updates(app: AppHandle) -> Result<OfflineUpdateCheck, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    check_offline_updates(&data_dir, &db_path).await
}

/// Sync all assets in a tier.
/// `force` controls whether existing files are re-downloaded.
#[tauri::command]
pub async fn sync_offline_data_tier(
    app: AppHandle,
    tier: u8,
    force: bool,
    sample_id: Option<i64>,
) -> Result<OfflineSyncResult, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    sync_offline_tier(&data_dir, &db_path, tier, force, sample_id, Some(&app)).await
}

/// Sync a single asset by asset_id (e.g. "clinvar_variant_summary").
/// `force = true` re-downloads even if the file already exists on disk.
#[tauri::command]
pub async fn sync_single_offline_asset(
    app: AppHandle,
    asset_id: String,
    force: bool,
    sample_id: Option<i64>,
) -> Result<OfflineSyncResult, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    sync_single_asset(&data_dir, &db_path, &asset_id, force, sample_id, Some(&app)).await
}

/// Sync all tiers for missing/outdated assets. Never forces re-download.
#[tauri::command]
pub async fn sync_all_offline_missing(
    app: AppHandle,
    sample_id: Option<i64>,
) -> Result<Vec<OfflineSyncResult>, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    sync_all_missing(&data_dir, &db_path, sample_id, Some(&app)).await
}

/// Legacy: sync all tiers. `force` applies to every asset.
#[tauri::command]
pub async fn sync_all_offline_data(
    app: AppHandle,
    force: bool,
    sample_id: Option<i64>,
) -> Result<Vec<OfflineSyncResult>, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    let mut results = Vec::new();
    for tier in 0..=2 {
        results.push(
            sync_offline_tier(&data_dir, &db_path, tier, force, sample_id, Some(&app)).await?,
        );
    }
    Ok(results)
}

#[tauri::command]
pub async fn build_offline_tier2(
    app: AppHandle,
    sample_id: i64,
) -> Result<OfflineSyncResult, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    build_tier2_for_sample(&data_dir, &db_path, sample_id, Some(&app)).await
}

/// Cancel an in-progress catalog import (cooperative — checked between chunks).
#[tauri::command]
pub async fn cancel_offline_import() -> Result<(), String> {
    super::sync::cancel_offline_import();
    Ok(())
}

/// Export marker-pack coverage + genome-wide catalog findings JSON for pack authoring.
#[tauri::command]
pub async fn export_discovery_findings(
    app: AppHandle,
    sample_id: i64,
) -> Result<serde_json::Value, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        super::discovery_export::export_discovery_jsons(&data_dir, &db_path, sample_id)
    })
    .await
    .map_err(|e| format!("Export worker failed: {e}"))?
}

/// Browse genome×catalog associations in-app (ranked, filterable, paginated).
#[tauri::command]
pub async fn query_discovery_findings(
    app: AppHandle,
    sample_id: i64,
    beyond_packs_only: Option<bool>,
    source_filter: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<serde_json::Value, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    let beyond = beyond_packs_only.unwrap_or(true);
    let limit = limit.unwrap_or(100) as usize;
    let offset = offset.unwrap_or(0) as usize;
    tauri::async_runtime::spawn_blocking(move || {
        super::discovery_export::query_discovery_findings(
            &data_dir,
            &db_path,
            sample_id,
            beyond,
            source_filter.as_deref(),
            query.as_deref(),
            limit,
            offset,
            Some(&app),
        )
    })
    .await
    .map_err(|e| format!("Discovery query worker failed: {e}"))?
}

#[tauri::command]
pub async fn cancel_discovery_query() -> Result<(), String> {
    super::discovery_export::cancel_discovery_query();
    Ok(())
}

#[tauri::command]
pub async fn get_custom_download_dir(app: AppHandle) -> Result<Option<String>, String> {
    let db_path = get_db_path(&app);
    let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
    crate::offline::schema::migrate_offline_schema(&conn).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT value FROM app_metadata WHERE key = 'custom_download_dir'")
        .map_err(|e| e.to_string())?;
    let val: Option<String> = match stmt.query_row([], |row| row.get(0)) {
        Ok(v) => Some(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e.to_string()),
    };
    Ok(val)
}

#[tauri::command]
pub async fn set_custom_download_dir(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
    crate::offline::schema::migrate_offline_schema(&conn).map_err(|e| e.to_string())?;

    match path {
        Some(p) => {
            let trimmed = p.trim().to_string();
            if trimmed.is_empty() {
                conn.execute(
                    "DELETE FROM app_metadata WHERE key = 'custom_download_dir'",
                    [],
                )
                .map_err(|e| e.to_string())?;
            } else {
                conn.execute(
                    "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('custom_download_dir', ?)",
                    rusqlite::params![trimmed],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        None => {
            conn.execute(
                "DELETE FROM app_metadata WHERE key = 'custom_download_dir'",
                [],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceStatusDetails {
    pub clinvar_raw_found: bool,
    pub clinvar_indexed_rows: u64,
    pub clinvar_rsid_hits: u64,
    pub clinvar_last_indexed: Option<i64>,
    pub dbsnp_merged_raw_found: bool,
    pub dbsnp_merge_mappings_indexed: u64,
    pub dbsnp_rsids_normalized: u64,
    pub dbsnp_merge_index_available: bool,
    pub dbsnp_placement_index_available: bool,
    pub orientation_verification_available: bool,
}

#[tauri::command]
pub async fn get_offline_reference_status(
    app: AppHandle,
    report_rsids: Option<Vec<String>>,
) -> Result<ReferenceStatusDetails, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);

    let custom_dir = super::sync::get_custom_download_dir_from_db(&db_path);

    // 1. ClinVar raw file path check
    let clinvar_def = super::manifest::all_assets()
        .iter()
        .find(|a| a.id == super::manifest::OfflineAssetId::ClinvarVariantSummary)
        .ok_or_else(|| "ClinVar manifest definition missing".to_string())?;
    let clinvar_gz_path =
        super::manifest::local_path(&data_dir, custom_dir.as_deref(), clinvar_def);
    let clinvar_raw_found = super::compress::resolve_local_asset_path(&clinvar_gz_path).is_some();

    // 2. dbSNP merged raw file path check
    let dbsnp_def = super::manifest::all_assets()
        .iter()
        .find(|a| a.id == super::manifest::OfflineAssetId::DbsnpMergedJson)
        .ok_or_else(|| "dbSNP manifest definition missing".to_string())?;
    let dbsnp_bz2_path = super::manifest::local_path(&data_dir, custom_dir.as_deref(), dbsnp_def);
    let dbsnp_merged_raw_found =
        super::compress::resolve_local_asset_path(&dbsnp_bz2_path).is_some();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;

        let clinvar_indexed_rows = conn
            .query_row("SELECT COUNT(*) FROM clinvar.clinvar_reference", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as u64;

        let dbsnp_merge_mappings_indexed = conn
            .query_row("SELECT COUNT(*) FROM dbsnp.rsid_aliases WHERE source = 'dbsnp_merged'", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as u64;

        let clinvar_last_indexed: Option<i64> = conn.query_row(
            "SELECT synced_at FROM reference.offline_asset_registry WHERE asset_id = 'clinvar_variant_summary'",
            [],
            |r| r.get::<_, i64>(0),
        ).ok();

        let mut clinvar_rsid_hits = 0;
        let mut dbsnp_rsids_normalized = 0;

        if let Some(ref rsids) = report_rsids {
            if !rsids.is_empty() {
                // Pre-normalize all rsIDs passed from the report
                let normalized_rsids: Vec<String> = rsids
                    .iter()
                    .map(|r| r.trim().to_lowercase())
                    .filter(|r| !r.is_empty())
                    .collect();

                if !normalized_rsids.is_empty() {
                    let placeholders = normalized_rsids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", i + 1))
                        .collect::<Vec<_>>()
                        .join(",");

                    // Count ClinVar hits
                    let sql_clinvar = format!(
                        "SELECT COUNT(DISTINCT LOWER(rsid)) FROM clinvar.clinvar_reference WHERE LOWER(rsid) IN ({})",
                        placeholders
                    );
                    let params_clinvar: Vec<&dyn rusqlite::types::ToSql> = normalized_rsids
                         .iter()
                         .map(|s| s as &dyn rusqlite::types::ToSql)
                         .collect();
                    if let Ok(c) = conn.query_row(&sql_clinvar, params_clinvar.as_slice(), |r| r.get::<_, i64>(0)) {
                        clinvar_rsid_hits = c as u64;
                    }

                    // Count dbSNP normalized
                    let sql_dbsnp = format!(
                        "SELECT COUNT(DISTINCT LOWER(rsid)) FROM dbsnp.rsid_aliases WHERE LOWER(rsid) IN ({}) AND merged_into IS NOT NULL",
                        placeholders
                    );
                    let params_dbsnp: Vec<&dyn rusqlite::types::ToSql> = normalized_rsids
                        .iter()
                        .map(|s| s as &dyn rusqlite::types::ToSql)
                        .collect();
                    if let Ok(c) = conn.query_row(&sql_dbsnp, params_dbsnp.as_slice(), |r| r.get::<_, i64>(0)) {
                        dbsnp_rsids_normalized = c as u64;
                    }
                }
            }
        }

        let dbsnp_placement_index_available = conn
            .query_row("SELECT COUNT(*) FROM variant_locus", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) > 0;

        let orientation_verification_available = conn
            .query_row("SELECT COUNT(*) FROM dbsnp.rsid_aliases WHERE withdrawn = 1", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) > 0;

        Ok(ReferenceStatusDetails {
            clinvar_raw_found,
            clinvar_indexed_rows,
            clinvar_rsid_hits,
            clinvar_last_indexed,
            dbsnp_merged_raw_found,
            dbsnp_merge_mappings_indexed,
            dbsnp_rsids_normalized,
            dbsnp_merge_index_available: dbsnp_merge_mappings_indexed > 0,
            dbsnp_placement_index_available,
            orientation_verification_available,
        })
    })
    .await
    .map_err(|e| format!("Database query failed: {e}"))?
}
