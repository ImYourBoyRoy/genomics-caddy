// ./src-tauri/src/offline/sync.rs
use super::compress::{
    DEFAULT_COMPRESSION_THRESHOLD_BYTES, compress_if_large, resolve_local_asset_path,
};
use super::download::{download_to_path, head_remote, remote_changed, sha256_file};
use super::import_clingen::import_clingen_gene_validity;
use super::import_clinvar::import_clinvar_variant_summary;
use super::import_dbsnp::{import_dbsnp_merged, import_dbsnp_withdrawn};
use super::import_mane::import_mane_summary;
use super::import_pharmgkb::{import_pharmgkb_clinical_variants, import_pharmgkb_genes};
use super::manifest::{
    AssetKind, OfflineAssetDef, OfflineAssetId, OfflineAssetStatus, OfflineTierStatus, all_assets,
    asset_def, local_path, tier_budget_bytes,
};
use super::registry::{mark_update_available, read_registry, row_count_for_asset, upsert_registry};
use super::schema::migrate_offline_schema;
use super::tier2::{
    build_variant_locus_all_samples, build_variant_locus_for_sample, ensure_tier2_meta,
};
use crate::paths;
use crate::research::references::import_gwas_from_local_files;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

pub(crate) fn get_custom_download_dir_from_db(db_path: &Path) -> Option<PathBuf> {
    let conn = crate::db::connect(db_path).ok()?;
    // Ensure offline schema exists in case table hasn't migrated yet
    let _ = migrate_offline_schema(&conn);
    let mut stmt = conn
        .prepare("SELECT value FROM app_metadata WHERE key = 'custom_download_dir'")
        .ok()?;
    let val: Option<String> = stmt
        .query_row([], |row| row.get(0))
        .optional()
        .ok()
        .flatten();
    if let Some(s) = val {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineIndexedSummary {
    pub gwas_rows: u64,
    pub clinvar_rows: u64,
    pub variant_locus_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineUpdateCheck {
    pub tiers: Vec<OfflineTierStatus>,
    pub total_updates_available: u32,
    pub indexed_summary: OfflineIndexedSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineSyncResult {
    pub tier: u8,
    pub assets_synced: Vec<String>,
    pub messages: Vec<String>,
    pub errors: Vec<String>,
}

fn with_conn<T>(
    db_path: &Path,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    migrate_offline_schema(&conn).map_err(|e| e.to_string())?;
    f(&conn)
}

pub async fn check_offline_updates(
    data_dir: &Path,
    db_path: &Path,
) -> Result<OfflineUpdateCheck, String> {
    let data_dir = data_dir.to_path_buf();
    let db_path = db_path.to_path_buf();

    with_conn(&db_path, |_| Ok(()))?;

    let custom_dir = get_custom_download_dir_from_db(&db_path);

    let mut tiers: Vec<OfflineTierStatus> = Vec::new();
    let mut total_updates = 0u32;

    for tier in 0..=2 {
        let mut assets = Vec::new();
        let mut updates = 0u32;
        let mut ready_count = 0u32;
        let defs: Vec<&OfflineAssetDef> = all_assets().iter().filter(|a| a.tier == tier).collect();

        for def in defs {
            let path = local_path(&data_dir, custom_dir.as_deref(), def);
            let local_present = asset_local_present(&data_dir, custom_dir.as_deref(), def, &path);
            let local_bytes = resolve_local_asset_path(&path)
                .and_then(|local_path| std::fs::metadata(local_path).ok())
                .map(|metadata| metadata.len())
                .unwrap_or(0);

            let db = db_path.clone();
            let asset_id = def.id.as_str().to_string();
            let (row_count, reg) = tauri::async_runtime::spawn_blocking(move || {
                with_conn(&db, |conn| {
                    let reg = read_registry(conn, &asset_id);
                    let rows = row_count_for_asset(
                        conn,
                        OfflineAssetId::from_str_id(&asset_id)
                            .unwrap_or(OfflineAssetId::GwasCatalog),
                    );
                    Ok((rows, reg))
                })
            })
            .await
            .map_err(|e| format!("Status worker failed: {e}"))??;

            let mut update_available = reg.as_ref().map(|r| r.update_available).unwrap_or(false);
            let mut remote_len = None;
            if let Some(url) = def.url {
                if let Ok(head) = head_remote(url).await {
                    remote_len = head.content_length;
                    update_available = remote_changed(
                        &head,
                        reg.as_ref().and_then(|r| r.remote_etag.as_deref()),
                        reg.as_ref().and_then(|r| r.remote_last_modified.as_deref()),
                        reg.as_ref().and_then(|r| r.remote_content_length),
                    ) && local_present;
                }
                let db = db_path.clone();
                let aid = def.id.as_str().to_string();
                let flag = update_available;
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    with_conn(&db, |conn| mark_update_available(conn, &aid, flag))
                })
                .await;
            }

            if local_present && (row_count > 0 || def.kind != AssetKind::Derived) {
                ready_count += 1;
            }
            if update_available {
                updates += 1;
            }

            let message = if !local_present {
                "Not downloaded".to_string()
            } else if update_available {
                "Update available on server".to_string()
            } else if row_count > 0 {
                format!("{row_count} rows indexed")
            } else {
                "File present — import pending".to_string()
            };

            assets.push(OfflineAssetStatus {
                asset_id: def.id.as_str().to_string(),
                tier: def.tier,
                label: def.label.to_string(),
                local_present,
                local_bytes,
                row_count,
                synced_at: reg.as_ref().map(|r| r.synced_at),
                update_available,
                remote_content_length: remote_len,
                version_label: reg.and_then(|r| r.version_label),
                message,
                display_size: def.display_size.to_string(),
            });
        }

        let asset_len = assets.len().max(1) as u32;
        tiers.push(OfflineTierStatus {
            tier,
            ready: ready_count >= asset_len.saturating_sub(1),
            updates_available: updates,
            assets,
        });
        total_updates += updates;
    }

    let db = db_path.clone();
    let indexed_summary = tauri::async_runtime::spawn_blocking(move || {
        with_conn(&db, |conn| Ok(offline_status_summary(conn)))
    })
    .await
    .map_err(|e| format!("Summary worker failed: {e}"))??;

    Ok(OfflineUpdateCheck {
        tiers,
        total_updates_available: total_updates,
        indexed_summary: OfflineIndexedSummary {
            gwas_rows: indexed_summary.0,
            clinvar_rows: indexed_summary.1,
            variant_locus_rows: indexed_summary.2,
        },
    })
}

pub async fn sync_offline_assets_subset(
    data_dir: &Path,
    db_path: &Path,
    asset_ids: Vec<OfflineAssetId>,
    force: bool,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<OfflineSyncResult, String> {
    let custom_dir = get_custom_download_dir_from_db(db_path);
    let effective_dir = custom_dir.as_deref().unwrap_or(data_dir);
    paths::ensure_data_layout(effective_dir).map_err(|e| e.to_string())?;

    let tier = if let Some(first_id) = asset_ids.first() {
        asset_def(*first_id).map(|d| d.tier).unwrap_or(0)
    } else {
        0
    };

    let mut result = OfflineSyncResult {
        tier,
        assets_synced: Vec::new(),
        messages: Vec::new(),
        errors: Vec::new(),
    };

    let mut bytes_used = 0u64;
    let budget = tier_budget_bytes(tier);
    let mut pending_imports: Vec<OfflineAssetId> = Vec::new();

    for id in asset_ids {
        let Some(def) = asset_def(id) else { continue };
        match def.kind {
            AssetKind::RemoteFile => {
                let Some(url) = def.url else { continue };
                let path = local_path(data_dir, custom_dir.as_deref(), def);
                if !force && let Some(local_path) = resolve_local_asset_path(&path) {
                    if let Ok(meta) = std::fs::metadata(local_path) {
                        bytes_used += meta.len();
                    }
                    pending_imports.push(def.id);
                    continue;
                }
                if bytes_used >= budget {
                    result.errors.push(format!(
                        "{} skipped: tier {} byte budget exhausted",
                        def.label, tier
                    ));
                    continue;
                }

                // Clone values for the progress closure.
                let asset_id_str = def.id.as_str().to_string();
                let app_clone = app.cloned();
                let label = def.label.to_string();

                let progress_cb = move |bytes_done: u64, total: u64| {
                    if let Some(ref handle) = app_clone {
                        let _ = handle.emit(
                            "offline:download_progress",
                            serde_json::json!({
                                "asset_id": asset_id_str,
                                "label": label,
                                "bytes_downloaded": bytes_done,
                                "total_bytes": total,
                            }),
                        );
                    }
                };

                match download_to_path(url, &path, def.max_bytes, progress_cb).await {
                    Ok((n, head)) => {
                        bytes_used += n;
                        let hash = sha256_file(&path).ok();
                        let row_count = 0u64;
                        let _ = with_conn(db_path, |conn| {
                            upsert_registry(
                                conn,
                                def.id,
                                def.tier,
                                &path,
                                url,
                                Some(&head),
                                n,
                                hash.as_deref(),
                                row_count,
                                head.last_modified.as_deref(),
                                false,
                            )
                        });
                        pending_imports.push(def.id);
                        result
                            .messages
                            .push(format!("Downloaded {} ({} bytes)", def.label, n));
                    }
                    Err(e) => result
                        .errors
                        .push(format!("{} download failed: {}", def.label, e)),
                }
            }
            AssetKind::GwasSync => {
                if def.id == OfflineAssetId::GwasCatalog {
                    let path = local_path(data_dir, custom_dir.as_deref(), def);
                    if (force || !asset_local_present(data_dir, custom_dir.as_deref(), def, &path))
                        && let Some(url) = def.url
                    {
                        if bytes_used >= budget
                            && !asset_local_present(data_dir, custom_dir.as_deref(), def, &path)
                        {
                            result.errors.push(format!(
                                "GWAS catalog skipped: tier {} byte budget exhausted",
                                tier
                            ));
                        } else {
                            let asset_id_str = def.id.as_str().to_string();
                            let app_clone = app.cloned();
                            let label = def.label.to_string();
                            let progress_cb = move |bytes_done: u64, total: u64| {
                                if let Some(ref handle) = app_clone {
                                    let _ = handle.emit(
                                        "offline:download_progress",
                                        serde_json::json!({
                                            "asset_id": asset_id_str,
                                            "label": label,
                                            "bytes_downloaded": bytes_done,
                                            "total_bytes": total,
                                        }),
                                    );
                                }
                            };
                            match download_to_path(url, &path, def.max_bytes, progress_cb).await {
                                Ok((n, _)) => {
                                    bytes_used += n;
                                    result
                                        .messages
                                        .push(format!("Downloaded GWAS catalog ({} bytes)", n));
                                }
                                Err(e) => result.errors.push(format!("GWAS download: {e}")),
                            }
                        }
                    }
                }
                pending_imports.push(def.id);
            }
            AssetKind::Derived => {
                pending_imports.push(def.id);
            }
        }
    }

    let data_dir_owned = data_dir.to_path_buf();
    let db_path_owned = db_path.to_path_buf();
    let app_clone = app.cloned();
    let import_out = tauri::async_runtime::spawn_blocking(move || {
        import_assets_sync(
            &data_dir_owned,
            &db_path_owned,
            &pending_imports,
            sample_id,
            app_clone,
        )
    })
    .await
    .map_err(|e| format!("Import worker failed: {e}"))??;

    for msg in import_out.messages {
        result.messages.push(msg);
    }
    for err in import_out.errors {
        result.errors.push(err);
    }
    result.assets_synced = import_out.synced;
    Ok(result)
}

pub async fn sync_offline_tier(
    data_dir: &Path,
    db_path: &Path,
    tier: u8,
    force: bool,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<OfflineSyncResult, String> {
    let ids: Vec<OfflineAssetId> = all_assets()
        .iter()
        .filter(|a| a.tier == tier)
        .map(|a| a.id)
        .collect();
    sync_offline_assets_subset(data_dir, db_path, ids, force, sample_id, app).await
}

struct ImportBatchResult {
    synced: Vec<String>,
    messages: Vec<String>,
    errors: Vec<String>,
}

fn import_assets_sync(
    data_dir: &Path,
    db_path: &Path,
    ids: &[OfflineAssetId],
    sample_id: Option<i64>,
    app: Option<tauri::AppHandle>,
) -> Result<ImportBatchResult, String> {
    let mut out = ImportBatchResult {
        synced: Vec::new(),
        messages: Vec::new(),
        errors: Vec::new(),
    };
    with_conn(db_path, |conn| {
        for id in ids {
            match import_asset_sync(conn, data_dir, db_path, *id, sample_id, app.as_ref()) {
                Ok(msg) => {
                    out.synced.push(id.as_str().to_string());
                    out.messages.push(msg);
                }
                Err(e) => out.errors.push(e),
            }
        }
        Ok(())
    })?;
    Ok(out)
}

pub async fn build_tier2_for_sample(
    data_dir: &Path,
    db_path: &Path,
    sample_id: i64,
    app: Option<&AppHandle>,
) -> Result<OfflineSyncResult, String> {
    sync_offline_assets_subset(
        data_dir,
        db_path,
        vec![OfflineAssetId::Tier2VariantLocus],
        false,
        Some(sample_id),
        app,
    )
    .await
}

/// Sync a single asset by id (force controls whether to re-download existing files).
pub async fn sync_single_asset(
    data_dir: &Path,
    db_path: &Path,
    asset_id: &str,
    force: bool,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<OfflineSyncResult, String> {
    let ids = match asset_id {
        "pharmgkb_clinical_variants" => vec![
            OfflineAssetId::PharmgkbClinicalVariants,
            OfflineAssetId::PharmgkbGenes,
            OfflineAssetId::ClingenGeneValidity,
            OfflineAssetId::ManeSelectSummary,
        ],
        "dbsnp_merged_json" => vec![
            OfflineAssetId::DbsnpMergedJson,
            OfflineAssetId::DbsnpWithdrawnJson,
        ],
        other => {
            let id = OfflineAssetId::from_str_id(other)
                .ok_or_else(|| format!("Unknown asset id: {other}"))?;
            vec![id]
        }
    };
    sync_offline_assets_subset(data_dir, db_path, ids, force, sample_id, app).await
}

/// Sync all tiers, downloading only missing or outdated assets. Does not force re-download.
pub async fn sync_all_missing(
    data_dir: &Path,
    db_path: &Path,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<Vec<OfflineSyncResult>, String> {
    let mut results = Vec::new();
    for tier in 0u8..=2 {
        results.push(sync_offline_tier(data_dir, db_path, tier, false, sample_id, app).await?);
    }
    Ok(results)
}

fn import_asset_sync(
    conn: &Connection,
    data_dir: &Path,
    db_path: &Path,
    id: OfflineAssetId,
    sample_id: Option<i64>,
    app: Option<&tauri::AppHandle>,
) -> Result<String, String> {
    let custom_dir = get_custom_download_dir_from_db(db_path);
    let effective_dir = custom_dir.as_deref().unwrap_or(data_dir);
    let def = asset_def(id).ok_or_else(|| format!("Unknown asset {:?}", id))?;
    let path = local_path(data_dir, custom_dir.as_deref(), def);

    let row_count = match id {
        OfflineAssetId::GwasCatalog => import_gwas_from_local_files(effective_dir, db_path)?,
        OfflineAssetId::LiftoverChain => {
            if !path.exists() {
                return Err("Liftover chain missing — sync Tier 0".into());
            }
            let n = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let hash = sha256_file(&path).ok();
            upsert_registry(
                conn,
                id,
                def.tier,
                &path,
                def.url.unwrap_or(""),
                None,
                n,
                hash.as_deref(),
                1,
                None,
                false,
            )?;
            1
        }
        OfflineAssetId::GnomadIndexManifest => {
            if path.exists() {
                upsert_registry(
                    conn,
                    id,
                    def.tier,
                    &path,
                    "local",
                    None,
                    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                    None,
                    1,
                    None,
                    false,
                )?;
            }
            1
        }
        OfflineAssetId::ClinvarVariantSummary => {
            let data_dir = path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| Path::new("."));
            crate::db::ensure_catalog_db_attached(conn, data_dir, "clinvar")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "ClinVar file missing — run Tier 1 sync".to_string())?;
            import_clinvar_variant_summary(conn, &eff_path, app)?
        }
        OfflineAssetId::PharmgkbClinicalVariants => {
            if !path.exists() {
                return Err("PharmGKB clinical zip missing".into());
            }
            import_pharmgkb_clinical_variants(conn, &path)?
        }
        OfflineAssetId::PharmgkbGenes => {
            if !path.exists() {
                return Err("PharmGKB genes zip missing".into());
            }
            import_pharmgkb_genes(conn, &path)?
        }
        OfflineAssetId::ClingenGeneValidity => {
            let eff_path =
                resolve_local_asset_path(&path).ok_or_else(|| "ClinGen CSV missing".to_string())?;
            import_clingen_gene_validity(conn, &eff_path)?
        }
        OfflineAssetId::ManeSelectSummary => {
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "MANE summary missing".to_string())?;
            import_mane_summary(conn, &eff_path)?
        }
        OfflineAssetId::DbsnpMergedJson => {
            let data_dir = path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| Path::new("."));
            crate::db::ensure_catalog_db_attached(conn, data_dir, "dbsnp")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "dbSNP merged JSON missing".to_string())?;
            import_dbsnp_merged(conn, &eff_path, app)?
        }
        OfflineAssetId::DbsnpWithdrawnJson => {
            let data_dir = path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| Path::new("."));
            crate::db::ensure_catalog_db_attached(conn, data_dir, "dbsnp")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "dbSNP withdrawn JSON missing".to_string())?;
            import_dbsnp_withdrawn(conn, &eff_path, app)?
        }
        OfflineAssetId::Tier2VariantLocus => {
            ensure_tier2_meta(effective_dir)?;
            let meta_path = local_path(data_dir, custom_dir.as_deref(), def);
            let count = if let Some(sid) = sample_id {
                build_variant_locus_for_sample(conn, sid)?
            } else {
                build_variant_locus_all_samples(conn)?
            };
            upsert_registry(
                conn, id, def.tier, &meta_path, "derived", None, 0, None, count, None, false,
            )?;
            count
        }
    };

    if id != OfflineAssetId::Tier2VariantLocus && id != OfflineAssetId::GnomadIndexManifest {
        let final_path = if let Some(local_path) = resolve_local_asset_path(&path) {
            compress_if_large(&local_path, DEFAULT_COMPRESSION_THRESHOLD_BYTES)?
        } else {
            path.clone()
        };
        let n = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
        let hash = sha256_file(&final_path).ok();
        let _ = upsert_registry(
            conn,
            id,
            def.tier,
            &final_path,
            def.url.unwrap_or("derived"),
            None,
            n,
            hash.as_deref(),
            row_count_for_asset(conn, id),
            None,
            false,
        );
    }

    Ok(format!("{}: {row_count} rows loaded", def.label))
}

fn asset_local_present(
    data_dir: &Path,
    custom_dir: Option<&Path>,
    def: &OfflineAssetDef,
    path: &Path,
) -> bool {
    let base = custom_dir.unwrap_or(data_dir);
    resolve_local_asset_path(path).is_some()
        || (def.id == OfflineAssetId::GwasCatalog
            && (base
                .join("references")
                .join("gwas-catalog-associations_ontology-annotated.tsv")
                .exists()
                || base
                    .join("references")
                    .join("gwas-catalog-associations_ontology-annotated.tsv.gz")
                    .exists()
                || base
                    .join("references")
                    .join("gwas-catalog-associations_ontology-annotated-full.zip")
                    .exists()))
}

pub fn offline_status_summary(conn: &Connection) -> (u64, u64, u64) {
    let clinvar = row_count_for_asset(conn, OfflineAssetId::ClinvarVariantSummary);
    let gwas = row_count_for_asset(conn, OfflineAssetId::GwasCatalog);
    let locus = row_count_for_asset(conn, OfflineAssetId::Tier2VariantLocus);
    (gwas, clinvar, locus)
}
