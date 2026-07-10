// ./src-tauri/src/research/job.rs
use super::state::{clear_stale_running_flag, force_stop_research_runtime, research_loop_active};
use super::types::*;
use super::util::unix_now;
use rusqlite::{Connection, params};
use std::path::Path;

fn normalize_job_record(conn: &Connection, job: &mut ResearchJob) {
    clear_stale_running_flag(&job.status);

    if job.status == "running" && !research_loop_active() {
        job.status = "paused".to_string();
        let _ = conn.execute(
            "UPDATE research_jobs SET status = 'paused', last_updated = ?2 WHERE job_id = ?1",
            params![job.job_id, unix_now()],
        );
    }

    if job.total_markers > 0 && job.enriched_count >= job.total_markers {
        job.enriched_count = job.total_markers;
        if job.status != "error" {
            job.status = "complete".to_string();
            job.priority_complete = true;
            job.current_rsid = None;
            job.current_source = None;
            let _ = conn.execute(
                "UPDATE research_jobs SET status = 'complete', enriched_count = ?2,
                    priority_complete = 1, current_rsid = NULL, current_source = NULL,
                    last_updated = ?3 WHERE job_id = ?1",
                params![job.job_id, job.enriched_count, unix_now()],
            );
        }
    }
}

pub fn get_research_job_from_db(db_path: &Path, sample_id: i64) -> Option<ResearchJob> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id).ok()?;
    let mut job = conn
        .query_row(
            "SELECT job_id, sample_id, status, total_markers, enriched_count,
                priority_complete, current_rsid, current_source,
                started_at, last_updated, error_message, scope_json, session_started_at
         FROM research_jobs WHERE sample_id = ?
         ORDER BY started_at DESC LIMIT 1",
            params![sample_id],
            |row| {
                Ok(ResearchJob {
                    job_id: row.get(0)?,
                    sample_id: row.get(1)?,
                    status: row.get(2)?,
                    total_markers: row.get(3)?,
                    enriched_count: row.get(4)?,
                    priority_complete: row.get::<_, i32>(5)? != 0,
                    current_rsid: row.get(6)?,
                    current_source: row.get(7)?,
                    started_at: row.get(8)?,
                    last_updated: row.get(9)?,
                    error_message: row.get(10)?,
                    scope_json: row.get(11).ok(),
                    session_started_at: row.get(12).ok(),
                    loop_active: None,
                    live_message: None,
                    activity_phase: None,
                    batch_prepared: None,
                    batch_prefetch_done: None,
                    batch_total: None,
                    batch_elapsed_secs: None,
                    qdrant_sample_count: None,
                    session_elapsed_secs: None,
                })
            },
        )
        .ok()?;

    normalize_job_record(&conn, &mut job);
    job.loop_active = Some(research_loop_active());
    Some(job)
}

// ---------------------------------------------------------------------------
// 13. save_research_job
// ---------------------------------------------------------------------------

/// After a Qdrant purge, invalidate in-progress enrichment so the user must start fresh.
pub fn reset_research_job_after_purge(
    db_path: &Path,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    use super::util::unix_now;

    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let now = unix_now();
    let updated = conn
        .execute(
            "UPDATE research_jobs SET
                status = 'idle',
                enriched_count = 0,
                priority_complete = 0,
                current_rsid = NULL,
                current_source = NULL,
                error_message = 'Vector collection purged — start a fresh sweep.',
                last_updated = ?2
             WHERE sample_id = ?1",
            params![sample_id, now],
        )
        .map_err(|e| e.to_string())?;

    if updated == 0 {
        return Ok(None);
    }

    Ok(get_research_job_from_db(db_path, sample_id))
}

pub fn cancel_research_job_in_db(
    db_path: &Path,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    force_stop_research_runtime();
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let now = unix_now();
    let updated = conn
        .execute(
            "UPDATE research_jobs SET
                status = 'idle',
                current_rsid = NULL,
                current_source = NULL,
                error_message = 'Sweep cancelled by user.',
                last_updated = ?2
             WHERE sample_id = ?1 AND status IN ('running', 'paused')",
            params![sample_id, now],
        )
        .map_err(|e| e.to_string())?;

    if updated == 0 {
        return Ok(get_research_job_from_db(db_path, sample_id));
    }

    Ok(get_research_job_from_db(db_path, sample_id))
}

pub fn pause_research_job_in_db(
    db_path: &Path,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, sample_id)
        .map_err(|e| e.to_string())?;
    let now = unix_now();
    let _ = conn.execute(
        "UPDATE research_jobs SET status = 'paused', last_updated = ?2
         WHERE sample_id = ?1 AND status = 'running'",
        params![sample_id, now],
    );
    Ok(get_research_job_from_db(db_path, sample_id))
}

pub fn save_research_job(db_path: &Path, job: &ResearchJob) -> Result<(), String> {
    let conn = crate::db::connect_sample_from_registry_path(db_path, job.sample_id)
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO research_jobs (
            job_id, sample_id, status, total_markers, enriched_count,
            priority_complete, current_rsid, current_source,
            started_at, last_updated, error_message, scope_json, session_started_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            job.job_id,
            job.sample_id,
            job.status,
            job.total_markers,
            job.enriched_count,
            job.priority_complete as i32,
            job.current_rsid,
            job.current_source,
            job.started_at,
            job.last_updated,
            job.error_message,
            job.scope_json,
            job.session_started_at,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
