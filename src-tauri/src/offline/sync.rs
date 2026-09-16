// ./src-tauri/src/offline/sync.rs
use super::compress::{
    DEFAULT_COMPRESSION_THRESHOLD_BYTES, compress_if_large, resolve_local_asset_path,
};
use super::download::{
    download_to_path, head_remote, probe_remote, remote_changed, sha256_file,
    verify_local_hash_sidecar, RemoteProbeMethod,
};
use super::import_clingen::import_clingen_gene_validity;
use super::import_clinvar::import_clinvar_variant_summary;
use super::import_dbsnp::{import_dbsnp_merged, import_dbsnp_withdrawn};
use super::import_mane::import_mane_summary;
use super::import_pharmgkb::{import_pharmgkb_clinical_variants, import_pharmgkb_genes};
use super::manifest::{
    AssetKind, OfflineAssetDef, OfflineAssetId, OfflineAssetStatus, OfflineTierStatus, all_assets,
    asset_def, local_path, tier_budget_bytes,
};
use super::registry::{
    clear_update_flags, mark_update_available, read_registry, row_count_for_asset,
    store_remote_content_length, upsert_registry,
};
use super::schema::migrate_offline_schema;
use super::tier2::{
    build_variant_locus_all_samples, build_variant_locus_for_sample_in_data_dir, ensure_tier2_meta,
};
use crate::paths;
use crate::research::references::import_gwas_from_local_files;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinSet;

/// Cooperative cancel for long catalog imports (dbSNP / ClinVar).
static IMPORT_CANCEL: AtomicBool = AtomicBool::new(false);

pub fn reset_offline_import_cancel() {
    IMPORT_CANCEL.store(false, Ordering::SeqCst);
}

pub fn cancel_offline_import() {
    IMPORT_CANCEL.store(true, Ordering::SeqCst);
}

pub fn is_offline_import_cancelled() -> bool {
    IMPORT_CANCEL.load(Ordering::SeqCst)
}

/// Serializes SQLite import work so downloads can run in parallel safely.
fn import_mutex() -> &'static AsyncMutex<()> {
    static LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| AsyncMutex::new(()))
}

/// Sync phases are download (1) then import (2). There is no separate finalize step.
const SYNC_TOTAL_STEPS: u8 = 2;

fn emit_sync_phase(
    app: Option<&AppHandle>,
    asset_id: &str,
    phase: &str,
    step: u8,
    message: &str,
) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "offline:sync_phase",
            serde_json::json!({
                "asset_id": asset_id,
                "phase": phase,
                "step": step,
                "total_steps": SYNC_TOTAL_STEPS,
                "message": message,
            }),
        );
    }
}

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

/// Resolve the canonical on-disk liftover path, including a configured custom
/// download directory, so legacy commands and offline sync inspect the same asset.
pub fn liftover_chain_path(data_dir: &Path, db_path: &Path) -> PathBuf {
    let custom_dir = get_custom_download_dir_from_db(db_path);
    let definition = asset_def(OfflineAssetId::LiftoverChain)
        .expect("canonical liftover asset must exist in the offline manifest");
    super::manifest::local_path(data_dir, custom_dir.as_deref(), definition)
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineIndexedSummary {
    pub gwas_rows: u64,
    pub clinvar_rows: u64,
    pub variant_locus_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineUpdateCheck {
    pub checked_at: i64,
    pub tiers: Vec<OfflineTierStatus>,
    pub total_updates_available: u32,
    pub indexed_summary: OfflineIndexedSummary,
    pub remote_check: OfflineRemoteCheckStatus,
    pub runtime_artifacts: OfflineRuntimeArtifactStatus,
}

/// Aggregate status for generated runtime artifacts.
///
/// This deliberately reports counts and bytes only. It does not expose paths,
/// filenames, cache keys, sample data, or genotype values. SQLite WAL/SHM
/// files are reported as sidecars, not as stale locks: they may be expected
/// while the app is open and must not be deleted by an update check.
#[derive(Debug, Clone, Serialize)]
pub struct OfflineRuntimeArtifactStatus {
    pub partial_download_files: u32,
    pub partial_download_bytes: u64,
    pub sqlite_sidecar_files: u32,
    pub rebuildable_cache_rows: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfflineRemoteCheckStatus {
    pub assets_checked: u32,
    pub assets_failed: u32,
    pub head_fallbacks: u32,
    pub timed_out: bool,
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

fn format_byte_size(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = n as f64;
    if n >= GB {
        format!("{:.2} GB", n / GB)
    } else if n >= MB {
        format!("{:.2} MB", n / MB)
    } else if n >= KB {
        format!("{:.2} KB", n / KB)
    } else {
        format!("{n} B")
    }
}

fn display_size_for(def: &OfflineAssetDef, remote_len: Option<u64>) -> String {
    match remote_len {
        Some(n) if n > 0 => format!("~{}", format_byte_size(n)),
        _ => def.display_size.to_string(),
    }
}

/// Returns true when a catalog has rows but its meaning-bearing columns are
/// empty. This catches a silent schema drift that a row-count-only freshness
/// check cannot see (for example, ClinPGx rows imported without chemicals or
/// phenotypes).
fn asset_semantic_refresh_required(conn: &Connection, asset_id: OfflineAssetId) -> bool {
    if asset_id != OfflineAssetId::PharmgkbClinicalVariants {
        return false;
    }
    let table = if super::schema::schema_attached(conn, "pharmgkb") {
        "pharmgkb.pharmgkb_clinical_variants"
    } else {
        "reference.pharmgkb_clinical_variants"
    };
    conn.query_row(
        &format!(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN trim(COALESCE(drug, '')) <> ''
             OR trim(COALESCE(phenotype, '')) <> '' THEN 1 ELSE 0 END), 0)
             FROM {table}"
        ),
        [],
        |row| {
            let rows: i64 = row.get(0)?;
            let semantic_rows: i64 = row.get(1)?;
            Ok(rows > 0 && semantic_rows == 0)
        },
    )
    .unwrap_or(false)
}

const APP_OWNED_SQLITE_DATABASES: &[&str] = &[
    "api_cache.db",
    "genomics_reference.db",
    "clinvar.db",
    "dbsnp.db",
    "gwas.db",
    "pharmgkb.db",
    "clingen.db",
    "mane.db",
];

fn add_partial_download(path: &Path, files: &mut u32, bytes: &mut u64) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if !metadata.is_file() {
        return;
    }
    *files = files.saturating_add(1);
    *bytes = bytes.saturating_add(metadata.len());
}

/// Scan only the known reference-download directories, with a fixed depth.
/// In particular, this never walks `samples/`, `exports/`, or the data root.
fn scan_partial_downloads(dir: &Path, remaining_depth: u8, files: &mut u32, bytes: &mut u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".part"))
            {
                add_partial_download(&path, files, bytes);
            }
        } else if file_type.is_dir() && remaining_depth > 0 {
            scan_partial_downloads(&path, remaining_depth - 1, files, bytes);
        }
    }
}

