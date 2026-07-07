// ./src-tauri/src/offline/mod.rs
/*
Offline genomics reference layer (Tiers 0–2).
Downloads source files with size checks, tracks versions in SQLite, imports into local tables,
and serves lookups before live APIs during enrichment sweeps.
*/

pub mod commands;
pub mod lookup;
pub mod manifest;
pub mod schema;

mod download;
mod import_clinvar;
mod import_clingen;
mod import_dbsnp;
mod import_mane;
mod import_pharmgkb;
mod registry;
mod sync;
mod tier2;

pub use lookup::*;
pub use manifest::{OfflineAssetId, OfflineAssetStatus, OfflineTierStatus};
pub use sync::{
    build_tier2_for_sample, check_offline_updates, sync_all_missing, sync_offline_tier,
    sync_single_asset, OfflineIndexedSummary, OfflineSyncResult, OfflineUpdateCheck,
};
