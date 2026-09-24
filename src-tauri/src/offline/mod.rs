// ./src-tauri/src/offline/mod.rs
/*
Offline genomics reference layer (Tiers 0–2).
Downloads source files with size checks, tracks versions in SQLite, imports into local tables,
and serves lookups before live APIs during enrichment sweeps.
*/

pub mod commands;
pub mod compress;
pub mod discovery_export;
pub mod lookup;
pub mod manifest;
pub mod schema;

mod download;
mod import_clingen;
mod import_clinvar;
mod import_clinvar_submission;
mod import_dbsnp;
mod import_mane;
mod import_pharmgkb;
mod registry;
mod sync;
pub mod tier2;

pub use lookup::*;
pub use manifest::{OfflineAssetId, OfflineAssetStatus, OfflineTierStatus};
pub use sync::{
    OfflineIndexedSummary, OfflineRemoteCheckStatus, OfflineRuntimeArtifactStatus,
    OfflineSyncResult, OfflineUpdateCheck,
    build_tier2_for_sample, cancel_offline_import, check_offline_updates,
    liftover_chain_path, reset_offline_import_cancel, sync_all_missing, sync_offline_tier,
    sync_single_asset,
};
