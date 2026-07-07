// ./src-tauri/src/research/mod.rs
/*
Module: research
Purpose: Autonomous marker research, enrichment sources, and Qdrant vector indexing.
*/

pub(crate) mod debug_log;
mod types;
mod state;
pub(crate) mod util;
mod http;
mod tuning;
mod sources_config;
mod qdrant;
mod embed;
mod sources;
mod enrich;
mod markers;
mod job;
mod sweep_runtime;
mod sweep;
mod sweep_metrics;
mod prefetch;
mod headless;
mod crossmap;
mod promote;
pub mod evidence;
pub mod gnomad;
pub mod references;
pub mod commands;

pub use types::*;
pub use commands::*;

pub use qdrant::{
    check_existing_points, count_qdrant_points, count_qdrant_points_with_filter,
    ensure_qdrant_collection, ensure_qdrant_collection_named, find_point_payload_by_rsid,
    initialize_qdrant_collection, purge_qdrant_collection, sample_index_embedding_model,
    scroll_qdrant_points, search_qdrant, test_qdrant_connection, upsert_points_batch,
    upsert_points_batch_named, upsert_to_qdrant,
};
pub use embed::{embed_query_cached, embed_text, embed_texts_batch};
pub use sources::{
    fetch_gnomad_frequency, fetch_gtex_eqtls_for_rsid, fetch_gtex_gencode_id,
    fetch_gtex_median_expression, fetch_gwas_associations,
};
pub use enrich::enrich_marker;
pub use markers::{
    collect_markers_for_scopes, get_all_sample_rsids, preview_research_scope,
    score_marker_significance,
};
pub use job::{get_research_job_from_db, reset_research_job_after_purge, save_research_job, cancel_research_job_in_db, pause_research_job_in_db};
pub use sweep::run_research_loop;
pub use state::{RESEARCH_PAUSED, RESEARCH_RUNNING};

pub use tuning::{install_sweep_tuning, clear_sweep_tuning, resolve_pipeline_tuning, probe_service_latencies, PipelineTuning, PipelineTuningPublic, SweepTuningGuard};
pub use sweep_runtime::SweepProgressSink;
pub use headless::{parse_headless_args, print_headless_usage, run_headless_sweep, HeadlessSweepArgs};
pub use sweep_metrics::{snapshot_sweep_metrics, SweepPhaseMetricsSnapshot};
