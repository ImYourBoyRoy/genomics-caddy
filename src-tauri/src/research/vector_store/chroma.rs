// ./src-tauri/src/research/vector_store/chroma.rs
/*
Purpose: Chroma HTTP dense vector adapter (self-host / cloud).
*/

use super::{flatten_metadata, hit_from_metadata};
use crate::research::http::QDRANT_CLIENT;
use crate::research::types::{QdrantConfig, QdrantConnectionStatus, QdrantHit};
use std::collections::HashMap;

fn http() -> &'static reqwest::Client {
    &QDRANT_CLIENT
}

fn base(config: &QdrantConfig) -> String {
    config.url.trim().trim_end_matches('/').to_string()
}

fn auth(mut req: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        req = req.header("Authorization", format!("Bearer {key}"));
        req = req.header("X-Chroma-Token", key);
    }
    req
}

pub async fn test_connection(config: &QdrantConfig) -> QdrantConnectionStatus {
    let heartbeat = format!("{}/api/v2/heartbeat", base(config));
    let req = auth(http().get(&heartbeat), config.api_key.as_deref());
    let hb = match req.send().await {
        Err(e) => {
            return QdrantConnectionStatus {
                success: false,
                vectors_count: None,
                collection_exists: false,
                collections: None,
                error: Some(format!("Chroma connection failed: {e}")),
            };
        }
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            // Fall back to v1 heartbeat used by older Chroma servers.
            let hb1 = format!("{}/api/v1/heartbeat", base(config));
            match auth(http().get(&hb1), config.api_key.as_deref())
                .send()
                .await
            {
                Ok(r2) if r2.status().is_success() => r2,
                _ => {
                    return QdrantConnectionStatus {
                        success: false,
                        vectors_count: None,
                        collection_exists: false,
                        collections: None,
                        error: Some(format!("Chroma HTTP {}", r.status())),
                    };
                }
            }
        }
    };
    let _ = hb;

    let (exists, names, count) = list_collection_info(config).await;
    QdrantConnectionStatus {
        success: true,
        vectors_count: count,
        collection_exists: exists,
        collections: Some(names),
        error: None,
    }
}

async fn list_collection_info(
    config: &QdrantConfig,
) -> (bool, Vec<String>, Option<u64>) {
    let coll = config.collection.trim();
    // Prefer v2 list; fall back to v1.
    for path in ["/api/v2/collections", "/api/v1/collections"] {
        let endpoint = format!("{}{path}", base(config));
        let Ok(res) = auth(http().get(&endpoint), config.api_key.as_deref())
            .send()
            .await
        else {
            continue;
        };
        if !res.status().is_success() {
            continue;
        }
        let Ok(body) = res.json::<serde_json::Value>().await else {
            continue;
        };
        let names = extract_collection_names(&body);
        let exists = names.iter().any(|n| n == coll);
        return (exists || coll.is_empty(), names, None);
    }
    (!coll.is_empty(), Vec::new(), None)
}

