// ./src-tauri/src/research/headless.rs
/*
Headless sweep runner for Docker and server deployments.
Loads config from GENOMICS_DATA_DIR SQLite, emits NDJSON progress on stdout.
*/

use super::sweep_runtime::SweepProgressSink;
use super::{get_research_job_from_db, run_research_loop};
use crate::config;
use crate::paths;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone)]
pub struct HeadlessSweepArgs {
    pub sample_id: i64,
    pub resume: bool,
    pub force_reenrich: bool,
}

fn resolve_ollama_url() -> String {
    crate::config::resolve_ollama_service_url()
}

pub async fn run_headless_sweep(args: HeadlessSweepArgs) -> Result<(), String> {
    let data_dir = paths::resolve_data_dir();
    paths::ensure_data_layout(&data_dir).map_err(|e| e.to_string())?;
    let db_path = paths::db_path(&data_dir);

    let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
    config::install_research_debug_logging(&conn);
    let cfg = config::load_qdrant_config(&conn)?;
    let scope = config::load_research_scope(&conn)?;
    let ollama_url = resolve_ollama_url();

    if super::state::RESEARCH_RUNNING.load(Ordering::SeqCst) {
        return Err("A research sweep is already running in this process.".to_string());
    }

    let resume_job = if args.resume {
        get_research_job_from_db(&db_path, args.sample_id)
    } else {
        None
    };

    super::state::RESEARCH_PAUSED.store(false, Ordering::SeqCst);
    super::state::RESEARCH_RUNNING.store(true, Ordering::SeqCst);

    eprintln!(
        "Headless sweep: sample={} data_dir={} resume={} force_reenrich={} ollama={} qdrant={}",
        args.sample_id,
        data_dir.display(),
        args.resume,
        args.force_reenrich,
        ollama_url,
        cfg.url
    );

    let result = run_research_loop(
        args.sample_id,
        db_path,
        ollama_url,
        cfg,
        scope,
        SweepProgressSink::Headless,
        resume_job,
        args.force_reenrich,
    )
    .await;

    super::state::RESEARCH_RUNNING.store(false, Ordering::SeqCst);
    result
}

pub fn parse_headless_args(args: &[String]) -> Option<HeadlessSweepArgs> {
    if !args.iter().any(|a| a == "--headless-sweep") {
        return None;
    }

    let mut sample_id: Option<i64> = None;
    let mut resume = false;
    let mut force_reenrich = false;

    for arg in args {
        if let Some(id) = arg.strip_prefix("--sample-id=") {
            sample_id = id.parse().ok();
        } else if arg == "--resume" {
            resume = true;
        } else if arg == "--force-reenrich" {
            force_reenrich = true;
        }
    }

    let sample_id = sample_id.or_else(|| {
        std::env::var("GENOMICS_SAMPLE_ID")
            .ok()
            .and_then(|v| v.parse().ok())
    })?;

    Some(HeadlessSweepArgs {
        sample_id,
        resume,
        force_reenrich,
    })
}

pub fn print_headless_usage() {
    eprintln!(
        "Usage: DNA-Tools --headless-sweep --sample-id=<id> [--resume] [--force-reenrich]\n\
         Environment:\n\
           GENOMICS_DATA_DIR   SQLite + reference data root (required in Docker)\n\
           GENOMICS_SAMPLE_ID  Default sample id when --sample-id is omitted\n\
           GENOMICS_CPU_LIMIT  Host logical cores (optional); sweep uses (limit − 1)\n\
           GENOMICS_OLLAMA_URL / OLLAMA_URL  Embedding server\n\
           QDRANT_URL / keyring / SQLite     Qdrant connection (via saved config)\n\
         Progress: NDJSON lines prefixed with GENOMICS_PROGRESS / GENOMICS_FINDING on stdout."
    );
}
