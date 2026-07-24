// ./src-tauri/src/research/sweep.rs
use super::embed::embed_texts_batch;
use super::enrich::{PreparedEnrichment, is_fatal_enrichment_error, process_enrichment_batch};
use super::evidence::named_vectors::{build_named_vector_texts, embed_named_vectors};
use super::http::enrich_batch_size;
use super::job::save_research_job;
use super::markers::{collect_markers_for_scopes, preview_research_scope};
use super::promote::promote_enrichment_batch;
use super::state::{RESEARCH_PAUSED, RESEARCH_RUNNING, SweepStopKind, pending_sweep_stop};
use super::sweep_metrics::{SweepPhase, set_live_message, timed_async};
use super::sweep_runtime::SweepProgressSink;
use super::tuning::{self, SweepTuningGuard, install_sweep_tuning, named_vectors_in_sweep};
use super::types::*;
use super::util::{string_to_u64, unix_now};
use super::vector_store::{self, VectorProvider};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

struct ProgressTracker {
    loop_started_at: i64,
    /// Rolling samples of *newly enriched this loop* — not total progress (resume/skip safe).
    speed_samples: Vec<(i64, i64)>,
}

impl ProgressTracker {
    fn new(started_at: i64) -> Self {
        Self {
            loop_started_at: started_at,
            speed_samples: vec![(started_at, 0)],
        }
    }

    fn speeds(&mut self, status: &str, newly_enriched: i64) -> (Option<f64>, Option<f64>) {
        let now = unix_now();
        let elapsed = (now - self.loop_started_at).max(1) as f64;

        let session = if status == "running" && newly_enriched > 0 {
            Some(newly_enriched as f64 / elapsed)
        } else {
            None
        };

        if status == "running" {
            self.speed_samples.push((now, newly_enriched));
            let window_secs = 120_i64;
            self.speed_samples.retain(|(t, _)| now - *t <= window_secs);
        }

        let recent = if status != "running" || self.speed_samples.len() < 2 {
            None
        } else {
            let (t0, c0) = self.speed_samples[0];
            let (t1, c1) = *self.speed_samples.last().unwrap();
            let dt = t1 - t0;
            let delta = c1 - c0;
            if dt >= 10 && delta > 0 {
                Some(delta as f64 / dt as f64)
            } else {
                None
            }
        };

        (session, recent)
    }
}

#[allow(clippy::type_complexity)]
fn batch_activity_payload() -> (
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<i64>,
    Option<String>,
) {
    let Some(snap) = super::sweep_metrics::snapshot_batch_activity() else {
        return (None, None, None, None, None, None);
    };
    let elapsed = (unix_now() - snap.started_at).max(0);
    (
        Some(snap.phase),
        Some(snap.prepared),
        Some(snap.prefetch_done),
        Some(snap.total),
        Some(elapsed),
        Some(snap.current_rsid),
    )
}

enum ResumePlan {
    None,
    /// Jump to marker index within priority (0) or background (1) pass.
    StartInPass {
        pass_index: usize,
        marker_index: usize,
    },
    /// Saved rsid is no longer in the queue — rely on Qdrant cache checks.
    NotFound,
}

fn plan_resume(
    priority: &[&ScoredMarker],
    background: &[&ScoredMarker],
    resume_rsid: Option<&str>,
) -> ResumePlan {
    let Some(target) = resume_rsid else {
        return ResumePlan::None;
    };
    if let Some(pos) = priority.iter().position(|m| m.rsid == target) {
        return ResumePlan::StartInPass {
            pass_index: 0,
            marker_index: pos,
        };
    }
    if let Some(pos) = background.iter().position(|m| m.rsid == target) {
        return ResumePlan::StartInPass {
            pass_index: 1,
            marker_index: pos,
        };
    }
    eprintln!(
        "[sweep] Resume rsid {} not found in marker queue — using Qdrant cache to skip completed variants",
        target
    );
    ResumePlan::NotFound
}

fn sweep_dbg(sink: &SweepProgressSink, msg: impl Into<String>) {
    sink.debug_log("sweep", msg);
}

