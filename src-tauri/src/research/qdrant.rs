// ./src-tauri/src/research/qdrant.rs
use super::embed::embed_text;
use super::http::{HEALTH_CHECK_CLIENT, QDRANT_CLIENT};
use super::types::*;
use super::util::{is_current_enrichment_version, string_to_u64};
use std::collections::HashMap;

fn qdrant_http() -> &'static reqwest::Client {
    &QDRANT_CLIENT
}

fn apply_named_vector(body: &mut serde_json::Value, vector_name: Option<&str>) {
    if let Some(name) = vector_name.filter(|n| !n.is_empty()) {
        body["using"] = serde_json::json!(name);
    }
}

pub async fn test_qdrant_connection(
    url: &str,
    api_key: Option<&str>,
    collection: Option<&str>,
) -> QdrantConnectionStatus {
    let client = &*HEALTH_CHECK_CLIENT;

    let coll = collection
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .unwrap_or("genomics_evidence");

    let base = url.trim_end_matches('/');

    // Step 1: Check if the database is live by calling /collections
    let collections_endpoint = format!("{base}/collections");
    let mut req = client.get(&collections_endpoint);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        req = req.header("api-key", key);
    }

    let res = match req.send().await {
        Err(e) => {
            return QdrantConnectionStatus {
                success: false,
                vectors_count: None,
                collection_exists: false,
                collections: None,
                error: Some(format!("Connection failed: {}", e)),
            };
        }
        Ok(r) => r,
    };

    if res.status() == 401 || res.status() == 403 {
        return QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some("401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings.".to_string()),
        };
    }

    if !res.status().is_success() {
        return QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some(format!(
                "Qdrant returned HTTP {} when checking database status",
                res.status()
            )),
        };
    }

    // Database is live! Parse collections
    let body: serde_json::Value = match res.json().await {
        Ok(v) => v,
        Err(e) => {
            return QdrantConnectionStatus {
                success: true,
                vectors_count: None,
                collection_exists: false,
                collections: None,
                error: Some(format!(
                    "Database is live, but failed to parse /collections response: {}",
                    e
                )),
            };
        }
    };

    let collections = parse_collection_names(&body);
    let mut exists = collection_name_matches(&collections, coll);

    // Step 1b: Some proxies return an incomplete list — probe the collection directly.
    if !exists {
        exists = probe_collection_exists(client, base, api_key, coll).await;
    }

    if !exists {
        return QdrantConnectionStatus {
            success: true,
            vectors_count: None,
            collection_exists: false,
            collections: Some(collections),
            error: None,
        };
    }

    // Step 2: Since collection exists, fetch its details to get vectors_count
    let detail_endpoint = format!("{base}/collections/{}", urlencoding_path_segment(coll));
    let mut detail_req = client.get(&detail_endpoint);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        detail_req = detail_req.header("api-key", key);
    }

    match detail_req.send().await {
        Ok(detail_res) => {
            if detail_res.status().is_success()
                && let Ok(detail_body) = detail_res.json::<serde_json::Value>().await
            {
                let vectors_count = detail_body["result"]["points_count"]
                    .as_u64()
                    .or_else(|| detail_body["result"]["vectors_count"].as_u64());
                return QdrantConnectionStatus {
                    success: true,
                    vectors_count,
                    collection_exists: true,
                    collections: Some(collections),
                    error: None,
                };
            }
            QdrantConnectionStatus {
                success: true,
                vectors_count: None,
                collection_exists: true,
                collections: Some(collections),
                error: None,
            }
        }
        Err(_) => QdrantConnectionStatus {
            success: true,
            vectors_count: None,
            collection_exists: true,
            collections: Some(collections),
            error: None,
        },
    }
}

fn parse_collection_names(body: &serde_json::Value) -> Vec<String> {
    let mut collections = Vec::new();
    let candidates = [
        body.pointer("/result/collections"),
        body.pointer("/collections"),
        body.get("result").filter(|v| v.is_array()),
    ];
    for node in candidates.into_iter().flatten() {
        if let Some(arr) = node.as_array() {
            for c in arr {
                if let Some(name) = c.as_str() {
                    collections.push(name.trim().to_string());
                } else if let Some(name) = c.get("name").and_then(|v| v.as_str()) {
                    collections.push(name.trim().to_string());
                }
            }
        }
    }
    collections.sort_unstable();
    collections.dedup();
    collections
}

