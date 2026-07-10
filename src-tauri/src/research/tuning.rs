// ./src-tauri/src/research/tuning.rs
/*
Runtime pipeline tuning for vector enrichment sweeps.
Derives batch sizes and concurrency from CPU count + measured Ollama/Qdrant latency (dynamic profile).
*/

use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone, Serialize)]
pub struct PipelineTuningPublic {
    /// Human label, e.g. "dynamic" or "dynamic (manual override)".
    pub profile: String,
    pub cpus: usize,
    pub ollama_latency_ms: Option<u64>,
    pub qdrant_latency_ms: Option<u64>,
    pub prepare_concurrency: usize,
    pub enrich_batch_size: usize,
    pub gwas_api_concurrency: usize,
    pub gnomad_concurrency: usize,
    pub embed_parallel: usize,
    pub pipeline_depth: usize,
    pub prefetch_concurrency: usize,
    pub fast_sweep: bool,
    pub skip_gnomad: bool,
    pub local_gwas_only: bool,
    pub named_vectors_in_sweep: bool,
}

impl From<PipelineTuning> for PipelineTuningPublic {
    fn from(t: PipelineTuning) -> Self {
        Self {
            profile: t.profile,
            cpus: t.cpus,
            ollama_latency_ms: t.ollama_latency_ms,
            qdrant_latency_ms: t.qdrant_latency_ms,
            prepare_concurrency: t.prepare_concurrency,
            enrich_batch_size: t.enrich_batch_size,
            gwas_api_concurrency: t.gwas_api_concurrency,
            gnomad_concurrency: t.gnomad_concurrency,
            embed_parallel: t.embed_parallel,
            pipeline_depth: t.pipeline_depth,
            prefetch_concurrency: t.prefetch_concurrency,
            fast_sweep: t.fast_sweep,
            skip_gnomad: t.skip_gnomad,
            local_gwas_only: t.local_gwas_only,
            named_vectors_in_sweep: t.named_vectors_in_sweep,
        }
    }
}

pub struct SweepTuningGuard;

impl Drop for SweepTuningGuard {
    fn drop(&mut self) {
        clear_sweep_tuning();
    }
}

#[derive(Debug, Clone)]
pub struct PipelineTuning {
    pub profile: String,
    pub cpus: usize,
    pub ollama_latency_ms: Option<u64>,
    pub qdrant_latency_ms: Option<u64>,
    pub prepare_concurrency: usize,
    pub enrich_batch_size: usize,
    pub gwas_api_concurrency: usize,
    pub gnomad_concurrency: usize,
    pub embed_parallel: usize,
    pub pipeline_depth: usize,
    pub prefetch_concurrency: usize,
    pub fast_sweep: bool,
    pub skip_gnomad: bool,
    pub local_gwas_only: bool,
    pub named_vectors_in_sweep: bool,
}

impl Default for PipelineTuning {
    fn default() -> Self {
        resolve_pipeline_tuning(
            "http://localhost:11434",
            "http://localhost:6333",
            false,
            None,
            None,
        )
    }
}

static ACTIVE_TUNING: OnceLock<RwLock<Option<PipelineTuning>>> = OnceLock::new();
static GWAS_SEMAPHORE: OnceLock<RwLock<Arc<Semaphore>>> = OnceLock::new();
static GNOMAD_SEMAPHORE: OnceLock<RwLock<Arc<Semaphore>>> = OnceLock::new();

fn gwas_sem() -> Arc<Semaphore> {
    GWAS_SEMAPHORE
        .get_or_init(|| {
            RwLock::new(Arc::new(Semaphore::new(
                active_tuning().gwas_api_concurrency.max(1),
            )))
        })
        .read()
        .expect("gwas semaphore lock")
        .clone()
}

fn gnomad_sem() -> Arc<Semaphore> {
    GNOMAD_SEMAPHORE
        .get_or_init(|| {
            RwLock::new(Arc::new(Semaphore::new(
                active_tuning().gnomad_concurrency.max(1),
            )))
        })
        .read()
        .expect("gnomad semaphore lock")
        .clone()
}

