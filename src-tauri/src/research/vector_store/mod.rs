// ./src-tauri/src/research/vector_store/mod.rs
/*
Purpose: Provider-agnostic vector store facade for research dense indexing/search.
Responsibilities: Dispatch health, ensure/purge, dense upsert, search, and point
classification across Qdrant (full), Pinecone, Chroma, and Weaviate (dense path).
Named vectors / recommend / payload indexes remain Qdrant-only.
How to run: Used by research sweep, search commands, and Connections probes.
Key inputs: QdrantConfig (includes vector_provider + namespace).
Key outputs: Connection status, upsert/search results, capability helpers.
*/

mod chroma;
mod pinecone;
mod weaviate;

use super::qdrant::{
    classify_points_index_state as qdrant_classify_index,
    classify_points_sweep_state as qdrant_classify_sweep, count_qdrant_points,
    ensure_payload_indexes, ensure_qdrant_collection, find_point_payload_by_rsid,
    map_payload_to_hit, purge_qdrant_collection, scroll_qdrant_payload_batch, search_qdrant,
    test_qdrant_connection, upsert_points_batch, SweepPointClassifyResult,
};
use super::types::{QdrantConfig, QdrantConnectionStatus, QdrantHit};
use super::util::is_current_enrichment_version;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorProvider {
    Qdrant,
    Pinecone,
    Chroma,
    Weaviate,
}

impl VectorProvider {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pinecone" => Self::Pinecone,
            "chroma" => Self::Chroma,
            "weaviate" => Self::Weaviate,
            _ => Self::Qdrant,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qdrant => "qdrant",
            Self::Pinecone => "pinecone",
            Self::Chroma => "chroma",
            Self::Weaviate => "weaviate",
        }
    }

    pub fn supports_named_vectors(self) -> bool {
        matches!(self, Self::Qdrant)
    }

    pub fn supports_payload_indexes(self) -> bool {
        matches!(self, Self::Qdrant)
    }

    /// Dense research sweep + semantic search supported.
    pub fn supports_dense_research(self) -> bool {
        matches!(
            self,
            Self::Qdrant | Self::Pinecone | Self::Chroma | Self::Weaviate
        )
    }
}

pub fn provider_from_config(config: &QdrantConfig) -> VectorProvider {
    VectorProvider::parse(&config.vector_provider)
}

pub fn capabilities_json(provider: VectorProvider) -> serde_json::Value {
    serde_json::json!({
        "provider": provider.as_str(),
        "dense_research": provider.supports_dense_research(),
        "named_vectors": provider.supports_named_vectors(),
        "payload_indexes": provider.supports_payload_indexes(),
        "recommend": matches!(provider, VectorProvider::Qdrant),
        "hosting_notes": match provider {
            VectorProvider::Qdrant => "Self-host or cloud. LAN hosts (e.g. http://192.168.x.x:6333) are fully supported.",
            VectorProvider::Pinecone => "Managed cloud. URL must be the index host (https://….svc.…pinecone.io).",
            VectorProvider::Chroma => "Self-host or cloud. Dense upsert/search via Chroma HTTP API.",
            VectorProvider::Weaviate => "Self-host or cloud. Dense upsert/search via Weaviate objects + GraphQL.",
        }
    })
}

pub async fn test_connection(config: &QdrantConfig) -> QdrantConnectionStatus {
    let provider = provider_from_config(config);
    match provider {
        VectorProvider::Qdrant => {
            test_qdrant_connection(
                &config.url,
                config.api_key.as_deref(),
                Some(&config.collection),
            )
            .await
        }
        VectorProvider::Pinecone => pinecone::test_connection(config).await,
        VectorProvider::Chroma => chroma::test_connection(config).await,
        VectorProvider::Weaviate => weaviate::test_connection(config).await,
    }
}

pub async fn ensure_collection(config: &QdrantConfig, dims: u32) -> Result<(), String> {
    let provider = provider_from_config(config);
    match provider {
        VectorProvider::Qdrant => {
            ensure_qdrant_collection(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                dims,
            )
            .await
        }
        VectorProvider::Pinecone => pinecone::ensure_ready(config, dims).await,
        VectorProvider::Chroma => chroma::ensure_collection(config, dims).await,
        VectorProvider::Weaviate => weaviate::ensure_class(config, dims).await,
    }
}