fn collection_name_matches(collections: &[String], target: &str) -> bool {
    let target = target.trim();
    collections
        .iter()
        .any(|name| name.eq_ignore_ascii_case(target))
}

fn urlencoding_path_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => ch.to_string(),
            _ => format!("%{:02X}", ch as u8),
        })
        .collect()
}

async fn probe_collection_exists(
    client: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    collection: &str,
) -> bool {
    let endpoint = format!(
        "{}/collections/{}",
        base_url,
        urlencoding_path_segment(collection.trim())
    );
    let mut req = client.get(&endpoint);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        req = req.header("api-key", key);
    }
    match req.send().await {
        Ok(res) if res.status().is_success() => true,
        Ok(res) if res.status() == 404 => false,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// 2. ensure_qdrant_collection
// ---------------------------------------------------------------------------

pub async fn ensure_qdrant_collection(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    dims: u32,
) -> Result<(), String> {
    let client = qdrant_http();

    let endpoint = format!("{}/collections/{}", url.trim_end_matches('/'), collection);
    let body = serde_json::json!({
        "vectors": {
            "size": dims,
            "distance": "Cosine"
        }
    });

    let mut req = client.put(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant request failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err("401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings.".to_string());
    }
    // 200 OK = created, 409 Conflict = already exists — both are acceptable
    if res.status().is_success() || res.status() == 409 {
        return Ok(());
    }
    Err(format!(
        "Qdrant returned HTTP {} when creating collection",
        res.status()
    ))
}

/// Delete a Qdrant collection (testing / reset). Missing collection is treated as success.
pub async fn purge_qdrant_collection(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
) -> Result<(), String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}",
        url.trim_end_matches('/'),
        collection.trim()
    );
    let mut req = client.delete(&endpoint);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant purge request failed: {}", e))?;

    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if res.status() == 404 || res.status().is_success() {
        return Ok(());
    }
    Err(format!(
        "Qdrant returned HTTP {} when deleting collection",
        res.status()
    ))
}

/// Probe Ollama embedding dimensions and create the configured Qdrant collection.
pub async fn initialize_qdrant_collection(
    ollama_url: &str,
    config: &QdrantConfig,
) -> Result<u32, String> {
    let vector = embed_text("dimension probe", ollama_url, &config.embedding_model).await?;
    let dims = vector.len() as u32;
    if dims == 0 {
        return Err("Embedding model returned an empty vector".to_string());
    }
    ensure_qdrant_collection(
        &config.url,
        config.api_key.as_deref(),
        &config.collection,
        dims,
    )
    .await?;
    Ok(dims)
}

// ---------------------------------------------------------------------------
// 2b. check_existing_points
// ---------------------------------------------------------------------------

pub async fn check_existing_points(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    ids: Vec<u64>,
) -> Result<Vec<u64>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "ids": ids,
        "with_payload": false,
        "with_vector": false
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant points check failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err("401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings.".to_string());
    }
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant points check returned HTTP {}",
            res.status()
        ));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant points response: {}", e))?;

    let result = val["result"]
        .as_array()
        .ok_or("Invalid Qdrant points result format")?;
    let mut existing_ids = Vec::new();
    for item in result {
        if let Some(id) = item["id"].as_u64() {
            existing_ids.push(id);
        }
    }
    Ok(existing_ids)
}

/// Result of chunk classification for sweep: fully complete vs needs prepare+upsert.
pub struct SweepPointClassifyResult {
    pub complete_ids: Vec<u64>,
}