fn refresh_rate_limiters(tuning: &PipelineTuning) {
    let gwas = GWAS_SEMAPHORE
        .get_or_init(|| RwLock::new(Arc::new(Semaphore::new(tuning.gwas_api_concurrency.max(1)))));
    let gnomad = GNOMAD_SEMAPHORE
        .get_or_init(|| RwLock::new(Arc::new(Semaphore::new(tuning.gnomad_concurrency.max(1)))));
    if let Ok(mut guard) = gwas.write() {
        *guard = Arc::new(Semaphore::new(tuning.gwas_api_concurrency.max(1)));
    }
    if let Ok(mut guard) = gnomad.write() {
        *guard = Arc::new(Semaphore::new(tuning.gnomad_concurrency.max(1)));
    }
}

pub async fn acquire_gwas_permit() -> Option<OwnedSemaphorePermit> {
    gwas_sem().acquire_owned().await.ok()
}

pub async fn acquire_gnomad_permit() -> Option<OwnedSemaphorePermit> {
    if skip_gnomad_in_sweep() {
        return None;
    }
    gnomad_sem().acquire_owned().await.ok()
}

fn store() -> &'static RwLock<Option<PipelineTuning>> {
    ACTIVE_TUNING.get_or_init(|| RwLock::new(None))
}

pub fn install_sweep_tuning(
    ollama_url: &str,
    qdrant_url: &str,
    scope: &super::types::ResearchScopeConfig,
    ollama_latency_ms: Option<u64>,
    qdrant_latency_ms: Option<u64>,
) -> PipelineTuning {
    super::sweep_metrics::reset_sweep_metrics();
    super::sources_config::install_enrichment_sources(scope);
    let sources = super::sources_config::effective_sources_from_scope(scope);
    let sweep_fast = sources.is_fast_index();
    let mut tuning = resolve_pipeline_tuning(
        ollama_url,
        qdrant_url,
        sweep_fast,
        ollama_latency_ms,
        qdrant_latency_ms,
    );
    if !sources.gnomad {
        tuning.skip_gnomad = true;
        tuning.gnomad_concurrency = 0;
    }
    tuning.fast_sweep = sweep_fast;
    tuning.local_gwas_only = sweep_fast;
    refresh_rate_limiters(&tuning);
    if let Ok(mut guard) = store().write() {
        *guard = Some(tuning.clone());
    }
    tuning
}

pub fn clear_sweep_tuning() {
    if let Ok(mut guard) = store().write() {
        *guard = None;
    }
    super::sources_config::clear_enrichment_sources();
    refresh_rate_limiters(&PipelineTuning::default());
}

pub fn active_tuning() -> PipelineTuning {
    store()
        .read()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default()
}

fn parse_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

fn parse_env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

pub fn prepare_variant_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_PREPARE_TIMEOUT_SECS", 180)
}

pub fn gnomad_remote_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_GNOMAD_REMOTE_TIMEOUT_SECS", 120)
}

pub fn enrichment_batch_timeout_secs(batch_len: usize) -> u64 {
    let per = prepare_variant_timeout_secs();
    let concurrency = prepare_concurrency().max(1);
    let waves = batch_len.max(1).div_ceil(concurrency) as u64;
    let estimate = per.saturating_mul(waves).saturating_add(180);
    parse_env_u64("GENOMICS_BATCH_TIMEOUT_SECS", estimate.clamp(240, 900))
}

pub fn embed_batch_timeout_secs(batch_len: usize) -> u64 {
    let base = parse_env_u64("GENOMICS_EMBED_TIMEOUT_SECS", 120);
    parse_env_u64(
        "GENOMICS_EMBED_BATCH_TIMEOUT_SECS",
        (base.saturating_mul(batch_len.max(1) as u64)).clamp(120, 600),
    )
}

pub fn secondary_sources_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_SECONDARY_TIMEOUT_SECS", 90)
}

pub fn adapter_semaphore_wait_secs() -> u64 {
    parse_env_u64("GENOMICS_ADAPTER_SEM_WAIT_SECS", 45)
}

pub fn prefetch_batch_timeout_secs(batch_len: usize) -> u64 {
    parse_env_u64(
        "GENOMICS_PREFETCH_TIMEOUT_SECS",
        (60_u64 * batch_len.max(1) as u64).clamp(120, 600),
    )
}