fn extract_collection_names(body: &serde_json::Value) -> Vec<String> {
    let arr = body
        .as_array()
        .cloned()
        .or_else(|| body.get("collections").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();
    arr.into_iter()
        .filter_map(|c| {
            c.get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .or_else(|| c.as_str().map(str::to_string))
        })
        .collect()
}

pub async fn ensure_collection(config: &QdrantConfig, dims: u32) -> Result<(), String> {
    let _ = dims;
    let name = config.collection.trim();
    if name.is_empty() {
        return Err("Chroma requires a collection name.".into());
    }
    let status = test_connection(config).await;
    if !status.success {
        return Err(status
            .error
            .unwrap_or_else(|| "Chroma not reachable".into()));
    }
    if status.collection_exists {
        return Ok(());
    }
    for path in ["/api/v2/collections", "/api/v1/collections"] {
        let endpoint = format!("{}{path}", base(config));
        let body = serde_json::json!({
            "name": name,
            "get_or_create": true,
            "metadata": { "hnsw:space": "cosine" }
        });
        let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Chroma create collection failed: {e}"))?;
        if res.status().is_success() || res.status().as_u16() == 409 {
            return Ok(());
        }
    }
    Err(format!(
        "Could not create Chroma collection '{name}'. Create it in Chroma first, then retry."
    ))
}

pub async fn purge_collection(config: &QdrantConfig) -> Result<(), String> {
    let name = config.collection.trim();
    if name.is_empty() {
        return Err("Chroma collection name required for purge.".into());
    }
    for path in [
        format!("/api/v2/collections/{name}"),
        format!("/api/v1/collections/{name}"),
    ] {
        let endpoint = format!("{}{path}", base(config));
        let res = auth(http().delete(&endpoint), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Chroma purge failed: {e}"))?;
        if res.status().is_success() || res.status().as_u16() == 404 {
            return Ok(());
        }
    }
    Err("Chroma purge failed".into())
}

async fn collection_id(config: &QdrantConfig) -> Result<String, String> {
    let name = config.collection.trim();
    if name.is_empty() {
        return Err("Chroma collection name required.".into());
    }
    // Many Chroma deployments accept name in the path; return name as id.
    Ok(name.to_string())
}

pub async fn upsert_batch(
    config: &QdrantConfig,
    points: Vec<(u64, Vec<f32>, serde_json::Value)>,
) -> Result<(), String> {
    let coll = collection_id(config).await?;
    let ids: Vec<String> = points.iter().map(|(id, _, _)| id.to_string()).collect();
    let embeddings: Vec<Vec<f32>> = points.iter().map(|(_, v, _)| v.clone()).collect();
    let documents: Vec<String> = points
        .iter()
        .map(|(_, _, p)| {
            p.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    let metadatas: Vec<serde_json::Value> = points
        .iter()
        .map(|(_, _, p)| serde_json::Value::Object(flatten_metadata(p)))
        .collect();

    let body = serde_json::json!({
        "ids": ids,
        "embeddings": embeddings,
        "documents": documents,
        "metadatas": metadatas,
    });

    for path in [
        format!("/api/v2/collections/{coll}/upsert"),
        format!("/api/v1/collections/{coll}/upsert"),
        format!("/api/v1/collections/{coll}/add"),
    ] {
        let endpoint = format!("{}{path}", base(config));
        let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Chroma upsert failed: {e}"))?;
        if res.status().is_success() {
            return Ok(());
        }
    }
    Err("Chroma upsert failed on v1/v2 endpoints".into())
}

pub async fn search(
    config: &QdrantConfig,
    vector: Vec<f32>,
    sample_id: Option<i64>,
    limit: u32,
    trait_category: Option<&str>,
) -> Result<Vec<QdrantHit>, String> {
    let coll = collection_id(config).await?;
    let mut where_filter = serde_json::Map::new();
    if let Some(sid) = sample_id {
        where_filter.insert("sample_id".into(), serde_json::json!({ "$eq": sid }));
    }
    if let Some(cat) = trait_category.map(str::trim).filter(|c| !c.is_empty()) {
        where_filter.insert(
            "trait_categories".into(),
            serde_json::json!({ "$contains": cat }),
        );
    }
    let mut body = serde_json::json!({
        "query_embeddings": [vector],
        "n_results": limit.max(1),
        "include": ["metadatas", "distances", "documents"],
    });
    if !where_filter.is_empty() {
        body["where"] = serde_json::Value::Object(where_filter);
    }

    for path in [
        format!("/api/v2/collections/{coll}/query"),
        format!("/api/v1/collections/{coll}/query"),
    ] {
        let endpoint = format!("{}{path}", base(config));
        let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Chroma query failed: {e}"))?;
        if !res.status().is_success() {
            continue;
        }
        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        return Ok(parse_chroma_query(&val));
    }
    Err("Chroma query failed".into())
}

fn parse_chroma_query(val: &serde_json::Value) -> Vec<QdrantHit> {
    let metadatas = val
        .pointer("/metadatas/0")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let distances = val
        .pointer("/distances/0")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let documents = val
        .pointer("/documents/0")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    metadatas
        .into_iter()
        .enumerate()
        .map(|(i, mut meta)| {
            if meta.get("text").is_none() {
                if let Some(doc) = documents.get(i).and_then(|d| d.as_str()) {
                    if let Some(obj) = meta.as_object_mut() {
                        obj.insert("text".into(), serde_json::Value::String(doc.into()));
                    }
                }
            }
            let dist = distances
                .get(i)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            // Cosine distance → rough similarity
            let score = 1.0 - dist;
            hit_from_metadata(&meta, score)
        })
        .collect()
}

pub async fn fetch_payloads(
    config: &QdrantConfig,
    ids: &[u64],
) -> Result<HashMap<u64, serde_json::Value>, String> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let coll = collection_id(config).await?;
    let id_strs: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    let body = serde_json::json!({
        "ids": id_strs,
        "include": ["metadatas"],
    });
    for path in [
        format!("/api/v2/collections/{coll}/get"),
        format!("/api/v1/collections/{coll}/get"),
    ] {
        let endpoint = format!("{}{path}", base(config));
        let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Chroma get failed: {e}"))?;
        if !res.status().is_success() {
            continue;
        }
        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let got_ids = val
            .get("ids")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let metas = val
            .get("metadatas")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for (i, id_val) in got_ids.into_iter().enumerate() {
            let id = id_val
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| id_val.as_u64());
            if let Some(id) = id {
                let meta = metas.get(i).cloned().unwrap_or(serde_json::json!({}));
                out.insert(id, meta);
            }
        }
        return Ok(out);
    }
    Ok(out)
}