fn collect_runtime_artifacts(
    data_dir: &Path,
    custom_dir: Option<&Path>,
    conn: &Connection,
) -> OfflineRuntimeArtifactStatus {
    let mut bases = vec![data_dir.to_path_buf()];
    if let Some(custom) = custom_dir
        && custom != data_dir
    {
        bases.push(custom.to_path_buf());
    }

    let mut partial_download_files = 0u32;
    let mut partial_download_bytes = 0u64;
    for base in bases {
        // The chain is the only known root-level download target.
        add_partial_download(
            &base.join("GRCh37_to_GRCh38.chain.gz.part"),
            &mut partial_download_files,
            &mut partial_download_bytes,
        );
        // GWAS downloads live directly in references; other catalog downloads
        // live one directory below raw_downloads/<category>/.
        scan_partial_downloads(
            &base.join("references"),
            0,
            &mut partial_download_files,
            &mut partial_download_bytes,
        );
        scan_partial_downloads(
            &base.join("raw_downloads"),
            1,
            &mut partial_download_files,
            &mut partial_download_bytes,
        );
        scan_partial_downloads(
            &base.join("gnomad_indexes"),
            0,
            &mut partial_download_files,
            &mut partial_download_bytes,
        );
    }

    let sqlite_sidecar_files = APP_OWNED_SQLITE_DATABASES
        .iter()
        .flat_map(|database| [format!("{database}-wal"), format!("{database}-shm")])
        .map(|suffix| data_dir.join(suffix))
        .filter(|path| path.is_file())
        .count() as u32;

    let rebuildable_cache_rows = if crate::offline::schema::schema_attached(conn, "api_cache_db") {
        crate::offline::schema::table_count(conn, "api_cache_db.api_cache")
            + crate::offline::schema::table_count(conn, "api_cache_db.api_cache_entries")
    } else {
        0
    };

    OfflineRuntimeArtifactStatus {
        partial_download_files,
        partial_download_bytes,
        sqlite_sidecar_files,
        rebuildable_cache_rows,
    }
}

/// Best-effort parallel HEAD probes for assets missing a stored remote size.
/// Failures are ignored — UI falls back to the static estimate.
async fn refresh_missing_remote_sizes(db_path: &Path) {
    let mut missing: Vec<(OfflineAssetId, u8, String)> = Vec::new();
    let _ = with_conn(db_path, |conn| {
        for def in all_assets() {
            if def.url.is_none() || def.kind == AssetKind::Derived {
                continue;
            }
            let reg = read_registry(conn, def.id.as_str());
            let has_len = reg
                .as_ref()
                .and_then(|r| r.remote_content_length)
                .is_some_and(|n| n > 0);
            if !has_len
                && let Some(url) = def.url
            {
                missing.push((def.id, def.tier, url.to_string()));
            }
        }
        Ok(())
    });
    if missing.is_empty() {
        return;
    }

    let mut handles = JoinSet::new();
    for (id, tier, url) in missing {
        handles.spawn(async move {
            let head = head_remote(&url).await.ok()?;
            let len = head.content_length.filter(|n| *n > 0)?;
            Some((id, tier, url, len))
        });
    }

    let db_path = db_path.to_path_buf();
    while let Some(result) = handles.join_next().await {
        if let Ok(Some((id, tier, url, len))) = result {
            let _ = with_conn(&db_path, |conn| {
                store_remote_content_length(conn, id, tier, &url, len)
            });
        }
    }
}

struct RemoteProbeSummary {
    assets_checked: u32,
    assets_failed: u32,
    head_fallbacks: u32,
}

