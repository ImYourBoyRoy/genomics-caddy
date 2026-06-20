// ./src-tauri/src/agent_commands.rs
/*
Module: agent_commands.rs
Purpose: Tauri command handlers for variant evidence and safety audit workflows.
Responsibilities:
- Fetch per-variant evidence for the research agent UI.
- Export discovered findings and run deterministic safety checks.
Key Inputs: AppHandle, sample IDs, rsIDs, report text.
Key Outputs: VariantEvidence and SafetyCheckResult payloads.
*/

use crate::agent::{
    append_discovered_findings_to_db, append_discovered_findings_to_path,
    get_db_discovered_findings as load_discovered_findings, get_variant_evidence_core,
    run_deterministic_safety_checks, SafetyCheckResult, VariantEvidence,
};
use crate::config;
use crate::{db, db_runtime, get_db_path};
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
pub async fn get_variant_evidence(
    app: AppHandle,
    sample_id: i64,
    rsid: String,
    ncbi_api_key: Option<String>,
    custom_export_path: Option<String>,
) -> Result<VariantEvidence, String> {
    let db_path = get_db_path(&app);
    let app_data = crate::get_data_dir(&app);

    let effective_key = if ncbi_api_key.as_ref().is_some_and(|k| !k.trim().is_empty()) {
        ncbi_api_key
    } else {
        db_runtime::with_connection(db_path.clone(), |conn| {
            Ok(config::load_qdrant_config(conn)
                .ok()
                .and_then(|c| c.ncbi_api_key))
        })
        .await
        .ok()
        .flatten()
    };

    let evidence = get_variant_evidence_core(
        &db_path,
        sample_id,
        &rsid,
        effective_key.as_deref(),
        &app_data,
    )
    .await?;

    if crate::agent::should_persist_discovery(&evidence.interpretation_status) {
        let db_path_persist = db_path.clone();
        let evidence_clone = evidence.clone();
        if let Err(e) = tauri::async_runtime::spawn_blocking(move || {
            append_discovered_findings_to_db(&db_path_persist, sample_id, vec![evidence_clone])
        })
        .await
        {
            eprintln!("Failed to persist discovered finding to DB: {e}");
        }

        if let Some(path) = custom_export_path.filter(|p| !p.trim().is_empty()) {
            config::validate_export_path(&path)?;
            let evidence_export = evidence.clone();
            let path_clone = path.clone();
            if let Err(e) = tauri::async_runtime::spawn_blocking(move || {
                append_discovered_findings_to_path(Path::new(&path_clone), vec![evidence_export])
            })
            .await
            {
                eprintln!("Failed to export discovered finding: {e}");
            }
        }
    }

    Ok(evidence)
}

#[tauri::command]
pub async fn get_discovered_findings_summary(
    app: AppHandle,
    sample_id: i64,
) -> Result<Vec<db::DiscoveredFindingSummary>, String> {
    let db_path = get_db_path(&app);
    db_runtime::with_connection(db_path, move |conn| {
        db::get_discovered_findings_summary(conn, sample_id)
    })
    .await
}

#[tauri::command]
pub async fn get_db_discovered_findings(
    app: AppHandle,
    sample_id: i64,
) -> Result<Vec<VariantEvidence>, String> {
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || load_discovered_findings(&db_path, sample_id))
        .await
        .map_err(|e| format!("Findings load worker failed: {}", e))?
}

#[tauri::command]
pub fn run_safety_audit(
    report_text: String,
    variant_evidence_list: Vec<VariantEvidence>,
) -> SafetyCheckResult {
    run_deterministic_safety_checks(&report_text, &variant_evidence_list)
}

#[tauri::command]
pub async fn export_discovered_findings(
    app: AppHandle,
    sample_id: i64,
    export_path: String,
) -> Result<usize, String> {
    config::validate_export_path(&export_path)?;
    let db_path = get_db_path(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let findings = load_discovered_findings(&db_path, sample_id)?;
        append_discovered_findings_to_path(Path::new(&export_path), findings.clone())?;
        Ok(findings.len())
    })
    .await
    .map_err(|e| format!("Export worker failed: {}", e))?
}

#[tauri::command]
pub async fn chat_ollama(
    url: String,
    token: Option<String>,
    model: String,
    messages: Vec<serde_json::Value>,
    temperature: Option<f64>,
) -> Result<String, String> {
    let clean_url = crate::config::validate_service_url(&url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut req = client.post(format!("{}/api/chat", clean_url));

    if let Some(t) = token
        && !t.trim().is_empty() {
            let t_val = t.trim();
            req = req.header(
                "Authorization",
                if t_val.to_lowercase().starts_with("bearer ") {
                    t_val.to_string()
                } else {
                    format!("Bearer {}", t_val)
                },
            );
        }

    let payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "options": {
            "temperature": temperature.unwrap_or(0.0)
        }
    });

    let res = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Connection error: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Ollama returned HTTP error: {}", res.status()));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    body["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Ollama response did not include message content".to_string())
}
