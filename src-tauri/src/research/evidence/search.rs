// ./src-tauri/src/research/evidence/search.rs
//! Hybrid vector search, similar associations, and match explanations.

use super::card::evidence_card_from_payload;
use super::types::{EvidenceCard, HybridSearchParams, MatchExplanation, SimilarSearchParams};
use crate::research::embed::embed_query_cached;
use crate::research::qdrant::{
    find_point_payload_by_rsid, recommend_qdrant_points, search_qdrant_filtered,
};
use crate::research::evidence::named_vectors::query_vector_name_for_text;
use crate::research::types::{QdrantConfig, QdrantHit};
use crate::research::util::string_to_u64;
use std::path::Path;

fn load_rejected_rsids(db_path: &Path) -> Vec<String> {
    let conn = match crate::db::connect(db_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    super::store::rejected_rsids(&conn).unwrap_or_default()
}

pub async fn search_associations_hybrid(
    params: &HybridSearchParams,
    ollama_url: &str,
    config: &QdrantConfig,
    db_path: Option<&Path>,
) -> Result<Vec<EvidenceCard>, String> {
    let vector = embed_query_cached(&params.query, ollama_url, &config.embedding_model).await?;
    let limit = params.limit.clamp(5, 50);
    let vector_name = query_vector_name_for_text(&params.query, config);
    let hits = search_qdrant_filtered(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        vector,
        Some(params.sample_id),
        limit * 2,
        params.trait_category.as_deref(),
        params.evidence_tier.as_deref(),
        params.has_direction,
        params.min_data_quality,
        params.min_wellness_actionability,
        vector_name,
    )
    .await?;

    let rejected: std::collections::HashSet<String> = db_path
        .map(load_rejected_rsids)
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.to_uppercase())
        .collect();

    let mut cards: Vec<(f32, EvidenceCard)> = hits
        .into_iter()
        .filter(|(hit, _, _)| !rejected.contains(&hit.rsid.to_uppercase()))
        .map(|(hit, point_id, raw_payload)| {
            let hybrid = rerank_hybrid(&hit, &raw_payload, params);
            let explanation = build_match_explanation(&hit, &raw_payload, hybrid, None, &params.query);
            let card = evidence_card_from_payload(&raw_payload, hit.score, Some(explanation), Some(point_id));
            (hybrid, card)
        })
        .collect();

    cards.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    cards.truncate(limit as usize);
    Ok(cards.into_iter().map(|(_, c)| c).collect())
}

pub async fn get_similar_associations(
    params: &SimilarSearchParams,
    config: &QdrantConfig,
) -> Result<Vec<EvidenceCard>, String> {
    let (source_payload, point_id) = if let Some(rsid) = params.rsid.as_deref() {
        find_point_payload_by_rsid(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            params.sample_id,
            rsid,
        )
        .await?
        .ok_or_else(|| format!("No Qdrant point found for rsID {}", rsid))?
    } else if let Some(id) = params.qdrant_point_id {
        (serde_json::json!({}), id)
    } else {
        return Err("Provide rsid or qdrant_point_id".into());
    };

    let vector_name = similarity_vector_name(&params.similarity_mode, config);
    let filter = build_similarity_filter(
        params.sample_id,
        &params.similarity_mode,
        &source_payload,
    );
    let hits = recommend_qdrant_points(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        point_id,
        params.limit.clamp(3, 30),
        filter,
        vector_name,
    )
    .await?;

    let source_rsid = params.rsid.clone();
    Ok(hits
        .into_iter()
        .filter(|(hit, _, _)| params.include_self || source_rsid.as_deref() != Some(hit.rsid.as_str()))
        .map(|(hit, pid, payload)| {
            let explanation = build_match_explanation(
                &hit,
                &payload,
                hit.score,
                Some(&params.similarity_mode),
                "",
            );
            evidence_card_from_payload(&payload, hit.score, Some(explanation), Some(pid))
        })
        .collect())
}

pub fn explain_vector_match(
    query: &str,
    hit_payload: &serde_json::Value,
    vector_score: f32,
) -> MatchExplanation {
    build_match_explanation(
        &QdrantHit {
            rsid: hit_payload["rsid"].as_str().unwrap_or("").into(),
            gene: None,
            category: None,
            text: hit_payload["text"].as_str().unwrap_or("").into(),
            source: "qdrant".into(),
            score: vector_score,
            pmid: None,
            gwas_trait: None,
            gnomad_af: None,
            gnomad_lookup_status: None,
            gnomad_source_mode: None,
            gnomad_release: None,
            has_gnomad: None,
            significance_score: 0.0,
            genotype: None,
            gene_confidence: None,
            enrichment_version: None,
            clinvar_significance: None,
            chromosome: None,
            position: None,
            gwas_associations: None,
            gene_candidates: None,
            sources_provenance: None,
            cross_refs: None,
            trait_categories: None,
            consultation_modes: None,
            pack_refs: None,
            searchable_tags: None,
            association_summary: None,
        },
        hit_payload,
        vector_score,
        None,
        query,
    )
}

