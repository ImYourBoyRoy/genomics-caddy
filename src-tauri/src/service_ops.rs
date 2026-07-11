// ./src-tauri/src/service_ops.rs
/*
Purpose: Ollama / vector-DB service operations for Advanced → Connections.
Responsibilities:
  - Pull / delete / version Ollama models via HTTP API.
  - Read Qdrant service version; optional GitHub latest release compare.
  - Probe localhost for missing Ollama/Qdrant processes.
Key inputs: Service URLs, optional tokens/API keys, model names.
Key outputs: JSON status payloads + optional progress events (`ollama:pull_progress`).
*/

use crate::config;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const LOCAL_OLLAMA_URL: &str = "http://127.0.0.1:11434";
pub const LOCAL_QDRANT_URL: &str = "http://127.0.0.1:6333";

#[derive(Debug, Clone, Serialize)]
pub struct LocalhostServiceStatus {
    pub ollama_url: String,
    pub qdrant_url: String,
    pub ollama_reachable: bool,
    pub qdrant_reachable: bool,
    pub ollama_version: Option<String>,
    pub qdrant_version: Option<String>,
    pub ollama_error: Option<String>,
    pub qdrant_error: Option<String>,
    pub notes: Vec<String>,
}

fn auth_header(token: Option<&str>) -> Option<String> {
    let t = token.map(str::trim).filter(|s| !s.is_empty())?;
    Some(if t.to_ascii_lowercase().starts_with("bearer ") {
        t.to_string()
    } else {
        format!("Bearer {t}")
    })
}

pub async fn probe_localhost_services() -> LocalhostServiceStatus {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut notes = Vec::new();
    let (ollama_reachable, ollama_version, ollama_error) =
        match client.get(format!("{LOCAL_OLLAMA_URL}/api/version")).send().await {
            Ok(res) if res.status().is_success() => {
                let v = res
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|j| j.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()));
                (true, v, None)
            }
            Ok(res) => (
                false,
                None,
                Some(format!("Ollama localhost HTTP {}", res.status())),
            ),
            Err(e) => (false, None, Some(format!("Ollama not reachable on localhost: {e}"))),
        };

    let (qdrant_reachable, qdrant_version, qdrant_error) =
        match client.get(LOCAL_QDRANT_URL).send().await {
            Ok(res) if res.status().is_success() => {
                let v = res
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|j| j.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()));
                (true, v, None)
            }
            Ok(res) => (
                false,
                None,
                Some(format!("Qdrant localhost HTTP {}", res.status())),
            ),
            Err(e) => (false, None, Some(format!("Qdrant not reachable on localhost: {e}"))),
        };

    if !ollama_reachable {
        notes.push(
            "No Ollama on 127.0.0.1:11434. Install Ollama locally, or point Connections at a remote host (LAN / Tailscale)."
                .into(),
        );
    }
    if !qdrant_reachable {
        notes.push(
            "No Qdrant on 127.0.0.1:6333. Start Qdrant locally (Docker/binary), or use a remote vector DB URL in Connections."
                .into(),
        );
    }
    if ollama_reachable && qdrant_reachable {
        notes.push("Localhost Ollama and Qdrant both responded.".into());
    }

    LocalhostServiceStatus {
        ollama_url: LOCAL_OLLAMA_URL.into(),
        qdrant_url: LOCAL_QDRANT_URL.into(),
        ollama_reachable,
        qdrant_reachable,
        ollama_version,
        qdrant_version,
        ollama_error,
        qdrant_error,
        notes,
    }
}

