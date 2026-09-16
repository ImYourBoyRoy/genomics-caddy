// ./src-tauri/src/research/gnomad/commands.rs
use super::batch::{batch_enrich_gnomad_context, prefetch_gnomad_batch};
use super::cache::clear_gnomad_cache;
use super::config::{load_effective_gnomad_config, load_gnomad_config, save_gnomad_config};
use super::lookup::get_gnomad_context;
use super::readiness::{download_missing_gnomad_indexes, get_gnomad_readiness};
use super::types::{
    GnomadBatchProgress, GnomadBatchRequest, GnomadConfig, GnomadContext, GnomadIndexSyncResult,
    GnomadLookupRequest, GnomadReadinessStatus, GnomadSourceMode, GnomadSourceTestResult,
};
use super::validate::test_gnomad_source_urls;
use crate::db_runtime;
use crate::{get_data_dir, get_db_path};
use tauri::AppHandle;

#[tauri::command]
pub async fn get_gnomad_context_cmd(
    app: AppHandle,
    request: GnomadLookupRequest,
) -> Result<GnomadContext, String> {
    let db_path = get_db_path(&app);
    let data_dir = get_data_dir(&app);
    Ok(get_gnomad_context(&db_path, &data_dir, request).await)
}

#[tauri::command]
pub async fn batch_enrich_gnomad_context_cmd(
    app: AppHandle,
    request: GnomadBatchRequest,
) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let data_dir = get_data_dir(&app);
    let (progress, results) = batch_enrich_gnomad_context(&db_path, &data_dir, request).await;
    Ok(serde_json::json!({
        "progress": progress,
        "results": results.into_iter().map(|(rsid, ctx)| serde_json::json!({ "rsid": rsid, "context": ctx })).collect::<Vec<_>>(),
    }))
}

#[tauri::command]
pub async fn get_gnomad_config_cmd(app: AppHandle) -> Result<GnomadConfig, String> {
    load_effective_gnomad_config(&get_db_path(&app), &get_data_dir(&app)).await
}

#[tauri::command]
pub async fn save_gnomad_config_cmd(app: AppHandle, config: GnomadConfig) -> Result<(), String> {
    db_runtime::with_connection(get_db_path(&app), move |conn| {
        save_gnomad_config(conn, &config)
    })
    .await
}

#[tauri::command]
pub async fn test_gnomad_source_urls_cmd(app: AppHandle) -> Result<GnomadSourceTestResult, String> {
    let cfg = load_effective_gnomad_config(&get_db_path(&app), &get_data_dir(&app)).await?;
    Ok(test_gnomad_source_urls(&cfg).await)
}

#[tauri::command]
pub async fn get_gnomad_readiness_cmd(app: AppHandle) -> Result<GnomadReadinessStatus, String> {
    let db_path = get_db_path(&app);
    let data_dir = get_data_dir(&app);
    let cfg = load_effective_gnomad_config(&db_path, &data_dir).await?;
    Ok(get_gnomad_readiness(&cfg, &data_dir, &db_path).await)
}

#[tauri::command]
pub async fn download_gnomad_indexes_cmd(app: AppHandle) -> Result<GnomadIndexSyncResult, String> {
    let data_dir = get_data_dir(&app);
    let cfg = load_effective_gnomad_config(&get_db_path(&app), &data_dir).await?;
    Ok(download_missing_gnomad_indexes(&cfg, &data_dir).await)
}

/// Refresh only the profile's report markers against the effective current
/// gnomAD release. Older cache rows remain retained for diagnostics but are
/// never promoted into a current report without a fresh lookup.
#[tauri::command]
pub async fn refresh_gnomad_frequency_cache_cmd(
    app: AppHandle,
    sample_id: i64,
    rsids: Vec<String>,
) -> Result<GnomadBatchProgress, String> {
    let candidates: Vec<String> = rsids
        .into_iter()
        .map(|rsid| rsid.trim().to_string())
        .filter(|rsid| !rsid.is_empty())
        .collect();
    if candidates.is_empty() {
        return Err("There are no report markers to refresh.".into());
    }
    let db_path = get_db_path(&app);
    let data_dir = get_data_dir(&app);
    Ok(prefetch_gnomad_batch(&db_path, &data_dir, sample_id, &candidates).await)
}

#[tauri::command]
pub async fn select_gnomad_local_dir_cmd(app: AppHandle) -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Choose gnomAD local VCF folder")
        .pick_folder();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path_str = path.to_string_lossy().to_string();
    db_runtime::with_connection(get_db_path(&app), move |conn| {
        let mut cfg = load_gnomad_config(conn)?;
        cfg.local_vcf_dir = Some(path_str.clone());
        cfg.source_mode = GnomadSourceMode::LocalIndexedVcf;
        save_gnomad_config(conn, &cfg)?;
        Ok(Some(path_str))
    })
    .await
}

#[tauri::command]
pub async fn clear_gnomad_cache_cmd(app: AppHandle) -> Result<usize, String> {
    db_runtime::with_connection(get_db_path(&app), clear_gnomad_cache).await
}