pub async fn purge_collection(config: &QdrantConfig) -> Result<(), String> {
    let provider = provider_from_config(config);
    match provider {
        VectorProvider::Qdrant => {
            purge_qdrant_collection(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
            )
            .await
        }
        VectorProvider::Pinecone => pinecone::purge_namespace(config).await,
        VectorProvider::Chroma => chroma::purge_collection(config).await,
        VectorProvider::Weaviate => weaviate::purge_class(config).await,
    }
}

pub async fn upsert_dense_batch(
    config: &QdrantConfig,
    points: Vec<(u64, Vec<f32>, serde_json::Value)>,
) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }
    let provider = provider_from_config(config);
    match provider {
        VectorProvider::Qdrant => {
            upsert_points_batch(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                points,
            )
            .await
        }
        VectorProvider::Pinecone => pinecone::upsert_batch(config, points).await,
        VectorProvider::Chroma => chroma::upsert_batch(config, points).await,
        VectorProvider::Weaviate => weaviate::upsert_batch(config, points).await,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn search_dense(
    config: &QdrantConfig,
    vector: Vec<f32>,
    sample_id: Option<i64>,
    limit: u32,
    trait_category: Option<&str>,
    vector_name: Option<&str>,
) -> Result<Vec<QdrantHit>, String> {
    let provider = provider_from_config(config);
    match provider {
        VectorProvider::Qdrant => {
            search_qdrant(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                vector,
                sample_id,
                limit,
                trait_category,
                vector_name,
            )
            .await
        }
        VectorProvider::Pinecone => {
            if vector_name.is_some() {
                return Err(
                    "Named-vector search is Qdrant-only. Disable named vectors or switch provider."
                        .into(),
                );
            }
            pinecone::search(config, vector, sample_id, limit, trait_category).await
        }
        VectorProvider::Chroma => {
            if vector_name.is_some() {
                return Err(
                    "Named-vector search is Qdrant-only. Disable named vectors or switch provider."
                        .into(),
                );
            }
            chroma::search(config, vector, sample_id, limit, trait_category).await
        }
        VectorProvider::Weaviate => {
            if vector_name.is_some() {
                return Err(
                    "Named-vector search is Qdrant-only. Disable named vectors or switch provider."
                        .into(),
                );
            }
            weaviate::search(config, vector, sample_id, limit, trait_category).await
        }
    }
}

pub async fn maybe_ensure_payload_indexes(config: &QdrantConfig) -> Result<(), String> {
    if provider_from_config(config).supports_payload_indexes() {
        let _ = ensure_payload_indexes(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
        )
        .await;
    }
    Ok(())
}

pub async fn classify_points_sweep_state(
    config: &QdrantConfig,
    ids: Vec<u64>,
    sources: &super::types::EnrichmentSourcesConfig,
    supplement_missing: bool,
) -> Result<SweepPointClassifyResult, String> {
    match provider_from_config(config) {
        VectorProvider::Qdrant => {
            qdrant_classify_sweep(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                ids,
                sources,
                supplement_missing,
            )
            .await
        }
        other => classify_via_fetch(config, other, ids, sources, supplement_missing).await,
    }
}

pub async fn classify_points_index_state(
    config: &QdrantConfig,
    ids: Vec<u64>,
) -> Result<Vec<u64>, String> {
    match provider_from_config(config) {
        VectorProvider::Qdrant => {
            qdrant_classify_index(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                ids,
            )
            .await
        }
        other => {
            let payloads = fetch_payloads(config, other, &ids).await?;
            Ok(ids
                .into_iter()
                .filter(|id| {
                    payloads
                        .get(id)
                        .map(|p| {
                            is_current_enrichment_version(
                                p.get("enrichment_version").and_then(|v| v.as_str()),
                            )
                        })
                        .unwrap_or(false)
                })
                .collect())
        }
    }
}

async fn fetch_payloads(
    config: &QdrantConfig,
    provider: VectorProvider,
    ids: &[u64],
) -> Result<std::collections::HashMap<u64, serde_json::Value>, String> {
    match provider {
        VectorProvider::Pinecone => pinecone::fetch_payloads(config, ids).await,
        VectorProvider::Chroma => chroma::fetch_payloads(config, ids).await,
        VectorProvider::Weaviate => weaviate::fetch_payloads(config, ids).await,
        VectorProvider::Qdrant => Ok(std::collections::HashMap::new()),
    }
}

async fn classify_via_fetch(
    config: &QdrantConfig,
    provider: VectorProvider,
    ids: Vec<u64>,
    sources: &super::types::EnrichmentSourcesConfig,
    supplement_missing: bool,
) -> Result<SweepPointClassifyResult, String> {
    if ids.is_empty() {
        return Ok(SweepPointClassifyResult {
            complete_ids: Vec::new(),
        });
    }
    let payloads = fetch_payloads(config, provider, &ids).await?;
    let mut complete_ids = Vec::new();
    for id in ids {
        let Some(p) = payloads.get(&id) else {
            continue;
        };
        if !is_current_enrichment_version(p.get("enrichment_version").and_then(|v| v.as_str())) {
            continue;
        }
        if supplement_missing {
            let provenance = p
                .get("sources_provenance")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            // Nested provenance may have been stringified for cloud metadata stores.
            let provenance = match provenance {
                serde_json::Value::String(s) => {
                    serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
                }
                other => other,
            };
            if payload_needs_supplement(&provenance, sources) {
                continue;
            }
        }
        complete_ids.push(id);
    }
    Ok(SweepPointClassifyResult { complete_ids })
}

fn payload_needs_supplement(
    payload: &serde_json::Value,
    sources: &super::types::EnrichmentSourcesConfig,
) -> bool {
    super::sources_config::payload_needs_source_supplement(payload, sources)
}

/// Flatten nested JSON into Pinecone/Chroma-friendly metadata (primitives + stringified nests).
pub(crate) fn flatten_metadata(payload: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    let Some(obj) = payload.as_object() else {
        return out;
    };
    for (k, v) in obj {
        match v {
            serde_json::Value::Null => {}
            serde_json::Value::Bool(_) | serde_json::Value::Number(_) | serde_json::Value::String(_) => {
                out.insert(k.clone(), v.clone());
            }
            serde_json::Value::Array(arr) => {
                let all_str = arr.iter().all(|x| x.is_string());
                if all_str {
                    out.insert(k.clone(), v.clone());
                } else {
                    out.insert(
                        k.clone(),
                        serde_json::Value::String(v.to_string()),
                    );
                }
            }
            serde_json::Value::Object(_) => {
                out.insert(k.clone(), serde_json::Value::String(v.to_string()));
            }
        }
    }
    out
}

#[derive(Debug, Serialize)]
pub struct VectorBrowsePage {
    pub provider: String,
    pub points: Vec<QdrantHit>,
    pub next_offset: Option<serde_json::Value>,
    pub total_hint: Option<u64>,
    pub note: Option<String>,
}

/// Page through sample payloads with provider-native pagination.
pub async fn browse_sample_points(
    config: &QdrantConfig,
    ollama_url: &str,
    sample_id: i64,
    limit: u32,
    offset: Option<serde_json::Value>,
    rsid_query: Option<&str>,
) -> Result<VectorBrowsePage, String> {
    let lim = limit.clamp(1, 200);
    let provider = provider_from_config(config);
    if let Some(rsid) = rsid_query.map(str::trim).filter(|s| !s.is_empty()) {
        return browse_by_rsid(config, provider, ollama_url, sample_id, rsid).await;
    }
    match provider {
        VectorProvider::Qdrant => {
            let batch = scroll_qdrant_payload_batch(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                sample_id,
                lim,
                offset,
            )
            .await?;
            let total_hint = count_qdrant_points(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                Some(sample_id),
            )
            .await
            .ok();
            let points = batch
                .points
                .into_iter()
                .map(|(_, payload)| map_payload_to_hit(&payload, 1.0))
                .collect();
            Ok(VectorBrowsePage {
                provider: provider.as_str().into(),
                points,
                next_offset: batch.next_offset,
                total_hint,
                note: None,
            })
        }
        VectorProvider::Pinecone => {
            let (points, next, total) =
                pinecone::browse_page(config, sample_id, lim, offset.as_ref()).await?;
            Ok(VectorBrowsePage {
                provider: "pinecone".into(),
                points,
                next_offset: next,
                total_hint: total,
                note: Some(
                    "Pinecone serverless list+fetch. Sample filter applied on metadata when present."
                        .into(),
                ),
            })
        }
        VectorProvider::Chroma => {
            let (points, next, total) =
                chroma::browse_page(config, sample_id, lim, offset.as_ref()).await?;
            Ok(VectorBrowsePage {
                provider: "chroma".into(),
                points,
                next_offset: next,
                total_hint: total,
                note: None,
            })
        }
        VectorProvider::Weaviate => {
            let (points, next, total) =
                weaviate::browse_page(config, sample_id, lim, offset.as_ref()).await?;
            Ok(VectorBrowsePage {
                provider: "weaviate".into(),
                points,
                next_offset: next,
                total_hint: total,
                note: None,
            })
        }
    }
}

/// Dense vectors + light payload for atlas projection (all providers).
pub async fn sample_vectors_for_atlas(
    config: &QdrantConfig,
    sample_id: i64,
    limit: usize,
) -> Result<Vec<super::qdrant::ScrollVectorItem>, String> {
    let cap = limit.min(2500);
    match provider_from_config(config) {
        VectorProvider::Qdrant => {
            super::qdrant::scroll_qdrant_vectors_sample(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                sample_id,
                cap,
                "",
            )
            .await
        }
        VectorProvider::Pinecone => pinecone::sample_vectors(config, sample_id, cap).await,
        VectorProvider::Chroma => chroma::sample_vectors(config, sample_id, cap).await,
        VectorProvider::Weaviate => weaviate::sample_vectors(config, sample_id, cap).await,
    }
}

async fn browse_by_rsid(
    config: &QdrantConfig,
    provider: VectorProvider,
    ollama_url: &str,
    sample_id: i64,
    rsid: &str,
) -> Result<VectorBrowsePage, String> {
    match provider {
        VectorProvider::Qdrant => {
            let payload = find_point_payload_by_rsid(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                sample_id,
                rsid,
            )
            .await?;
            let points = match payload {
                Some((p, _)) => vec![map_payload_to_hit(&p, 1.0)],
                None => Vec::new(),
            };
            let n = points.len() as u64;
            Ok(VectorBrowsePage {
                provider: "qdrant".into(),
                points,
                next_offset: None,
                total_hint: Some(n),
                note: None,
            })
        }
        other => {
            let ollama = ollama_url.trim();
            if ollama.is_empty() {
                return Err("Ollama URL required for rsID lookup on non-Qdrant providers.".into());
            }
            let vector =
                crate::research::embed::embed_text(rsid, ollama, &config.embedding_model).await?;
            let points: Vec<QdrantHit> = search_dense(config, vector, Some(sample_id), 20, None, None)
                .await?
                .into_iter()
                .filter(|h| h.rsid.eq_ignore_ascii_case(rsid))
                .collect();
            let n = points.len() as u64;
            Ok(VectorBrowsePage {
                provider: other.as_str().into(),
                points,
                next_offset: None,
                total_hint: Some(n),
                note: Some(format!(
                    "Exact scroll is Qdrant-only; rsID lookup on {} uses embed+filter.",
                    other.as_str()
                )),
            })
        }
    }
}

pub(crate) fn hit_from_metadata(meta: &serde_json::Value, score: f32) -> QdrantHit {
    // Metadata may have stringified nested fields — prefer as-is for map_payload_to_hit.
    let mut payload = meta.clone();
    if let Some(obj) = payload.as_object_mut() {
        for key in [
            "gwas_associations",
            "gene_candidates",
            "sources_provenance",
            "cross_refs",
            "pack_refs",
            "association_summary",
            "trait_categories",
            "consultation_modes",
            "searchable_tags",
        ] {
            if let Some(serde_json::Value::String(s)) = obj.get(key).cloned() {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
                    obj.insert(key.to_string(), parsed);
                }
            }
        }
    }
    map_payload_to_hit(&payload, score)
}
