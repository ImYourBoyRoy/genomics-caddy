// ./src-tauri/src/research/evidence/named_vectors.rs
//! Named Qdrant vectors: trait_dense, gene_mechanism_dense, evidence_dense, actionability_dense.

use super::normalize::build_structured_embedding_text;
use crate::research::embed::embed_texts_batch;
use crate::research::types::QdrantConfig;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub const NAMED_VECTOR_DEFAULT: &str = "";
pub const NAMED_TRAIT: &str = "trait_dense";
pub const NAMED_GENE: &str = "gene_mechanism_dense";
pub const NAMED_EVIDENCE: &str = "evidence_dense";
pub const NAMED_ACTIONABILITY: &str = "actionability_dense";

pub fn named_vectors_enabled(config: &QdrantConfig) -> bool {
    config.named_vectors_enabled
}

pub fn build_named_vector_texts(payload: &Map<String, Value>, full_text: &str) -> HashMap<&'static str, String> {
    let rsid = payload.get("rsid").and_then(|v| v.as_str()).unwrap_or("unknown");
    let genotype = payload.get("genotype").and_then(|v| v.as_str()).unwrap_or("unknown");
    let chromosome = payload.get("chromosome").and_then(|v| v.as_str());
    let position = payload.get("position_grch38").and_then(|v| v.as_i64());
    let trait_name = payload
        .get("trait_name")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("trait_name_mapped").and_then(|v| v.as_str()));
    let trait_category = payload.get("trait_category").and_then(|v| v.as_str());
    let gene = payload
        .get("gene_symbol")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("mapped_gene_symbol").and_then(|v| v.as_str()));
    let gene_confidence = payload.get("gene_confidence").and_then(|v| v.as_str()).unwrap_or("unknown");
    let evidence_tier = payload.get("evidence_tier").and_then(|v| v.as_str()).unwrap_or("unknown");
    let association_type = payload
        .get("association_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let p_value = payload.get("p_value_min").and_then(|v| v.as_f64());
    let personal_direction = payload
        .get("personal_direction")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let dq = payload.get("data_quality_score").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
    let wellness = payload
        .get("wellness_actionability_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    let source_names: Vec<String> = payload
        .get("source_names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let quality_flags: Vec<String> = payload
        .get("quality_flags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let missing_fields: Vec<String> = payload
        .get("missing_fields")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let pathways = payload
        .get("pathway_names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let trait_text = format!(
        "[TRAIT]\nreported_trait={}\nmapped_trait={}\ntrait_category={}\nontology_id={}\n",
        trait_name.unwrap_or("unknown"),
        payload
            .get("trait_name_mapped")
            .and_then(|v| v.as_str())
            .unwrap_or(trait_name.unwrap_or("unknown")),
        trait_category.unwrap_or("unknown"),
        payload
            .get("trait_ontology_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
    );

    let gene_text = format!(
        "[GENE_MECHANISM]\nmapped_gene={}\ngene_confidence={}\npathways={}\n",
        gene.unwrap_or("unknown"),
        gene_confidence,
        if pathways.is_empty() { "unknown" } else { &pathways },
    );

    let evidence_text = build_structured_embedding_text(
        rsid,
        genotype,
        chromosome,
        position,
        trait_name,
        trait_category,
        gene,
        gene_confidence,
        evidence_tier,
        association_type,
        p_value,
        personal_direction,
        dq,
        wellness,
        &source_names,
        &quality_flags,
        &missing_fields,
    );

    let actionability_text = format!(
        "[ACTIONABILITY]\nwellness={:.3}\nclinical={:.3}\npersonal_direction={}\nverification={}\n",
        wellness,
        payload
            .get("clinical_actionability_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        personal_direction,
        payload
            .get("verification_ideas")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default(),
    );

    let mut map = HashMap::new();
    map.insert(NAMED_TRAIT, trait_text);
    map.insert(NAMED_GENE, gene_text);
    map.insert(NAMED_EVIDENCE, evidence_text);
    map.insert(NAMED_ACTIONABILITY, actionability_text);
    map.insert(NAMED_VECTOR_DEFAULT, full_text.to_string());
    map
}

pub async fn embed_named_vectors(
    texts: &HashMap<&'static str, String>,
    ollama_url: &str,
    model: &str,
) -> Result<HashMap<String, Vec<f32>>, String> {
    let order = [
        NAMED_VECTOR_DEFAULT,
        NAMED_TRAIT,
        NAMED_GENE,
        NAMED_EVIDENCE,
        NAMED_ACTIONABILITY,
    ];
    let batch: Vec<String> = order
        .iter()
        .filter_map(|k| texts.get(k).cloned())
        .collect();
    let keys: Vec<&str> = order
        .iter()
        .copied()
        .filter(|k| texts.contains_key(*k))
        .collect();
    let vectors = embed_texts_batch(&batch, ollama_url, model).await?;
    let mut out = HashMap::new();
    for (i, key) in keys.iter().enumerate() {
        if let Some(v) = vectors.get(i) {
            let name = if *key == NAMED_VECTOR_DEFAULT {
                "".to_string()
            } else {
                (*key).to_string()
            };
            out.insert(name, v.clone());
        }
    }
    Ok(out)
}

pub fn extend_payload_named_vector_meta(payload: &mut Map<String, Value>, enabled: bool) {
    payload.insert("named_vectors_enabled".into(), json!(enabled));
    if enabled {
        payload.insert(
            "named_vector_names".into(),
            json!([NAMED_TRAIT, NAMED_GENE, NAMED_EVIDENCE, NAMED_ACTIONABILITY]),
        );
    }
}
