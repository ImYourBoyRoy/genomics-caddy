// ./src-tauri/src/research/commands.rs
/*
Module: research/commands.rs
Purpose: Tauri command handlers for Qdrant research loop and related settings.
Responsibilities:
- Expose Qdrant connection test, collection create/purge, and research job control to the UI.
- Load merged config from SQLite + keyring + `.env`.
Key Inputs: AppHandle, sample IDs, Ollama URL, research scope toggles.
Key Outputs: Typed JSON payloads for the Svelte frontend.
*/

use super::references::{self, GwasSyncResult, ReferenceStatus};
use super::state::{RESEARCH_PAUSED, RESEARCH_RUNNING, clear_stale_running_flag};
use super::{
    QdrantConfigPublic, QdrantConfigUpdate, QdrantConnectionStatus, QdrantHit,
    ResearchFindingPreview, ResearchJob, ResearchScopeConfig, ResearchScopePreview,
    VectorResearchDiagnostics,
};
use crate::config;
use crate::{db_runtime, get_data_dir, get_db_path};
use std::sync::atomic::Ordering;
use tauri::AppHandle;

async fn load_config(app: &AppHandle) -> Result<super::QdrantConfig, String> {
    let db_path = get_db_path(app);
    db_runtime::with_connection(db_path, config::load_qdrant_config).await
}

async fn with_db<F, T>(app: &AppHandle, f: F) -> Result<T, String>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    db_runtime::with_connection(get_db_path(app), f).await
}

pub(crate) fn set_research_debug_enabled(enabled: bool) {
    super::debug_log::set_enabled(enabled);
}

#[tauri::command]
pub async fn test_qdrant_connection(
    app: AppHandle,
    url: String,
    api_key: Option<String>,
    collection: Option<String>,
) -> Result<QdrantConnectionStatus, String> {
    let resolved_key = api_key.filter(|k| !k.trim().is_empty());

    let final_key = if resolved_key.is_none() {
        load_config(&app).await.ok().and_then(|cfg| cfg.api_key)
    } else {
        resolved_key
    };

    let clean_url = crate::config::validate_service_url(&url)?;
    Ok(
        super::test_qdrant_connection(&clean_url, final_key.as_deref(), collection.as_deref())
            .await,
    )
}

#[tauri::command]
pub async fn get_qdrant_config(app: AppHandle) -> Result<QdrantConfigPublic, String> {
    with_db(&app, config::load_qdrant_config_for_ui).await
}

#[tauri::command]
pub async fn save_qdrant_config(app: AppHandle, update: QdrantConfigUpdate) -> Result<(), String> {
    with_db(&app, move |conn| {
        crate::config::save_qdrant_config(conn, &update)
    })
    .await
}

#[tauri::command]
pub async fn get_research_scope(app: AppHandle) -> Result<ResearchScopeConfig, String> {
    with_db(&app, config::load_research_scope).await
}

#[tauri::command]
pub async fn save_research_scope(app: AppHandle, scope: ResearchScopeConfig) -> Result<(), String> {
    with_db(&app, move |conn| config::save_research_scope(conn, &scope)).await
}

#[tauri::command]
pub async fn preview_research_scope(
    app: AppHandle,
    sample_id: i64,
    scope: ResearchScopeConfig,
) -> Result<ResearchScopePreview, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        super::preview_research_scope(&db_path, sample_id, &scope)
    })
    .await
    .map_err(|e| format!("Scope preview worker failed: {}", e))?
}

#[tauri::command]
pub async fn get_reference_status(app: AppHandle) -> Result<ReferenceStatus, String> {
    let data_dir = get_data_dir(&app);
    with_db(&app, move |conn| {
        references::reference_status(&data_dir, conn)
    })
    .await
}

#[tauri::command]
pub async fn sync_gwas_reference(app: AppHandle) -> Result<GwasSyncResult, String> {
    let data_dir = get_data_dir(&app);
    let db_path = get_db_path(&app);
    references::sync_gwas_reference(&data_dir, &db_path).await
}

#[tauri::command]
pub async fn create_qdrant_collection(
    app: AppHandle,
    ollama_url: String,
) -> Result<QdrantConnectionStatus, String> {
    let cfg = load_config(&app).await?;
    super::initialize_qdrant_collection(&ollama_url, &cfg).await?;
    Ok(
        super::test_qdrant_connection(&cfg.url, cfg.api_key.as_deref(), Some(&cfg.collection))
            .await,
    )
}

#[derive(Debug, serde::Serialize)]
pub struct PurgeCollectionResult {
    pub job_reset: bool,
    pub message: String,
}