/// Classify chunk points: current enrichment version and optional per-source supplement gaps.
pub async fn classify_points_sweep_state(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    ids: Vec<u64>,
    sources: &super::types::EnrichmentSourcesConfig,
    supplement_missing: bool,
) -> Result<SweepPointClassifyResult, String> {
    if ids.is_empty() {
        return Ok(SweepPointClassifyResult {
            complete_ids: Vec::new(),
        });
    }

    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "ids": ids,
        "with_payload": ["enrichment_version", "sources_provenance"],
        "with_vector": false
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant points classify failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant points classify returned HTTP {}",
            res.status()
        ));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant classify response: {}", e))?;

    let result = val["result"]
        .as_array()
        .ok_or("Invalid Qdrant classify result format")?;
    let mut complete_ids = Vec::new();
    for item in result {
        let Some(id) = item["id"].as_u64() else {
            continue;
        };
        let version = item["payload"]["enrichment_version"].as_str();
        if !is_current_enrichment_version(version) {
            continue;
        }
        if supplement_missing {
            let provenance = item["payload"]["sources_provenance"].clone();
            if super::sources_config::payload_needs_source_supplement(&provenance, sources) {
                continue;
            }
        }
        complete_ids.push(id);
    }
    Ok(SweepPointClassifyResult { complete_ids })
}

/// Split chunk IDs into points indexed at the current enrichment version vs stale/missing.
pub async fn classify_points_index_state(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    ids: Vec<u64>,
) -> Result<Vec<u64>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "ids": ids,
        "with_payload": ["enrichment_version", "gene_symbol", "has_gwas"],
        "with_vector": false
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant points classify failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant points classify returned HTTP {}",
            res.status()
        ));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant classify response: {}", e))?;

    let result = val["result"]
        .as_array()
        .ok_or("Invalid Qdrant classify result format")?;
    let mut current = Vec::new();
    for item in result {
        let Some(id) = item["id"].as_u64() else {
            continue;
        };
        let version = item["payload"]["enrichment_version"].as_str();
        if is_current_enrichment_version(version) {
            current.push(id);
        }
    }
    Ok(current)
}

// ---------------------------------------------------------------------------
// 3. embed_text
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 4. upsert_to_qdrant
// ---------------------------------------------------------------------------

pub async fn upsert_to_qdrant(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    point_id: &str,
    vector: Vec<f32>,
    payload: serde_json::Value,
) -> Result<(), String> {
    let client = qdrant_http();

    let numeric_id = string_to_u64(point_id);
    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "points": [{
            "id": numeric_id,
            "vector": vector,
            "payload": payload
        }]
    });

    let mut req = client.put(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant upsert failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err("401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings.".to_string());
    }
    if !res.status().is_success() {
        return Err(format!("Qdrant upsert returned HTTP {}", res.status()));
    }
    Ok(())
}

/// Upsert many Qdrant points in one HTTP request.
pub async fn upsert_points_batch(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    points: Vec<(u64, Vec<f32>, serde_json::Value)>,
) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }

    let client = qdrant_http();

    let qdrant_points: Vec<serde_json::Value> = points
        .into_iter()
        .map(|(id, vector, payload)| {
            serde_json::json!({
                "id": id,
                "vector": vector,
                "payload": payload
            })
        })
        .collect();

    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({ "points": qdrant_points });

    let mut req = client.put(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant batch upsert failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant batch upsert returned HTTP {}",
            res.status()
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. search_qdrant
// ---------------------------------------------------------------------------

fn build_payload_filter(
    sample_id: Option<i64>,
    trait_category: Option<&str>,
) -> Option<serde_json::Value> {
    let mut must: Vec<serde_json::Value> = Vec::new();
    if let Some(id) = sample_id {
        must.push(serde_json::json!({
            "key": "sample_id",
            "match": { "value": id }
        }));
    }
    if let Some(category) = trait_category.filter(|c| !c.trim().is_empty()) {
        must.push(serde_json::json!({
            "key": "trait_categories",
            "match": { "any": [category.trim()] }
        }));
    }
    if must.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "must": must }))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn search_qdrant(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    vector: Vec<f32>,
    sample_id: Option<i64>,
    limit: u32,
    trait_category: Option<&str>,
    vector_name: Option<&str>,
) -> Result<Vec<QdrantHit>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/search",
        url.trim_end_matches('/'),
        collection
    );

    let filter = build_payload_filter(sample_id, trait_category);
    let mut body = if let Some(f) = filter {
        serde_json::json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true,
            "filter": f
        })
    } else {
        serde_json::json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true
        })
    };
    apply_named_vector(&mut body, vector_name);

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant search failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err("401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings.".to_string());
    }
    if !res.status().is_success() {
        return Err(format!("Qdrant search returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant search response: {}", e))?;

    let results = val["result"].as_array().cloned().unwrap_or_default();
    let mut hits = Vec::new();
    for item in results {
        let score = item["score"].as_f64().unwrap_or(0.0) as f32;
        let p = &item["payload"];
        hits.push(map_payload_to_hit(p, score));
    }
    Ok(hits)
}

