// ./src-tauri/src/research/vector_store/pinecone.rs
/*
Purpose: Pinecone dense vector adapter (index host + Api-Key).
*/

use super::{flatten_metadata, hit_from_metadata};
use crate::research::http::QDRANT_CLIENT;
use crate::research::types::{QdrantConfig, QdrantConnectionStatus, QdrantHit};
use std::collections::HashMap;

const API_VERSION: &str = "2025-10";

fn http() -> &'static reqwest::Client {
    &QDRANT_CLIENT
}

fn base(config: &QdrantConfig) -> String {
    config.url.trim().trim_end_matches('/').to_string()
}

fn ns(config: &QdrantConfig) -> String {
    let n = config.namespace.trim();
    if n.is_empty() {
        "__default__".into()
    } else {
        n.to_string()
    }
}

fn apply_key(mut req: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    req = req
        .header("Content-Type", "application/json")
        .header("X-Pinecone-Api-Version", API_VERSION);
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        req = req.header("Api-Key", key);
    }
    req
}

pub async fn test_connection(config: &QdrantConfig) -> QdrantConnectionStatus {
    if config.api_key.as_deref().map(str::trim).unwrap_or("").is_empty() {
        return QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some("Pinecone requires an API key.".into()),
        };
    }
    let endpoint = format!("{}/describe_index_stats", base(config));
    let req = apply_key(http().post(&endpoint).json(&serde_json::json!({})), config.api_key.as_deref());
    match req.send().await {
        Err(e) => QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some(format!("Pinecone connection failed: {e}")),
        },
        Ok(res) if res.status().is_success() => {
            let body: serde_json::Value = res.json().await.unwrap_or_default();
            let total = body
                .get("totalVectorCount")
                .or_else(|| body.get("total_vector_count"))
                .and_then(|v| v.as_u64());
            QdrantConnectionStatus {
                success: true,
                vectors_count: total,
                collection_exists: true,
                collections: Some(vec![ns(config)]),
                error: None,
            }
        }
        Ok(res) => QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some(format!("Pinecone HTTP {}", res.status())),
        },
    }
}

pub async fn ensure_ready(config: &QdrantConfig, dims: u32) -> Result<(), String> {
    let status = test_connection(config).await;
    if !status.success {
        return Err(status
            .error
            .unwrap_or_else(|| "Pinecone index not reachable".into()));
    }
    let _ = dims; // dimension is fixed at index creation time in Pinecone
    Ok(())
}

pub async fn purge_namespace(config: &QdrantConfig) -> Result<(), String> {
    let endpoint = format!("{}/vectors/delete", base(config));
    let body = serde_json::json!({
        "deleteAll": true,
        "namespace": ns(config),
    });
    let req = apply_key(http().post(&endpoint).json(&body), config.api_key.as_deref());
    let res = req
        .send()
        .await
        .map_err(|e| format!("Pinecone purge failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Pinecone purge HTTP {}", res.status()));
    }
    Ok(())
}

pub async fn upsert_batch(
    config: &QdrantConfig,
    points: Vec<(u64, Vec<f32>, serde_json::Value)>,
) -> Result<(), String> {
    let endpoint = format!("{}/vectors/upsert", base(config));
    let vectors: Vec<serde_json::Value> = points
        .into_iter()
        .map(|(id, values, payload)| {
            serde_json::json!({
                "id": id.to_string(),
                "values": values,
                "metadata": flatten_metadata(&payload),
            })
        })
        .collect();
    // Pinecone recommends batches <= ~100 vectors / 2MB; chunk defensively.
    for chunk in vectors.chunks(64) {
        let body = serde_json::json!({
            "vectors": chunk,
            "namespace": ns(config),
        });
        let req = apply_key(http().post(&endpoint).json(&body), config.api_key.as_deref());
        let res = req
            .send()
            .await
            .map_err(|e| format!("Pinecone upsert failed: {e}"))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("Pinecone upsert failed: {text}"));
        }
    }
    Ok(())
}