#[tauri::command]
pub async fn purge_qdrant_collection(
    app: AppHandle,
    sample_id: Option<i64>,
) -> Result<PurgeCollectionResult, String> {
    RESEARCH_PAUSED.store(true, Ordering::SeqCst);
    RESEARCH_RUNNING.store(false, Ordering::SeqCst);

    let cfg = load_config(&app).await?;
    super::purge_qdrant_collection(&cfg.url, cfg.api_key.as_deref(), &cfg.collection).await?;

    let db_path = get_db_path(&app);
    let job_reset = if let Some(sid) = sample_id {
        tauri::async_runtime::spawn_blocking(move || {
            super::reset_research_job_after_purge(&db_path, sid)
        })
        .await
        .map_err(|e| format!("Job reset worker failed: {}", e))??
        .is_some()
    } else {
        false
    };

    Ok(PurgeCollectionResult {
        job_reset,
        message: if job_reset {
            "Collection purged and enrichment progress reset. Start a fresh sweep.".to_string()
        } else {
            "Collection purged. Start a fresh sweep to rebuild vectors.".to_string()
        },
    })
}

#[tauri::command]
pub async fn get_research_job_status(
    app: AppHandle,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let mut job = super::get_research_job_from_db(&db_path, sample_id);
        if let Some(ref mut j) = job {
            j.loop_active =
                Some(super::state::RESEARCH_RUNNING.load(std::sync::atomic::Ordering::SeqCst));
            if j.status == "running" || j.loop_active == Some(true) {
                super::sweep_metrics::attach_live_job_fields(j);
            }
        }
        job
    })
    .await
    .map_err(|e| format!("Job status worker failed: {}", e))
}

#[tauri::command]
pub async fn start_research_job(
    app: AppHandle,
    sample_id: i64,
    ollama_url: String,
    scope: Option<ResearchScopeConfig>,
    force_reenrich: Option<bool>,
) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let existing = tauri::async_runtime::spawn_blocking({
        let db_path = db_path.clone();
        move || super::get_research_job_from_db(&db_path, sample_id)
    })
    .await
    .map_err(|e| format!("Job lookup worker failed: {}", e))?;

    if let Some(ref job) = existing {
        clear_stale_running_flag(&job.status);
    }

    if RESEARCH_RUNNING.load(Ordering::SeqCst) {
        return Err("A research job is already running.".to_string());
    }

    RESEARCH_PAUSED.store(false, Ordering::SeqCst);
    RESEARCH_RUNNING.store(true, Ordering::SeqCst);

    with_db(&app, |conn| {
        config::install_research_debug_logging(conn);
        Ok(())
    })
    .await?;

    let (cfg, effective_scope) = with_db(&app, move |conn| {
        let cfg = config::load_qdrant_config(conn)?;
        let effective_scope =
            scope.unwrap_or_else(|| config::load_research_scope(conn).unwrap_or_default());
        if let Err(e) = config::save_research_scope(conn, &effective_scope) {
            eprintln!("Could not persist research scope: {}", e);
        }
        Ok((cfg, effective_scope))
    })
    .await?;

    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = super::run_research_loop(
            sample_id,
            db_path,
            ollama_url,
            cfg,
            effective_scope,
            super::SweepProgressSink::Desktop(app_handle),
            None,
            force_reenrich.unwrap_or(false),
        )
        .await;
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            eprintln!("Research loop error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn pause_research_job(
    app: AppHandle,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    RESEARCH_PAUSED.store(true, Ordering::SeqCst);
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        super::pause_research_job_in_db(&db_path, sample_id)
    })
    .await
    .map_err(|e| format!("Pause worker failed: {}", e))?
}

#[tauri::command]
pub async fn cancel_research_job(
    app: AppHandle,
    sample_id: i64,
) -> Result<Option<ResearchJob>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        super::cancel_research_job_in_db(&db_path, sample_id)
    })
    .await
    .map_err(|e| format!("Cancel worker failed: {}", e))?
}

