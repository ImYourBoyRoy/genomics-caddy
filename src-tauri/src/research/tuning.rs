// ./src-tauri/src/research/tuning.rs
/*
Runtime pipeline tuning for vector enrichment sweeps.
Auto-detects laptop vs remote-server profiles from Ollama/Qdrant URLs and env overrides.
*/

use std::sync::{Arc, OnceLock, RwLock};
use serde::Serialize;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone, Serialize)]
pub struct PipelineTuningPublic {
    pub profile: String,
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
    /// When false during fast sweep, only the default dense vector is upserted (much faster).
    pub named_vectors_in_sweep: bool,
}

impl Default for PipelineTuning {
    fn default() -> Self {
        resolve_pipeline_tuning("http://localhost:11434", "http://localhost:6333", false)
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
    let gwas = GWAS_SEMAPHORE.get_or_init(|| {
        RwLock::new(Arc::new(Semaphore::new(
            tuning.gwas_api_concurrency.max(1),
        )))
    });
    let gnomad = GNOMAD_SEMAPHORE.get_or_init(|| {
        RwLock::new(Arc::new(Semaphore::new(
            tuning.gnomad_concurrency.max(1),
        )))
    });
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

pub fn install_sweep_tuning(ollama_url: &str, qdrant_url: &str, scope: &super::types::ResearchScopeConfig) -> PipelineTuning {
    super::sweep_metrics::reset_sweep_metrics();
    super::sources_config::install_enrichment_sources(scope);
    let sources = super::sources_config::effective_sources_from_scope(scope);
    let sweep_fast = sources.is_fast_index();
    let mut tuning = resolve_pipeline_tuning(ollama_url, qdrant_url, sweep_fast);
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

/// Max seconds for one variant's prepare step (GWAS+gnomAD+NCBI+GTEx). Prevents infinite hangs.
pub fn prepare_variant_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_PREPARE_TIMEOUT_SECS", 180)
}

/// Max seconds for one remote gnomAD tabix lookup inside spawn_blocking.
pub fn gnomad_remote_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_GNOMAD_REMOTE_TIMEOUT_SECS", 120)
}

/// Hard cap for an entire enrichment sub-batch (prefetch + parallel prepare).
pub fn enrichment_batch_timeout_secs(batch_len: usize) -> u64 {
    let per = prepare_variant_timeout_secs();
    let concurrency = prepare_concurrency().max(1);
    let waves = batch_len.max(1).div_ceil(concurrency) as u64;
    // Parallel prepare waves + gnomAD/API prefetch slack (not batch_len × per serial).
    let estimate = per
        .saturating_mul(waves)
        .saturating_add(180);
    parse_env_u64(
        "GENOMICS_BATCH_TIMEOUT_SECS",
        estimate.clamp(240, 900),
    )
}

/// Max seconds for one Ollama embed request (batch or single).
pub fn embed_batch_timeout_secs(batch_len: usize) -> u64 {
    let base = parse_env_u64("GENOMICS_EMBED_TIMEOUT_SECS", 120);
    parse_env_u64(
        "GENOMICS_EMBED_BATCH_TIMEOUT_SECS",
        (base.saturating_mul(batch_len.max(1) as u64)).clamp(120, 600),
    )
}

/// Max seconds for live secondary-source adapter fan-out during prepare.
pub fn secondary_sources_timeout_secs() -> u64 {
    parse_env_u64("GENOMICS_SECONDARY_TIMEOUT_SECS", 90)
}

/// Max seconds to wait for adapter rate-limit semaphores during prepare.
pub fn adapter_semaphore_wait_secs() -> u64 {
    parse_env_u64("GENOMICS_ADAPTER_SEM_WAIT_SECS", 45)
}

/// Hard cap for API-cache prefetch before prepare.
pub fn prefetch_batch_timeout_secs(batch_len: usize) -> u64 {
    parse_env_u64(
        "GENOMICS_PREFETCH_TIMEOUT_SECS",
        (60_u64 * batch_len.max(1) as u64).clamp(120, 600),
    )
}

fn env_sweep_fast() -> bool {
    std::env::var("GENOMICS_SWEEP_FAST")
        .ok()
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

pub fn sweep_fast_enabled(scope_fast: bool) -> bool {
    scope_fast || env_sweep_fast() || active_tuning().fast_sweep || super::sources_config::enrichment_fast_index_mode()
}

fn tuning_profile_name(_ollama_url: &str, _qdrant_url: &str) -> &'static str {
    match std::env::var("GENOMICS_TUNING")
        .ok()
        .map(|s| s.trim().to_lowercase())
        .as_deref()
    {
        Some("laptop") => "laptop",
        Some("server") => "server",
        _ => {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            // Remote Ollama/Qdrant does not mean this machine can run batch-32 prepare.
            // Reserve server profile for high-core hosts unless GENOMICS_TUNING=server.
            if cpus >= 16 {
                "server"
            } else {
                "laptop"
            }
        }
    }
}

fn is_remote_host(url: &str) -> bool {
    let lower = url.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.contains("localhost")
        || lower.contains("127.0.0.1")
        || lower.contains("[::1]")
        || lower.contains("0.0.0.0")
    {
        return false;
    }
    lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn resolve_pipeline_tuning(
    ollama_url: &str,
    qdrant_url: &str,
    sweep_fast: bool,
) -> PipelineTuning {
    let profile = tuning_profile_name(ollama_url, qdrant_url);
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let fast = sweep_fast || env_sweep_fast();

    let (prepare, batch, gwas, gnomad, embed_par, pipe, prefetch) = match profile {
        "server" => (
            parse_env_usize("GENOMICS_PREPARE_CONCURRENCY", (cpus * 3).clamp(16, 32)),
            parse_env_usize("GENOMICS_ENRICH_BATCH_SIZE", 32),
            parse_env_usize("GENOMICS_GWAS_API_CONCURRENCY", 12),
            if fast {
                0
            } else {
                parse_env_usize("GENOMICS_GNOMAD_CONCURRENCY", 16)
            },
            parse_env_usize("GENOMICS_EMBED_PARALLEL", 6),
            parse_env_usize("GENOMICS_PIPELINE_DEPTH", 2),
            parse_env_usize("GENOMICS_PREFETCH_CONCURRENCY", 6),
        ),
        _ => (
            parse_env_usize("GENOMICS_PREPARE_CONCURRENCY", cpus.clamp(4, 8)),
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
        ),
    };

    let skip_gnomad = fast
        || parse_env_usize("GENOMICS_SKIP_GNOMAD", 0) != 0
        || gnomad == 0;

    let remote_embed = is_remote_host(ollama_url) || is_remote_host(qdrant_url);
    let embed_parallel = if profile == "laptop" && remote_embed {
        embed_par.max(4)
    } else {
        embed_par
    };

    PipelineTuning {
        profile: profile.to_string(),
        prepare_concurrency: prepare,
        enrich_batch_size: batch,
        gwas_api_concurrency: gwas,
        gnomad_concurrency: if skip_gnomad { 0 } else { gnomad.max(1) },
        embed_parallel,
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