/// Apply cooperative cancel/pause. Returns true when the loop must exit.
#[allow(clippy::too_many_arguments)]
fn apply_pending_stop(
    job: &mut ResearchJob,
    db_path: &Path,
    sink: &SweepProgressSink,
    job_id: &str,
    enriched_count: i64,
    newly_enriched_count: i64,
    total_markers: i64,
    progress_tracker: &mut ProgressTracker,
) -> bool {
    match pending_sweep_stop() {
        Some(SweepStopKind::Cancel) => {
            job.status = "idle".to_string();
            job.error_message = Some("Sweep cancelled by user.".to_string());
            job.current_rsid = None;
            job.current_source = None;
            job.last_updated = unix_now();
            let _ = save_research_job(db_path, job);
            emit_progress(
                sink,
                job_id,
                "idle",
                enriched_count,
                newly_enriched_count,
                total_markers,
                None,
                None,
                "Sweep cancelled by user.".to_string(),
                progress_tracker,
                false,
            );
            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
            // Drop the pause latch so a later Start→Pause is not treated as cancel residue.
            // Keep RESEARCH_CANCELLED until the next start/resume so late saves cannot resurrect.
            RESEARCH_PAUSED.store(false, Ordering::SeqCst);
            true
        }
        Some(SweepStopKind::Pause) => {
            // Re-check cancel: force_stop sets both flags; prefer cancel if it landed mid-pause.
            if super::state::is_research_cancelled() {
                job.status = "idle".to_string();
                job.error_message = Some("Sweep cancelled by user.".to_string());
                job.current_rsid = None;
                job.current_source = None;
                job.last_updated = unix_now();
                let _ = save_research_job(db_path, job);
                emit_progress(
                    sink,
                    job_id,
                    "idle",
                    enriched_count,
                    newly_enriched_count,
                    total_markers,
                    None,
                    None,
                    "Sweep cancelled by user.".to_string(),
                    progress_tracker,
                    false,
                );
                RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                return true;
            }
            // If cancel already persisted idle, do not overwrite with paused.
            if job.status == "idle"
                && job
                    .error_message
                    .as_deref()
                    .is_some_and(|m| m.to_lowercase().contains("cancel"))
            {
                RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                return true;
            }
            job.status = "paused".to_string();
            job.last_updated = unix_now();
            let _ = save_research_job(db_path, job);
            emit_progress(
                sink,
                job_id,
                "paused",
                enriched_count,
                newly_enriched_count,
                total_markers,
                job.current_rsid.clone(),
                None,
                "Research job paused by user.".to_string(),
                progress_tracker,
                false,
            );
            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
            true
        }
        None => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    sink: &SweepProgressSink,
    job_id: &str,
    status: &str,
    enriched_count: i64,
    newly_enriched: i64,
    total_markers: i64,
    current_rsid: Option<String>,
    current_source: Option<String>,
    message: String,
    progress: &mut ProgressTracker,
    include_phase_metrics: bool,
) {
    // Drop late running/paused emits after cancel so the UI cannot resurrect.
    if pending_sweep_stop() == Some(SweepStopKind::Cancel)
        && (status == "running" || status == "paused")
    {
        return;
    }
    if status == "running" && pending_sweep_stop().is_some() {
        return;
    }
    let (variants_per_sec, recent_variants_per_sec) = progress.speeds(status, newly_enriched);
    let phase_metrics = if include_phase_metrics {
        Some(super::sweep_metrics::snapshot_sweep_metrics())
    } else {
        None
    };
    let (
        activity_phase,
        batch_prepared,
        batch_prefetch_done,
        batch_total,
        batch_elapsed_secs,
        activity_rsid,
    ) = batch_activity_payload();
    let display_rsid = activity_rsid.or(current_rsid);
    set_live_message(&message);
    sink.emit_progress(ResearchProgress {
        job_id: job_id.to_string(),
        status: status.to_string(),
        enriched_count,
        total_markers,
        current_rsid: display_rsid,
        current_source,
        message,
        variants_per_sec,
        recent_variants_per_sec,
        phase_metrics,
        activity_phase,
        batch_prepared,
        batch_prefetch_done,
        batch_total,
        batch_elapsed_secs,
        qdrant_sample_count: super::sweep_metrics::qdrant_sample_baseline(),
        session_elapsed_secs: super::sweep_metrics::sweep_session_elapsed_secs(),
    });
}

fn emit_pulse(
    sink: &SweepProgressSink,
    job_id: &str,
    enriched_count: i64,
    total_markers: i64,
    current_rsid: Option<String>,
    current_source: Option<String>,
    message: String,
) {
    // Never emit a "running" pulse after cancel/pause was requested — UI would resurrect.
    if pending_sweep_stop().is_some() {
        return;
    }
    let (
        activity_phase,
        batch_prepared,
        batch_prefetch_done,
        batch_total,
        batch_elapsed_secs,
        activity_rsid,
    ) = batch_activity_payload();
    let display_rsid = activity_rsid.or(current_rsid);
    let batch_rate = match (
        batch_prepared,
        batch_prefetch_done,
        batch_total,
        batch_elapsed_secs,
    ) {
        (Some(p), _, Some(t), Some(el)) if el >= 5 && p > 0 && t > 0 => Some(p as f64 / el as f64),
        (_, Some(pref), Some(t), Some(el)) if el >= 5 && pref > 0 && t > 0 => {
            Some(pref as f64 / el as f64)
        }
        _ => None,
    };
    set_live_message(&message);
    sink.emit_progress(ResearchProgress {
        job_id: job_id.to_string(),
        status: "running".to_string(),
        enriched_count,
        total_markers,
        current_rsid: display_rsid,
        current_source,
        message,
        variants_per_sec: batch_rate,
        recent_variants_per_sec: batch_rate,
        phase_metrics: None,
        activity_phase,
        batch_prepared,
        batch_prefetch_done,
        batch_total,
        batch_elapsed_secs,
        qdrant_sample_count: super::sweep_metrics::qdrant_sample_baseline(),
        session_elapsed_secs: super::sweep_metrics::sweep_session_elapsed_secs(),
    });
}

fn payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn payload_string_array(payload: &serde_json::Value, key: &str) -> Vec<String> {
    payload
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn finding_impact_bucket(
    clinical: f32,
    wellness: f32,
    data_quality: f32,
    has_direction: bool,
) -> String {
    if clinical >= 0.35 {
        "clinical_review".to_string()
    } else if wellness >= 0.35 && data_quality >= 0.45 {
        "wellness_relevant".to_string()
    } else if has_direction && data_quality >= 0.35 {
        "interpretable_research".to_string()
    } else if data_quality < 0.35 {
        "low_confidence".to_string()
    } else {
        "research_context".to_string()
    }
}

pub(crate) fn finding_preview_from_payload(
    job_id: &str,
    sample_id: i64,
    payload: &serde_json::Value,
) -> ResearchFindingPreview {
    let rsid = payload_string(payload, "rsid").unwrap_or_else(|| "unknown".to_string());
    let gene_symbol =
        payload_string(payload, "gene_symbol").or_else(|| payload_string(payload, "gene"));
    let genotype = payload_string(payload, "genotype");
    let trait_name = payload_string(payload, "trait_name")
        .or_else(|| payload_string(payload, "primary_trait"))
        .or_else(|| payload_string(payload, "gwas_trait"));
    let trait_category = payload_string(payload, "trait_category").or_else(|| {
        payload
            .get("trait_categories")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(String::from)
    });
    let personal_direction =
        payload_string(payload, "personal_direction").unwrap_or_else(|| "unknown".to_string());
    let directionality_label = payload_string(payload, "directionality_label")
        .unwrap_or_else(|| personal_direction.replace('_', " "));
    let wellness = payload["wellness_actionability_score"]
        .as_f64()
        .unwrap_or(0.0) as f32;
    let clinical = payload["clinical_actionability_score"]
        .as_f64()
        .unwrap_or(0.0) as f32;
    let data_quality = payload["data_quality_score"].as_f64().unwrap_or(0.5) as f32;
    let source_names = payload_string_array(payload, "source_names")
        .into_iter()
        .chain(payload_string_array(payload, "sources_used"))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let source_count = payload["source_count"]
        .as_u64()
        .unwrap_or(source_names.len() as u64) as u32;
    let has_direction = payload["has_direction"].as_bool().unwrap_or(false);
    let impact_bucket = finding_impact_bucket(clinical, wellness, data_quality, has_direction);

    let trait_label = trait_name
        .as_deref()
        .or(trait_category.as_deref())
        .unwrap_or("an indexed trait association");
    let gene_label = gene_symbol
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|g| format!(" near {g}"))
        .unwrap_or_default();
    let genotype_label = genotype
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|g| format!(" ({g})"))
        .unwrap_or_default();
    let summary = format!(
        "{rsid}{genotype_label}{gene_label}: {trait_label}. {directionality_label}. Evidence quality {}%, wellness relevance {}%.",
        (data_quality * 100.0).round() as u32,
        (wellness * 100.0).round() as u32,
    );

    ResearchFindingPreview {
        job_id: job_id.to_string(),
        sample_id,
        rsid,
        gene_symbol,
        genotype,
        trait_name,
        trait_category,
        personal_direction,
        directionality_label,
        impact_bucket,
        summary,
        wellness_actionability_score: wellness,
        clinical_actionability_score: clinical,
        data_quality_score: data_quality,
        source_count,
        source_names,
    }
}