pub async fn get_ollama_version(url: &str, token: Option<&str>) -> Result<serde_json::Value, String> {
    let clean = config::validate_service_url(url)?;
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{clean}/api/version"));
    if let Some(a) = auth_header(token) {
        req = req.header("Authorization", a);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Ollama /api/version HTTP {}", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn get_qdrant_version(url: &str, api_key: Option<&str>) -> Result<serde_json::Value, String> {
    let clean = config::validate_service_url(url)?;
    let client = reqwest::Client::new();
    let mut req = client.get(clean.trim_end_matches('/').to_string());
    if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
        req = req.header("api-key", key);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Qdrant root HTTP {}", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceUpdateCheck {
    pub service: String,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: Option<bool>,
    pub notes: Vec<String>,
}

/// Parse `1.2.3` / `v1.2.3` / `1.2.3-rc0` into comparable (major, minor, patch).
fn parse_semver_triple(raw: &str) -> Option<(u64, u64, u64)> {
    let s = raw.trim().trim_start_matches('v');
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let patch = parts.next().unwrap_or("0").parse().unwrap_or(0);
    Some((major, minor, patch))
}

/// True when `latest` is strictly newer than `installed` (semver core). Falls back to string ≠.
fn is_update_available(installed: &str, latest: &str) -> bool {
    match (parse_semver_triple(installed), parse_semver_triple(latest)) {
        (Some(a), Some(b)) => a < b,
        _ => {
            installed.trim().trim_start_matches('v') != latest.trim().trim_start_matches('v')
        }
    }
}

async fn fetch_github_latest_tag(repo: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("GenomicsCaddy/0.2")
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("GitHub {repo} releases HTTP {}", res.status()));
    }
    let body: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    body.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("GitHub {repo} latest release missing tag_name"))
}

pub async fn check_ollama_update(url: &str, token: Option<&str>) -> Result<ServiceUpdateCheck, String> {
    let ver = get_ollama_version(url, token).await?;
    let installed = ver
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().trim_start_matches('v').to_string());

    let mut notes = vec![format!(
        "Connected Ollama reports version {}.",
        installed.clone().unwrap_or_else(|| "unknown".into())
    )];
    let mut latest = None;
    let mut update_available = None;

    match fetch_github_latest_tag("ollama/ollama").await {
        Ok(tag) => {
            latest = Some(tag.clone());
            if let Some(inst) = installed.as_ref() {
                let available = is_update_available(inst, &tag);
                update_available = Some(available);
                if available {
                    notes.push(format!(
                        "GitHub latest release is v{tag} (installed {inst}). Upgrade the Ollama package/binary on the host — there is no in-API self-update."
                    ));
                } else {
                    notes.push(format!(
                        "Installed Ollama matches or is newer than GitHub latest (v{tag})."
                    ));
                }
            } else {
                notes.push(format!("GitHub latest Ollama release is v{tag}."));
            }
        }
        Err(e) => notes.push(format!(
            "Could not query GitHub ollama/ollama releases: {e}"
        )),
    }
    notes.push(
        "To refresh an installed model to the newest registry tag, use Update (re-pull) on that model."
            .into(),
    );

    Ok(ServiceUpdateCheck {
        service: "ollama".into(),
        installed_version: installed,
        latest_version: latest,
        update_available,
        notes,
    })
}

pub async fn check_qdrant_update(url: &str, api_key: Option<&str>) -> Result<ServiceUpdateCheck, String> {
    let ver = get_qdrant_version(url, api_key).await?;
    let installed = ver
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().trim_start_matches('v').to_string());

    let mut notes = vec![format!(
        "Connected Qdrant reports version {}.",
        installed.clone().unwrap_or_else(|| "unknown".into())
    )];
    let mut latest = None;
    let mut update_available = None;

    match fetch_github_latest_tag("qdrant/qdrant").await {
        Ok(tag) => {
            latest = Some(tag.clone());
            if let Some(inst) = installed.as_ref() {
                let available = is_update_available(inst, &tag);
                update_available = Some(available);
                if available {
                    notes.push(format!(
                        "GitHub latest release is v{tag} (your server is {inst}). Upgrade Qdrant on the host — there is no in-API self-update."
                    ));
                } else {
                    notes.push("Installed Qdrant matches or is newer than the latest GitHub release tag.".into());
                }
            }
        }
        Err(e) => notes.push(format!("Could not query GitHub qdrant/qdrant releases: {e}")),
    }

    Ok(ServiceUpdateCheck {
        service: "qdrant".into(),
        installed_version: installed,
        latest_version: latest,
        update_available,
        notes,
    })
}

