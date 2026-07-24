// ./src-tauri/src/research/state.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub static RESEARCH_PAUSED: AtomicBool = AtomicBool::new(false);
pub static RESEARCH_RUNNING: AtomicBool = AtomicBool::new(false);
/// Distinct from pause — cancel must not be overwritten into `paused` by the sweep loop.
pub static RESEARCH_CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn research_loop_active() -> bool {
    RESEARCH_RUNNING.load(Ordering::SeqCst)
}

pub fn is_research_cancelled() -> bool {
    RESEARCH_CANCELLED.load(Ordering::SeqCst)
}

/// Clear in-memory run flag only for terminal job rows (crash/hang recovery).
///
/// Do **not** clear on `idle` or `paused`:
/// - After Start, polls can still read the previous idle row while RUNNING is already true.
/// - After Resume, polls can still read paused while RUNNING is already true.
///
/// Clearing in those windows makes `normalize_job_record` rewrite the live job to paused.
/// The sweep loop clears RUNNING when it actually stops (cancel/pause/complete/error).
pub fn clear_stale_running_flag(job_status: &str) -> bool {
    if !RESEARCH_RUNNING.load(Ordering::SeqCst) {
        return false;
    }
    if matches!(job_status, "complete" | "error") {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return true;
    }
    false
}

/// Cooperative cancel: loop must treat this before pause and persist `idle`, not `paused`.
///
/// Important: do **not** clear `RESEARCH_RUNNING` here. Clearing it early makes
/// `get_research_job_from_db` normalize an in-flight `running` row into `paused`
/// (cancel then looks like pause). The sweep loop clears RUNNING when it stops.
pub fn force_stop_research_runtime() {
    RESEARCH_CANCELLED.store(true, Ordering::SeqCst);
    RESEARCH_PAUSED.store(true, Ordering::SeqCst);
}

/// Reset control flags before starting or resuming a sweep.
pub fn clear_research_control_flags() {
    RESEARCH_CANCELLED.store(false, Ordering::SeqCst);
    RESEARCH_PAUSED.store(false, Ordering::SeqCst);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepStopKind {
    Cancel,
    Pause,
}

/// Prefer cancel over pause when both signals are set (cancel always sets pause too).
pub fn pending_sweep_stop() -> Option<SweepStopKind> {
    if RESEARCH_CANCELLED.load(Ordering::SeqCst) {
        Some(SweepStopKind::Cancel)
    } else if RESEARCH_PAUSED.load(Ordering::SeqCst) {
        Some(SweepStopKind::Pause)
    } else {
        None
    }
}

use std::sync::LazyLock;
use tokio::sync::Semaphore;

pub(crate) static NCBI_SEMAPHORE: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(3));

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn cancel_takes_priority_over_pause() {
        let _guard = TEST_LOCK.lock().unwrap();
        clear_research_control_flags();
        RESEARCH_RUNNING.store(true, Ordering::SeqCst);
        RESEARCH_PAUSED.store(true, Ordering::SeqCst);
        assert_eq!(pending_sweep_stop(), Some(SweepStopKind::Pause));
        force_stop_research_runtime();
        assert_eq!(pending_sweep_stop(), Some(SweepStopKind::Cancel));
        assert!(is_research_cancelled());
        // Cancel must not clear RUNNING early (avoids normalize→paused race).
        assert!(RESEARCH_RUNNING.load(Ordering::SeqCst));
        clear_research_control_flags();
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        assert_eq!(pending_sweep_stop(), None);
    }

    #[test]
    fn clear_stale_running_ignores_idle_and_paused() {
        let _guard = TEST_LOCK.lock().unwrap();
        clear_research_control_flags();
        RESEARCH_RUNNING.store(true, Ordering::SeqCst);
        // Start race: DB still idle while the new loop is already marked running.
        assert!(!clear_stale_running_flag("idle"));
        assert!(RESEARCH_RUNNING.load(Ordering::SeqCst));
        // Resume race: DB still paused while the resumed loop is marked running.
        assert!(!clear_stale_running_flag("paused"));
        assert!(RESEARCH_RUNNING.load(Ordering::SeqCst));
        assert!(clear_stale_running_flag("complete"));
        assert!(!RESEARCH_RUNNING.load(Ordering::SeqCst));
    }
}