#[tauri::command]
pub async fn resume_research_job(
    app: AppHandle,
    sample_id: i64,
    ollama_url: String,
) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let existing = tauri::async_runtime::spawn_blocking({
        let db_path = db_path.clone();
        move || super::get_research_job_from_db(&db_path, sample_id)
    })
    .await
    .map_err(|e| format!("Job lookup worker failed: {}", e))?
    .ok_or_else(|| "No paused research job found for this sample.".to_string())?;

    clear_stale_running_flag(&existing.status);

    if RESEARCH_RUNNING.load(Ordering::SeqCst) {
        return Err("A research job is already running.".to_string());
    }

    if existing.status == "complete" {
        return Err(
            "This sweep is already complete. Use Expand queue or Force re-enrich to run again."
                .to_string(),
        );
    }

    if existing.status != "paused" {
        return Err(format!(
            "Cannot resume job in '{}' state. Pause or cancel the job first.",
            existing.status
        ));
    }

    RESEARCH_PAUSED.store(false, Ordering::SeqCst);
    RESEARCH_RUNNING.store(true, Ordering::SeqCst);

    with_db(&app, |conn| {
        config::install_research_debug_logging(conn);
        Ok(())
    })
    .await?;

    let scope_json = existing.scope_json.clone();
    let (cfg, scope) = with_db(&app, move |conn| {
        let cfg = config::load_qdrant_config(conn)?;
        let scope = scope_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<ResearchScopeConfig>(json).ok())
            .unwrap_or_else(|| config::load_research_scope(conn).unwrap_or_default());
        Ok((cfg, scope))
    })
    .await?;

    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = super::run_research_loop(
            sample_id,
            db_path,
            ollama_url,
            cfg,
            scope,
            super::SweepProgressSink::Desktop(app_handle),
            Some(existing),
            false,
        )
        .await;
        RESEARCH_RUNNING.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            eprintln!("Research loop resume error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn search_qdrant_evidence(
    app: AppHandle,
    query: String,
    ollama_url: String,
    sample_id: Option<i64>,
    limit: Option<u32>,
    trait_category: Option<String>,
) -> Result<Vec<QdrantHit>, String> {
    let cfg = load_config(&app).await?;
    let vector =
        super::embed::embed_query_cached(&query, &ollama_url, &cfg.embedding_model).await?;
    let vector_name = super::evidence::named_vectors::query_vector_name_for_text(&query, &cfg);
    super::search_qdrant(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
        vector,
        sample_id,
        limit.unwrap_or(10),
        trait_category.as_deref(),
        vector_name,
    )
    .await
}

#[tauri::command]
pub async fn search_qdrant_trait_discovery(
    app: AppHandle,
    query: Option<String>,
    trait_category: Option<String>,
    sample_id: i64,
    limit: Option<u32>,
    ollama_url: String,
) -> Result<Vec<QdrantHit>, String> {
    let cfg = load_config(&app).await?;
    let lim = limit.unwrap_or(50).min(100);
    let category = trait_category.filter(|c| !c.trim().is_empty());

    if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
        let vector =
            super::embed::embed_query_cached(q.trim(), &ollama_url, &cfg.embedding_model).await?;
        let vector_name =
            super::evidence::named_vectors::query_vector_name_for_text(q.trim(), &cfg);
        return super::search_qdrant(
            &cfg.url,
            cfg.api_key.as_deref(),
            &cfg.collection,
            vector,
            Some(sample_id),
            lim,
            category.as_deref(),
            vector_name,
        )
        .await;
    }

    if category.is_some() {
        return super::scroll_qdrant_points(
            &cfg.url,
            cfg.api_key.as_deref(),
            &cfg.collection,
            sample_id,
            category.as_deref(),
            lim,
        )
        .await;
    }

    Err("Provide a semantic query and/or a trait category (e.g. bone_density).".to_string())
}

#[tauri::command]
pub async fn get_vector_promoted_findings(
    app: AppHandle,
    sample_id: i64,
) -> Result<Vec<crate::db::VectorPromotedFinding>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect_sample_from_registry_path(&db_path, sample_id)
            .map_err(|e| e.to_string())?;
        crate::db::get_vector_promoted_findings(&conn, sample_id)
    })
    .await
    .map_err(|e| format!("Promoted findings worker failed: {}", e))?
}