fn emit_finding_previews(
    sink: &SweepProgressSink,
    job_id: &str,
    sample_id: i64,
    prepared: &[PreparedEnrichment],
) {
    let mut previews = prepared
        .iter()
        .map(|p| finding_preview_from_payload(job_id, sample_id, &p.payload))
        .collect::<Vec<_>>();
    previews.sort_by(|a, b| {
        let a_score =
            a.clinical_actionability_score + a.wellness_actionability_score + a.data_quality_score;
        let b_score =
            b.clinical_actionability_score + b.wellness_actionability_score + b.data_quality_score;
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for preview in previews.into_iter().take(8) {
        sink.emit_finding(preview);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_with_activity_pulse<F, Fut, T>(
    sink: &SweepProgressSink,
    job_id: &str,
    enriched_count: i64,
    total_markers: i64,
    current_rsid: Option<String>,
    current_source: Option<String>,
    fallback_phase: &str,
    work: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let sink_clone = sink.clone();
    let job_id = job_id.to_string();
    let rsid = current_rsid.clone();
    let source = current_source.clone();
    let fallback_phase = fallback_phase.to_string();
    let pulse = tokio::spawn(async move {
        let mut ticks = 0u64;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            ticks += 1;
            let elapsed = ticks * 5;
            let (phase, prepared, prefetch_done, total, _, live_rsid) = batch_activity_payload();
            let rsid_label = live_rsid
                .or(rsid.clone())
                .unwrap_or_else(|| "…".to_string());
            let phase_label = phase.unwrap_or_else(|| fallback_phase.clone());
            let batch_note = match (prepared, prefetch_done, total) {
                (Some(p), Some(pref), Some(t)) if t > 0 => {
                    format!(" · prefetch {}/{} · prepare {}/{}", pref, t, p, t)
                }
                (Some(p), _, Some(t)) if t > 0 => format!(" · {}/{} prepared", p, t),
                _ => String::new(),
            };
            emit_pulse(
                &sink_clone,
                &job_id,
                enriched_count,
                total_markers,
                Some(rsid_label.clone()),
                source.clone(),
                format!(
                    "{} {}{} (+{}s)",
                    phase_label, rsid_label, batch_note, elapsed
                ),
            );
        }
    });
    let results = work().await;
    pulse.abort();
    let _ = pulse.await;
    results
}

async fn run_prepare_with_pulse<F, Fut>(
    sink: &SweepProgressSink,
    job_id: &str,
    enriched_count: i64,
    total_markers: i64,
    current_rsid: Option<String>,
    current_source: Option<String>,
    work: F,
) -> Vec<Result<PreparedEnrichment, String>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Vec<Result<PreparedEnrichment, String>>>,
{
    run_with_activity_pulse(
        sink,
        job_id,
        enriched_count,
        total_markers,
        current_rsid,
        current_source,
        "prepare",
        work,
    )
    .await
}

async fn count_markers_needing_enrichment(
    markers: &[ScoredMarker],
    sample_id: i64,
    config: &QdrantConfig,
    force_reenrich: bool,
) -> Result<usize, String> {
    if force_reenrich {
        return Ok(markers.len());
    }
    let mut needs_work = 0usize;
    for chunk in markers.chunks(200) {
        let chunk_ids: Vec<u64> = chunk
            .iter()
            .map(|m| string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1)))
            .collect();
        let current = vector_store::classify_points_index_state(config, chunk_ids).await?;
        let current_set: HashSet<u64> = current.into_iter().collect();
        needs_work += chunk
            .iter()
            .filter(|m| {
                let id = string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1));
                !current_set.contains(&id)
            })
            .count();
    }
    Ok(needs_work)
}

async fn embed_named_vectors_parallel(
    prepared_ok: &[PreparedEnrichment],
    default_vectors: &[Vec<f32>],
    ollama_url: &str,
    model: &str,
) -> Vec<(u64, HashMap<String, Vec<f32>>, serde_json::Value)> {
    let parallel = tuning::embed_parallel().max(1);
    let semaphore = Arc::new(Semaphore::new(parallel));
    let mut join_set = JoinSet::new();

    for (idx, prepared) in prepared_ok.iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await;
        let prepared = prepared.clone();
        let default_vec = default_vectors.get(idx).cloned().unwrap_or_default();
        let ollama = ollama_url.to_string();
        let model_name = model.to_string();
        join_set.spawn(async move {
            let _permit = permit;
            let texts = build_named_vector_texts(
                prepared
                    .payload
                    .as_object()
                    .unwrap_or(&serde_json::Map::new()),
                &prepared.full_text,
            );
            let mut named = embed_named_vectors(&texts, &ollama, &model_name)
                .await
                .unwrap_or_else(|_| {
                    let mut m = HashMap::new();
                    m.insert(String::new(), default_vec.clone());
                    m
                });
            if !named.contains_key("") {
                named.insert(String::new(), default_vec);
            }
            (
                idx,
                (
                    string_to_u64(&prepared.point_id),
                    named,
                    prepared.payload.clone(),
                ),
            )
        });
    }

    #[allow(clippy::type_complexity)]
    let mut indexed: Vec<(usize, (u64, HashMap<String, Vec<f32>>, serde_json::Value))> =
        Vec::with_capacity(prepared_ok.len());
    while let Some(joined) = join_set.join_next().await {
        if let Ok(tuple) = joined {
            indexed.push(tuple);
        }
    }
    indexed.sort_by_key(|(idx, _)| *idx);
    indexed.into_iter().map(|(_, rest)| rest).collect()
}