pub async fn pull_ollama_model(
    app: &AppHandle,
    url: &str,
    token: Option<&str>,
    name: &str,
) -> Result<serde_json::Value, String> {
    let clean = config::validate_service_url(url)?;
    let model = name.trim();
    if model.is_empty() {
        return Err("Model name is required".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.post(format!("{clean}/api/pull")).json(&serde_json::json!({
        "model": model,
        "stream": true
    }));
    if let Some(a) = auth_header(token) {
        req = req.header("Authorization", a);
    }
    let mut res = req.send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Ollama /api/pull HTTP {}", res.status()));
    }

    let mut last_status = serde_json::json!({ "status": "started", "model": model });
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(line) {
                if v.get("model").is_none() {
                    v["model"] = serde_json::json!(model);
                }
                let _ = app.emit("ollama:pull_progress", &v);
                last_status = v;
            }
        }
    }
    Ok(last_status)
}

pub async fn delete_ollama_model(
    url: &str,
    token: Option<&str>,
    name: &str,
) -> Result<(), String> {
    let clean = config::validate_service_url(url)?;
    let model = name.trim();
    if model.is_empty() {
        return Err("Model name is required".into());
    }
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "model": model });

    // Prefer DELETE; fall back to POST for older/newer Ollama variants.
    let mut del = client
        .delete(format!("{clean}/api/delete"))
        .json(&body);
    if let Some(a) = auth_header(token) {
        del = del.header("Authorization", a.clone());
    }
    let res = del.send().await.map_err(|e| e.to_string())?;
    if res.status().is_success() {
        return Ok(());
    }
    let status = res.status();
    let mut post = client.post(format!("{clean}/api/delete")).json(&body);
    if let Some(a) = auth_header(token) {
        post = post.header("Authorization", a);
    }
    let res2 = post.send().await.map_err(|e| e.to_string())?;
    if res2.status().is_success() {
        return Ok(());
    }
    Err(format!(
        "Ollama delete failed (DELETE {status}, POST {})",
        res2.status()
    ))
}

/// Lightweight health probe + capability report for vector backends.
pub async fn probe_vector_provider(
    provider: &str,
    url: &str,
    api_key: Option<&str>,
) -> Result<serde_json::Value, String> {
    let clean = config::validate_service_url(url)?;
    let provider_kind = crate::research::vector_store::VectorProvider::parse(provider);
    let cfg = crate::research::QdrantConfig {
        url: clean,
        api_key: api_key.map(|s| s.to_string()),
        collection: "genomics_evidence".into(),
        embedding_model: "mxbai-embed-large".into(),
        gwas_strict: true,
        ncbi_api_key: None,
        auto_start: false,
        named_vectors_enabled: false,
        vector_provider: provider_kind.as_str().into(),
        namespace: String::new(),
    };
    let status = crate::research::vector_store::test_connection(&cfg).await;
    let caps = crate::research::vector_store::capabilities_json(provider_kind);
    Ok(serde_json::json!({
        "provider": provider_kind.as_str(),
        "reachable": status.success,
        "collection_exists": status.collection_exists,
        "vectors_count": status.vectors_count,
        "collections": status.collections,
        "research_supported": provider_kind.supports_dense_research(),
        "capabilities": caps,
        "error": status.error,
        "note": if status.success {
            format!(
                "{} reachable. Dense research: {}. Named vectors: {}.",
                provider_kind.as_str(),
                provider_kind.supports_dense_research(),
                provider_kind.supports_named_vectors()
            )
        } else {
            status
                .error
                .clone()
                .unwrap_or_else(|| format!("{} unreachable", provider_kind.as_str()))
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::{is_update_available, parse_semver_triple};

    #[test]
    fn semver_parse_and_compare() {
        assert_eq!(parse_semver_triple("0.31.1"), Some((0, 31, 1)));
        assert_eq!(parse_semver_triple("v0.31.2"), Some((0, 31, 2)));
        assert_eq!(parse_semver_triple("0.31.2-rc0"), Some((0, 31, 2)));
        assert!(is_update_available("0.31.1", "0.31.2"));
        assert!(!is_update_available("0.31.2", "0.31.2"));
        assert!(!is_update_available("0.32.0", "0.31.2"));
    }
}