#[tauri::command]
pub async fn get_vector_research_diagnostics(
    app: AppHandle,
    sample_id: Option<i64>,
) -> Result<VectorResearchDiagnostics, String> {
    let cfg = load_config(&app).await?;
    let db_path = get_db_path(&app);
    let sid = sample_id;

    let conn_status =
        super::test_qdrant_connection(&cfg.url, cfg.api_key.as_deref(), Some(&cfg.collection))
            .await;

    let scope = with_db(&app, config::load_research_scope)
        .await
        .unwrap_or_default();

    let mut diag = VectorResearchDiagnostics {
        connected: conn_status.success,
        collection: cfg.collection.clone(),
        collection_exists: conn_status.collection_exists,
        qdrant_url: cfg.url.clone(),
        embedding_model: cfg.embedding_model.clone(),
        total_vectors: conn_status.vectors_count,
        sample_vectors: None,
        enrichment_enriched: None,
        enrichment_total: None,
        enrichment_status: None,
        error: conn_status.error.clone(),
        named_vectors_enabled: cfg.named_vectors_enabled,
        sweep_quality: if scope.sweep_fast {
            "fast".into()
        } else {
            "full".into()
        },
        index_embedding_model: None,
        embedding_model_mismatch: false,
        stale_vector_count: None,
    };

    if !conn_status.success {
        return Ok(diag);
    }

    if !conn_status.collection_exists {
        diag.error = Some(format!(
            "Collection '{}' not found on Qdrant server.",
            cfg.collection
        ));
        return Ok(diag);
    }

    if let Some(id) = sid {
        match super::count_qdrant_points(
            &cfg.url,
            cfg.api_key.as_deref(),
            &cfg.collection,
            Some(id),
        )
        .await
        {
            Ok(count) => diag.sample_vectors = Some(count),
            Err(e) => {
                if diag.error.is_none() {
                    diag.error = Some(e);
                }
            }
        }

        let job = tauri::async_runtime::spawn_blocking(move || {
            super::get_research_job_from_db(&db_path, id)
        })
        .await
        .map_err(|e| format!("Job status worker failed: {}", e))?;

        if let Some(job) = job {
            diag.enrichment_enriched = Some(job.enriched_count as u32);
            diag.enrichment_total = Some(job.total_markers as u32);
            diag.enrichment_status = Some(job.status);
        }

        let q_url = cfg.url.clone();
        let q_key = cfg.api_key.clone();
        let q_coll = cfg.collection.clone();
        let q_model = cfg.embedding_model.clone();

        if let Some(index_model) =
            super::sample_index_embedding_model(&q_url, q_key.as_deref(), &q_coll, id).await
        {
            diag.embedding_model_mismatch = !index_model.eq_ignore_ascii_case(&q_model);
            diag.index_embedding_model = Some(index_model);
        }

        diag.stale_vector_count = Some(
            super::count_qdrant_points_with_filter(
                &q_url,
                q_key.as_deref(),
                &q_coll,
                id,
                serde_json::json!({ "key": "stale", "match": { "value": true } }),
            )
            .await
            .unwrap_or(0),
        );
    }

    Ok(diag)
}

#[tauri::command]
pub async fn get_recent_finding_previews(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<Vec<ResearchFindingPreview>, String> {
    let cfg = load_config(&app).await?;
    let db_path = get_db_path(&app);
    let lim = limit.unwrap_or(12).clamp(1, 24) as usize;
    let job_id = tauri::async_runtime::spawn_blocking({
        let db_path = db_path.clone();
        move || {
            super::get_research_job_from_db(&db_path, sample_id)
                .map(|j| j.job_id)
                .unwrap_or_else(|| format!("job_{sample_id}_bootstrap"))
        }
    })
    .await
    .map_err(|e| format!("Job lookup worker failed: {}", e))?;

    let payloads = super::qdrant::scroll_sample_payloads(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
        sample_id,
        lim,
    )
    .await?;

    Ok(payloads
        .into_iter()
        .map(|payload| super::sweep::finding_preview_from_payload(&job_id, sample_id, &payload))
        .collect())
}

#[tauri::command]
pub fn get_ollama_token() -> Result<Option<String>, String> {
    Ok(config::get_ollama_token())
}

#[tauri::command]
pub fn get_ollama_service_config() -> Result<config::OllamaServiceConfig, String> {
    Ok(config::load_ollama_service_config())
}

#[tauri::command]
pub fn save_ollama_token(token: Option<String>) -> Result<(), String> {
    config::save_ollama_token(token.as_deref())
}

#[tauri::command]
pub async fn purge_database_cache(app: AppHandle) -> Result<u64, String> {
    with_db(&app, |conn| {
        let deleted = conn
            .execute("DELETE FROM api_cache", [])
            .map_err(|e| e.to_string())?;
        Ok(deleted as u64)
    })
    .await
}

#[tauri::command]
pub async fn preview_pipeline_tuning(
    ollama_url: String,
    qdrant_url: String,
    sweep_fast: Option<bool>,
) -> super::PipelineTuningPublic {
    let (o_lat, q_lat) = super::probe_service_latencies(&ollama_url, &qdrant_url).await;
    super::resolve_pipeline_tuning(
        &ollama_url,
        &qdrant_url,
        sweep_fast.unwrap_or(false),
        o_lat,
        q_lat,
    )
    .into()
}

#[tauri::command]
pub async fn get_research_debug_log(app: AppHandle) -> Result<bool, String> {
    with_db(&app, |conn| {
        config::install_research_debug_logging(conn);
        Ok(super::debug_log::is_enabled())
    })
    .await
}

#[tauri::command]
pub async fn set_research_debug_log(app: AppHandle, enabled: bool) -> Result<bool, String> {
    with_db(&app, move |conn| {
        config::save_research_debug_log(conn, enabled)?;
        set_research_debug_enabled(enabled);
        Ok(enabled)
    })
    .await
}

#[tauri::command]
pub async fn select_save_path(default_filename: String) -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_filename)
        .add_filter("JSON", &["json"])
        .save_file();
    Ok(file.map(|p| {
        let path_str = p.to_string_lossy().to_string();
        config::register_export_path(&path_str);
        path_str
    }))
}