/// Parallel Qdrant index scan at sweep start — pre-seeds progress for variants already indexed.
#[allow(clippy::too_many_arguments)]
async fn bootstrap_qdrant_index_cache(
    sink: &SweepProgressSink,
    job_id: &str,
    sample_id: i64,
    markers: &[ScoredMarker],
    config: &QdrantConfig,
    job: &mut ResearchJob,
    db_path: &Path,
    progress_tracker: &mut ProgressTracker,
    enriched_count: &mut i64,
    newly_enriched_count: i64,
    total_markers: i64,
) -> Result<HashSet<u64>, String> {
    const CHUNK: usize = 400;
    let parallel = tuning::prefetch_concurrency().clamp(4, 16);
    let semaphore = Arc::new(Semaphore::new(parallel));
    let mut join_set: JoinSet<Result<Vec<u64>, String>> = JoinSet::new();
    let chunks: Vec<&[ScoredMarker]> = markers.chunks(CHUNK).collect();
    let total_chunks = chunks.len();

    super::sweep_metrics::set_batch_activity("Qdrant bootstrap", "…", total_chunks.max(1) as u32);
    sweep_dbg(
        sink,
        format!(
            "Bootstrap: scanning {} markers in {} chunks (parallel={})",
            markers.len(),
            total_chunks,
            parallel
        ),
    );

    for chunk in chunks {
        if pending_sweep_stop().is_some() {
            break;
        }
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| e.to_string())?;
        let chunk_ids: Vec<u64> = chunk
            .iter()
            .map(|m| string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1)))
            .collect();
        let cfg = config.clone();
        join_set.spawn(async move {
            let _permit = permit;
            vector_store::classify_points_index_state(&cfg, chunk_ids).await
        });
    }

    let mut complete = HashSet::new();
    let mut chunks_done = 0usize;
    // Overall sweep % must not follow partial scan discoveries (climbs from 0 and
    // contradicts the known Qdrant sample baseline). Keep the caller's floor.
    let progress_floor = (*enriched_count).max(0);
    while let Some(joined) = join_set.join_next().await {
        if apply_pending_stop(
            job,
            db_path,
            sink,
            job_id,
            *enriched_count,
            newly_enriched_count,
            total_markers,
            progress_tracker,
        ) {
            join_set.abort_all();
            while join_set.join_next().await.is_some() {}
            return Ok(complete);
        }
        match joined {
            Ok(Ok(ids)) => {
                complete.extend(ids);
                chunks_done += 1;
                super::sweep_metrics::set_batch_prepared_count(chunks_done as u32);
                if chunks_done.is_multiple_of(4) || chunks_done == total_chunks {
                    let indexed_now = markers
                        .iter()
                        .filter(|m| {
                            let id =
                                string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1));
                            complete.contains(&id)
                        })
                        .count() as i64;
                    // Keep overall progress at the known Qdrant floor until the full scan finishes.
                    // Partial chunk discoveries climb from 0 and must not drive the hero %.
                    *enriched_count = progress_floor;
                    job.enriched_count = progress_floor;
                    job.last_updated = unix_now();
                    let _ = save_research_job(db_path, job);
                    emit_progress(
                        sink,
                        job_id,
                        "running",
                        progress_floor,
                        newly_enriched_count,
                        total_markers,
                        None,
                        Some("Qdrant bootstrap".to_string()),
                        format!(
                            "Bootstrap scan: {}/{} chunks — {} confirmed in scanned chunks (holding overall at {} already in Qdrant)",
                            chunks_done, total_chunks, indexed_now, progress_floor
                        ),
                        progress_tracker,
                        false,
                    );
                }
            }
            Ok(Err(e)) => {
                if is_fatal_enrichment_error(&e) {
                    return Err(e);
                }
                sweep_dbg(sink, format!("Bootstrap chunk failed (continuing): {}", e));
            }
            Err(e) => sweep_dbg(sink, format!("Bootstrap task join failed: {}", e)),
        }
    }

    if apply_pending_stop(
        job,
        db_path,
        sink,
        job_id,
        *enriched_count,
        newly_enriched_count,
        total_markers,
        progress_tracker,
    ) {
        return Ok(complete);
    }

    let final_indexed = markers
        .iter()
        .filter(|m| {
            let id = string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1));
            complete.contains(&id)
        })
        .count() as i64;
    *enriched_count = final_indexed;
    job.enriched_count = final_indexed;
    job.last_updated = unix_now();
    let _ = save_research_job(db_path, job);
    sweep_dbg(
        sink,
        format!(
            "Bootstrap complete: {} / {} markers already at enrichment v{}",
            final_indexed,
            total_markers,
            super::util::ENRICHMENT_VERSION
        ),
    );
    Ok(complete)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_research_loop(
    sample_id: i64,
    db_path: PathBuf,
    ollama_url: String,
    config: QdrantConfig,
    scope: ResearchScopeConfig,
    progress_sink: SweepProgressSink,
    resume_job: Option<ResearchJob>,
    force_reenrich: bool,
) -> Result<(), String> {
    struct SweepDebugAppGuard;
    impl Drop for SweepDebugAppGuard {
        fn drop(&mut self) {
            super::debug_log::set_emit_app(None);
        }
    }
    if let Some(app) = progress_sink.as_app() {
        super::debug_log::set_emit_app(Some(app.clone()));
    }
    let _sweep_debug_app = SweepDebugAppGuard;
    let sink = &progress_sink;

    let started_at = unix_now();
    let resume_from_rsid = resume_job.as_ref().and_then(|j| j.current_rsid.clone());

    // Mark resume jobs running immediately so the UI does not sit on "paused" through
    // long preflight (marker collect / Qdrant probes), and so Start cannot race with
    // a still-running worker that looks paused in SQLite.
    if let Some(ref existing) = resume_job {
        if pending_sweep_stop() == Some(SweepStopKind::Cancel) {
            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
            return Ok(());
        }
        let mut early = existing.clone();
        early.status = "running".to_string();
        early.last_updated = started_at;
        early.session_started_at = Some(started_at);
        early.error_message = None;
        let _ = save_research_job(&db_path, &early);
        emit_progress(
            sink,
            &early.job_id,
            "running",
            early.enriched_count,
            0,
            early.total_markers,
            resume_from_rsid.clone(),
            None,
            format!(
                "Resuming research job — preparing markers ({}/{})…",
                early.enriched_count, early.total_markers
            ),
            &mut ProgressTracker::new(started_at),
            false,
        );
    }

    let effective_scope = if let Some(ref existing) = resume_job {
        existing
            .scope_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<ResearchScopeConfig>(json).ok())
            .unwrap_or(scope)
    } else {
        scope
    };
    let scope_json = serde_json::to_string(&effective_scope).ok();

    // 1. Load markers for enabled scopes
    let markers =
        collect_markers_for_scopes(&db_path, sample_id, &effective_scope).unwrap_or_default();
    let total_markers = markers.len() as i64;

    if total_markers == 0 {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return Err(
            "No markers matched the selected sweep scopes. Enable at least one scope or sync GWAS reference data.".to_string(),
        );
    }

    // Pre-flight: check connection and collection/index existence.
    let provider = vector_store::provider_from_config(&config);
    if !provider.supports_dense_research() {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return Err(format!(
            "Vector provider '{}' is not supported for research sweeps.",
            provider.as_str()
        ));
    }
    let conn_status = vector_store::test_connection(&config).await;
    if !conn_status.success {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return Err(format!(
            "Cannot start research: {} is not reachable. Error: {}",
            provider.as_str(),
            conn_status
                .error
                .unwrap_or_else(|| "Unknown connection error".to_string())
        ));
    }
    if !conn_status.collection_exists {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        let hint = match provider {
            VectorProvider::Pinecone => {
                "Pinecone index host is reachable but stats failed — check API key and index host URL."
                    .to_string()
            }
            VectorProvider::Qdrant => format!(
                "Qdrant collection '{}' does not exist. Create the collection first.",
                config.collection
            ),
            VectorProvider::Chroma => format!(
                "Chroma collection '{}' does not exist. Create it from Connections or in Chroma.",
                config.collection
            ),
            VectorProvider::Weaviate => format!(
                "Weaviate class '{}' does not exist. Create it from Connections or in Weaviate.",
                config.collection
            ),
        };
        return Err(format!("Cannot start research: {hint}"));
    }

    let (ollama_lat, qdrant_lat) = tuning::probe_service_latencies(&ollama_url, &config.url).await;
    let pipeline_tuning = install_sweep_tuning(
        &ollama_url,
        &config.url,
        &effective_scope,
        ollama_lat,
        qdrant_lat,
    );
    let enrichment_sources = super::sources_config::active_enrichment_sources();
    let _tuning_guard = SweepTuningGuard;
    eprintln!(
        "Sweep tuning: profile={} batch={} prepare={} fast={} gnomad={} sources=[gnomad={} clinvar={} pubmed={} gtex={} vep={} secondary={} supplement={}]",
        pipeline_tuning.profile,
        pipeline_tuning.enrich_batch_size,
        pipeline_tuning.prepare_concurrency,
        pipeline_tuning.fast_sweep,
        !pipeline_tuning.skip_gnomad,
        enrichment_sources.gnomad,
        enrichment_sources.clinvar_live,
        enrichment_sources.pubmed,
        enrichment_sources.gtex,
        enrichment_sources.vep_dbsnp,
        enrichment_sources.secondary,
        enrichment_sources.supplement_missing,
    );
    sweep_dbg(
        sink,
        format!(
            "Loop start sample={} total_markers={} resume={:?} force_reenrich={}",
            sample_id, total_markers, resume_from_rsid, force_reenrich
        ),
    );

    let _ = vector_store::maybe_ensure_payload_indexes(&config).await;

    // Honor cancel during long preflight (marker collect / Qdrant probes) before a job row exists.
    if pending_sweep_stop() == Some(SweepStopKind::Cancel) {
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        return Ok(());
    }

    if resume_from_rsid.is_none() {
        let needs_work =
            count_markers_needing_enrichment(&markers, sample_id, &config, force_reenrich).await?;
        if pending_sweep_stop() == Some(SweepStopKind::Cancel) {
            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
            return Ok(());
        }
        if needs_work == 0 {
            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
            let scope_preview = preview_research_scope(&db_path, sample_id, &effective_scope)
                .unwrap_or(ResearchScopePreview {
                    curated: 0,
                    agent_discoveries: 0,
                    gwas_discovery: 0,
                    non_reference: 0,
                    total_unique: total_markers as u64,
                    genotype_total: 0,
                    gwas_reference_count: 0,
                    gwas_genome_overlap: 0,
                    gwas_beyond_cap: 0,
                });
            return Err(format!(
                "All {} queued markers are already enriched at v{} in Qdrant — nothing new to do.\n\
                 • Increase GWAS cap ({} of {} GWAS rsIDs in your genome are beyond the current cap)\n\
                 • Or use Force re-enrich to rebuild vectors with the latest enrichment pipeline",
                total_markers,
                super::util::ENRICHMENT_VERSION,
                scope_preview.gwas_beyond_cap,
                scope_preview.gwas_genome_overlap
            ));
        }
    }

    // 2. Create or restore job record
    let mut job = if let Some(existing) = resume_job {
        let mut restored = existing.clone();
        restored.status = "running".to_string();
        restored.total_markers = total_markers;
        restored.last_updated = started_at;
        restored.session_started_at = Some(started_at);
        restored.error_message = None;
        if restored.enriched_count > total_markers {
            restored.enriched_count = total_markers;
        }
        restored
    } else {
        ResearchJob {
            job_id: format!("job_{}_{}", sample_id, started_at),
            sample_id,
            status: "running".to_string(),
            total_markers,
            enriched_count: 0,
            priority_complete: false,
            current_rsid: None,
            current_source: None,
            started_at,
            last_updated: started_at,
            session_started_at: Some(started_at),
            error_message: None,
            scope_json: scope_json.clone(),
            loop_active: None,
            live_message: None,
            activity_phase: None,
            batch_prepared: None,
            batch_prefetch_done: None,
            batch_total: None,
            batch_elapsed_secs: None,
            qdrant_sample_count: None,
            session_elapsed_secs: None,
        }
    };
    let job_id = job.job_id.clone();
    let mut enriched_count = job.enriched_count;
    super::sweep_metrics::set_sweep_session_started();
    let qdrant_baseline = super::count_qdrant_points(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        Some(sample_id),
    )
    .await
    .unwrap_or(0);
    super::sweep_metrics::set_qdrant_sample_baseline(qdrant_baseline);
    // Seed overall progress from the live collection count so the hero % matches the
    // "already in Qdrant" banner instead of starting at 0% during bootstrap.
    if !force_reenrich {
        let seeded = (qdrant_baseline as i64).clamp(0, total_markers);
        if seeded > enriched_count {
            enriched_count = seeded;
            job.enriched_count = seeded;
        }
    }
    let sweep_started_at = unix_now();
    let mut progress_tracker = ProgressTracker::new(sweep_started_at);
    let mut newly_enriched_count = 0i64;

    // Cancel may have landed during preflight — persist idle and exit before emitting "running".
    if apply_pending_stop(
        &mut job,
        &db_path,
        sink,
        &job_id,
        enriched_count,
        newly_enriched_count,
        total_markers,
        &mut progress_tracker,
    ) {
        return Ok(());
    }

    let _ = save_research_job(&db_path, &job);

    // 3. Emit initial progress
    let initial_message = if resume_from_rsid.is_some() {
        format!(
            "Research job resumed at {} ({}/{})",
            resume_from_rsid.as_deref().unwrap_or(""),
            enriched_count,
            total_markers
        )
    } else {
        format!(
            "Research job started. {} markers queued (curated={}, agent={}, gwas={}, non-ref={}). {} already indexed for this sample in Qdrant.",
            total_markers,
            effective_scope.curated,
            effective_scope.agent_discoveries,
            effective_scope.gwas_discovery,
            effective_scope.non_reference,
            qdrant_baseline
        )
    };
    emit_progress(
        sink,
        &job_id,
        "running",
        enriched_count,
        newly_enriched_count,
        total_markers,
        resume_from_rsid.clone(),
        None,
        initial_message,
        &mut progress_tracker,
        false,
    );

    let supplement_at_start =
        super::sources_config::enrichment_supplement_missing() && !force_reenrich;
    let mut bootstrap_cache: Option<HashSet<u64>> = None;
    if !force_reenrich && !supplement_at_start && resume_from_rsid.is_none() {
        match bootstrap_qdrant_index_cache(
            sink,
            &job_id,
            sample_id,
            &markers,
            &config,
            &mut job,
            &db_path,
            &mut progress_tracker,
            &mut enriched_count,
            newly_enriched_count,
            total_markers,
        )
        .await
        {
            Ok(cache) => bootstrap_cache = Some(cache),
            Err(e) => sweep_dbg(sink, format!("Bootstrap skipped: {}", e)),
        }
        if apply_pending_stop(
            &mut job,
            &db_path,
            sink,
            &job_id,
            enriched_count,
            newly_enriched_count,
            total_markers,
            &mut progress_tracker,
        ) {
            return Ok(());
        }
    }

    // 4. Split into priority (score > 0.15) and background (score <= 0.15)
    let priority: Vec<&ScoredMarker> = markers
        .iter()
        .filter(|m| m.significance_score > 0.15)
        .collect();
    let background: Vec<&ScoredMarker> = markers
        .iter()
        .filter(|m| m.significance_score <= 0.15)
        .collect();

    let resume_plan = plan_resume(
        priority.as_slice(),
        background.as_slice(),
        resume_from_rsid.as_deref(),
    );
    let mut skip_until_resume = matches!(resume_plan, ResumePlan::NotFound);
    let mut resume_chunks_skipped = 0u32;
    let passes: [(&[&ScoredMarker], bool); 2] =
        [(priority.as_slice(), false), (background.as_slice(), true)];

    for (pass_idx, &(pass_markers, is_background)) in passes.iter().enumerate() {
        if is_background {
            job.priority_complete = true;
        }

        let pass_slice: &[&ScoredMarker] = match &resume_plan {
            ResumePlan::StartInPass {
                pass_index,
                marker_index,
            } if pass_idx < *pass_index => {
                sweep_dbg(
                    sink,
                    format!("Resume: skipping pass {} (already completed)", pass_idx),
                );
                continue;
            }
            ResumePlan::StartInPass {
                pass_index,
                marker_index,
            } if pass_idx == *pass_index => {
                let target = resume_from_rsid.as_deref().unwrap_or("?");
                sweep_dbg(
                    sink,
                    format!(
                        "Resume: jumping to {} at index {} in pass {}",
                        target, marker_index, pass_idx
                    ),
                );
                super::sweep_metrics::set_batch_activity(
                    "resume seek",
                    target,
                    pass_markers.len().max(1) as u32,
                );
                emit_pulse(
                    sink,
                    &job_id,
                    enriched_count,
                    total_markers,
                    resume_from_rsid.clone(),
                    Some("resume seek".to_string()),
                    format!(
                        "Jumping to resume point {} ({}/{})",
                        target, enriched_count, total_markers
                    ),
                );
                skip_until_resume = false;
                &pass_markers[*marker_index..]
            }
            _ => pass_markers,
        };

        for chunk in pass_slice.chunks(200) {
            // Cooperative cancel (must win over pause — same flag used to be shared).
            if apply_pending_stop(
                &mut job,
                &db_path,
                sink,
                &job_id,
                enriched_count,
                newly_enriched_count,
                total_markers,
                &mut progress_tracker,
            ) {
                return Ok(());
            }

            // Resume seek: skip whole chunks until the target rsid appears (legacy path when NotFound).
            if skip_until_resume
                && let Some(target) = resume_from_rsid.as_deref()
                && !chunk.iter().any(|m| m.rsid == target)
            {
                if resume_chunks_skipped.is_multiple_of(5) {
                    super::sweep_metrics::set_batch_activity(
                        "resume seek",
                        target,
                        pass_markers.len().max(1) as u32,
                    );
                    emit_pulse(
                        sink,
                        &job_id,
                        enriched_count,
                        total_markers,
                        resume_from_rsid.clone(),
                        Some("resume seek".to_string()),
                        format!(
                            "Seeking resume point {}… skipped {} chunks ({}/{})",
                            target, resume_chunks_skipped, enriched_count, total_markers
                        ),
                    );
                }
                resume_chunks_skipped += 1;
                continue;
            }

            // 1. Calculate IDs for all markers in this chunk
            let chunk_ids: Vec<u64> = chunk
                .iter()
                .map(|m| string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1)))
                .collect();

            // 2. Classify which points are complete vs need enrichment (missing, stale, or supplement)
            let supplement_missing =
                super::sources_config::enrichment_supplement_missing() && !force_reenrich;

            let enrichment_sources = super::sources_config::active_enrichment_sources();

            let chunk_first_rsid = chunk.first().map(|m| m.rsid.clone());
            if !force_reenrich {
                super::sweep_metrics::set_batch_activity(
                    "Checking Qdrant",
                    chunk_first_rsid.as_deref().unwrap_or("…"),
                    chunk.len().max(1) as u32,
                );
                sweep_dbg(
                    sink,
                    format!(
                        "Checking Qdrant cache for {} variants (starting {})",
                        chunk.len(),
                        chunk_first_rsid.as_deref().unwrap_or("?")
                    ),
                );
            }

            let complete_ids: HashSet<u64> = if force_reenrich {
                HashSet::new()
            } else if let Some(ref cache) = bootstrap_cache {
                chunk_ids
                    .iter()
                    .filter(|id| cache.contains(id))
                    .copied()
                    .collect()
            } else if supplement_missing {
                let cfg = config.clone();
                let ids_result = run_with_activity_pulse(
                    sink,
                    &job_id,
                    enriched_count,
                    total_markers,
                    chunk_first_rsid.clone(),
                    Some(format!("{} sweep check", provider.as_str())),
                    "Checking vector store",
                    || async move {
                        vector_store::classify_points_sweep_state(
                            &cfg,
                            chunk_ids,
                            &enrichment_sources,
                            true,
                        )
                        .await
                    },
                )
                .await;
                match ids_result {
                    Ok(result) => result.complete_ids.into_iter().collect(),
                    Err(e) => {
                        if e.contains("401 Unauthorized") {
                            job.status = "error".to_string();
                            job.error_message = Some(e.clone());
                            job.last_updated = unix_now();
                            let _ = save_research_job(&db_path, &job);
                            emit_progress(
                                sink,
                                &job_id,
                                "error",
                                enriched_count,
                                newly_enriched_count,
                                total_markers,
                                chunk.first().map(|m| m.rsid.clone()),
                                None,
                                format!("Fatal connection error: {}", e),
                                &mut progress_tracker,
                                false,
                            );
                            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                            return Err(e);
                        }
                        HashSet::new()
                    }
                }
            } else {
                let cfg = config.clone();
                let ids_result = run_with_activity_pulse(
                    sink,
                    &job_id,
                    enriched_count,
                    total_markers,
                    chunk_first_rsid.clone(),
                    Some(format!("{} cache", provider.as_str())),
                    "Checking vector store",
                    || async move {
                        vector_store::classify_points_index_state(&cfg, chunk_ids).await
                    },
                )
                .await;
                match ids_result {
                    Ok(ids) => ids.into_iter().collect(),
                    Err(e) => {
                        if e.contains("401 Unauthorized") {
                            job.status = "error".to_string();
                            job.error_message = Some(e.clone());
                            job.last_updated = unix_now();
                            let _ = save_research_job(&db_path, &job);
                            emit_progress(
                                sink,
                                &job_id,
                                "error",
                                enriched_count,
                                newly_enriched_count,
                                total_markers,
                                chunk.first().map(|m| m.rsid.clone()),
                                None,
                                format!("Fatal connection error: {}", e),
                                &mut progress_tracker,
                                false,
                            );
                            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                            return Err(e);
                        }
                        sweep_dbg(
                            sink,
                            format!("Vector classify failed (continuing as empty cache): {}", e),
                        );
                        HashSet::new()
                    }
                }
            };

            if skip_until_resume {
                emit_pulse(
                    sink,
                    &job_id,
                    enriched_count,
                    total_markers,
                    resume_from_rsid.clone(),
                    Some("resume seek".to_string()),
                    format!(
                        "Resuming at chunk containing {} ({}/{})",
                        resume_from_rsid.as_deref().unwrap_or("?"),
                        enriched_count,
                        total_markers
                    ),
                );
            }

            // 3. Separate current-indexed vs needs enrichment (missing or stale)
            let mut missing_markers = Vec::new();
            let mut skipped_count = 0;

            for marker in chunk {
                if skip_until_resume {
                    if resume_from_rsid.as_deref() == Some(marker.rsid.as_str()) {
                        skip_until_resume = false;
                    } else {
                        // Already counted in enriched_count before pause — do not double-count.
                        continue;
                    }
                }

                let p_id =
                    string_to_u64(&format!("{}_{}_{}", sample_id, marker.rsid, marker.allele1));
                if !force_reenrich && complete_ids.contains(&p_id) {
                    skipped_count += 1;
                } else {
                    missing_markers.push(*marker);
                }
            }

            if skipped_count > 0 {
                if bootstrap_cache.is_none() {
                    enriched_count += skipped_count;
                    job.enriched_count = enriched_count;
                }
                job.last_updated = unix_now();
                if let Some(last_skipped) = chunk.iter().rfind(|m| {
                    let p_id = string_to_u64(&format!("{}_{}_{}", sample_id, m.rsid, m.allele1));
                    complete_ids.contains(&p_id)
                }) {
                    job.current_rsid = Some(last_skipped.rsid.clone());
                }
                let _ = save_research_job(&db_path, &job);

                emit_progress(
                    sink,
                    &job_id,
                    "running",
                    enriched_count,
                    newly_enriched_count,
                    total_markers,
                    job.current_rsid.clone(),
                    Some("Qdrant Cache".to_string()),
                    format!(
                        "Skipped {} variants already at enrichment v{} in chunk",
                        skipped_count,
                        super::util::ENRICHMENT_VERSION
                    ),
                    &mut progress_tracker,
                    false,
                );
            }

            // 4. Process missing markers in sub-batches: parallel prepare (pipelined) → batch embed → batch upsert
            let sub_batches: Vec<Vec<ScoredMarker>> = missing_markers
                .chunks(enrich_batch_size())
                .map(|chunk| chunk.iter().map(|m| (*m).clone()).collect())
                .collect();

            let mut next_prepare: Option<
                tokio::task::JoinHandle<Vec<Result<PreparedEnrichment, String>>>,
            > = None;

            for (batch_idx, sub_batch) in sub_batches.iter().enumerate() {
                if let Some(first) = sub_batch.first() {
                    job.current_rsid = Some(first.rsid.clone());
                }
                if apply_pending_stop(
                    &mut job,
                    &db_path,
                    sink,
                    &job_id,
                    enriched_count,
                    newly_enriched_count,
                    total_markers,
                    &mut progress_tracker,
                ) {
                    return Ok(());
                }

                let fast = super::http::sweep_fast_mode();
                job.current_source = Some(if fast {
                    "fast: local GWAS+embed".to_string()
                } else {
                    "local GWAS+gnomAD+embed".to_string()
                });

                let sub_refs: Vec<&ScoredMarker> = sub_batch.iter().collect();
                if let Some(first) = sub_batch.first() {
                    job.current_rsid = Some(first.rsid.clone());
                    job.current_source = Some("preparing batch".to_string());
                    job.last_updated = unix_now();
                    let _ = save_research_job(&db_path, &job);
                    emit_progress(
                        sink,
                        &job_id,
                        "running",
                        enriched_count,
                        newly_enriched_count,
                        total_markers,
                        Some(first.rsid.clone()),
                        job.current_source.clone(),
                        format!(
                            "Preparing enrichment batch ({} variants, starting {})",
                            sub_batch.len(),
                            first.rsid
                        ),
                        &mut progress_tracker,
                        true,
                    );
                    sweep_dbg(
                        sink,
                        format!(
                            "Batch {}: {} variants starting at {}",
                            batch_idx + 1,
                            sub_batch.len(),
                            first.rsid
                        ),
                    );
                }

                let prepared_results = if let Some(handle) = next_prepare.take() {
                    run_prepare_with_pulse(
                        sink,
                        &job_id,
                        enriched_count,
                        total_markers,
                        job.current_rsid.clone(),
                        job.current_source.clone(),
                        || async move {
                            match handle.await {
                                Ok(results) => results,
                                Err(e) => vec![Err(format!("Prepare task failed: {}", e))],
                            }
                        },
                    )
                    .await
                } else {
                    run_prepare_with_pulse(
                        sink,
                        &job_id,
                        enriched_count,
                        total_markers,
                        job.current_rsid.clone(),
                        job.current_source.clone(),
                        || async {
                            process_enrichment_batch(
                                sample_id,
                                &sub_refs,
                                &db_path,
                                &ollama_url,
                                &config,
                            )
                            .await
                        },
                    )
                    .await
                };

                if batch_idx + 1 < sub_batches.len() && tuning::pipeline_depth() > 1 {
                    let next_batch = sub_batches[batch_idx + 1].clone();
                    let db_path_spawn = db_path.clone();
                    let config_spawn = config.clone();
                    next_prepare = Some(tokio::spawn(async move {
                        let refs: Vec<&ScoredMarker> = next_batch.iter().collect();
                        process_enrichment_batch(
                            sample_id,
                            &refs,
                            &db_path_spawn,
                            "",
                            &config_spawn,
                        )
                        .await
                    }));
                }

                let mut prepared_ok: Vec<PreparedEnrichment> = Vec::new();
                let mut batch_errors: Vec<String> = Vec::new();
                for (marker, result) in sub_batch.iter().zip(prepared_results) {
                    match result {
                        Ok(prepared) => prepared_ok.push(prepared),
                        Err(e) => {
                            if is_fatal_enrichment_error(&e) {
                                job.status = "error".to_string();
                                job.error_message = Some(e.clone());
                                job.last_updated = unix_now();
                                job.current_rsid = Some(marker.rsid.clone());
                                let _ = save_research_job(&db_path, &job);
                                emit_progress(
                                    sink,
                                    &job_id,
                                    "error",
                                    enriched_count,
                                    newly_enriched_count,
                                    total_markers,
                                    Some(marker.rsid.clone()),
                                    None,
                                    format!("Fatal connection error: {}", e),
                                    &mut progress_tracker,
                                    false,
                                );
                                RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                                return Err(e);
                            }
                            batch_errors.push(format!("{}: {}", marker.rsid, e));
                        }
                    }
                }

                if !batch_errors.is_empty() {
                    job.last_updated = unix_now();
                    let _ = save_research_job(&db_path, &job);
                }

                if prepared_ok.is_empty() {
                    let err_preview = batch_errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "all variants failed prepare".to_string());
                    emit_progress(
                        sink,
                        &job_id,
                        "running",
                        enriched_count,
                        newly_enriched_count,
                        total_markers,
                        job.current_rsid.clone(),
                        Some("batch errors".to_string()),
                        format!(
                            "Batch skipped (0 prepared, {} errors). Last: {}",
                            batch_errors.len(),
                            err_preview
                        ),
                        &mut progress_tracker,
                        true,
                    );
                    continue;
                }

                super::sweep_metrics::set_batch_phase("embedding");
                job.current_source = Some("embedding".to_string());
                if let Some(first) = prepared_ok.first() {
                    super::sweep_metrics::set_batch_embedding(
                        &first.rsid,
                        prepared_ok.len() as u32,
                    );
                }

                let texts: Vec<String> = prepared_ok.iter().map(|p| p.full_text.clone()).collect();

                let ollama_url_embed = ollama_url.clone();
                let embed_model = config.embedding_model.clone();
                let vectors = match run_with_activity_pulse(
                    sink,
                    &job_id,
                    enriched_count,
                    total_markers,
                    job.current_rsid.clone(),
                    job.current_source.clone(),
                    "embedding",
                    || async {
                        timed_async(
                            SweepPhase::Embed,
                            embed_texts_batch(&texts, &ollama_url_embed, &embed_model),
                        )
                        .await
                    },
                )
                .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        if is_fatal_enrichment_error(&e) {
                            job.status = "error".to_string();
                            job.error_message = Some(e.clone());
                            job.last_updated = unix_now();
                            let _ = save_research_job(&db_path, &job);
                            emit_progress(
                                sink,
                                &job_id,
                                "error",
                                enriched_count,
                                newly_enriched_count,
                                total_markers,
                                prepared_ok.last().map(|p| p.rsid.clone()),
                                None,
                                format!("Fatal connection error: {}", e),
                                &mut progress_tracker,
                                false,
                            );
                            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                            return Err(e);
                        }
                        continue;
                    }
                };

                let use_named = provider.supports_named_vectors()
                    && super::evidence::named_vectors::named_vectors_enabled(&config)
                    && named_vectors_in_sweep();
                if use_named {
                    let named_points = embed_named_vectors_parallel(
                        &prepared_ok,
                        &vectors,
                        &ollama_url,
                        &config.embedding_model,
                    )
                    .await;
                    super::sweep_metrics::set_batch_qdrant();
                    if let Err(e) = timed_async(
                        SweepPhase::Qdrant,
                        super::qdrant::upsert_points_batch_named(
                            &config.url,
                            config.api_key.as_deref(),
                            &config.collection,
                            named_points,
                        ),
                    )
                    .await
                    {
                        if is_fatal_enrichment_error(&e) {
                            job.status = "error".to_string();
                            job.error_message = Some(e.clone());
                            job.last_updated = unix_now();
                            let _ = save_research_job(&db_path, &job);
                            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                            return Err(e);
                        }
                        continue;
                    }
                } else {
                    let points: Vec<(u64, Vec<f32>, serde_json::Value)> = prepared_ok
                        .iter()
                        .zip(vectors)
                        .map(|(prepared, vector)| {
                            (
                                string_to_u64(&prepared.point_id),
                                vector,
                                prepared.payload.clone(),
                            )
                        })
                        .collect();

                    super::sweep_metrics::set_batch_qdrant();
                    if let Err(e) = timed_async(
                        SweepPhase::Qdrant,
                        vector_store::upsert_dense_batch(&config, points),
                    )
                    .await
                    {
                        if is_fatal_enrichment_error(&e) {
                            job.status = "error".to_string();
                            job.error_message = Some(e.clone());
                            job.last_updated = unix_now();
                            let _ = save_research_job(&db_path, &job);
                            RESEARCH_RUNNING.store(false, Ordering::SeqCst);
                            return Err(e);
                        }
                        continue;
                    }
                }

                let _ = tauri::async_runtime::spawn_blocking({
                    let db_path = db_path.clone();
                    let batch = prepared_ok.clone();
                    move || promote_enrichment_batch(&db_path, sample_id, &batch)
                })
                .await;
                emit_finding_previews(sink, &job_id, sample_id, &prepared_ok);

                let batch_count = prepared_ok.len() as i64;
                enriched_count += batch_count;
                newly_enriched_count += batch_count;
                job.enriched_count = enriched_count;
                job.last_updated = unix_now();
                job.current_rsid = prepared_ok.last().map(|p| p.rsid.clone());
                let msg = if batch_errors.is_empty() {
                    format!(
                        "Enriched {} variants ({}/{})",
                        batch_count, enriched_count, total_markers
                    )
                } else {
                    format!(
                        "Enriched {} variants ({}/{}), skipped {}",
                        batch_count,
                        enriched_count,
                        total_markers,
                        batch_errors.len()
                    )
                };
                emit_progress(
                    sink,
                    &job_id,
                    "running",
                    enriched_count,
                    newly_enriched_count,
                    total_markers,
                    job.current_rsid.clone(),
                    job.current_source.clone(),
                    msg,
                    &mut progress_tracker,
                    true,
                );
            }
        }
    }

    // 5. Complete
    job.status = "complete".to_string();
    job.priority_complete = true;
    job.last_updated = unix_now();
    job.current_rsid = None;
    job.current_source = None;
    let _ = save_research_job(&db_path, &job);
    emit_progress(
        sink,
        &job_id,
        "complete",
        enriched_count,
        newly_enriched_count,
        total_markers,
        None,
        None,
        if newly_enriched_count > 0 {
            format!(
                "Research complete. {} newly enriched ({} already indexed in queue).",
                newly_enriched_count,
                enriched_count - newly_enriched_count
            )
        } else {
            format!(
                "Research complete. All {} queued markers were already indexed in Qdrant.",
                enriched_count
            )
        },
        &mut progress_tracker,
        true,
    );

    super::sweep_metrics::clear_batch_activity();
    super::sweep_metrics::clear_sweep_session();
    RESEARCH_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}
