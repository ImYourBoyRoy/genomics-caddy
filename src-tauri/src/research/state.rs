// ./src-tauri/src/research/state.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub static RESEARCH_PAUSED: AtomicBool = AtomicBool::new(false);
pub static RESEARCH_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn research_loop_active() -> bool {
    RESEARCH_RUNNING.load(Ordering::SeqCst)
}

/// Clear in-memory run flag when the DB job is not actively running (crash/hang/resume race).
pub fn clear_stale_running_flag(job_status: &str) -> bool {
    if !RESEARCH_RUNNING.load(Ordering::SeqCst) {
        return false;
    }
    if matches!(job_status, "paused" | "complete" | "idle" | "error") {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return true;
    }
    false
}

pub fn force_stop_research_runtime() {
    RESEARCH_PAUSED.store(true, Ordering::SeqCst);
    RESEARCH_RUNNING.store(false, Ordering::SeqCst);
}

use std::sync::LazyLock;
use tokio::sync::Semaphore;

pub(crate) static NCBI_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(3));
