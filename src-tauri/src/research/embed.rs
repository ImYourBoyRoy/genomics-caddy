// ./src-tauri/src/research/embed.rs
use super::http::EMBED_CLIENT;
use super::tuning::embed_batch_timeout_secs;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

const QUERY_EMBED_CACHE_MAX: usize = 128;

static QUERY_EMBED_CACHE: LazyLock<Mutex<HashMap<String, Vec<f32>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn query_cache_key(text: &str, ollama_url: &str, model: &str) -> String {
    format!("{}|{}|{}", model, ollama_url.trim(), text)
}

fn read_query_cache(key: &str) -> Option<Vec<f32>> {
    QUERY_EMBED_CACHE.lock().ok()?.get(key).cloned()
}

fn write_query_cache(key: String, vector: Vec<f32>) {
    if let Ok(mut cache) = QUERY_EMBED_CACHE.lock() {
        if cache.len() >= QUERY_EMBED_CACHE_MAX
            && let Some(oldest) = cache.keys().next().cloned()
        {
            cache.remove(&oldest);
        }
        cache.insert(key, vector);
    }
}

fn normalize_ollama_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Deserialize)]
struct LegacyEmbeddingResponse {
    embedding: Vec<f32>,
}

async fn post_json_with_retries(
    client: &reqwest::Client,
    url: &str,
    payload: serde_json::Value,
    attempts: u32,
) -> Result<reqwest::Response, String> {
    let mut last_err = String::new();
    for attempt in 0..attempts {
        match client.post(url).json(&payload).send().await {
            Ok(res) => return Ok(res),
            Err(e) => {
                last_err = e.to_string();
                if attempt + 1 < attempts {
                    tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
                }
            }
        }
    }
    Err(format!(
        "Ollama embed connection failed after {attempts} attempts: {last_err}"
    ))
}

async fn embed_via_api_embed(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    input: serde_json::Value,
) -> Result<Vec<Vec<f32>>, String> {
    let url = format!("{}/api/embed", normalize_ollama_url(ollama_url));
    let payload = serde_json::json!({
        "model": model,
        "input": input,
    });

    let res = post_json_with_retries(client, &url, payload, 3).await?;

    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let resp: EmbedResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama embed response: {}", e))?;

    if resp.embeddings.is_empty() {
        return Err("Embedding model returned no vectors".to_string());
    }

    Ok(resp.embeddings)
}

async fn embed_via_legacy_api(
    client: &reqwest::Client,
    ollama_url: &str,
    model: &str,
    text: &str,
) -> Result<Vec<f32>, String> {
    let url = format!("{}/api/embeddings", normalize_ollama_url(ollama_url));
    let payload = serde_json::json!({
        "model": model,
        "prompt": text,
    });

    let res = post_json_with_retries(client, &url, payload, 3).await?;

    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let resp: LegacyEmbeddingResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama embedding response: {}", e))?;

    if resp.embedding.is_empty() {
        return Err("Embedding model returned an empty vector".to_string());
    }

    Ok(resp.embedding)
}

pub async fn embed_text(text: &str, ollama_url: &str, model: &str) -> Result<Vec<f32>, String> {
    let client = &*EMBED_CLIENT;

    match embed_via_api_embed(client, ollama_url, model, serde_json::Value::String(text.to_string()))
        .await
    {
        Ok(mut vectors) if !vectors.is_empty() => Ok(vectors.remove(0)),
        Ok(_) => embed_via_legacy_api(client, ollama_url, model, text).await,
        Err(_) => embed_via_legacy_api(client, ollama_url, model, text).await,
    }
}

/// Embed a user query with a small in-memory LRU-style cache (consultation + hybrid search).
pub async fn embed_query_cached(text: &str, ollama_url: &str, model: &str) -> Result<Vec<f32>, String> {
    let key = query_cache_key(text, ollama_url, model);
    if let Some(cached) = read_query_cache(&key) {
        return Ok(cached);
    }
    let vector = embed_text(text, ollama_url, model).await?;
    write_query_cache(key, vector.clone());
    Ok(vector)
}

async fn embed_texts_batch_inner(
    texts: &[String],
    ollama_url: &str,
    model: &str,
) -> Result<Vec<Vec<f32>>, String> {
    let client = &*EMBED_CLIENT;
    let input: Vec<&str> = texts.iter().map(String::as_str).collect();

    match embed_via_api_embed(
        client,
        ollama_url,
        model,
        serde_json::Value::Array(
            input
                .into_iter()
                .map(|s| serde_json::Value::String(s.to_string()))
                .collect(),
        ),
    )
    .await
    {
        Ok(vectors) if vectors.len() == texts.len() => Ok(vectors),
        Ok(vectors) if !vectors.is_empty() => Ok(vectors),
        _ => {
            let mut out = Vec::with_capacity(texts.len());
            for text in texts {
                out.push(embed_text(text, ollama_url, model).await?);
            }
            Ok(out)
        }
    }
}

pub async fn embed_texts_batch(
    texts: &[String],
    ollama_url: &str,
    model: &str,
) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(vec![]);
    }

    let deadline = Duration::from_secs(embed_batch_timeout_secs(texts.len()));
    match tokio::time::timeout(
        deadline,
        embed_texts_batch_inner(texts, ollama_url, model),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(format!(
            "Ollama embed timed out after {}s ({} texts)",
            deadline.as_secs(),
            texts.len()
        )),
    }
}