fn env_sweep_fast() -> bool {
    std::env::var("GENOMICS_SWEEP_FAST")
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn sweep_fast_enabled(scope_fast: bool) -> bool {
    scope_fast
        || env_sweep_fast()
        || active_tuning().fast_sweep
        || super::sources_config::enrichment_fast_index_mode()
}

/// Blocking HTTP GET latency probe (health endpoint). Returns None on failure.
pub fn probe_url_latency_ms(url: &str) -> Option<u64> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let probe_url = if trimmed.ends_with('/') {
        format!("{trimmed}collections")
    } else if trimmed.contains("/api/") || trimmed.ends_with("/collections") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/collections")
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let start = Instant::now();
    let ok = client.get(&probe_url).send().ok()?.status().is_success()
        || client
            .get(trimmed)
            .send()
            .ok()
            .map(|r| r.status().is_success())
            .unwrap_or(false);
    if ok {
        Some(start.elapsed().as_millis() as u64)
    } else {
        None
    }
}

pub async fn probe_service_latencies(
    ollama_url: &str,
    qdrant_url: &str,
) -> (Option<u64>, Option<u64>) {
    let ollama = ollama_url.to_string();
    let qdrant = qdrant_url.to_string();
    let (o_ms, q_ms) = tokio::join!(
        tokio::task::spawn_blocking(move || probe_url_latency_ms(&ollama)),
        tokio::task::spawn_blocking(move || probe_url_latency_ms(&qdrant)),
    );
    (o_ms.ok().flatten(), q_ms.ok().flatten())
}

fn clamp_usize(v: usize, min: usize, max: usize) -> usize {
    v.clamp(min, max)
}

fn latency_penalty(ms: Option<u64>) -> f64 {
    match ms {
        None => 1.35,
        Some(v) if v <= 15 => 0.85,
        Some(v) if v <= 50 => 1.0,
        Some(v) if v <= 150 => 1.25,
        Some(v) if v <= 400 => 1.6,
        _ => 2.0,
    }
}

/// Logical cores available for sweep work — always leaves one core for OS / UI / Docker overhead.
fn resolve_cpu_budget() -> usize {
    let detected = std::env::var("GENOMICS_CPU_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });
    detected.saturating_sub(1).max(1)
}

/// Dynamic tuning from CPU count + optional measured latencies. Env vars still override individual knobs.
pub fn resolve_pipeline_tuning(
    _ollama_url: &str,
    _qdrant_url: &str,
    sweep_fast: bool,
    ollama_latency_ms: Option<u64>,
    qdrant_latency_ms: Option<u64>,
) -> PipelineTuning {
    let cpus = resolve_cpu_budget();

    let manual = std::env::var("GENOMICS_TUNING")
        .ok()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s != "auto" && s != "dynamic");

    let fast = sweep_fast || env_sweep_fast();
    let penalty = latency_penalty(ollama_latency_ms).max(latency_penalty(qdrant_latency_ms));
    let throughput = (cpus as f64) / penalty;

    let (prepare, batch, gwas, gnomad, embed_par, pipe, prefetch) =
        if manual.as_deref() == Some("fixed") {
            (
                parse_env_usize("GENOMICS_PREPARE_CONCURRENCY", 8),
                parse_env_usize("GENOMICS_ENRICH_BATCH_SIZE", 16),
                parse_env_usize("GENOMICS_GWAS_API_CONCURRENCY", 6),
                if fast {
                    0
                } else {
                    parse_env_usize("GENOMICS_GNOMAD_CONCURRENCY", 4)
                },
                parse_env_usize("GENOMICS_EMBED_PARALLEL", 2),
                parse_env_usize("GENOMICS_PIPELINE_DEPTH", 1),
                parse_env_usize("GENOMICS_PREFETCH_CONCURRENCY", 4),
            )
        } else {
            let prepare = clamp_usize((throughput * 1.5).round() as usize, 4, 96);
            let batch = clamp_usize((throughput * 0.75).round() as usize, 8, 64);
            let gwas = clamp_usize((throughput * 0.35).round() as usize, 4, 24);
            let gnomad = if fast {
                0
            } else {
                clamp_usize((throughput * 0.45).round() as usize, 4, 32)
            };
            let embed_par = clamp_usize((throughput / 6.0).round() as usize, 2, 16);
            let pipe = if penalty <= 1.1 && cpus >= 24 { 2 } else { 1 };
            let prefetch = clamp_usize((throughput / 4.0).round() as usize, 4, 16);
            (
                parse_env_usize("GENOMICS_PREPARE_CONCURRENCY", prepare),
                parse_env_usize("GENOMICS_ENRICH_BATCH_SIZE", batch),
                parse_env_usize("GENOMICS_GWAS_API_CONCURRENCY", gwas),
                if fast {
                    0
                } else {
                    parse_env_usize("GENOMICS_GNOMAD_CONCURRENCY", gnomad)
                },
                parse_env_usize("GENOMICS_EMBED_PARALLEL", embed_par),
                parse_env_usize("GENOMICS_PIPELINE_DEPTH", pipe),
                parse_env_usize("GENOMICS_PREFETCH_CONCURRENCY", prefetch),
            )
        };

    let skip_gnomad = fast || parse_env_usize("GENOMICS_SKIP_GNOMAD", 0) != 0 || gnomad == 0;

    let profile = if manual.is_some() {
        format!("manual ({cpus}c)")
    } else {
        format!("dynamic ({cpus}c)")
    };

    PipelineTuning {
        profile,
        cpus,
        ollama_latency_ms,
        qdrant_latency_ms,
        prepare_concurrency: prepare,
        enrich_batch_size: batch,
        gwas_api_concurrency: gwas,
        gnomad_concurrency: if skip_gnomad { 0 } else { gnomad.max(1) },
        embed_parallel: embed_par.max(1),
        pipeline_depth: pipe.max(1),
        prefetch_concurrency: prefetch.max(1),
        fast_sweep: fast,
        skip_gnomad,
        local_gwas_only: fast,
        named_vectors_in_sweep: !fast,
    }
}

