// ./src-tauri/src/research/sweep_metrics.rs
//! In-memory sweep phase timers (atomics only — no per-variant DB writes).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum SweepPhase {
    Gwas,
    Gnomad,
    Clinvar,
    Pubmed,
    Gtex,
    Vep,
    Embed,
    Qdrant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhaseMetricSnapshot {
    pub count: u64,
    pub total_ms: f64,
    pub avg_ms: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cache_hits: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cache_lookups: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_hit_rate: Option<f64>,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SweepPhaseMetricsSnapshot {
    pub gwas: PhaseMetricSnapshot,
    pub gnomad: PhaseMetricSnapshot,
    pub clinvar: PhaseMetricSnapshot,
    pub pubmed: PhaseMetricSnapshot,
    pub gtex: PhaseMetricSnapshot,
    pub vep: PhaseMetricSnapshot,
    pub embed: PhaseMetricSnapshot,
    pub qdrant: PhaseMetricSnapshot,
    pub gnomad_cache_hits: u64,
    pub gnomad_cache_lookups: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gnomad_cache_hit_rate: Option<f64>,
}

struct PhaseSlot {
    count: AtomicU64,
    total_ns: AtomicU64,
    cache_hits: AtomicU64,
    cache_lookups: AtomicU64,
}

impl PhaseSlot {
    const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            total_ns: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_lookups: AtomicU64::new(0),
        }
    }

    fn record(&self, duration: Duration) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    fn record_cache(&self, hit: bool) {
        self.cache_lookups.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> PhaseMetricSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let total_ns = self.total_ns.load(Ordering::Relaxed);
        let total_ms = total_ns as f64 / 1_000_000.0;
        let avg_ms = if count > 0 {
            total_ms / count as f64
        } else {
            0.0
        };
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_lookups = self.cache_lookups.load(Ordering::Relaxed);
        let cache_hit_rate = if cache_lookups > 0 {
            Some(cache_hits as f64 / cache_lookups as f64)
        } else {
            None
        };
        PhaseMetricSnapshot {
            count,
            total_ms,
            avg_ms,
            cache_hits,
            cache_lookups,
            cache_hit_rate,
        }
    }

    fn reset(&self) {
        self.count.store(0, Ordering::Relaxed);
        self.total_ns.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_lookups.store(0, Ordering::Relaxed);
    }
}

struct MetricsStore {
    gwas: PhaseSlot,
    gnomad: PhaseSlot,
    clinvar: PhaseSlot,
    pubmed: PhaseSlot,
    gtex: PhaseSlot,
    vep: PhaseSlot,
    embed: PhaseSlot,
    qdrant: PhaseSlot,
    gnomad_cache_hits: AtomicU64,
    gnomad_cache_lookups: AtomicU64,
}

impl MetricsStore {
    const fn new() -> Self {
        Self {
            gwas: PhaseSlot::new(),
            gnomad: PhaseSlot::new(),
            clinvar: PhaseSlot::new(),
            pubmed: PhaseSlot::new(),
            gtex: PhaseSlot::new(),
            vep: PhaseSlot::new(),
            embed: PhaseSlot::new(),
            qdrant: PhaseSlot::new(),
            gnomad_cache_hits: AtomicU64::new(0),
            gnomad_cache_lookups: AtomicU64::new(0),
        }
    }

    fn slot(&self, phase: SweepPhase) -> &PhaseSlot {
        match phase {
            SweepPhase::Gwas => &self.gwas,
            SweepPhase::Gnomad => &self.gnomad,
            SweepPhase::Clinvar => &self.clinvar,
            SweepPhase::Pubmed => &self.pubmed,
            SweepPhase::Gtex => &self.gtex,
            SweepPhase::Vep => &self.vep,
            SweepPhase::Embed => &self.embed,
            SweepPhase::Qdrant => &self.qdrant,
        }
    }

    fn reset(&self) {
        self.gwas.reset();
        self.gnomad.reset();
        self.clinvar.reset();
        self.pubmed.reset();
        self.gtex.reset();
        self.vep.reset();
        self.embed.reset();
        self.qdrant.reset();
        self.gnomad_cache_hits.store(0, Ordering::Relaxed);
        self.gnomad_cache_lookups.store(0, Ordering::Relaxed);
    }

