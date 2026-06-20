// ./src-tauri/src/research/evidence/backfill.rs
//! Normalize existing Qdrant payloads without re-embedding when structured text is unchanged.

use super::normalize::extend_payload_normalized;
use super::types::EVIDENCE_SCHEMA_VERSION;
use crate::research::crossmap::CrossMapContext;
use crate::research::qdrant::{scroll_qdrant_payload_batch, set_qdrant_payload_batch};
use crate::research::types::QdrantConfig;
use crate::research::util::ENRICHMENT_VERSION;
use serde_json::{Map, Value};
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackfillResult {
    pub scanned: u64,
    pub updated_payload: u64,
    pub skipped_current: u64,
    pub needs_reembed: u64,
    pub errors: u64,
}

pub async fn backfill_evidence_payloads(
    db_path: &Path,
    sample_id: i64,
    config: &QdrantConfig,
    limit: u32,
) -> Result<BackfillResult, String> {
    let mut result = BackfillResult {
        scanned: 0,
        updated_payload: 0,
        skipped_current: 0,
        needs_reembed: 0,
        errors: 0,
    };

    let mut offset: Option<Value> = None;
    let batch_size = 64u32.min(limit.max(1));

    while result.scanned < limit as u64 {
        let batch = scroll_qdrant_payload_batch(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            sample_id,
            batch_size,
            offset.take(),
        )
        .await?;

        if batch.points.is_empty() {
            break;
        }
        offset = batch.next_offset;

        for (point_id, payload) in batch.points {
            result.scanned += 1;
            if result.scanned > limit as u64 {
                break;
            }

            let schema_ok = payload
                .get("schema_version")
                .and_then(|v| v.as_str())
                == Some(EVIDENCE_SCHEMA_VERSION);
            let stale = payload.get("stale").and_then(|v| v.as_bool()).unwrap_or(true);
            if schema_ok && !stale {
                result.skipped_current += 1;
                continue;
            }

            let rsid = payload
                .get("rsid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if rsid.is_empty() {
                result.errors += 1;
                continue;
            }

            let genotype = payload
                .get("genotype")
                .and_then(|v| v.as_str())
                .unwrap_or("--")
                .to_string();
            let gene = payload
                .get("gene_symbol")
                .and_then(|v| v.as_str())
                .map(String::from);
            let gene_confidence_s = payload
                .get("gene_confidence")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let evidence_tier_s = payload
                .get("evidence_tier")
                .and_then(|v| v.as_str())
                .unwrap_or("literature_context")
                .to_string();
            let clinvar_sig = payload
                .get("clinvar_significance")
                .and_then(|v| v.as_str());
            let gnomad_af = payload.get("gnomad_af").and_then(|v| v.as_f64());
            let gwas_associations = payload
                .get("gwas_associations")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let sources_provenance = payload
                .get("sources_provenance")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let crossmap = {
                let mut cm = CrossMapContext::default();
                if let Some(cats) = payload
                    .get("trait_categories")
                    .and_then(|v| v.as_array())
                {
                    cm.trait_categories = cats
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
                cm.association_summary = payload
                    .get("association_summary")
                    .cloned()
                    .unwrap_or(Value::Object(Map::new()));
                cm
            };
            let chromosome = payload
                .get("chromosome")
                .and_then(|v| v.as_str());
            let position = payload.get("position_grch38").and_then(|v| v.as_i64());
            let existing_text = payload
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut payload_map = payload.as_object().cloned().unwrap_or_default();
            extend_payload_normalized(
                &mut payload_map,
                sample_id,
                &rsid,
                &genotype,
                &gene,
                &gene_confidence_s,
                &evidence_tier_s,
                clinvar_sig,
                gnomad_af,
                &gwas_associations,
                &crossmap,
                chromosome,
                position,
                &sources_provenance,
                &config.embedding_model,
                &existing_text,
            );

            let new_text = payload_map
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if new_text != existing_text {
                payload_map.insert("stale".into(), Value::Bool(true));
                payload_map.insert(
                    "backfill_note".into(),
                    Value::String("structured text changed — re-embed via force re-enrich".into()),
                );
                result.needs_reembed += 1;
            }

            match set_qdrant_payload_batch(
                &config.url,
                config.api_key.as_deref(),
                &config.collection,
                vec![point_id],
                Value::Object(payload_map.clone()),
            )
            .await
            {
                Ok(()) => result.updated_payload += 1,
                Err(_) => result.errors += 1,
            }

            let _ = crate::db::connect(db_path).and_then(|conn| {
                conn.execute(
                    "UPDATE association_facts SET updated_at = ? WHERE sample_id = ? AND rsid = ?",
                    rusqlite::params![crate::research::util::unix_now(), sample_id, rsid],
                )
            });
        }

        if offset.is_none() || result.scanned >= limit as u64 {
            break;
        }
    }

    let _ = ENRICHMENT_VERSION;
    Ok(result)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReembedResult {
    pub scanned: u64,
    pub reembedded: u64,
    pub skipped_fresh: u64,
    pub errors: u64,
}

pub async fn reembed_stale_vectors(
    _db_path: &Path,
    sample_id: i64,
    config: &QdrantConfig,
    ollama_url: &str,
    limit: u32,
) -> Result<ReembedResult, String> {
    use crate::research::embed::embed_text;
    use crate::research::evidence::named_vectors;
    use crate::research::qdrant::{scroll_qdrant_payload_batch, upsert_points_batch_named, upsert_to_qdrant};

    let mut result = ReembedResult {
        scanned: 0,
        reembedded: 0,
        skipped_fresh: 0,
        errors: 0,
    };

    let mut offset: Option<Value> = None;
    let batch_size = 32u32.min(limit.max(1));

    while result.scanned < limit as u64 {
        let batch = scroll_qdrant_payload_batch(
            &config.url,
            config.api_key.as_deref(),
            &config.collection,
            sample_id,
            batch_size,
            offset.take(),
        )
        .await?;

        if batch.points.is_empty() {
            break;
        }
        offset = batch.next_offset;

        for (point_id, payload) in batch.points {
            result.scanned += 1;
            if result.scanned > limit as u64 {
                break;
            }

            let stale = payload.get("stale").and_then(|v| v.as_bool()).unwrap_or(false);
            let schema_ok = payload
                .get("schema_version")
                .and_then(|v| v.as_str())
                == Some(EVIDENCE_SCHEMA_VERSION);
            if !stale && schema_ok {
                result.skipped_fresh += 1;
                continue;
            }

            let text = payload
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                result.errors += 1;
                continue;
            }

            let vector = match embed_text(&text, ollama_url, &config.embedding_model).await {
                Ok(v) => v,
                Err(_) => {
                    result.errors += 1;
                    continue;
                }
            };

            let mut payload_map = payload.as_object().cloned().unwrap_or_default();
            payload_map.insert("stale".into(), Value::Bool(false));
            payload_map.insert(
                "enrichment_version".into(),
                Value::String(ENRICHMENT_VERSION.to_string()),
            );
            payload_map.insert(
                "schema_version".into(),
                Value::String(EVIDENCE_SCHEMA_VERSION.to_string()),
            );
            let payload_val = Value::Object(payload_map.clone());

            let upsert_ok = if named_vectors::named_vectors_enabled(config) {
                let texts = named_vectors::build_named_vector_texts(&payload_map, &text);
                match named_vectors::embed_named_vectors(&texts, ollama_url, &config.embedding_model)
                    .await
                {
                    Ok(mut named) => {
                        named.insert(String::new(), vector.clone());
                        upsert_points_batch_named(
                            &config.url,
                            config.api_key.as_deref(),
                            &config.collection,
                            vec![(point_id, named, payload_val)],
                        )
                        .await
                        .is_ok()
                    }
                    Err(_) => false,
                }
            } else {
                upsert_to_qdrant(
                    &config.url,
                    config.api_key.as_deref(),
                    &config.collection,
                    &point_id.to_string(),
                    vector,
                    payload_val,
                )
                .await
                .is_ok()
            };

            if upsert_ok {
                result.reembedded += 1;
            } else {
                result.errors += 1;
            }
        }

        if offset.is_none() || result.scanned >= limit as u64 {
            break;
        }
    }

    Ok(result)
}