pub async fn scroll_qdrant_points(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    trait_category: Option<&str>,
    limit: u32,
) -> Result<Vec<QdrantHit>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );

    let filter = build_payload_filter(Some(sample_id), trait_category).unwrap_or_else(|| {
        serde_json::json!({
            "must": [{
                "key": "sample_id",
                "match": { "value": sample_id }
            }]
        })
    });

    let body = serde_json::json!({
        "filter": filter,
        "limit": limit,
        "with_payload": true,
        "with_vector": false
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant scroll failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if !res.status().is_success() {
        return Err(format!("Qdrant scroll returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant scroll response: {}", e))?;

    let points = val["result"]["points"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut hits = Vec::new();
    for item in points {
        let p = &item["payload"];
        hits.push(map_payload_to_hit(p, 1.0));
    }
    hits.sort_by(|a, b| {
        b.significance_score
            .partial_cmp(&a.significance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(hits)
}

#[derive(Debug, Clone)]
pub struct ScrollVectorItem {
    pub point_id: String,
    pub rsid: String,
    pub vector: Vec<f32>,
    pub trait_category: Option<String>,
    pub gene_symbol: Option<String>,
    pub data_quality_score: f32,
}

pub async fn scroll_qdrant_vectors_sample(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    limit: usize,
    _vector_name: &str,
) -> Result<Vec<ScrollVectorItem>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );

    let body = serde_json::json!({
        "filter": {
            "must": [{
                "key": "sample_id",
                "match": { "value": sample_id }
            }]
        },
        "limit": limit.min(2500),
        "with_payload": ["rsid", "trait_category", "gene_symbol", "data_quality_score"],
        "with_vector": true
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant vector scroll failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant vector scroll returned HTTP {}",
            res.status()
        ));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant vector scroll: {}", e))?;

    let points = val["result"]["points"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for item in points {
        let vector = extract_default_vector(&item["vector"]);
        if vector.is_empty() {
            continue;
        }
        let p = &item["payload"];
        let id = item["id"]
            .as_u64()
            .map(|u| u.to_string())
            .or_else(|| item["id"].as_str().map(String::from))
            .unwrap_or_default();
        out.push(ScrollVectorItem {
            point_id: id,
            rsid: p["rsid"].as_str().unwrap_or("").to_string(),
            vector,
            trait_category: payload_opt_str(p, "trait_category"),
            gene_symbol: payload_opt_str(p, "gene_symbol"),
            data_quality_score: p["data_quality_score"].as_f64().unwrap_or(0.0) as f32,
        });
    }
    Ok(out)
}

/// Scroll recent point payloads for a sample (full payload for finding previews).
pub async fn scroll_sample_payloads(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let client = qdrant_http();
    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "filter": {
            "must": [{
                "key": "sample_id",
                "match": { "value": sample_id }
            }]
        },
        "limit": limit.min(24),
        "with_payload": true,
        "with_vector": false
    });
    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant payload scroll failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant payload scroll returned HTTP {}",
            res.status()
        ));
    }
    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant payload scroll: {}", e))?;
    let points = val["result"]["points"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(points
        .into_iter()
        .filter_map(|item| item.get("payload").cloned())
        .collect())
}

fn extract_default_vector(v: &serde_json::Value) -> Vec<f32> {
    if let Some(arr) = v.as_array() {
        return arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
    }
    if let Some(default) = v.get("")
        && let Some(arr) = default.as_array()
    {
        return arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
    }
    if let Some(obj) = v.as_object() {
        for key in ["", "default", "dense"] {
            if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
                return arr
                    .iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect();
            }
        }
        if let Some((_name, val)) = obj.iter().next()
            && let Some(arr) = val.as_array()
        {
            return arr
                .iter()
                .filter_map(|x| x.as_f64().map(|f| f as f32))
                .collect();
        }
    }
    vec![]
}