    fn snapshot(&self) -> SweepPhaseMetricsSnapshot {
        let gnomad_hits = self.gnomad_cache_hits.load(Ordering::Relaxed);
        let gnomad_lookups = self.gnomad_cache_lookups.load(Ordering::Relaxed);
        let gnomad_cache_hit_rate = if gnomad_lookups > 0 {
            Some(gnomad_hits as f64 / gnomad_lookups as f64)
        } else {
            None
        };

        let mut gnomad = self.gnomad.snapshot();
        gnomad.cache_hits = gnomad_hits;
        gnomad.cache_lookups = gnomad_lookups;
        gnomad.cache_hit_rate = gnomad_cache_hit_rate;

        SweepPhaseMetricsSnapshot {
            gwas: self.gwas.snapshot(),
            gnomad,
            clinvar: self.clinvar.snapshot(),
            pubmed: self.pubmed.snapshot(),
            gtex: self.gtex.snapshot(),
            vep: self.vep.snapshot(),
            embed: self.embed.snapshot(),
            qdrant: self.qdrant.snapshot(),
            gnomad_cache_hits: gnomad_hits,
            gnomad_cache_lookups: gnomad_lookups,
            gnomad_cache_hit_rate,
        }
    }
}

static METRICS: MetricsStore = MetricsStore::new();

pub fn reset_sweep_metrics() {
    METRICS.reset();
}

pub fn record_phase(phase: SweepPhase, duration: Duration) {
    METRICS.slot(phase).record(duration);
}

pub fn record_phase_cache(phase: SweepPhase, hit: bool) {
    METRICS.slot(phase).record_cache(hit);
}

pub fn record_gnomad_cache_lookup(hit: bool) {
    METRICS.gnomad_cache_lookups.fetch_add(1, Ordering::Relaxed);
    if hit {
        METRICS.gnomad_cache_hits.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn phase_for_api_source(source_name: &str) -> Option<SweepPhase> {
    match source_name {
        "gwas_catalog" => Some(SweepPhase::Gwas),
        "clinvar" => Some(SweepPhase::Clinvar),
        "pubmed" => Some(SweepPhase::Pubmed),
        "gtex" => Some(SweepPhase::Gtex),
        "ensembl" | "dbsnp" => Some(SweepPhase::Vep),
        _ => None,
    }
}

pub fn snapshot_sweep_metrics() -> SweepPhaseMetricsSnapshot {
    METRICS.snapshot()
}

// ── Live batch activity (for UI pulses during long prepare steps) ─────────────

use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchActivitySnapshot {
    pub phase: String,
    pub current_rsid: String,
    /// Variants finished in the prepare step.
    pub prepared: u32,
    /// Variants finished in gnomAD / API prefetch steps.
    pub prefetch_done: u32,
    pub total: u32,
    pub started_at: i64,
}

static BATCH_ACTIVITY: Mutex<Option<BatchActivitySnapshot>> = Mutex::new(None);
static LAST_LIVE_MESSAGE: Mutex<Option<String>> = Mutex::new(None);
static SWEEP_SESSION_STARTED: Mutex<Option<i64>> = Mutex::new(None);
static QDRANT_SAMPLE_BASELINE: Mutex<Option<u64>> = Mutex::new(None);

fn phase_rank(phase: &str) -> u8 {
    let p = phase.to_lowercase();
    if p.contains("qdrant") {
        4
    } else if p.contains("embedding") {
        3
    } else if p.contains("prepare") {
        2
    } else if p.contains("prefetch") || p.contains("gnomad") || p.contains("api") {
        1
    } else {
        0
    }
}

pub fn set_sweep_session_started() {
    let now = crate::research::util::unix_now();
    if let Ok(mut guard) = SWEEP_SESSION_STARTED.lock() {
        *guard = Some(now);
    }
}

pub fn clear_sweep_session() {
    if let Ok(mut guard) = SWEEP_SESSION_STARTED.lock() {
        *guard = None;
    }
    if let Ok(mut guard) = QDRANT_SAMPLE_BASELINE.lock() {
        *guard = None;
    }
}

pub fn set_qdrant_sample_baseline(count: u64) {
    if let Ok(mut guard) = QDRANT_SAMPLE_BASELINE.lock() {
        *guard = Some(count);
    }
}

pub fn qdrant_sample_baseline() -> Option<u64> {
    QDRANT_SAMPLE_BASELINE.lock().ok().and_then(|g| *g)
}

pub fn sweep_session_elapsed_secs() -> Option<i64> {
    let started = SWEEP_SESSION_STARTED.lock().ok().and_then(|g| *g)?;
    Some((crate::research::util::unix_now() - started).max(0))
}

pub fn set_live_message(message: impl Into<String>) {
    if let Ok(mut guard) = LAST_LIVE_MESSAGE.lock() {
        *guard = Some(message.into());
    }
}

pub fn get_live_message() -> Option<String> {
    LAST_LIVE_MESSAGE.lock().ok().and_then(|g| g.clone())
}

pub fn clear_live_message() {
    if let Ok(mut guard) = LAST_LIVE_MESSAGE.lock() {
        *guard = None;
    }
}

pub fn set_batch_activity(phase: &str, rsid: &str, total: u32) {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock() {
        let incoming_rank = phase_rank(phase);
        if let Some(ref existing) = *guard
            && phase_rank(&existing.phase) > incoming_rank
        {
            return;
        }
        *guard = Some(BatchActivitySnapshot {
            phase: phase.to_string(),
            current_rsid: rsid.to_string(),
            prepared: 0,
            prefetch_done: 0,
            total: total.max(1),
            started_at: crate::research::util::unix_now(),
        });
    }
}

pub fn set_batch_phase(phase: &str) {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.phase = phase.to_string();
        if phase == "prepare" {
            snap.prepared = 0;
        } else if phase == "embedding" {
            snap.prepared = snap.total;
            snap.prefetch_done = snap.total;
        } else if phase == "qdrant" {
            snap.prepared = snap.total;
        }
    }
}

pub fn bump_batch_prefetch() {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.prefetch_done = snap.prefetch_done.saturating_add(1).min(snap.total);
    }
}

