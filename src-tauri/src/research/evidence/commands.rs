// ./src-tauri/src/research/evidence/commands.rs
//! Tauri commands for the evidence workbench.

use super::atlas::{build_vector_atlas, load_atlas_points, VectorAtlasResult};
use super::backfill::{backfill_evidence_payloads, reembed_stale_vectors, BackfillResult, ReembedResult};
use super::dashboard::{
    build_quality_dashboard, build_trait_clusters, chromosome_trait_overlay,
    list_actionability_points, list_pathway_flow_rows,
};
use super::packet::export_evidence_packet;
use super::search::{
    explain_vector_match, get_similar_associations as search_similar,
    search_associations_hybrid as hybrid_search,
};
use super::store::{list_candidates, update_candidate_status};
use super::types::{
    ActionabilityPoint, ChromosomeTraitBand, EvidenceCard, EvidencePacket, HybridSearchParams,
    PathwayFlowRow, QualityDashboard, SimilarSearchParams, TraitClusterSummary,
    UpdateCandidateStatusRequest, CandidateMarkerRow,
};
use crate::config;
use crate::db_runtime;
use crate::{get_db_path};
use tauri::AppHandle;

async fn load_qdrant(app: &AppHandle) -> Result<super::super::QdrantConfig, String> {
    db_runtime::with_connection(get_db_path(app), config::load_qdrant_config).await
}

#[tauri::command]
pub async fn search_associations_hybrid(
    app: AppHandle,
    params: HybridSearchParams,
    ollama_url: String,
) -> Result<Vec<EvidenceCard>, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    hybrid_search(&params, &ollama_url, &cfg, Some(&db_path)).await
}

#[tauri::command]
pub async fn get_similar_associations(
    app: AppHandle,
    params: SimilarSearchParams,
) -> Result<Vec<EvidenceCard>, String> {
    let cfg = load_qdrant(&app).await?;
    search_similar(&params, &cfg).await
}

#[tauri::command]
pub async fn explain_vector_match_cmd(
    query: String,
    hit_payload: serde_json::Value,
    vector_score: f32,
) -> Result<super::types::MatchExplanation, String> {
    Ok(explain_vector_match(&query, &hit_payload, vector_score))
}

#[tauri::command]
pub async fn get_quality_dashboard(
    app: AppHandle,
    sample_id: i64,
) -> Result<QualityDashboard, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    let enrichment_status = {
        let db_path2 = db_path.clone();
        tauri::async_runtime::spawn_blocking(move || {
            super::super::get_research_job_from_db(&db_path2, sample_id).map(|j| j.status)
        })
        .await
        .ok()
        .flatten()
    };
    build_quality_dashboard(&db_path, sample_id, &cfg, enrichment_status).await
}

#[tauri::command]
pub async fn build_trait_clusters_cmd(
    app: AppHandle,
    sample_id: i64,
    trait_category: Option<String>,
    min_data_quality: Option<f32>,
    limit: Option<u32>,
) -> Result<Vec<TraitClusterSummary>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        build_trait_clusters(
            &conn,
            sample_id,
            trait_category.as_deref(),
            min_data_quality,
            limit.unwrap_or(20).min(50),
        )
    })
    .await
    .map_err(|e| format!("Cluster worker failed: {}", e))?
}

#[tauri::command]
pub async fn export_evidence_packet_cmd(
    app: AppHandle,
    sample_id: i64,
    rsid: Option<String>,
    cluster_id: Option<String>,
    association_ids: Option<Vec<String>>,
) -> Result<EvidencePacket, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    export_evidence_packet(
        &db_path,
        sample_id,
        rsid.as_deref(),
        cluster_id.as_deref(),
        association_ids,
        &cfg,
    )
    .await
}

#[tauri::command]
pub async fn list_candidate_markers(
    app: AppHandle,
    limit: Option<u32>,
) -> Result<Vec<CandidateMarkerRow>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        list_candidates(&conn, limit.unwrap_or(50).min(200))
    })
    .await
    .map_err(|e| format!("Candidate list worker failed: {}", e))?
}