pub async fn ensure_qdrant_collection_named(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    dims: u32,
) -> Result<(), String> {
    let client = qdrant_http();

    let endpoint = format!("{}/collections/{}", url.trim_end_matches('/'), collection);
    let body = serde_json::json!({
        "vectors": {
            "size": dims,
            "distance": "Cosine"
        },
        "sparse_vectors": {},
        "shard_number": 1,
        "replication_factor": 1,
        "write_consistency_factor": 1,
        "on_disk_payload": true
    });

    let mut req = client.put(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant request failed: {}", e))?;
    if res.status().is_success() || res.status() == 409 {
        return update_collection_named_vectors(url, api_key, collection, dims).await;
    }
    Err(format!(
        "Qdrant returned HTTP {} when creating collection",
        res.status()
    ))
}

async fn update_collection_named_vectors(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    dims: u32,
) -> Result<(), String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/vectors",
        url.trim_end_matches('/'),
        collection
    );
    for name in [
        "trait_dense",
        "gene_mechanism_dense",
        "evidence_dense",
        "actionability_dense",
    ] {
        let body = serde_json::json!({
            "vectors": {
                name: {
                    "size": dims,
                    "distance": "Cosine"
                }
            }
        });
        let mut req = client.patch(&endpoint).json(&body);
        if let Some(key) = api_key {
            req = req.header("api-key", key);
        }
        let _ = req.send().await;
    }
    Ok(())
}

pub async fn upsert_points_batch_named(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    points: Vec<(u64, HashMap<String, Vec<f32>>, serde_json::Value)>,
) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }

    let client = qdrant_http();

    let qdrant_points: Vec<serde_json::Value> = points
        .into_iter()
        .map(|(id, vectors, payload)| {
            let vector_json = if vectors.len() == 1 && vectors.contains_key("") {
                serde_json::json!(vectors.get("").cloned().unwrap_or_default())
            } else {
                serde_json::json!(vectors)
            };
            serde_json::json!({
                "id": id,
                "vector": vector_json,
                "payload": payload
            })
        })
        .collect();

    let endpoint = format!(
        "{}/collections/{}/points",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({ "points": qdrant_points });

    let mut req = client.put(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant named batch upsert failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant named batch upsert returned HTTP {}",
            res.status()
        ));
    }
    Ok(())
}

