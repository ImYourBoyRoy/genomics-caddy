// ./src-tauri/src/research/mod.rs
/*
Module: research
Purpose: Autonomous marker research, enrichment sources, and Qdrant vector indexing.
*/

pub mod commands;
mod crossmap;
pub(crate) mod debug_log;
mod embed;
mod enrich;
pub mod evidence;
pub mod gnomad;
mod headless;
mod http;
mod job;
mod markers;
mod prefetch;
mod promote;
mod qdrant;
pub mod references;
mod sources;
mod sources_config;
mod state;
mod sweep;
mod sweep_metrics;
mod sweep_runtime;
mod tuning;
mod types;
pub(crate) mod util;

pub use commands::*;
pub use types::*;

pub use embed::{embed_query_cached, embed_text, embed_texts_batch};
pub use enrich::enrich_marker;
pub use job::{
    cancel_research_job_in_db, get_research_job_from_db, pause_research_job_in_db,
    reset_research_job_after_purge, save_research_job,
};
pub use markers::{
    collect_markers_for_scopes, get_all_sample_rsids, preview_research_scope,
    score_marker_significance,
};
pub use qdrant::{
    check_existing_points, count_qdrant_points, count_qdrant_points_with_filter,
    ensure_qdrant_collection, ensure_qdrant_collection_named, find_point_payload_by_rsid,
    initialize_qdrant_collection, purge_qdrant_collection, sample_index_embedding_model,
    scroll_qdrant_points, search_qdrant, test_qdrant_connection, upsert_points_batch,
    upsert_points_batch_named, upsert_to_qdrant,
};
pub use sources::{
    fetch_gnomad_frequency, fetch_gtex_eqtls_for_rsid, fetch_gtex_gencode_id,
    fetch_gtex_median_expression, fetch_gwas_associations,
};
pub use state::{RESEARCH_PAUSED, RESEARCH_RUNNING};
pub use sweep::run_research_loop;

pub use headless::{
    HeadlessSweepArgs, parse_headless_args, print_headless_usage, run_headless_sweep,
};
pub use sweep_metrics::{SweepPhaseMetricsSnapshot, snapshot_sweep_metrics};
pub use sweep_runtime::SweepProgressSink;
pub use tuning::{
    PipelineTuning, PipelineTuningPublic, SweepTuningGuard, clear_sweep_tuning,
    install_sweep_tuning, probe_service_latencies, resolve_pipeline_tuning,
};
