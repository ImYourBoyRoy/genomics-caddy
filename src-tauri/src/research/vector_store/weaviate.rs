// ./src-tauri/src/research/vector_store/weaviate.rs
/*
Purpose: Weaviate dense vector adapter (objects batch + GraphQL nearVector).
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

fn class_name(config: &QdrantConfig) -> String {
    let raw = config.collection.trim();
    if raw.is_empty() {
        "GenomicsEvidence".into()
    } else {
        // Weaviate class names are typically PascalCase; keep user value if already valid-ish.
        let mut chars = raw.chars();
        match chars.next() {
            Some(c) if c.is_ascii_uppercase() => raw.to_string(),
            Some(c) => format!("{}{}", c.to_ascii_uppercase(), chars.as_str()),
            None => "GenomicsEvidence".into(),
        }
    }
}

fn auth(mut req: reqwest::RequestBuilder, api_key: Option<&str>) -> reqwest::RequestBuilder {
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    req
}

pub async fn test_connection(config: &QdrantConfig) -> QdrantConnectionStatus {
    let endpoint = format!("{}/v1/meta", base(config));
    let req = auth(http().get(&endpoint), config.api_key.as_deref());
    match req.send().await {
        Err(e) => QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some(format!("Weaviate connection failed: {e}")),
        },
        Ok(res) if res.status().is_success() => {
            let class = class_name(config);
            let exists = class_exists(config, &class).await;
            QdrantConnectionStatus {
                success: true,
                vectors_count: None,
                collection_exists: exists,
                collections: Some(vec![class]),
                error: None,
            }
        }
        Ok(res) => QdrantConnectionStatus {
            success: false,
            vectors_count: None,
            collection_exists: false,
            collections: None,
            error: Some(format!("Weaviate HTTP {}", res.status())),
        },
    }
}

async fn class_exists(config: &QdrantConfig, class: &str) -> bool {
    let endpoint = format!("{}/v1/schema/{class}", base(config));
    auth(http().get(&endpoint), config.api_key.as_deref())
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn ensure_class(config: &QdrantConfig, dims: u32) -> Result<(), String> {
    let class = class_name(config);
    if class_exists(config, &class).await {
        return Ok(());
    }
    let endpoint = format!("{}/v1/schema", base(config));
    let body = serde_json::json!({
        "class": class,
        "vectorizer": "none",
        "vectorIndexConfig": { "distance": "cosine" },
        "properties": [
            { "name": "rsid", "dataType": ["text"] },
            { "name": "text", "dataType": ["text"] },
            { "name": "sample_id", "dataType": ["int"] },
            { "name": "enrichment_version", "dataType": ["text"] },
            { "name": "gene_symbol", "dataType": ["text"] },
            { "name": "sources_provenance", "dataType": ["text"] },
            { "name": "payload_json", "dataType": ["text"] },
        ],
        "moduleConfig": {
            "note": format!("dims={dims}")
        }
    });
    let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
        .send()
        .await
        .map_err(|e| format!("Weaviate schema create failed: {e}"))?;
    if res.status().is_success() || res.status().as_u16() == 422 {
        // 422 often means class already exists
        return Ok(());
    }
    Err(format!("Weaviate schema create HTTP {}", res.status()))
}

pub async fn purge_class(config: &QdrantConfig) -> Result<(), String> {
    let class = class_name(config);
    let endpoint = format!("{}/v1/schema/{class}", base(config));
    let res = auth(http().delete(&endpoint), config.api_key.as_deref())
        .send()
        .await
        .map_err(|e| format!("Weaviate purge failed: {e}"))?;
    if res.status().is_success() || res.status().as_u16() == 404 {
        return Ok(());
    }
    Err(format!("Weaviate purge HTTP {}", res.status()))
}

fn uuid_from_u64(id: u64) -> String {
    // Deterministic UUID-ish string Weaviate accepts as id (hex padded).
    format!("{:032x}", id)
}

pub async fn upsert_batch(
    config: &QdrantConfig,
    points: Vec<(u64, Vec<f32>, serde_json::Value)>,
) -> Result<(), String> {
    let class = class_name(config);
    let objects: Vec<serde_json::Value> = points
        .into_iter()
        .map(|(id, vector, payload)| {
            let flat = flatten_metadata(&payload);
            let text = payload
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rsid = payload
                .get("rsid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let sample_id = payload.get("sample_id").and_then(|v| v.as_i64());
            let enrichment_version = payload
                .get("enrichment_version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let gene = payload
                .get("gene_symbol")
                .or_else(|| payload.get("gene"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let provenance = payload
                .get("sources_provenance")
                .map(|v| v.to_string())
                .unwrap_or_default();
            serde_json::json!({
                "class": class,
                "id": uuid_from_u64(id),
                "properties": {
                    "rsid": rsid,
                    "text": text,
                    "sample_id": sample_id,
                    "enrichment_version": enrichment_version,
                    "gene_symbol": gene,
                    "sources_provenance": provenance,
                    "payload_json": serde_json::Value::Object(flat).to_string(),
                },
                "vector": vector,
            })
        })
        .collect();

    let endpoint = format!("{}/v1/batch/objects", base(config));
    for chunk in objects.chunks(64) {
        let body = serde_json::json!({ "objects": chunk });
        let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Weaviate batch upsert failed: {e}"))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("Weaviate upsert failed: {text}"));
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
    let class = class_name(config);
    let mut where_parts = Vec::new();
    if let Some(sid) = sample_id {
        where_parts.push(format!(
            "{{ path: [\"sample_id\"], operator: Equal, valueInt: {sid} }}"
        ));
    }
    if let Some(cat) = trait_category.map(str::trim).filter(|c| !c.is_empty()) {
        // Stored in payload_json string — soft filter after fetch if needed.
        let _ = cat;
    }
    let where_clause = if where_parts.is_empty() {
        String::new()
    } else if where_parts.len() == 1 {
        format!(", where: {}", where_parts[0])
    } else {
        format!(
            ", where: {{ operator: And, operands: [{}] }}",
            where_parts.join(",")
        )
    };
    let vec_lit = vector
        .iter()
        .map(|f| format!("{f}"))
        .collect::<Vec<_>>()
        .join(",");
    let gql = format!(
        "{{ Get {{ {class}(nearVector: {{ vector: [{vec_lit}] }}, limit: {limit}{where_clause}) {{ rsid text sample_id enrichment_version gene_symbol sources_provenance payload_json _additional {{ distance certainty }} }} }} }}"
    );
    let endpoint = format!("{}/v1/graphql", base(config));
    let body = serde_json::json!({ "query": gql });
    let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
        .send()
        .await
        .map_err(|e| format!("Weaviate GraphQL failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Weaviate GraphQL HTTP {}", res.status()));
    }
    let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    if let Some(errs) = val.get("errors").and_then(|v| v.as_array()) {
        if !errs.is_empty() {
            return Err(format!("Weaviate GraphQL errors: {errs:?}"));
        }
    }
    let items = val
        .pointer(&format!("/data/Get/{class}"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut hits = Vec::new();
    for item in items {
        let score = item
            .pointer("/_additional/certainty")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                item.pointer("/_additional/distance")
                    .and_then(|v| v.as_f64())
                    .map(|d| 1.0 - d)
            })
            .unwrap_or(0.0) as f32;
        let mut meta = if let Some(s) = item.get("payload_json").and_then(|v| v.as_str()) {
            serde_json::from_str(s).unwrap_or_else(|_| item.clone())
        } else {
            item.clone()
        };
        if let Some(obj) = meta.as_object_mut() {
            for key in ["rsid", "text", "enrichment_version", "gene_symbol"] {
                if obj.get(key).is_none() {
                    if let Some(v) = item.get(key) {
                        obj.insert(key.into(), v.clone());
                    }
                }
            }
            if let Some(sid) = item.get("sample_id") {
                obj.insert("sample_id".into(), sid.clone());
            }
        }
        if let Some(cat) = trait_category.map(str::trim).filter(|c| !c.is_empty()) {
            let cats = meta
                .get("trait_categories")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let ok = cats.iter().any(|v| v.as_str() == Some(cat))
                || meta
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|t| t.contains(cat))
                    .unwrap_or(false);
            if !ok {
                continue;
            }
        }
        hits.push(hit_from_metadata(&meta, score));
    }
    Ok(hits)
}

pub async fn fetch_payloads(
    config: &QdrantConfig,
    ids: &[u64],
) -> Result<HashMap<u64, serde_json::Value>, String> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let class = class_name(config);
    for id in ids {
        let uuid = uuid_from_u64(*id);
        let endpoint = format!("{}/v1/objects/{class}/{uuid}", base(config));
        let res = auth(http().get(&endpoint), config.api_key.as_deref())
            .send()
            .await
            .map_err(|e| format!("Weaviate get failed: {e}"))?;
        if !res.status().is_success() {
            continue;
        }
        let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        let props = val.get("properties").cloned().unwrap_or_default();
        let mut meta = if let Some(s) = props.get("payload_json").and_then(|v| v.as_str()) {
            serde_json::from_str(s).unwrap_or(props.clone())
        } else {
            props.clone()
        };
        if let Some(obj) = meta.as_object_mut() {
            if let Some(v) = props.get("enrichment_version") {
                obj.insert("enrichment_version".into(), v.clone());
            }
            if let Some(s) = props.get("sources_provenance").and_then(|v| v.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                    obj.insert("sources_provenance".into(), parsed);
                }
            }
        }
        out.insert(*id, meta);
    }
    Ok(out)
}

/// GraphQL list with offset pagination.
pub async fn browse_page(
    config: &QdrantConfig,
    sample_id: i64,
    limit: u32,
    offset: Option<&serde_json::Value>,
) -> Result<(Vec<QdrantHit>, Option<serde_json::Value>, Option<u64>), String> {
    let class = class_name(config);
    let lim = limit.clamp(1, 200);
    let skip = offset.and_then(|v| v.as_u64()).unwrap_or(0);
    let gql = format!(
        "{{ Get {{ {class}(where: {{ path: [\"sample_id\"], operator: Equal, valueInt: {sample_id} }}, limit: {lim}, offset: {skip}) {{ rsid text sample_id enrichment_version gene_symbol sources_provenance payload_json }} }} }}"
    );
    let endpoint = format!("{}/v1/graphql", base(config));
    let body = serde_json::json!({ "query": gql });
    let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
        .send()
        .await
        .map_err(|e| format!("Weaviate browse failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Weaviate browse HTTP {}", res.status()));
    }
    let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let items = val
        .pointer(&format!("/data/Get/{class}"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut points = Vec::new();
    for item in &items {
        let mut meta = if let Some(s) = item.get("payload_json").and_then(|v| v.as_str()) {
            serde_json::from_str(s).unwrap_or_else(|_| item.clone())
        } else {
            item.clone()
        };
        if let Some(obj) = meta.as_object_mut() {
            for key in ["rsid", "text", "enrichment_version", "gene_symbol"] {
                if obj.get(key).is_none() {
                    if let Some(v) = item.get(key) {
                        obj.insert(key.into(), v.clone());
                    }
                }
            }
        }
        points.push(hit_from_metadata(&meta, 1.0));
    }
    let next = if items.len() as u32 >= lim {
        Some(serde_json::json!(skip + u64::from(lim)))
    } else {
        None
    };
    Ok((points, next, None))
}

pub async fn sample_vectors(
    config: &QdrantConfig,
    sample_id: i64,
    limit: usize,
) -> Result<Vec<crate::research::qdrant::ScrollVectorItem>, String> {
    let class = class_name(config);
    let lim = limit.min(2500);
    let gql = format!(
        "{{ Get {{ {class}(where: {{ path: [\"sample_id\"], operator: Equal, valueInt: {sample_id} }}, limit: {lim}) {{ rsid gene_symbol payload_json _additional {{ id vector }} }} }} }}"
    );
    let endpoint = format!("{}/v1/graphql", base(config));
    let body = serde_json::json!({ "query": gql });
    let res = auth(http().post(&endpoint).json(&body), config.api_key.as_deref())
        .send()
        .await
        .map_err(|e| format!("Weaviate sample failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Weaviate sample HTTP {}", res.status()));
    }
    let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let items = val
        .pointer(&format!("/data/Get/{class}"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for item in items {
        let vector = item
            .pointer("/_additional/vector")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if vector.is_empty() {
            continue;
        }
        let meta = if let Some(s) = item.get("payload_json").and_then(|v| v.as_str()) {
            serde_json::from_str::<serde_json::Value>(s).unwrap_or_default()
        } else {
            serde_json::json!({})
        };
        out.push(crate::research::qdrant::ScrollVectorItem {
            point_id: item
                .pointer("/_additional/id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            rsid: item
                .get("rsid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            vector,
            trait_category: meta
                .get("trait_category")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            gene_symbol: item
                .get("gene_symbol")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            data_quality_score: meta
                .get("data_quality_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
        });
    }
    Ok(out)
}