fn payload_opt_str(p: &serde_json::Value, key: &str) -> Option<String> {
    p.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn payload_opt_array(p: &serde_json::Value, key: &str) -> Option<Vec<serde_json::Value>> {
    p.get(key).and_then(|v| v.as_array()).cloned()
}

fn map_payload_to_hit(p: &serde_json::Value, score: f32) -> QdrantHit {
    QdrantHit {
        rsid: p["rsid"].as_str().unwrap_or("").to_string(),
        gene: payload_opt_str(p, "gene_symbol").or_else(|| payload_opt_str(p, "gene")),
        category: payload_opt_str(p, "evidence_tier").or_else(|| payload_opt_str(p, "category")),
        text: p["text"].as_str().unwrap_or("").to_string(),
        source: p["indexed_by"]
            .as_str()
            .or_else(|| p["source"].as_str())
            .unwrap_or("qdrant")
            .to_string(),
        score,
        pmid: payload_opt_str(p, "pmid"),
        gwas_trait: payload_opt_str(p, "gwas_traits").or_else(|| payload_opt_str(p, "gwas_trait")),
        gnomad_af: p["gnomad_af"].as_f64(),
        gnomad_lookup_status: payload_opt_str(p, "gnomad_lookup_status"),
        gnomad_source_mode: payload_opt_str(p, "gnomad_source_mode"),
        gnomad_release: payload_opt_str(p, "gnomad_release"),
        has_gnomad: p.get("has_gnomad").and_then(|v| v.as_bool()),
        significance_score: p["significance_score"].as_f64().unwrap_or(0.0) as f32,
        genotype: payload_opt_str(p, "genotype"),
        gene_confidence: payload_opt_str(p, "gene_confidence"),
        enrichment_version: payload_opt_str(p, "enrichment_version"),
        clinvar_significance: payload_opt_str(p, "clinvar_significance"),
        chromosome: payload_opt_str(p, "chromosome"),
        position: p.get("position").and_then(|v| v.as_i64()),
        gwas_associations: payload_opt_array(p, "gwas_associations"),
        gene_candidates: payload_opt_array(p, "gene_candidates"),
        sources_provenance: p.get("sources_provenance").cloned(),
        cross_refs: p.get("cross_refs").cloned(),
        trait_categories: p
            .get("trait_categories")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        consultation_modes: p
            .get("consultation_modes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        pack_refs: payload_opt_array(p, "pack_refs"),
        searchable_tags: p
            .get("searchable_tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        association_summary: p.get("association_summary").cloned(),
    }
}

pub async fn count_qdrant_points(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: Option<i64>,
) -> Result<u64, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/count",
        url.trim_end_matches('/'),
        collection.trim()
    );

    let mut body = serde_json::json!({ "exact": true });
    if let Some(id) = sample_id {
        body["filter"] = serde_json::json!({
            "must": [{
                "key": "sample_id",
                "match": { "value": id }
            }]
        });
    }

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant count failed: {}", e))?;
    if res.status() == 401 || res.status() == 403 {
        return Err(
            "401 Unauthorized: Qdrant requires a valid API key. Please check your credentials in settings."
                .to_string(),
        );
    }
    if !res.status().is_success() {
        return Err(format!("Qdrant count returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant count response: {}", e))?;

    Ok(val["result"]["count"].as_u64().unwrap_or(0))
}

// ---------------------------------------------------------------------------
// 6. Extended filtered search + recommend + payload indexes
// ---------------------------------------------------------------------------

fn build_extended_filter(
    sample_id: Option<i64>,
    trait_category: Option<&str>,
    evidence_tier: Option<&str>,
    has_direction: Option<bool>,
    min_data_quality: Option<f32>,
    min_wellness_actionability: Option<f32>,
) -> Option<serde_json::Value> {
    let mut must: Vec<serde_json::Value> = Vec::new();
    if let Some(id) = sample_id {
        must.push(serde_json::json!({
            "key": "sample_id",
            "match": { "value": id }
        }));
    }
    if let Some(category) = trait_category.filter(|c| !c.trim().is_empty()) {
        must.push(serde_json::json!({
            "key": "trait_category",
            "match": { "value": category.trim() }
        }));
    }
    if let Some(tier) = evidence_tier.filter(|t| !t.trim().is_empty()) {
        must.push(serde_json::json!({
            "key": "evidence_tier",
            "match": { "value": tier.trim() }
        }));
    }
    if let Some(true) = has_direction {
        must.push(serde_json::json!({
            "key": "has_direction",
            "match": { "value": true }
        }));
    }
    if let Some(min_dq) = min_data_quality {
        must.push(serde_json::json!({
            "key": "data_quality_score",
            "range": { "gte": min_dq }
        }));
    }
    if let Some(min_w) = min_wellness_actionability {
        must.push(serde_json::json!({
            "key": "wellness_actionability_score",
            "range": { "gte": min_w }
        }));
    }
    if must.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "must": must }))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn search_qdrant_filtered(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    vector: Vec<f32>,
    sample_id: Option<i64>,
    limit: u32,
    trait_category: Option<&str>,
    evidence_tier: Option<&str>,
    has_direction: Option<bool>,
    min_data_quality: Option<f32>,
    min_wellness_actionability: Option<f32>,
    vector_name: Option<&str>,
) -> Result<Vec<(QdrantHit, u64, serde_json::Value)>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/search",
        url.trim_end_matches('/'),
        collection
    );

    let filter = build_extended_filter(
        sample_id,
        trait_category,
        evidence_tier,
        has_direction,
        min_data_quality,
        min_wellness_actionability,
    );
    let mut body = if let Some(f) = filter {
        serde_json::json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true,
            "filter": f
        })
    } else {
        serde_json::json!({
            "vector": vector,
            "limit": limit,
            "with_payload": true
        })
    };
    apply_named_vector(&mut body, vector_name);

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant filtered search failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "Qdrant filtered search returned HTTP {}",
            res.status()
        ));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant search response: {}", e))?;

    let results = val["result"].as_array().cloned().unwrap_or_default();
    let mut hits = Vec::new();
    for item in results {
        let score = item["score"].as_f64().unwrap_or(0.0) as f32;
        let p = &item["payload"];
        let point_id = item["id"].as_u64().unwrap_or(0);
        hits.push((map_payload_to_hit(p, score), point_id, p.clone()));
    }
    Ok(hits)
}