/// Compare local registry metadata to remote identity metadata for already-downloaded assets.
async fn probe_remote_updates(data_dir: &Path, db_path: &Path) -> RemoteProbeSummary {
    struct UpdateProbe {
        id: OfflineAssetId,
        url: String,
        etag: Option<String>,
        last_modified: Option<String>,
        content_length: Option<i64>,
        version_label: Option<String>,
    }

    let custom_dir = get_custom_download_dir_from_db(db_path);
    let mut probes: Vec<UpdateProbe> = Vec::new();
    let _ = with_conn(db_path, |conn| {
        for def in all_assets() {
            let Some(url) = def.url else { continue };
            let path = local_path(data_dir, custom_dir.as_deref(), def);
            if !asset_local_present(data_dir, custom_dir.as_deref(), def, &path) {
                continue;
            }
            let reg = read_registry(conn, def.id.as_str());
            probes.push(UpdateProbe {
                id: def.id,
                url: url.to_string(),
                etag: reg.as_ref().and_then(|r| r.remote_etag.clone()),
                last_modified: reg.as_ref().and_then(|r| r.remote_last_modified.clone()),
                content_length: reg.as_ref().and_then(|r| r.remote_content_length),
                version_label: reg.as_ref().and_then(|r| r.version_label.clone()),
            });
        }
        Ok(())
    });
    if probes.is_empty() {
        return RemoteProbeSummary {
            assets_checked: 0,
            assets_failed: 0,
            head_fallbacks: 0,
        };
    }

    let mut handles = JoinSet::new();
    for probe in probes {
        handles.spawn(async move {
            let remote = probe_remote(&probe.url).await.ok()?;
            let newer = remote_changed(
                &remote.head,
                probe.etag.as_deref(),
                probe.last_modified.as_deref(),
                probe.content_length,
                probe.version_label.as_deref(),
            );
            Some((
                probe.id,
                newer,
                remote.head.content_length,
                remote.method,
                probe.url,
            ))
        });
    }

    let db_path = db_path.to_path_buf();
    let assets_checked = handles.len() as u32;
    let mut assets_succeeded = 0u32;
    let mut head_fallbacks = 0u32;
    while let Some(result) = handles.join_next().await {
        if let Ok(Some((id, newer, content_length, method, url))) = result {
            if method == RemoteProbeMethod::RangeGetFallback {
                head_fallbacks += 1;
            }
            let persisted = with_conn(&db_path, |conn| {
                if let Some(len) = content_length.filter(|n| *n > 0)
                    && let Some(def) = asset_def(id)
                {
                    store_remote_content_length(conn, id, def.tier, &url, len)?;
                }
                // Always write the probe result so stale false-positives clear.
                mark_update_available(conn, id.as_str(), newer)
            });
            if persisted.is_ok() {
                assets_succeeded += 1;
            }
        }
    }
    RemoteProbeSummary {
        assets_checked,
        assets_failed: assets_checked.saturating_sub(assets_succeeded),
        head_fallbacks,
    }
}

