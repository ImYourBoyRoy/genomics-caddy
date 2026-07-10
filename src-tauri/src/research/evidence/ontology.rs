// ./src-tauri/src/research/evidence/ontology.rs
//! OLS4 / EFO / MONDO trait normalization.

use super::cache::{fetch_json_cached, ols4_ttl};
use super::source_records;
use serde::{Deserialize, Serialize};
use std::path::Path;

const OLS4_SEARCH: &str = "https://www.ebi.ac.uk/ols4/api/search";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitOntologyMapping {
    pub reported_trait: String,
    pub mapped_label: String,
    pub ontology_id: String,
    pub ontology_prefix: String,
    pub confidence: f32,
    pub synonyms: Vec<String>,
}

pub async fn map_trait_via_ols4(db_path: &Path, trait_name: &str) -> Option<TraitOntologyMapping> {
    let query = trait_name.trim();
    if query.is_empty() || query.len() < 3 {
        return None;
    }

    let url = format!(
        "{}?q={}&ontology=efo,mondo&size=5&exact=false",
        OLS4_SEARCH,
        url_encode(query)
    );
    let cache_key = format!("ols4|efo_mondo|{}", query.to_lowercase());

    let body = fetch_json_cached(
        db_path,
        "ols4",
        "search",
        &cache_key,
        &url,
        ols4_ttl(),
        None,
    )
    .await
    .ok()?;

    if let Ok(conn) = crate::db::connect(db_path) {
        let _ = source_records::record_generic_json(
            &conn,
            "ols4",
            "search",
            None,
            None,
            Some(query),
            Some(&url),
            &body,
        );
    }

    let docs = body["response"]["docs"].as_array()?;
    let best = docs.first()?;
    let label = best["label"].as_str()?.to_string();
    let ontology_id = best["obo_id"]
        .as_str()
        .or_else(|| best["iri"].as_str())
        .unwrap_or("")
        .to_string();
    if ontology_id.is_empty() {
        return None;
    }
    let prefix = ontology_id.split(':').next().unwrap_or("EFO").to_string();
    let score = best["score"].as_f64().unwrap_or(1.0) as f32;
    let confidence = (score / 10.0).clamp(0.35, 0.98);

    let synonyms: Vec<String> = best["synonym"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .take(5)
                .collect()
        })
        .unwrap_or_default();

    Some(TraitOntologyMapping {
        reported_trait: query.to_string(),
        mapped_label: label,
        ontology_id,
        ontology_prefix: prefix,
        confidence,
        synonyms,
    })
}

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