pub async fn recommend_qdrant_points(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    positive_id: u64,
    limit: u32,
    filter: Option<serde_json::Value>,
    vector_name: Option<&str>,
) -> Result<Vec<(QdrantHit, u64, serde_json::Value)>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/recommend",
        url.trim_end_matches('/'),
        collection
    );

    let mut body = serde_json::json!({
        "positive": [positive_id],
        "limit": limit,
        "with_payload": true
    });
    if let Some(f) = filter {
        body["filter"] = f;
    }
    if let Some(name) = vector_name.filter(|n| !n.is_empty()) {
        body["using"] = serde_json::json!(name);
    }

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant recommend failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Qdrant recommend returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant recommend response: {}", e))?;

    let results = val["result"].as_array().cloned().unwrap_or_default();
    let mut hits = Vec::new();
    for item in results {
        let score = item["score"].as_f64().unwrap_or(0.0) as f32;
        let p = &item["payload"];
        let point_id = item["id"].as_u64().unwrap_or(0);
        hits.push((map_payload_to_hit(p, score), point_id, p.clone()));
    }
    Ok(hits)
}

pub async fn find_point_payload_by_rsid(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    rsid: &str,
) -> Result<Option<(serde_json::Value, u64)>, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );

    let body = serde_json::json!({
        "filter": {
            "must": [
                { "key": "sample_id", "match": { "value": sample_id } },
                { "key": "rsid", "match": { "value": rsid } }
            ]
        },
        "limit": 1,
        "with_payload": true,
        "with_vector": false
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant scroll by rsid failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Qdrant scroll returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant scroll response: {}", e))?;

    let points = val["result"]["points"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(item) = points.first() {
        let id = item["id"].as_u64().unwrap_or(0);
        let payload = item["payload"].clone();
        return Ok(Some((payload, id)));
    }
    Ok(None)
}

const PAYLOAD_INDEX_FIELDS: &[(&str, &str)] = &[
    ("sample_id", "integer"),
    ("rsid", "keyword"),
    ("gene_symbol", "keyword"),
    ("trait_name", "keyword"),
    ("trait_category", "keyword"),
    ("trait_ontology_id", "keyword"),
    ("primary_source", "keyword"),
    ("evidence_tier", "keyword"),
    ("association_type", "keyword"),
    ("has_gwas", "bool"),
    ("has_clinvar", "bool"),
    ("has_pgs", "bool"),
    ("has_gtex", "bool"),
    ("has_pharmgkb", "bool"),
    ("has_pubmed", "bool"),
    ("personal_genotype_matched", "bool"),
    ("personal_direction", "keyword"),
    ("has_direction", "bool"),
    ("p_value_mlog10_max", "float"),
    ("association_strength_score", "float"),
    ("clinical_actionability_score", "float"),
    ("wellness_actionability_score", "float"),
    ("data_quality_score", "float"),
    ("novelty_score", "float"),
    ("enrichment_version", "keyword"),
    ("stale", "bool"),
];

pub async fn ensure_payload_indexes(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
) -> Result<Vec<String>, String> {
    let client = qdrant_http();

    let mut created = Vec::new();
    for (field, schema) in PAYLOAD_INDEX_FIELDS {
        let endpoint = format!(
            "{}/collections/{}/index",
            url.trim_end_matches('/'),
            collection
        );
        let body = serde_json::json!({
            "field_name": field,
            "field_schema": schema
        });
        let mut req = client.put(&endpoint).json(&body);
        if let Some(key) = api_key {
            req = req.header("api-key", key);
        }
        match req.send().await {
            Ok(res) if res.status().is_success() || res.status() == 409 => {
                created.push(format!("{}:{}", field, schema));
            }
            Ok(res) => {
                created.push(format!("{}:skipped:HTTP{}", field, res.status()));
            }
            Err(e) => {
                created.push(format!("{}:error:{}", field, e));
            }
        }
    }
    Ok(created)
}

pub struct ScrollPayloadBatch {
    pub points: Vec<(u64, serde_json::Value)>,
    pub next_offset: Option<serde_json::Value>,
}

pub async fn scroll_qdrant_payload_batch(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    limit: u32,
    offset: Option<serde_json::Value>,
) -> Result<ScrollPayloadBatch, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );

    let mut body = serde_json::json!({
        "filter": {
            "must": [{
                "key": "sample_id",
                "match": { "value": sample_id }
            }]
        },
        "limit": limit,
        "with_payload": true,
        "with_vector": false
    });
    if let Some(off) = offset {
        body["offset"] = off;
    }

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant scroll failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Qdrant scroll returned HTTP {}", res.status()));
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant scroll response: {}", e))?;

    let points_raw = val["result"]["points"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut points = Vec::new();
    for item in points_raw {
        let id = item["id"]
            .as_u64()
            .or_else(|| item["id"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let payload = item["payload"].clone();
        points.push((id, payload));
    }

    Ok(ScrollPayloadBatch {
        points,
        next_offset: {
            let off = &val["result"]["next_page_offset"];
            if off.is_null() {
                None
            } else {
                Some(off.clone())
            }
        },
    })
}

pub async fn set_qdrant_payload_batch(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    point_ids: Vec<u64>,
    payload: serde_json::Value,
) -> Result<(), String> {
    if point_ids.is_empty() {
        return Ok(());
    }
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/payload",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "payload": payload,
        "points": point_ids,
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant set payload failed: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("Qdrant set payload returned HTTP {}", res.status()));
    }
    Ok(())
}

pub async fn count_qdrant_points_with_filter(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
    extra_must: serde_json::Value,
) -> Result<u64, String> {
    let client = qdrant_http();

    let endpoint = format!(
        "{}/collections/{}/points/count",
        url.trim_end_matches('/'),
        collection.trim()
    );

    let body = serde_json::json!({
        "exact": true,
        "filter": {
            "must": [
                {
                    "key": "sample_id",
                    "match": { "value": sample_id }
                },
                extra_must
            ]
        }
    });

    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }

    let res = req
        .send()
        .await
        .map_err(|e| format!("Qdrant count failed: {}", e))?;
    if !res.status().is_success() {
        return Ok(0);
    }

    let val: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Qdrant count response: {}", e))?;

    Ok(val["result"]["count"].as_u64().unwrap_or(0))
}