pub fn prepare_concurrency() -> usize {
    active_tuning().prepare_concurrency
}

pub fn enrich_batch_size() -> usize {
    active_tuning().enrich_batch_size
}

pub fn embed_parallel() -> usize {
    active_tuning().embed_parallel
}

pub fn pipeline_depth() -> usize {
    active_tuning().pipeline_depth
}

pub fn prefetch_concurrency() -> usize {
    active_tuning().prefetch_concurrency
}

pub fn skip_gnomad_in_sweep() -> bool {
    active_tuning().skip_gnomad || !super::sources_config::enrichment_gnomad_enabled()
}

pub fn local_gwas_only_in_sweep() -> bool {
    active_tuning().local_gwas_only
}

pub fn named_vectors_in_sweep() -> bool {
    active_tuning().named_vectors_in_sweep
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn reserves_one_core_for_system() {
        let _guard = ENV_LOCK.lock().expect("env test lock");
        unsafe {
            std::env::set_var("GENOMICS_CPU_LIMIT", "4");
        }
        let t = resolve_pipeline_tuning("http://x", "http://y", false, Some(10), Some(10));
        assert_eq!(t.cpus, 3);
        assert!(t.profile.contains("dynamic (3c)"));
        unsafe {
            std::env::remove_var("GENOMICS_CPU_LIMIT");
        }
    }

    #[test]
    fn dynamic_profile_scales_with_cpus() {
        let _guard = ENV_LOCK.lock().expect("env test lock");
        unsafe {
            std::env::set_var("GENOMICS_CPU_LIMIT", "80");
        }
        let t = resolve_pipeline_tuning(
            "http://localhost:11434",
            "http://localhost:6333",
            false,
            Some(5),
            Some(5),
        );
        assert_eq!(t.cpus, 79);
        assert!(t.profile.contains("dynamic (79c)"));
        assert!(t.prepare_concurrency >= 32);
        assert!(t.enrich_batch_size >= 24);
        unsafe {
            std::env::remove_var("GENOMICS_CPU_LIMIT");
        }
    }

    #[test]
    fn high_latency_reduces_concurrency() {
        let _guard = ENV_LOCK.lock().expect("env test lock");
        unsafe {
            std::env::set_var("GENOMICS_CPU_LIMIT", "17");
        }
        let fast = resolve_pipeline_tuning("http://x", "http://y", false, Some(10), Some(10));
        let slow = resolve_pipeline_tuning("http://x", "http://y", false, Some(500), Some(500));
        assert_eq!(fast.cpus, 16);
        assert_eq!(slow.cpus, 16);
        assert!(slow.prepare_concurrency <= fast.prepare_concurrency);
        unsafe {
            std::env::remove_var("GENOMICS_CPU_LIMIT");
        }
    }
}