#[tauri::command]
pub async fn update_candidate_marker_status(
    app: AppHandle,
    request: UpdateCandidateStatusRequest,
) -> Result<(), String> {
    if request.status == "curated_core" {
        return Err("curated_core promotion requires manual review outside automated tools".into());
    }
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        update_candidate_status(
            &conn,
            &request.candidate_id,
            &request.status,
            request.reviewer_note.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Candidate update worker failed: {}", e))?
}

#[tauri::command]
pub async fn ensure_qdrant_payload_indexes(app: AppHandle) -> Result<Vec<String>, String> {
    let cfg = load_qdrant(&app).await?;
    super::super::qdrant::ensure_payload_indexes(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
    )
    .await
}

#[tauri::command]
pub async fn get_variant_evidence_card(
    app: AppHandle,
    sample_id: i64,
    rsid: String,
) -> Result<Option<EvidenceCard>, String> {
    let cfg = load_qdrant(&app).await?;
    let payload = super::super::qdrant::find_point_payload_by_rsid(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
        sample_id,
        &rsid,
    )
    .await?;
    Ok(payload.map(|(p, pid)| {
        super::card::evidence_card_from_payload(&p, 1.0, None, Some(pid))
    }))
}

#[tauri::command]
pub async fn backfill_evidence_payloads_cmd(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<BackfillResult, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    backfill_evidence_payloads(
        &db_path,
        sample_id,
        &cfg,
        limit.unwrap_or(500).min(5000),
    )
    .await
}

#[tauri::command]
pub async fn reembed_stale_vectors_cmd(
    app: AppHandle,
    sample_id: i64,
    ollama_url: String,
    limit: Option<u32>,
) -> Result<ReembedResult, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    reembed_stale_vectors(
        &db_path,
        sample_id,
        &cfg,
        &ollama_url,
        limit.unwrap_or(200).min(2000),
    )
    .await
}

#[tauri::command]
pub fn get_actionability_matrix(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<Vec<ActionabilityPoint>, String> {
    let db_path = get_db_path(&app);
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    list_actionability_points(&conn, sample_id, limit.unwrap_or(400).min(1000))
}

#[tauri::command]
pub fn get_chromosome_trait_overlay(
    app: AppHandle,
    sample_id: i64,
) -> Result<Vec<ChromosomeTraitBand>, String> {
    let db_path = get_db_path(&app);
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    chromosome_trait_overlay(&conn, sample_id)
}

#[tauri::command]
pub fn get_pathway_flow_rows(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<Vec<PathwayFlowRow>, String> {
    let db_path = get_db_path(&app);
    let conn = crate::db::connect(db_path).map_err(|e| e.to_string())?;
    list_pathway_flow_rows(&conn, sample_id, limit.unwrap_or(80).min(300))
}

#[tauri::command]
pub async fn build_vector_atlas_cmd(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<VectorAtlasResult, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    build_vector_atlas(&db_path, sample_id, &cfg, limit.unwrap_or(1500).min(2500)).await
}

#[tauri::command]
pub async fn get_vector_atlas_cached(
    app: AppHandle,
    sample_id: i64,
    limit: Option<u32>,
) -> Result<Vec<super::atlas::AtlasPoint>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let conn = crate::db::connect(&db_path).map_err(|e| e.to_string())?;
        load_atlas_points(&conn, sample_id, limit.unwrap_or(1500).min(2500))
    })
    .await
    .map_err(|e| format!("Atlas load failed: {}", e))?
}

#[tauri::command]
pub async fn enable_named_vectors_collection(
    app: AppHandle,
    ollama_url: String,
) -> Result<String, String> {
    let cfg = load_qdrant(&app).await?;
    let db_path = get_db_path(&app);
    let vector = crate::research::embed::embed_text("dimension probe", &ollama_url, &cfg.embedding_model).await?;
    let dims = vector.len() as u32;
    super::super::qdrant::ensure_qdrant_collection_named(
        &cfg.url,
        cfg.api_key.as_deref(),
        &cfg.collection,
        dims,
    )
    .await?;
    db_runtime::with_connection(db_path, |conn| {
        conn.execute(
            "UPDATE qdrant_config SET named_vectors_enabled = 1 WHERE id = 1",
            [],
        )
        .map_err(|e| e.to_string())
    })
    .await?;
    Ok(format!(
        "Named vectors enabled on collection '{}' ({} dims): trait_dense, gene_mechanism_dense, evidence_dense, actionability_dense",
        cfg.collection, dims
    ))
}