/// Read `embedding_model` from one indexed point (for config vs index mismatch checks).
pub async fn sample_index_embedding_model(
    url: &str,
    api_key: Option<&str>,
    collection: &str,
    sample_id: i64,
) -> Option<String> {
    let client = qdrant_http();
    let endpoint = format!(
        "{}/collections/{}/points/scroll",
        url.trim_end_matches('/'),
        collection
    );
    let body = serde_json::json!({
        "filter": {
            "must": [{ "key": "sample_id", "match": { "value": sample_id } }]
        },
        "limit": 1,
        "with_payload": true,
        "with_vector": false
    });
    let mut req = client.post(&endpoint).json(&body);
    if let Some(key) = api_key {
        req = req.header("api-key", key);
    }
    let res = req.send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    let val: serde_json::Value = res.json().await.ok()?;
    val["result"]["points"]
        .as_array()
        .and_then(|pts| pts.first())
        .and_then(|pt| pt["payload"]["embedding_model"].as_str())
        .map(String::from)
}

#[cfg(test)]
mod connection_tests {
    use super::{collection_name_matches, parse_collection_names};
    use serde_json::json;

    #[test]
    fn parses_collection_names_from_result_objects_and_strings() {
        let body = json!({
            "result": {
                "collections": [
                    { "name": "genomics_roy" },
                    "genomics_evidence"
                ]
            }
        });
        let names = parse_collection_names(&body);
        assert!(collection_name_matches(&names, "genomics_roy"));
        assert!(collection_name_matches(&names, "GENOMICS_EVIDENCE"));
    }

    #[test]
    fn parses_top_level_collections_array() {
        let body = json!({
            "collections": [{ "name": "genomics_roy" }]
        });
        let names = parse_collection_names(&body);
        assert_eq!(names, vec!["genomics_roy"]);
    }
}