pub async fn check_offline_updates(
    data_dir: &Path,
    db_path: &Path,
) -> Result<OfflineUpdateCheck, String> {
    let data_dir = data_dir.to_path_buf();
    let db_path = db_path.to_path_buf();

    with_conn(&db_path, |conn| {
        clear_update_flags(conn)?;
        Ok(())
    })?;

    // Probe before returning so the frontend receives one authoritative status
    // snapshot instead of stale flags from a previous check. Missing-size
    // enrichment is best effort and must not turn an otherwise authoritative
    // update probe into a timeout.
    let remote_probe_result = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        probe_remote_updates(&data_dir, &db_path),
    )
    .await;
    let (remote_probe, remote_check_timed_out) = match remote_probe_result {
        Ok(summary) => (summary, false),
        Err(_) => (
            RemoteProbeSummary {
                assets_checked: 0,
                assets_failed: 0,
                head_fallbacks: 0,
            },
            true,
        ),
    };

    // Size lookup is display metadata only. Bound it separately so a slow
    // missing resource cannot make the authoritative update state ambiguous.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(4),
        refresh_missing_remote_sizes(&db_path),
    )
    .await;

    let custom_dir = get_custom_download_dir_from_db(&db_path);

    // --- Batch all per-asset DB reads into a single spawn_blocking call ---
    // Previously N serial spawn_blocking awaits (one per asset); now one call
    // that reads all registry rows and row counts in a single SQLite transaction.
    struct AssetDbRow {
        #[allow(dead_code)]
        asset_id: String,
        asset_enum: OfflineAssetId,
        #[allow(dead_code)]
        tier: u8,
        row_count: u64,
        semantic_refresh_required: bool,
        reg: Option<crate::offline::registry::RegistryRow>,
    }

    let db = db_path.clone();
    let data_dir_for_count = data_dir.clone();
    let all_defs_snapshot: Vec<(String, OfflineAssetId, u8)> = all_assets()
        .iter()
        .map(|d| (d.id.as_str().to_string(), d.id, d.tier))
        .collect();

    let db_rows: Vec<AssetDbRow> = tauri::async_runtime::spawn_blocking(move || {
        with_conn(&db, |conn| {
            let mut rows = Vec::new();
            for (asset_id, asset_enum, tier) in &all_defs_snapshot {
                let reg = read_registry(conn, asset_id);
                let row_count = if *asset_enum == OfflineAssetId::Tier2VariantLocus {
                    crate::offline::tier2::count_variant_locus_rows(&data_dir_for_count, conn)
                } else {
                    row_count_for_asset(conn, *asset_enum)
                };
                rows.push(AssetDbRow {
                    asset_id: asset_id.clone(),
                    asset_enum: *asset_enum,
                    tier: *tier,
                    row_count,
                    semantic_refresh_required: asset_semantic_refresh_required(conn, *asset_enum),
                    reg,
                });
            }
            Ok(rows)
        })
    })
    .await
    .map_err(|e| format!("Status batch worker failed: {e}"))??;

    let mut tiers: Vec<OfflineTierStatus> = Vec::new();
    let mut total_updates = 0u32;

    for tier in 0..=2u8 {
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

            // Retrieve the pre-fetched row from the batch read.
            let db_row = db_rows.iter().find(|r| r.asset_enum == def.id);
            let (row_count, semantic_refresh_required, reg) = db_row
                .map(|r| (r.row_count, r.semantic_refresh_required, r.reg.clone()))
                .unwrap_or((0, false, None));

            let mut update_available = semantic_refresh_required
                || (local_present
                    && reg.as_ref().map(|r| r.update_available).unwrap_or(false));
            // Ignore stale flags that were set from Content-Length-only probes
            // (no ETag / Last-Modified baseline means we cannot prove an update).
            let has_identity = reg.as_ref().is_some_and(|r| {
                r.remote_etag
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|s| !s.is_empty())
                    || r.remote_last_modified
                        .as_deref()
                        .map(str::trim)
                        .is_some_and(|s| !s.is_empty())
            });
            if update_available && !semantic_refresh_required && !has_identity {
                update_available = false;
            }
            let remote_len = reg.as_ref().and_then(|r| r.remote_content_length.map(|n| n as u64));

            if local_present && (row_count > 0 || def.kind != AssetKind::Derived) {
                ready_count += 1;
            }
            if update_available {
                updates += 1;
            }

            let message = if semantic_refresh_required {
                "Refresh required: ClinPGx annotations need re-import".to_string()
            } else if !local_present {
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
                display_size: display_size_for(def, remote_len),
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
    let data_dir_for_summary = data_dir.clone();
    let indexed_summary = tauri::async_runtime::spawn_blocking(move || {
        with_conn(&db, |conn| Ok(offline_status_summary(conn, &data_dir_for_summary)))
    })
    .await
    .map_err(|e| format!("Summary worker failed: {e}"))??;

    let db_for_artifacts = db_path.clone();
    let data_dir_for_artifacts = data_dir.clone();
    let custom_dir_for_artifacts = custom_dir.clone();
    let runtime_artifacts = tauri::async_runtime::spawn_blocking(move || {
        with_conn(&db_for_artifacts, |conn| {
            Ok(collect_runtime_artifacts(
                &data_dir_for_artifacts,
                custom_dir_for_artifacts.as_deref(),
                conn,
            ))
        })
    })
    .await
    .map_err(|e| format!("Runtime artifact worker failed: {e}"))??;

    Ok(OfflineUpdateCheck {
        checked_at: crate::research::util::unix_now(),
        tiers,
        total_updates_available: total_updates,
        indexed_summary: OfflineIndexedSummary {
            gwas_rows: indexed_summary.0,
            clinvar_rows: indexed_summary.1,
            variant_locus_rows: indexed_summary.2,
        },
        remote_check: OfflineRemoteCheckStatus {
            assets_checked: remote_probe.assets_checked,
            assets_failed: remote_probe.assets_failed,
            head_fallbacks: remote_probe.head_fallbacks,
            timed_out: remote_check_timed_out,
        },
        runtime_artifacts,
    })
}