pub async fn search(
    config: &QdrantConfig,
    vector: Vec<f32>,
    sample_id: Option<i64>,
    limit: u32,
    trait_category: Option<&str>,
) -> Result<Vec<QdrantHit>, String> {
    let endpoint = format!("{}/query", base(config));
    let mut filter = serde_json::Map::new();
    if let Some(sid) = sample_id {
        filter.insert(
            "sample_id".into(),
            serde_json::json!({ "$eq": sid }),
        );
    }
    if let Some(cat) = trait_category.map(str::trim).filter(|c| !c.is_empty()) {
        // Arrays in metadata: Pinecone $in on list fields
        filter.insert(
            "trait_categories".into(),
            serde_json::json!({ "$in": [cat] }),
        );
    }
    let mut body = serde_json::json!({
        "vector": vector,
        "topK": limit.max(1),
        "includeMetadata": true,
        "namespace": ns(config),
    });
    if !filter.is_empty() {
        body["filter"] = serde_json::Value::Object(filter);
    }
    let req = apply_key(http().post(&endpoint).json(&body), config.api_key.as_deref());
    let res = req
        .send()
        .await
        .map_err(|e| format!("Pinecone query failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Pinecone query HTTP {}", res.status()));
    }
    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Pinecone query parse failed: {e}"))?;
    let matches = val
        .get("matches")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(matches
        .into_iter()
        .map(|m| {
            let score = m.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let meta = m.get("metadata").cloned().unwrap_or(serde_json::json!({}));
            hit_from_metadata(&meta, score)
        })
        .collect())
}

pub async fn fetch_payloads(
    config: &QdrantConfig,
    ids: &[u64],
) -> Result<HashMap<u64, serde_json::Value>, String> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    for chunk in ids.chunks(100) {
        let mut url = reqwest::Url::parse(&format!("{}/vectors/fetch", base(config)))
            .map_err(|e| e.to_string())?;
        {
            let mut q = url.query_pairs_mut();
            for id in chunk {
                q.append_pair("ids", &id.to_string());
            }
            q.append_pair("namespace", &ns(config));
        }
        let req = apply_key(http().get(url), config.api_key.as_deref());
        let res = req
            .send()
            .await
            .map_err(|e| format!("Pinecone fetch failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("Pinecone fetch HTTP {}", res.status()));
        }
        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        if let Some(vectors) = val.get("vectors").and_then(|v| v.as_object()) {
            for (id_str, rec) in vectors {
                if let Ok(id) = id_str.parse::<u64>() {
                    let meta = rec
                        .get("metadata")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    out.insert(id, meta);
                }
            }
        }
    }
    Ok(out)
}

/// List IDs then fetch payloads (serverless). `offset` = pagination token string.
pub async fn browse_page(
    config: &QdrantConfig,
    sample_id: i64,
    limit: u32,
    offset: Option<&serde_json::Value>,
) -> Result<(Vec<QdrantHit>, Option<serde_json::Value>, Option<u64>), String> {
    let lim = limit.clamp(1, 100);
    let mut url = reqwest::Url::parse(&format!("{}/vectors/list", base(config)))
        .map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("namespace", &ns(config));
        q.append_pair("limit", &lim.to_string());
        if let Some(tok) = offset.and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            q.append_pair("paginationToken", tok);
        }
    }
    let req = apply_key(http().get(url), config.api_key.as_deref());
    let res = req
        .send()
        .await
        .map_err(|e| format!("Pinecone list failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!(
            "Pinecone list HTTP {status} (serverless list required for browse): {body}"
        ));
    }
    let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let ids: Vec<String> = val
        .get("vectors")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| item.as_str().map(str::to_string))
        })
        .collect();
    let next = val
        .pointer("/pagination/next")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::Value::String(s.to_string()));

    if ids.is_empty() {
        return Ok((Vec::new(), next, None));
    }

    // Fetch full records
    let mut fetch_url = reqwest::Url::parse(&format!("{}/vectors/fetch", base(config)))
        .map_err(|e| e.to_string())?;
    {
        let mut q = fetch_url.query_pairs_mut();
        for id in &ids {
            q.append_pair("ids", id);
        }
        q.append_pair("namespace", &ns(config));
    }
    let fetch_req = apply_key(http().get(fetch_url), config.api_key.as_deref());
    let fetch_res = fetch_req
        .send()
        .await
        .map_err(|e| format!("Pinecone fetch failed: {e}"))?;
    if !fetch_res.status().is_success() {
        return Err(format!("Pinecone fetch HTTP {}", fetch_res.status()));
    }
    let fetched: serde_json::Value = fetch_res.json().await.map_err(|e| e.to_string())?;
    let mut points = Vec::new();
    if let Some(vectors) = fetched.get("vectors").and_then(|v| v.as_object()) {
        for (_id, rec) in vectors {
            let meta = rec
                .get("metadata")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let sid = meta.get("sample_id").and_then(|v| v.as_i64());
            if sid.is_some_and(|s| s != sample_id) {
                continue;
            }
            // If sample_id missing in meta, still include (namespace may be sample-scoped).
            if sid.is_none() {
                // keep — user may store sample elsewhere
            }
            points.push(hit_from_metadata(&meta, 1.0));
        }
    }
    let stats = test_connection(config).await;
    Ok((points, next, stats.vectors_count))
}