pub fn set_batch_current_rsid(rsid: &str) {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.current_rsid = rsid.to_string();
    }
}

pub fn bump_batch_prepared() {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.prepared = snap.prepared.saturating_add(1);
    }
}

/// Set absolute prepared count (e.g. bootstrap chunks completed).
pub fn set_batch_prepared_count(prepared: u32) {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.prepared = prepared.min(snap.total);
    }
}

pub fn set_batch_embedding(rsid: &str, total: u32) {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock() {
        let total = total.max(1);
        if let Some(ref mut snap) = *guard {
            snap.phase = "embedding".to_string();
            snap.current_rsid = rsid.to_string();
            snap.total = total;
            snap.prepared = total;
            snap.prefetch_done = total;
        } else {
            *guard = Some(BatchActivitySnapshot {
                phase: "embedding".to_string(),
                current_rsid: rsid.to_string(),
                prepared: total,
                prefetch_done: total,
                total,
                started_at: crate::research::util::unix_now(),
            });
        }
    }
}

pub fn set_batch_qdrant() {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock()
        && let Some(ref mut snap) = *guard
    {
        snap.phase = "qdrant upsert".to_string();
        snap.prepared = snap.total;
    }
}

pub fn clear_batch_activity() {
    if let Ok(mut guard) = BATCH_ACTIVITY.lock() {
        *guard = None;
    }
    clear_live_message();
}

/// Attach in-memory sweep activity to a DB job row for UI polling.
pub fn attach_live_job_fields(job: &mut super::types::ResearchJob) {
    job.live_message = get_live_message();
    job.qdrant_sample_count = qdrant_sample_baseline();
    job.session_elapsed_secs = sweep_session_elapsed_secs();
    if let Some(snap) = snapshot_batch_activity() {
        job.activity_phase = Some(snap.phase);
        job.batch_prepared = Some(snap.prepared);
        job.batch_prefetch_done = Some(snap.prefetch_done);
        job.batch_total = Some(snap.total);
        job.batch_elapsed_secs = Some((crate::research::util::unix_now() - snap.started_at).max(0));
        if job.current_rsid.is_none() && !snap.current_rsid.is_empty() {
            job.current_rsid = Some(snap.current_rsid);
        }
    }
}

pub fn snapshot_batch_activity() -> Option<BatchActivitySnapshot> {
    BATCH_ACTIVITY.lock().ok().and_then(|g| g.clone())
}

pub async fn timed_async<Fut, T>(phase: SweepPhase, fut: Fut) -> T
where
    Fut: std::future::Future<Output = T>,
{
    let started = Instant::now();
    let out = fut.await;
    record_phase(phase, started.elapsed());
    out
}