pub async fn sync_offline_assets_subset(
    data_dir: &Path,
    db_path: &Path,
    asset_ids: Vec<OfflineAssetId>,
    force: bool,
    // When true (Sync All Missing / Sync Tier without force), skip assets that
    // are already on disk, not outdated, and indexed. Single-asset Re-sync
    // passes false so local re-import still runs.
    skip_if_current: bool,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<OfflineSyncResult, String> {
    reset_offline_import_cancel();
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

    emit_sync_phase(
        app,
        "__bulk__",
        "bulk",
        1,
        &format!("Sync · preparing tier {tier} ({} asset(s))…", asset_ids.len()),
    );

    for id in asset_ids {
        let Some(def) = asset_def(id) else { continue };
        emit_sync_phase(
            app,
            def.id.as_str(),
            "check",
            1,
            &format!("Checking {}…", def.label),
        );
        match def.kind {
            AssetKind::RemoteFile => {
                let Some(url) = def.url else { continue };
                let path = local_path(data_dir, custom_dir.as_deref(), def);
                if !force && let Some(existing) = resolve_local_asset_path(&path) {
                    // Quick sync-time update probe (not used during sidebar status).
                    let reg = with_conn(db_path, |conn| Ok(read_registry(conn, def.id.as_str()))).ok().flatten();
                    let semantic_refresh_required = with_conn(db_path, |conn| {
                        Ok(asset_semantic_refresh_required(conn, def.id))
                    })
                    .unwrap_or(false);
                    let remote_is_newer = match head_remote(url).await {
                        Ok(head) => {
                            let newer = remote_changed(
                                &head,
                                reg.as_ref().and_then(|r| r.remote_etag.as_deref()),
                                reg.as_ref().and_then(|r| r.remote_last_modified.as_deref()),
                                reg.as_ref().and_then(|r| r.remote_content_length),
                                reg.as_ref().and_then(|r| r.version_label.as_deref()),
                            );
                            let _ = with_conn(db_path, |conn| {
                                mark_update_available(conn, def.id.as_str(), newer)
                            });
                            newer
                        }
                        Err(_) => false,
                    };
                    if !remote_is_newer {
                        if let Ok(meta) = std::fs::metadata(existing) {
                            bytes_used += meta.len();
                        }
                        let indexed_rows = with_conn(db_path, |conn| {
                            Ok(row_count_for_asset(conn, def.id))
                        })
                        .unwrap_or(0);
                        // File-only assets (chain / manifest) are done once present + not newer.
                        let file_only = matches!(
                            def.id,
                            OfflineAssetId::LiftoverChain | OfflineAssetId::GnomadIndexManifest
                        );
                        if skip_if_current
                            && !semantic_refresh_required
                            && (file_only || indexed_rows > 0)
                        {
                            emit_sync_phase(
                                app,
                                def.id.as_str(),
                                "skip",
                                2,
                                &format!("{} up to date — skipping", def.label),
                            );
                            result
                                .messages
                                .push(format!("{} already up to date", def.label));
                            continue;
                        }
                        emit_sync_phase(
                            app,
                            def.id.as_str(),
                            "import",
                            2,
                            &format!(
                                "Step 2/2 · Indexing local {}…",
                                def.label
                            ),
                        );
                        pending_imports.push(def.id);
                        continue;
                    }
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
                emit_sync_phase(
                    app,
                    def.id.as_str(),
                    "download",
                    1,
                    &format!("Step 1/2 · Downloading {label}…"),
                );

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
                        if let Err(e) = with_conn(db_path, |conn| {
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
                                head
                                    .content_filename
                                    .as_deref()
                                    .or(head.last_modified.as_deref()),
                                false,
                            )
                        }) {
                            result.errors.push(format!(
                                "{} downloaded but its local status could not be recorded: {e}",
                                def.label
                            ));
                            continue;
                        }
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
                    let present = asset_local_present(data_dir, custom_dir.as_deref(), def, &path);
                    // GWAS has a dedicated sync kind because its importer handles
                    // several source-file shapes. It still needs the same update
                    // decision as ordinary remote assets: an installed catalog
                    // marked with a proven newer remote identity must download
                    // before the local catalog is re-indexed.
                    let registry_update_available = with_conn(db_path, |conn| {
                        Ok(read_registry(conn, def.id.as_str()))
                    })
                    .ok()
                    .flatten()
                    .is_some_and(|registry| registry.update_available);
                    if gwas_download_required(force, present, registry_update_available)
                        && let Some(url) = def.url
                    {
                        if bytes_used >= budget && !present {
                            result.errors.push(format!(
                                "GWAS catalog skipped: tier {} byte budget exhausted",
                                tier
                            ));
                        } else {
                            let asset_id_str = def.id.as_str().to_string();
                            let app_clone = app.cloned();
                            let label = def.label.to_string();
                            emit_sync_phase(
                                app,
                                def.id.as_str(),
                                "download",
                                1,
                                "Step 1/2 · Downloading GWAS catalog…",
                            );
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
                                    if let Err(e) = with_conn(db_path, |conn| {
                                        upsert_registry(
                                            conn,
                                            def.id,
                                            def.tier,
                                            &path,
                                            url,
                                            Some(&head),
                                            n,
                                            hash.as_deref(),
                                            0,
                                            head
                                                .content_filename
                                                .as_deref()
                                                .or(head.last_modified.as_deref()),
                                            false,
                                        )
                                    }) {
                                        result.errors.push(format!(
                                            "GWAS downloaded but its local status could not be recorded: {e}"
                                        ));
                                        continue;
                                    }
                                    result
                                        .messages
                                        .push(format!("Downloaded GWAS catalog ({} bytes)", n));
                                    pending_imports.push(def.id);
                                }
                                Err(e) => result.errors.push(format!("GWAS download: {e}")),
                            }
                        }
                    } else if present {
                        let indexed_rows = with_conn(db_path, |conn| {
                            Ok(row_count_for_asset(conn, OfflineAssetId::GwasCatalog))
                        })
                        .unwrap_or(0);
                        if skip_if_current && indexed_rows > 0 {
                            emit_sync_phase(
                                app,
                                "gwas_catalog",
                                "skip",
                                2,
                                "GWAS catalog up to date — skipping",
                            );
                            result
                                .messages
                                .push("GWAS catalog already up to date".to_string());
                        } else if let Some(handle) = app {
                            // Import-only — tell UI immediately (no download bar).
                            emit_sync_phase(
                                Some(handle),
                                "gwas_catalog",
                                "import",
                                2,
                                "Step 2/2 · Re-indexing local GWAS catalog (CPU-bound)…",
                            );
                            let _ = handle.emit(
                                "offline:import_progress",
                                serde_json::json!({
                                    "asset_id": "gwas_catalog",
                                    "rows_processed": 0,
                                    "percent": 0,
                                    "rows_per_second": 0,
                                    "eta_seconds": null,
                                    "message": "Step 2/2 · Re-indexing local GWAS catalog (CPU-bound)…"
                                }),
                            );
                            pending_imports.push(def.id);
                        } else {
                            pending_imports.push(def.id);
                        }
                    }
                }
            }
            AssetKind::Derived => {
                let indexed_rows = with_conn(db_path, |conn| {
                    if def.id == OfflineAssetId::Tier2VariantLocus {
                        Ok(crate::offline::tier2::count_variant_locus_rows(data_dir, conn))
                    } else {
                        Ok(row_count_for_asset(conn, def.id))
                    }
                })
                .unwrap_or(0);
                if skip_if_current && indexed_rows > 0 && sample_id.is_none() {
                    emit_sync_phase(
                        app,
                        def.id.as_str(),
                        "skip",
                        2,
                        &format!("{} already built — skipping", def.label),
                    );
                    result
                        .messages
                        .push(format!("{} already built", def.label));
                } else {
                    emit_sync_phase(
                        app,
                        def.id.as_str(),
                        "import",
                        2,
                        &format!("Building {}…", def.label),
                    );
                    pending_imports.push(def.id);
                }
            }
        }
    }

    let data_dir_owned = data_dir.to_path_buf();
    let db_path_owned = db_path.to_path_buf();
    let app_clone = app.cloned();
    // Serialize imports so parallel downloads don't contend on SQLite writes.
    let _import_guard = import_mutex().lock().await;
    let import_queue_len = pending_imports.len();
    for (idx, id) in pending_imports.iter().enumerate() {
        let waiting = if idx == 0 {
            format!(
                "Step 2/2 · Indexing {} into SQLite…",
                asset_def(*id).map(|d| d.label).unwrap_or(id.as_str())
            )
        } else {
            format!(
                "Step 2/2 · Queued for import ({idx} of {import_queue_len} ahead) — downloads finished; waiting for SQLite slot…"
            )
        };
        emit_sync_phase(app, id.as_str(), "import", 2, &waiting);
    }
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
    sync_offline_assets_subset(data_dir, db_path, ids, force, !force, sample_id, app).await
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
        for (idx, id) in ids.iter().enumerate() {
            if is_offline_import_cancelled() {
                out.errors
                    .push("Import cancelled by user.".to_string());
                break;
            }
            if let Some(ref handle) = app {
                let label = asset_def(*id).map(|d| d.label).unwrap_or(id.as_str());
                let remaining = ids.len().saturating_sub(idx + 1);
                let msg = if remaining == 0 {
                    format!("Step 2/2 · Indexing {label}…")
                } else {
                    format!("Step 2/2 · Indexing {label} ({remaining} more queued)…")
                };
                let _ = handle.emit(
                    "offline:sync_phase",
                    serde_json::json!({
                        "asset_id": id.as_str(),
                        "phase": "import",
                        "step": 2,
                        "total_steps": 2,
                        "message": msg,
                    }),
                );
            }
            match import_asset_sync(conn, data_dir, db_path, *id, sample_id, app.as_ref()) {
                Ok(msg) => {
                    out.synced.push(id.as_str().to_string());
                    out.messages.push(msg);
                }
                Err(e) => {
                    if e.contains("cancelled") {
                        out.errors.push(e);
                        break;
                    }
                    out.errors.push(e);
                }
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
    // Re-sync must re-import even when the remote file is unchanged.
    sync_offline_assets_subset(data_dir, db_path, ids, force, false, sample_id, app).await
}

/// Sync all tiers, downloading only missing or outdated assets. Does not force re-download.
pub async fn sync_all_missing(
    data_dir: &Path,
    db_path: &Path,
    sample_id: Option<i64>,
    app: Option<&AppHandle>,
) -> Result<Vec<OfflineSyncResult>, String> {
    let mut results = Vec::new();
    emit_sync_phase(
        app,
        "__bulk__",
        "bulk",
        1,
        "Sync All Missing · scanning tiers 0–2…",
    );
    for tier in 0u8..=2 {
        emit_sync_phase(
            app,
            "__bulk__",
            "bulk",
            1,
            &format!("Sync All Missing · tier {tier}…"),
        );
        results.push(sync_offline_tier(data_dir, db_path, tier, false, sample_id, app).await?);
    }
    emit_sync_phase(app, "__bulk__", "bulk", 2, "Sync All Missing · complete");
    Ok(results)
}

fn gwas_download_required(force: bool, local_present: bool, update_available: bool) -> bool {
    force || !local_present || update_available
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

    // Hash-gate large catalog imports: refuse to index when a sibling .md5/.sha256
    // proves the local file is corrupt (remote .md5 already checked at download).
    if matches!(
        id,
        OfflineAssetId::ClinvarVariantSummary
            | OfflineAssetId::DbsnpMergedJson
            | OfflineAssetId::DbsnpWithdrawnJson
            | OfflineAssetId::GwasCatalog
    ) {
        if let Some(local) = resolve_local_asset_path(&path) {
            verify_local_hash_sidecar(&local)?;
        }
    }

    let row_count = match id {
        OfflineAssetId::GwasCatalog => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "gwas")
                .map_err(|e| e.to_string())?;
            import_gwas_from_local_files(effective_dir, db_path, app)?
        }
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
            // Always attach under App/Data (not raw_downloads/) so status + reports see the same DB.
            crate::db::ensure_catalog_db_attached(conn, data_dir, "clinvar")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "ClinVar file missing — run Tier 1 sync".to_string())?;
            import_clinvar_variant_summary(conn, &eff_path, app)?
        }
        OfflineAssetId::PharmgkbClinicalVariants => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "pharmgkb")
                .map_err(|e| e.to_string())?;
            if !path.exists() {
                return Err("PharmGKB clinical zip missing".into());
            }
            import_pharmgkb_clinical_variants(conn, &path)?
        }
        OfflineAssetId::PharmgkbGenes => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "pharmgkb")
                .map_err(|e| e.to_string())?;
            if !path.exists() {
                return Err("PharmGKB genes zip missing".into());
            }
            import_pharmgkb_genes(conn, &path)?
        }
        OfflineAssetId::ClingenGeneValidity => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "clingen")
                .map_err(|e| e.to_string())?;
            let eff_path =
                resolve_local_asset_path(&path).ok_or_else(|| "ClinGen CSV missing".to_string())?;
            import_clingen_gene_validity(conn, &eff_path)?
        }
        OfflineAssetId::ManeSelectSummary => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "mane")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "MANE summary missing".to_string())?;
            import_mane_summary(conn, &eff_path)?
        }
        OfflineAssetId::DbsnpMergedJson => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "dbsnp")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "dbSNP merged JSON missing".to_string())?;
            import_dbsnp_merged(conn, &eff_path, app)?
        }
        OfflineAssetId::DbsnpWithdrawnJson => {
            crate::db::ensure_catalog_db_attached(conn, data_dir, "dbsnp")
                .map_err(|e| e.to_string())?;
            let eff_path = resolve_local_asset_path(&path)
                .ok_or_else(|| "dbSNP withdrawn JSON missing".to_string())?;
            import_dbsnp_withdrawn(conn, &eff_path, app)?
        }
        OfflineAssetId::Tier2VariantLocus => {
            ensure_tier2_meta(effective_dir)?;
            let meta_path = local_path(data_dir, custom_dir.as_deref(), def);
            // Genotypes live in per-sample DBs — never query them on the registry conn.
            let count = if let Some(sid) = sample_id {
                build_variant_locus_for_sample_in_data_dir(data_dir, sid)?
            } else {
                build_variant_locus_all_samples(data_dir, conn)?
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
        upsert_registry(
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
        )?;
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

pub fn offline_status_summary(conn: &Connection, data_dir: &Path) -> (u64, u64, u64) {
    let clinvar = row_count_for_asset(conn, OfflineAssetId::ClinvarVariantSummary);
    let gwas = row_count_for_asset(conn, OfflineAssetId::GwasCatalog);
    let locus = crate::offline::tier2::count_variant_locus_rows(data_dir, conn);
    (gwas, clinvar, locus)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_data_dir(label: &str) -> std::path::PathBuf {
        static NEXT_ID: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "dna_tools_{label}_{}_{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }

    #[test]
    fn gwas_downloads_when_a_newer_remote_identity_is_proven() {
        assert!(gwas_download_required(false, true, true));
    }

    #[test]
    fn gwas_skips_an_installed_current_catalog() {
        assert!(!gwas_download_required(false, true, false));
    }

    #[test]
    fn gwas_downloads_when_missing_or_forced() {
        assert!(gwas_download_required(false, false, false));
        assert!(gwas_download_required(true, true, false));
    }

    #[test]
    fn semantic_catalog_check_detects_empty_clinpgx_annotations() {
        let data_dir = fixture_data_dir("clinpgx_semantics");
        std::fs::create_dir_all(&data_dir).expect("create ClinPGx semantic fixture directory");
        let db_path = data_dir.join("user_genome.db");
        let conn = crate::db::connect(&db_path).expect("open ClinPGx semantic fixture DB");
        crate::db::ensure_catalog_db_attached(&conn, &data_dir, "pharmgkb")
            .expect("attach ClinPGx fixture catalog");

        conn.execute(
            "INSERT INTO pharmgkb.pharmgkb_clinical_variants
             (rsid, gene, drug, phenotype, evidence_level, raw_json)
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params!["rs-semantic", "GENE", "", "", "1A", "fixture"],
        )
        .expect("seed semantically incomplete ClinPGx row");
        assert!(asset_semantic_refresh_required(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants
        ));

        conn.execute(
            "UPDATE pharmgkb.pharmgkb_clinical_variants SET drug = ? WHERE rsid = ?",
            rusqlite::params!["fixture drug", "rs-semantic"],
        )
        .expect("repair semantic fixture row");
        assert!(!asset_semantic_refresh_required(
            &conn,
            OfflineAssetId::PharmgkbClinicalVariants
        ));
        assert!(!asset_semantic_refresh_required(
            &conn,
            OfflineAssetId::ClinvarVariantSummary
        ));

        drop(conn);
        std::fs::remove_dir_all(&data_dir).expect("remove ClinPGx semantic fixture directory");
    }

    #[test]
    fn local_import_reports_missing_fixture_then_recovers() {
        let data_dir = fixture_data_dir("import_recovery");
        std::fs::create_dir_all(&data_dir).expect("create import fixture directory");
        let db_path = data_dir.join("user_genome.db");

        let missing = import_assets_sync(
            &data_dir,
            &db_path,
            &[OfflineAssetId::LiftoverChain],
            None,
            None,
        )
        .expect("missing fixture should be reported in the batch result");
        assert!(missing.synced.is_empty());
        assert_eq!(missing.errors.len(), 1);
        assert!(missing.errors[0].contains("Liftover chain missing"));

        let definition = asset_def(OfflineAssetId::LiftoverChain).expect("liftover definition");
        let fixture_path = local_path(&data_dir, None, definition);
        std::fs::write(&fixture_path, b"chain-fixture").expect("write recovery fixture");

        let recovered = import_assets_sync(
            &data_dir,
            &db_path,
            &[OfflineAssetId::LiftoverChain],
            None,
            None,
        )
        .expect("recovery import should complete");
        assert_eq!(recovered.synced, vec!["liftover_chain"]);
        assert!(recovered.errors.is_empty());

        let conn = crate::db::connect(&db_path).expect("open recovered registry");
        let registry = read_registry(&conn, "liftover_chain").expect("recovered registry row");
        assert_eq!(registry.local_bytes, 13);
        assert!(!registry.update_available);
        drop(conn);
        std::fs::remove_dir_all(&data_dir).expect("remove import fixture directory");
    }

    #[test]
    fn clingen_import_reports_missing_fixture_then_recovers_with_real_csv_shape() {
        let data_dir = fixture_data_dir("clingen_recovery");
        std::fs::create_dir_all(&data_dir).expect("create ClinGen fixture directory");
        let db_path = data_dir.join("user_genome.db");

        let missing = import_assets_sync(
            &data_dir,
            &db_path,
            &[OfflineAssetId::ClingenGeneValidity],
            None,
            None,
        )
        .expect("missing ClinGen fixture should be reported in the batch result");
        assert!(missing.synced.is_empty());
        assert_eq!(missing.errors.len(), 1);
        assert!(missing.errors[0].contains("ClinGen CSV missing"));

        let definition =
            asset_def(OfflineAssetId::ClingenGeneValidity).expect("ClinGen definition");
        let fixture_path = local_path(&data_dir, None, definition);
        std::fs::create_dir_all(fixture_path.parent().expect("ClinGen fixture parent"))
            .expect("create ClinGen fixture parent");
        std::fs::write(
            &fixture_path,
            "ClinGen fixture preamble\n\"GENE SYMBOL\",\"DISEASE LABEL\",\"CLASSIFICATION\",\"MOI\",\"ONLINE REPORT\",\"GENE ID (HGNC)\"\n\"FIXTURE1\",\"Fixture condition\",\"Definitive\",\"Autosomal dominant\",\"https://example.invalid/fixture\",\"HGNC:1\"\n",
        )
        .expect("write ClinGen CSV fixture");

        let recovered = import_assets_sync(
            &data_dir,
            &db_path,
            &[OfflineAssetId::ClingenGeneValidity],
            None,
            None,
        )
        .expect("ClinGen recovery import should complete");
        assert_eq!(recovered.synced, vec!["clingen_gene_validity"]);
        assert!(recovered.errors.is_empty());

        let conn = crate::db::connect(&db_path).expect("open recovered ClinGen registry");
        assert_eq!(
            row_count_for_asset(&conn, OfflineAssetId::ClingenGeneValidity),
            1
        );
        let registry = read_registry(&conn, "clingen_gene_validity")
            .expect("recovered ClinGen registry row");
        assert!(registry.local_bytes > 0);
        assert!(!registry.update_available);
        drop(conn);
        std::fs::remove_dir_all(&data_dir).expect("remove ClinGen fixture directory");
    }

    #[test]
    fn runtime_artifact_inventory_is_bounded_and_aggregate_only() {
        let data_dir = fixture_data_dir("runtime_artifacts");
        std::fs::create_dir_all(data_dir.join("raw_downloads/clinvar"))
            .expect("create raw download fixture directory");
        std::fs::create_dir_all(data_dir.join("references"))
            .expect("create references fixture directory");
        std::fs::create_dir_all(data_dir.join("samples/1"))
            .expect("create private sample fixture directory");
        std::fs::write(data_dir.join("raw_downloads/clinvar/catalog.part"), b"abc")
            .expect("write catalog partial");
        std::fs::write(data_dir.join("references/gwas.part"), b"defg").expect("write GWAS partial");
        std::fs::write(data_dir.join("GRCh37_to_GRCh38.chain.gz.part"), b"hijkl")
            .expect("write chain partial");
        std::fs::write(data_dir.join("samples/1/private.part"), b"private")
            .expect("write private partial");
        std::fs::write(data_dir.join("api_cache.db-wal"), b"sidecar")
            .expect("write app cache sidecar");
        std::fs::write(data_dir.join("user_genome.db-wal"), b"private")
            .expect("write private database sidecar");

        let conn = rusqlite::Connection::open_in_memory().expect("open status fixture database");
        conn.execute_batch(
            "
            ATTACH DATABASE ':memory:' AS api_cache_db;
            CREATE TABLE api_cache_db.api_cache (url TEXT PRIMARY KEY, response_json TEXT NOT NULL, fetched_at INTEGER NOT NULL);
            CREATE TABLE api_cache_db.api_cache_entries (cache_id TEXT PRIMARY KEY, response_body TEXT);
            INSERT INTO api_cache_db.api_cache VALUES ('fixture-url', '{}', 1);
            INSERT INTO api_cache_db.api_cache_entries VALUES ('fixture-cache', '{}');
            ",
        )
        .expect("seed cache status fixture");

        let status = collect_runtime_artifacts(&data_dir, None, &conn);
        assert_eq!(status.partial_download_files, 3);
        assert_eq!(status.partial_download_bytes, 12);
        assert_eq!(status.sqlite_sidecar_files, 1);
        assert_eq!(status.rebuildable_cache_rows, 2);

        drop(conn);
        std::fs::remove_dir_all(&data_dir).expect("remove runtime artifact fixture");
    }
}