fn rerank_hybrid(hit: &QdrantHit, payload: &serde_json::Value, params: &HybridSearchParams) -> f32 {
    let mut score = hit.score;
    if payload["personal_genotype_matched"].as_bool().unwrap_or(true) {
        score += 0.08;
    }
    if let Some(min_dq) = params.min_data_quality {
        let dq = payload["data_quality_score"].as_f64().unwrap_or(0.5) as f32;
        if dq >= min_dq {
            score += 0.05;
        } else {
            score -= 0.1;
        }
    }
    if payload["stale"].as_bool().unwrap_or(false) {
        score -= 0.12;
    }
    if params.has_direction == Some(true) && !payload["has_direction"].as_bool().unwrap_or(false) {
        score -= 0.15;
    }
    score += payload["data_quality_score"].as_f64().unwrap_or(0.0) as f32 * 0.05;
    score
}

fn build_match_explanation(
    hit: &QdrantHit,
    payload: &serde_json::Value,
    hybrid_score: f32,
    similarity_mode: Option<&str>,
    query: &str,
) -> MatchExplanation {
    let mut boost_reasons = Vec::new();
    if payload["personal_genotype_matched"].as_bool().unwrap_or(false) {
        boost_reasons.push("Genotype-matched variant in your sample".into());
    }
    if payload["data_quality_score"].as_f64().unwrap_or(0.0) > 0.7 {
        boost_reasons.push("Higher metadata completeness".into());
    }
    if payload["has_direction"].as_bool().unwrap_or(false) {
        boost_reasons.push("Structured effect direction available".into());
    }
    if let Some(mode) = similarity_mode {
        boost_reasons.push(format!("Similarity mode: {}", mode.replace('_', " ")));
    }

    let mut missing_warnings: Vec<String> = payload
        .get("missing_fields")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| format!("Missing: {}", s)))
                .collect()
        })
        .unwrap_or_default();

    if !payload["has_direction"].as_bool().unwrap_or(false) {
        missing_warnings.push("Personal direction unknown — do not infer from p-value".into());
    }

    let shared_traits = payload
        .get("trait_name")
        .or_else(|| payload.get("gwas_trait"))
        .and_then(|v| v.as_str())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default();

    let shared_genes = payload
        .get("gene_symbol")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default();

    let shared_categories = payload
        .get("trait_categories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let shared_sources = payload
        .get("source_names")
        .or_else(|| payload.get("sources_used"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let matched_tags: Vec<String> = hit
        .searchable_tags
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|t| query.is_empty() || t.to_lowercase().contains(&query.to_lowercase()))
        .take(5)
        .collect();

    MatchExplanation {
        vector_score: hit.score,
        hybrid_score,
        shared_traits,
        shared_genes,
        shared_categories,
        shared_sources,
        matched_tags,
        boost_reasons,
        missing_metadata_warnings: missing_warnings,
        similarity_mode: similarity_mode.map(String::from),
    }
}

fn similarity_vector_name(mode: &str, config: &QdrantConfig) -> Option<&'static str> {
    if !config.named_vectors_enabled {
        return None;
    }
    match mode {
        "trait_similarity" | "same_trait_different_gene" | "same_gene_different_trait" => {
            Some("trait_dense")
        }
        "gene_mechanism_similarity" => Some("gene_mechanism_dense"),
        "evidence_similarity" | "higher_quality_similar" => Some("evidence_dense"),
        "wellness_actionable" | "clinically_stronger" => Some("actionability_dense"),
        _ => None,
    }
}

fn payload_str(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            payload
                .get("gwas_trait")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
        })
}

fn build_similarity_filter(
    sample_id: i64,
    mode: &str,
    source_payload: &serde_json::Value,
) -> Option<serde_json::Value> {
    let mut must = vec![serde_json::json!({
        "key": "sample_id",
        "match": { "value": sample_id }
    })];
    let mut must_not: Vec<serde_json::Value> = Vec::new();

    match mode {
        "clinically_stronger" => {
            must.push(serde_json::json!({
                "key": "clinical_actionability_score",
                "range": { "gte": 0.3 }
            }));
        }
        "wellness_actionable" => {
            must.push(serde_json::json!({
                "key": "wellness_actionability_score",
                "range": { "gte": 0.35 }
            }));
        }
        "higher_quality_similar" => {
            must.push(serde_json::json!({
                "key": "data_quality_score",
                "range": { "gte": 0.55 }
            }));
        }
        "same_trait_different_gene" => {
            if let Some(trait_name) = payload_str(
                source_payload,
                "trait_name",
            )
            .or_else(|| payload_str(source_payload, "trait_name_mapped"))
            {
                must.push(serde_json::json!({
                    "key": "trait_name",
                    "match": { "value": trait_name }
                }));
            }
            if let Some(gene) = payload_str(source_payload, "gene_symbol") {
                must_not.push(serde_json::json!({
                    "key": "gene_symbol",
                    "match": { "value": gene }
                }));
            }
        }
        "same_gene_different_trait" => {
            if let Some(gene) = payload_str(source_payload, "gene_symbol") {
                must.push(serde_json::json!({
                    "key": "gene_symbol",
                    "match": { "value": gene }
                }));
            }
            if let Some(trait_name) = payload_str(source_payload, "trait_name") {
                must_not.push(serde_json::json!({
                    "key": "trait_name",
                    "match": { "value": trait_name }
                }));
            }
        }
        _ => {}
    }

    let mut filter = serde_json::json!({ "must": must });
    if !must_not.is_empty() {
        filter["must_not"] = serde_json::json!(must_not);
    }
    Some(filter)
}

pub fn point_id_for_marker(sample_id: i64, rsid: &str, allele1: &str) -> u64 {
    string_to_u64(&format!("{}_{}_{}", sample_id, rsid, allele1))
}