/// Sample dense vectors for atlas (list → fetch with values).
pub async fn sample_vectors(
    config: &QdrantConfig,
    sample_id: i64,
    limit: usize,
) -> Result<Vec<crate::research::qdrant::ScrollVectorItem>, String> {
    let mut out = Vec::new();
    let mut token: Option<String> = None;
    let mut pages = 0usize;
    while out.len() < limit && pages < 40 {
        pages += 1;
        let mut url = reqwest::Url::parse(&format!("{}/vectors/list", base(config)))
            .map_err(|e| e.to_string())?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("namespace", &ns(config));
            q.append_pair("limit", "100");
            if let Some(t) = &token {
                q.append_pair("paginationToken", t);
            }
        }
        let req = apply_key(http().get(url), config.api_key.as_deref());
        let res = req.send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            break;
        }
        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let ids: Vec<String> = val
            .get("vectors")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                item.get("id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        token = val
            .pointer("/pagination/next")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if ids.is_empty() {
            break;
        }
        let mut fetch_url = reqwest::Url::parse(&format!("{}/vectors/fetch", base(config)))
            .map_err(|e| e.to_string())?;
        {
            let mut q = fetch_url.query_pairs_mut();
            for id in &ids {
                q.append_pair("ids", id);
            }
            q.append_pair("namespace", &ns(config));
        }
        let fetch_req = apply_key(http().get(fetch_url), config.api_key.as_deref());
        let fetch_res = fetch_req.send().await.map_err(|e| e.to_string())?;
        if !fetch_res.status().is_success() {
            break;
        }
        let fetched: serde_json::Value = fetch_res.json().await.map_err(|e| e.to_string())?;
        if let Some(vectors) = fetched.get("vectors").and_then(|v| v.as_object()) {
            for (id, rec) in vectors {
                let meta = rec.get("metadata").cloned().unwrap_or_default();
                let sid = meta.get("sample_id").and_then(|v| v.as_i64());
                if sid.is_some_and(|s| s != sample_id) {
                    continue;
                }
                let values = rec
                    .get("values")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_f64().map(|f| f as f32))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if values.is_empty() {
                    continue;
                }
                out.push(crate::research::qdrant::ScrollVectorItem {
                    point_id: id.clone(),
                    rsid: meta
                        .get("rsid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    vector: values,
                    trait_category: meta
                        .get("trait_category")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    gene_symbol: meta
                        .get("gene_symbol")
                        .or_else(|| meta.get("gene"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    data_quality_score: meta
                        .get("data_quality_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32,
                });
                if out.len() >= limit {
                    break;
                }
            }
        }
        if token.is_none() {
            break;
        }
    }
    Ok(out)
}
